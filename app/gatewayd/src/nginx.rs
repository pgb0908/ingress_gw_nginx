use crate::models::{PathMatchType, RevisionBundle};
use anyhow::{Context, Result};
use std::{env, fs};
use std::path::{Path, PathBuf};
use std::process::Command;

// ── Path helpers ──────────────────────────────────────────────────────────────

fn gateway_root() -> PathBuf {
    if let Ok(val) = env::var("GATEWAY_ROOT") {
        return PathBuf::from(val)
            .canonicalize()
            .expect("GATEWAY_ROOT must point to an existing directory");
    }
    let exe = env::current_exe().expect("cannot determine current executable path");
    exe.parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .and_then(|p| p.canonicalize().ok())
        .expect("cannot resolve gateway root; set GATEWAY_ROOT")
}

pub(crate) fn runtime_dir() -> PathBuf     { gateway_root().join("runtime/dataplane") }
pub fn live_dir() -> PathBuf               { runtime_dir().join("live") }
pub fn nginx_runtime_dir() -> PathBuf      { runtime_dir().join("nginx") }
pub fn nginx_conf_dir() -> PathBuf         { nginx_runtime_dir().join("conf") }
pub fn generated_dir() -> PathBuf          { nginx_runtime_dir().join("generated") }
pub fn log_dir() -> PathBuf                { runtime_dir().join("logs") }
pub fn state_file() -> PathBuf             { runtime_dir().join("state.json") }

pub fn project_nginx_bin() -> PathBuf {
    if let Ok(val) = env::var("GATEWAY_NGINX_BIN") {
        return PathBuf::from(val);
    }
    gateway_root().join("bin/nginx")
}

const MIME_TYPES: &str = include_str!("mime.types");

pub fn ensure_runtime_layout() -> Result<()> {
    for path in [
        runtime_dir(),
        nginx_runtime_dir(),
        nginx_conf_dir(),
        generated_dir(),
        log_dir(),
        nginx_runtime_dir().join("logs"),
        nginx_runtime_dir().join("client_body_temp"),
        nginx_runtime_dir().join("proxy_temp"),
    ] {
        fs::create_dir_all(&path).with_context(|| format!("failed to create {}", path.display()))?;
    }
    let mime_types_path = nginx_conf_dir().join("mime.types");
    if !mime_types_path.exists() {
        fs::write(&mime_types_path, MIME_TYPES)
            .with_context(|| format!("failed to write {}", mime_types_path.display()))?;
    }
    Ok(())
}

// ── NginxManager ─────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct NginxManager {
    binary: PathBuf,
}

impl NginxManager {
    pub fn new() -> Self {
        Self { binary: project_nginx_bin() }
    }

    pub fn render(&self, bundle: &RevisionBundle) -> Result<PathBuf> {
        ensure_runtime_layout()?;
        let revision_dir = generated_dir().join(&bundle.manifest.revision);
        fs::create_dir_all(&revision_dir)?;
        let conf_path = revision_dir.join("nginx.conf");
        fs::write(&conf_path, self.build_conf(bundle)?)?;
        Ok(conf_path)
    }

    pub fn validate(&self, conf_path: &Path) -> Result<String> {
        let output = Command::new(&self.binary)
            .arg("-p").arg(nginx_runtime_dir())
            .arg("-c").arg(conf_path)
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
        let active_conf = nginx_conf_dir().join("nginx.conf");
        fs::create_dir_all(nginx_conf_dir())?;
        fs::copy(conf_path, &active_conf)?;

        let pid_file = nginx_runtime_dir().join("logs/nginx.pid");
        let pid_ready = pid_file.exists()
            && fs::read_to_string(&pid_file)
                .map(|text| text.trim().chars().all(|v| v.is_ascii_digit()) && !text.trim().is_empty())
                .unwrap_or(false);

        let mut command = Command::new(&self.binary);
        command
            .arg("-p").arg(nginx_runtime_dir())
            .arg("-c").arg(&active_conf);
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
        Ok(if combined.trim().is_empty() { "ok".to_string() } else { combined })
    }

