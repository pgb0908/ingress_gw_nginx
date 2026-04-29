use crate::models::RevisionBundle;
use crate::paths;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub struct NginxManager {
    binary: PathBuf,
}

impl NginxManager {
    pub fn new() -> Self {
        let binary = paths::project_nginx_bin();
        Self { binary }
    }

    pub fn render(&self, bundle: &RevisionBundle) -> Result<PathBuf> {
        paths::ensure_runtime_layout()?;
        let revision_dir = paths::generated_dir().join(&bundle.manifest.revision);
        fs::create_dir_all(&revision_dir)?;
        let conf_path = revision_dir.join("nginx.conf");
        fs::write(&conf_path, self.build_conf(bundle)?)?;
        Ok(conf_path)
    }

    pub fn validate(&self, conf_path: &Path) -> Result<String> {
        let output = Command::new(&self.binary)
            .arg("-p")
            .arg(paths::nginx_runtime_dir())
            .arg("-c")
            .arg(conf_path)
            .arg("-t")
            .output()
            .with_context(|| format!("failed to execute {}", self.binary.display()))?;

        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if !output.status.success() {
            anyhow::bail!(combined.trim().to_string());
        }
        Ok(combined)
    }

    pub fn activate(&self, conf_path: &Path) -> Result<String> {
        let active_conf = paths::nginx_conf_dir().join("nginx.conf");
        fs::create_dir_all(paths::nginx_conf_dir())?;
        fs::copy(conf_path, &active_conf)?;

        let pid_file = paths::nginx_runtime_dir().join("logs/nginx.pid");
        let pid_ready = pid_file.exists()
            && fs::read_to_string(&pid_file)
                .map(|text| {
                    text.trim().chars().all(|v| v.is_ascii_digit()) && !text.trim().is_empty()
                })
                .unwrap_or(false);

        let mut command = Command::new(&self.binary);
        command
            .arg("-p")
            .arg(paths::nginx_runtime_dir())
            .arg("-c")
            .arg(&active_conf);
        if pid_ready {
            command.arg("-s").arg("reload");
        }
        let output = command
            .output()
            .with_context(|| format!("failed to execute {}", self.binary.display()))?;
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if !output.status.success() {
            anyhow::bail!(combined.trim().to_string());
        }
        Ok(if combined.trim().is_empty() {
            "ok".to_string()
        } else {
            combined
        })
    }

