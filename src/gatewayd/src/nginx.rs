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
        fs::write(&conf_path, self.build_conf(bundle))?;
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
        let pid_ready = pid_file
            .exists()
            && fs::read_to_string(&pid_file)
                .map(|text| text.trim().chars().all(|value| value.is_ascii_digit()) && !text.trim().is_empty())
                .unwrap_or(false);

        let mut command = Command::new(&self.binary);
        command.arg("-p").arg(paths::nginx_runtime_dir()).arg("-c").arg(&active_conf);
        if pid_ready {
            command.arg("-s").arg("reload");
        }
        let output = command.output().with_context(|| format!("failed to execute {}", self.binary.display()))?;
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

    fn build_conf(&self, bundle: &RevisionBundle) -> String {
        let access_log_path = paths::log_dir().join("access.log");
        let error_log_path = paths::log_dir().join("error.log");
        let bootstrap_error_log = paths::log_dir().join("bootstrap-error.log");
        let server_names = if bundle.listener.spec.allowed_hostnames.is_empty() {
            "_".to_string()
        } else {
            bundle.listener.spec.allowed_hostnames.join(" ")
        };
        let plugin_chain = bundle.plugin_chain.join(",");
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
            let service = bundle.services.get(&destination.destination_ref.name).expect("service must exist");
            let methods = if router.spec.rules.iter().flat_map(|rule| rule.methods.iter()).count() == 0 {
                "GET POST".to_string()
            } else {
                router
                    .spec
                    .rules
                    .iter()
                    .flat_map(|rule| rule.methods.iter().cloned())
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
            auth_request /__plugins_preflight;
            auth_request_set $gateway_request_id $upstream_http_x_request_id;
            auth_request_set $gateway_trace_id $upstream_http_x_trace_id;
            auth_request_set $gateway_tenant_id $upstream_http_x_tenant_id;
            auth_request_set $gateway_route_id $upstream_http_x_route_id;
            auth_request_set $gateway_service_id $upstream_http_x_service_id;
            auth_request_set $gateway_revision $upstream_http_x_gateway_revision;
            auth_request_set $gateway_plugin_chain $upstream_http_x_gateway_plugin_chain;
            auth_request_set $gateway_decision $upstream_http_x_gateway_decision;
            auth_request_set $gateway_decision_reason $upstream_http_x_gateway_decision_reason;
            auth_request_set $gateway_policy_profile $upstream_http_x_gateway_policy_profile;
            auth_request_set $gateway_plugin_version $upstream_http_x_gateway_plugin_version;
            proxy_set_header X-Request-Id $gateway_request_id;
            proxy_set_header X-Trace-Id $gateway_trace_id;
            proxy_set_header X-Tenant-Id $gateway_tenant_id;
            proxy_set_header X-Route-Id $gateway_route_id;
            proxy_set_header X-Service-Id $gateway_service_id;
            proxy_set_header X-Gateway-Revision $gateway_revision;
            proxy_set_header X-Gateway-Plugin-Chain $gateway_plugin_chain;
            proxy_set_header X-Gateway-Decision $gateway_decision;
            proxy_set_header X-Gateway-Decision-Reason $gateway_decision_reason;
            proxy_set_header X-Gateway-Policy-Profile $gateway_policy_profile;
            proxy_set_header X-Gateway-Plugin-Version $gateway_plugin_version;
            proxy_pass http://svc_{service_name};
        }}"#,
                    path = rule.path,
                    route_id = route_id,
                    service_name = service.metadata.name,
                    methods = methods
                ));
            }
        }

        format!(
            r#"worker_processes  1;
pid logs/nginx.pid;
error_log {bootstrap_error_log} info;

events {{
    worker_connections  1024;
}}

http {{
    include       /etc/nginx/mime.types;
    default_type  application/octet-stream;
    client_body_temp_path client_body_temp;
    proxy_temp_path proxy_temp;

    log_format gateway_json escape=json
      '{{"timestamp":"$time_iso8601","remote_addr":"$remote_addr","method":"$request_method","path":"$request_uri","status":$status,"request_time":"$request_time","upstream_time":"$upstream_response_time","request_id":"$gateway_request_id","trace_id":"$gateway_trace_id","tenant_id":"$gateway_tenant_id","route_id":"$gateway_route_id","service_id":"$gateway_service_id","revision":"$gateway_revision","plugin_chain":"$gateway_plugin_chain","decision":"$gateway_decision","decision_reason":"$gateway_decision_reason"}}';
    access_log {access_log_path} gateway_json;
    error_log {error_log_path} info;

    {upstream_blocks}

    server {{
        listen {listen_port};
        server_name {server_names};

        location = /__plugins_preflight {{
            internal;
            proxy_pass http://127.0.0.1:19080/plugin-check;
            proxy_set_header X-Gateway-Revision {revision};
            proxy_set_header X-Gateway-Plugin-Chain "{plugin_chain}";
            proxy_set_header X-Original-Method $request_method;
            proxy_set_header X-Original-Uri $request_uri;
            proxy_set_header X-Original-Host $host;
            proxy_set_header X-Request-Id $http_x_request_id;
            proxy_set_header X-Trace-Id $http_x_trace_id;
            proxy_set_header X-Tenant-Id $http_x_tenant_id;
            proxy_set_header X-Api-Key $http_x_api_key;
            proxy_set_header X-Route-Id $gateway_route_seed;
            proxy_set_header X-Service-Id $gateway_service_seed;
        }}

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
            access_log_path = access_log_path.display(),
            error_log_path = error_log_path.display(),
            upstream_blocks = upstream_blocks,
            listen_port = bundle.listener.spec.port,
            server_names = server_names,
            revision = bundle.manifest.revision,
            plugin_chain = plugin_chain,
            metrics_path = bundle.gateway.spec.metrics.path,
            locations = locations.join("\n\n        ")
        )
    }
}