    pub fn build_conf(&self, bundle: &RevisionBundle) -> Result<String> {
        let mime_types_path = nginx_conf_dir().join("mime.types");
        let access_log_path = log_dir().join("access.log");
        let error_log_path = log_dir().join("error.log");
        let bootstrap_error_log = log_dir().join("bootstrap-error.log");
        let plugins_dir = bundle.root.join("plugins");

        let server_names = if bundle.listener.spec.allowed_hostnames.is_empty() {
            "_".to_string()
        } else {
            bundle.listener.spec.allowed_hostnames.join(" ")
        };

        let plugin_chain_str = bundle.plugin_chain.join(",");

        let auth_config = load_json_or_empty(&bundle.root.join("data/secrets.json"));
        let rate_limit_config = load_json_or_empty(&bundle.root.join("data/rate-limit.json"));
        let header_config = format!(
            r#"{{"revision":"{}","plugin_chain":"{}"}}"#,
            bundle.manifest.revision, plugin_chain_str
        );

        let auth_config_escaped = auth_config.replace('\'', "\\'");
        let rate_limit_config_escaped = rate_limit_config.replace('\'', "\\'");
        let header_config_escaped = header_config.replace('\'', "\\'");

        let filter_module_name = |name: &str| name.replace('-', "_");
        let filter_config = |name: &str| -> Option<String> {
            match name {
                "auth-filter"        => Some(format!("'{auth_config_escaped}'")),
                "header-filter"      => Some(format!("'{header_config_escaped}'")),
                "rate-limit-filter"  => Some(format!("'{rate_limit_config_escaped}'")),
                _                    => None,
            }
        };

        let wasm_block = if bundle.plugin_chain.is_empty() {
            String::new()
        } else {
            let module_lines = bundle.plugin_chain.iter()
                .map(|name| format!(
                    "    module {} {}/{}.wasm;",
                    filter_module_name(name), plugins_dir.display(), name
                ))
                .collect::<Vec<_>>()
                .join("\n");
            format!("wasm {{\n{module_lines}\n}}\n\n")
        };

        let proxy_wasm_directives = bundle.plugin_chain.iter()
            .map(|name| match filter_config(name) {
                Some(cfg) => format!("            proxy_wasm {} {cfg};", filter_module_name(name)),
                None      => format!("            proxy_wasm {};", filter_module_name(name)),
            })
            .collect::<Vec<_>>()
            .join("\n");

        let keepalive = bundle.gateway.spec.server.keepalive_connections;
        let upstream_blocks = bundle.services.values()
            .map(|service| {
                let mut lines = vec![format!("upstream svc_{} {{", service.metadata.name)];
                for target in &service.spec.load_balancing.targets {
                    lines.push(format!(
                        "    server {}:{} weight={};",
                        target.host, target.port, target.weight
                    ));
                }
                lines.push(format!("    keepalive {keepalive};"));
                lines.push("}".to_string());
                lines.join("\n")
            })
            .collect::<Vec<_>>()
            .join("\n\n    ");

        let mut ordered_routers: Vec<_> = bundle.routers.iter().collect();
        ordered_routers.sort_by_key(|r| {
            let type_order = match r.spec.match_type {
                PathMatchType::Exact  => 0u8,
                PathMatchType::Prefix => 1,
                PathMatchType::Regex  => 2,
            };
            (type_order, std::cmp::Reverse(r.spec.priority))
        });

        let mut locations = Vec::new();
        for router in ordered_routers {
            let destination = &router.spec.config.destinations[0];
            let service = bundle.services
                .get(&destination.destination_ref.name)
                .expect("service must exist");

            let location_keyword = match router.spec.match_type {
                PathMatchType::Exact  => "location =",
                PathMatchType::Prefix => "location ^~",
                PathMatchType::Regex  => "location ~",
            };

            for (index, rule) in router.spec.rules.iter().enumerate() {
                let route_id = format!("{}-{}", router.metadata.name, index);
                let methods = rule.methods.join(" ");
                let wasm_lines = if proxy_wasm_directives.is_empty() {
                    String::new()
                } else {
                    format!("\n{proxy_wasm_directives}")
                };
                locations.push(format!(
                    r#"{location_keyword} {path} {{
            set $gateway_route_seed "{route_id}";
            set $gateway_service_seed "{service_name}";
            limit_except {methods} {{ deny all; }}{wasm_lines}
            proxy_set_header X-Route-Id $gateway_route_seed;
            proxy_set_header X-Service-Id $gateway_service_seed;
            proxy_pass http://svc_{service_name};
        }}"#,
                    location_keyword = location_keyword,
                    path = rule.path,
                    route_id = route_id,
                    service_name = service.metadata.name,
                    methods = methods,
                    wasm_lines = wasm_lines,
                ));
            }
        }

        let worker_processes = &bundle.gateway.spec.server.worker_processes;
        let worker_connections = bundle.gateway.spec.server.worker_connections;

        Ok(format!(
            r#"worker_processes  {worker_processes};
pid logs/nginx.pid;
error_log {bootstrap_error_log} info;

{wasm_block}

events {{
    worker_connections  {worker_connections};
}}

http {{
    include       {mime_types_path};
    default_type  application/octet-stream;
    client_body_temp_path    client_body_temp;
    proxy_temp_path          proxy_temp;
    client_body_buffer_size  128k;

    proxy_buffer_size        128k;
    proxy_buffers            4 128k;
    proxy_busy_buffers_size  256k;
    proxy_http_version       1.1;

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
            wasm_block = wasm_block,
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