    pub fn build_conf(&self, bundle: &RevisionBundle) -> Result<String> {
        let mime_types_path = paths::nginx_conf_dir().join("mime.types");
        let access_log_path = paths::log_dir().join("access.log");
        let error_log_path = paths::log_dir().join("error.log");
        let bootstrap_error_log = paths::log_dir().join("bootstrap-error.log");
        let plugins_dir = bundle.root.join("plugins");

        let server_names = if bundle.listener.spec.allowed_hostnames.is_empty() {
            "_".to_string()
        } else {
            bundle.listener.spec.allowed_hostnames.join(" ")
        };

        let plugin_chain = bundle.plugin_chain.join(",");

        // Load data configs for wasm filter configuration
        let auth_config = load_json_or_empty(&bundle.root.join("data/secrets.json"));
        let rate_limit_config = load_json_or_empty(&bundle.root.join("data/rate-limit.json"));
        let header_config = format!(
            r#"{{"revision":"{}","plugin_chain":"{}"}}"#,
            bundle.manifest.revision, plugin_chain
        );

        // Escape single quotes for nginx config embedding
        let auth_config_escaped = auth_config.replace('\'', "\\'");
        let rate_limit_config_escaped = rate_limit_config.replace('\'', "\\'");
        let header_config_escaped = header_config.replace('\'', "\\'");

        let upstream_blocks = bundle
            .services
            .values()
            .map(|service| {
                let mut lines = vec![format!("upstream svc_{} {{", service.metadata.name)];
                for target in &service.spec.load_balancing.targets {
                    lines.push(format!(
                        "    server {}:{} weight={};",
                        target.host, target.port, target.weight
                    ));
                }
                lines.push("}".to_string());
                lines.join("\n")
            })
            .collect::<Vec<_>>()
            .join("\n\n    ");

        let mut locations = Vec::new();
        for router in &bundle.routers {
            let destination = &router.spec.config.destinations[0];
            let service = bundle
                .services
                .get(&destination.destination_ref.name)
                .expect("service must exist");
            let methods =
                if router.spec.rules.iter().flat_map(|r| r.methods.iter()).count() == 0 {
                    "GET POST".to_string()
                } else {
                    router
                        .spec
                        .rules
                        .iter()
                        .flat_map(|r| r.methods.iter().cloned())
                        .collect::<Vec<_>>()
                        .join(" ")
                };
            for (index, rule) in router.spec.rules.iter().enumerate() {
                let route_id = format!("{}-{}", router.metadata.name, index);
                locations.push(format!(
                    r#"location ~ {path} {{
            set $gateway_route_seed "{route_id}";
            set $gateway_service_seed "{service_name}";
            limit_except {methods} {{ deny all; }}
            proxy_wasm tenant_filter;
            proxy_wasm auth_filter '{auth_cfg}';
            proxy_wasm header_filter '{header_cfg}';
            proxy_wasm rate_limit_filter '{rl_cfg}';
            proxy_wasm observe_filter;
            proxy_set_header X-Route-Id $gateway_route_seed;
            proxy_set_header X-Service-Id $gateway_service_seed;
            proxy_pass http://svc_{service_name};
        }}"#,
                    path = rule.path,
                    route_id = route_id,
                    service_name = service.metadata.name,
                    methods = methods,
                    auth_cfg = auth_config_escaped,
                    header_cfg = header_config_escaped,
                    rl_cfg = rate_limit_config_escaped,
                ));
            }
        }

        Ok(format!(
            r#"worker_processes  1;
pid logs/nginx.pid;
error_log {bootstrap_error_log} info;

wasm {{
    module tenant_filter {plugins_dir}/tenant-filter.wasm;
    module auth_filter {plugins_dir}/auth-filter.wasm;
    module header_filter {plugins_dir}/header-filter.wasm;
    module rate_limit_filter {plugins_dir}/rate-limit-filter.wasm;
    module observe_filter {plugins_dir}/observe-filter.wasm;
}}

events {{
    worker_connections  1024;
}}

http {{
    include       {mime_types_path};
    default_type  application/octet-stream;
    client_body_temp_path client_body_temp;
    proxy_temp_path proxy_temp;

    log_format gateway_json escape=json
      '{{"timestamp":"$time_iso8601","remote_addr":"$remote_addr","method":"$request_method","path":"$request_uri","status":$status,"request_time":"$request_time","upstream_time":"$upstream_response_time"}}';
    access_log {access_log_path} gateway_json;
    error_log {error_log_path} info;

    {upstream_blocks}

    server {{
        listen {listen_port};
        server_name {server_names};

        location = {metrics_path} {{
            proxy_pass http://127.0.0.1:19080/metrics;
        }}

        location = /__gateway_status {{
            proxy_pass http://127.0.0.1:19080/status;
        }}

        {locations}
    }}
}}
"#,
            bootstrap_error_log = bootstrap_error_log.display(),
            plugins_dir = plugins_dir.display(),
            mime_types_path = mime_types_path.display(),
            access_log_path = access_log_path.display(),
            error_log_path = error_log_path.display(),
            upstream_blocks = upstream_blocks,
            listen_port = bundle.listener.spec.port,
            server_names = server_names,
            metrics_path = bundle.gateway.spec.metrics.path,
            locations = locations.join("\n\n        ")
        ))
    }
}

fn load_json_or_empty(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|_| "{}".to_string())
}
