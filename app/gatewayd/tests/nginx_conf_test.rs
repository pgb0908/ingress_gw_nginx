use gatewayd::models::*;
use gatewayd::nginx::NginxManager;
use std::collections::HashMap;
use std::sync::Mutex;
use tempfile::TempDir;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn lock() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn make_bundle(root: &std::path::Path) -> RevisionBundle {
    let mut services = HashMap::new();
    services.insert(
        "api-svc".to_string(),
        ServiceDocument {
            metadata: Metadata { name: "api-svc".to_string() },
            spec: ServiceSpec {
                protocol: "HTTP".to_string(),
                load_balancing: LoadBalancing {
                    targets: vec![UpstreamTarget {
                        host: "127.0.0.1".to_string(),
                        port: 9000,
                        weight: 100,
                    }],
                },
            },
        },
    );

    RevisionBundle {
        root: root.to_path_buf(),
        manifest: RevisionManifest {
            revision: "test-rev-001".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            runtime_compat: "test".to_string(),
            plugins: vec![],
        },
        gateway: GatewayDocument {
            metadata: Metadata { name: "gw".to_string() },
            spec: GatewaySpec::default(),
        },
        listener: ListenerDocument {
            metadata: Metadata { name: "lst".to_string() },
            spec: ListenerSpec {
                protocol: "HTTP".to_string(),
                port: 8080,
                host: "0.0.0.0".to_string(),
                allowed_hostnames: vec!["example.com".to_string()],
            },
        },
        routers: vec![RouterDocument {
            metadata: Metadata { name: "api-route".to_string() },
            spec: RouterSpec {
                priority: 100,
                match_type: PathMatchType::Prefix,
                target_ref: TargetRef { kind: "Listener".to_string(), name: "lst".to_string() },
                rules: vec![RouterRule {
                    path: "/api".to_string(),
                    methods: vec!["GET".to_string(), "POST".to_string()],
                }],
                config: RouterConfig {
                    destinations: vec![DestinationConfig {
                        destination_ref: TargetRef {
                            kind: "Service".to_string(),
                            name: "api-svc".to_string(),
                        },
                        weight: 100,
                    }],
                },
            },
        }],
        services,
        policies: vec![],
        plugin_chain: vec![
            "tenant-filter".to_string(),
            "auth-filter".to_string(),
            "header-filter".to_string(),
        ],
    }
}

#[test]
fn build_conf_wasm_block_declares_plugin_chain_filters() {
    let _g = lock();
    let dir = TempDir::new().unwrap();
    unsafe { std::env::set_var("GATEWAY_ROOT", dir.path()); }

    let conf = NginxManager::new().build_conf(&make_bundle(dir.path())).unwrap();
    unsafe { std::env::remove_var("GATEWAY_ROOT"); }

    assert!(conf.contains("wasm {"), "missing wasm block");
    assert!(conf.contains("module tenant_filter"), "missing tenant_filter module");
    assert!(conf.contains("module auth_filter"), "missing auth_filter module");
    assert!(!conf.contains("module rate_limit_filter"), "unexpected rate_limit_filter module");
}

#[test]
fn build_conf_upstream_block_matches_service_targets() {
    let _g = lock();
    let dir = TempDir::new().unwrap();
    unsafe { std::env::set_var("GATEWAY_ROOT", dir.path()); }

    let conf = NginxManager::new().build_conf(&make_bundle(dir.path())).unwrap();
    unsafe { std::env::remove_var("GATEWAY_ROOT"); }

    assert!(conf.contains("upstream svc_api-svc"), "missing upstream block");
    assert!(conf.contains("127.0.0.1:9000 weight=100"), "missing upstream server directive");
    assert!(conf.contains("keepalive 32"), "missing keepalive directive");
    assert!(conf.contains("keepalive_requests 10000"), "missing upstream keepalive_requests");
    assert!(conf.contains("keepalive_timeout  60s"), "missing upstream keepalive_timeout");
    assert!(conf.contains("keepalive_time     1h"), "missing upstream keepalive_time");
}

#[test]
fn build_conf_performance_directives_present() {
    let _g = lock();
    let dir = TempDir::new().unwrap();
    unsafe { std::env::set_var("GATEWAY_ROOT", dir.path()); }

    let conf = NginxManager::new().build_conf(&make_bundle(dir.path())).unwrap();
    unsafe { std::env::remove_var("GATEWAY_ROOT"); }

    assert!(conf.contains("client_body_buffer_size  128k"), "missing client_body_buffer_size");
    assert!(conf.contains("proxy_buffer_size        128k"), "missing proxy_buffer_size");
    assert!(conf.contains("proxy_buffers            4 128k"), "missing proxy_buffers");
    assert!(conf.contains("proxy_http_version       1.1"), "missing proxy_http_version 1.1");
    assert!(!conf.contains("proxy_request_buffering off"), "proxy_request_buffering off must not be set with wasmx");

    assert!(conf.contains("sendfile                 on"), "missing sendfile");
    assert!(conf.contains("tcp_nopush               on"), "missing tcp_nopush");
    assert!(conf.contains("tcp_nodelay              on"), "missing tcp_nodelay");
    assert!(conf.contains("keepalive_timeout        65"), "missing keepalive_timeout");
    assert!(conf.contains("keepalive_requests       10000"), "missing keepalive_requests");
    assert!(conf.contains("use epoll"), "missing use epoll");
    assert!(conf.contains("multi_accept on"), "missing multi_accept");
    assert!(conf.contains("worker_rlimit_nofile"), "missing worker_rlimit_nofile");
    assert!(conf.contains("access_log off"), "access_log should be off");
    assert!(conf.contains("error_log /dev/null crit"), "error_log should be silenced");
    assert!(!conf.contains(" info;"), "logs should not be info level");
    assert!(!conf.contains(" warn;"), "logs should be fully silenced");

    assert!(conf.contains("pcre_jit on"), "missing pcre_jit");
    assert!(conf.contains("accept_mutex off"), "missing accept_mutex off");
    assert!(conf.contains("gzip                     off"), "missing gzip off");
    assert!(conf.contains("gzip_proxied             off"), "missing gzip_proxied off");
    assert!(conf.contains("client_max_body_size     1m"), "missing client_max_body_size");
    assert!(conf.contains("reset_timedout_connection on"), "missing reset_timedout_connection");
    assert!(conf.contains("proxy_connect_timeout    5s"), "missing proxy_connect_timeout");
    assert!(conf.contains("proxy_read_timeout       60s"), "missing proxy_read_timeout");
    assert!(conf.contains("proxy_set_header         Connection \"\""), "missing Connection header reset");
}

#[test]
fn build_conf_server_listen_and_server_name() {
    let _g = lock();
    let dir = TempDir::new().unwrap();
    unsafe { std::env::set_var("GATEWAY_ROOT", dir.path()); }

    let conf = NginxManager::new().build_conf(&make_bundle(dir.path())).unwrap();
    unsafe { std::env::remove_var("GATEWAY_ROOT"); }

    assert!(conf.contains("listen 8080 backlog=4096"), "missing listen with backlog");
    assert!(!conf.contains("reuseport"), "reuseport must not be set");
    assert!(conf.contains("server_name example.com"), "missing server_name");
}

#[test]
fn build_conf_location_has_proxy_wasm_chain_and_proxy_pass() {
    let _g = lock();
    let dir = TempDir::new().unwrap();
    unsafe { std::env::set_var("GATEWAY_ROOT", dir.path()); }

    let conf = NginxManager::new().build_conf(&make_bundle(dir.path())).unwrap();
    unsafe { std::env::remove_var("GATEWAY_ROOT"); }

    assert!(conf.contains("proxy_wasm tenant_filter"), "missing tenant_filter directive");
    assert!(conf.contains("proxy_wasm auth_filter"), "missing auth_filter directive");
    assert!(!conf.contains("proxy_wasm observe_filter"), "observe_filter not in chain, should be absent");
    assert!(conf.contains("proxy_pass http://svc_api-svc"), "missing proxy_pass");
}

#[test]
fn build_conf_empty_allowed_hostnames_uses_catch_all() {
    let _g = lock();
    let dir = TempDir::new().unwrap();
    unsafe { std::env::set_var("GATEWAY_ROOT", dir.path()); }

    let mut bundle = make_bundle(dir.path());
    bundle.listener.spec.allowed_hostnames.clear();
    let conf = NginxManager::new().build_conf(&bundle).unwrap();
    unsafe { std::env::remove_var("GATEWAY_ROOT"); }

    assert!(conf.contains("server_name _"), "should use _ when no allowed_hostnames");
}

#[test]
fn build_conf_revision_embedded_in_header_filter_config() {
    let _g = lock();
    let dir = TempDir::new().unwrap();
    unsafe { std::env::set_var("GATEWAY_ROOT", dir.path()); }

    let conf = NginxManager::new().build_conf(&make_bundle(dir.path())).unwrap();
    unsafe { std::env::remove_var("GATEWAY_ROOT"); }

    assert!(conf.contains("test-rev-001"), "revision should appear in header_filter config");
}

#[test]
fn build_conf_metrics_and_status_internal_locations_present() {
    let _g = lock();
    let dir = TempDir::new().unwrap();
    unsafe { std::env::set_var("GATEWAY_ROOT", dir.path()); }

    let conf = NginxManager::new().build_conf(&make_bundle(dir.path())).unwrap();
    unsafe { std::env::remove_var("GATEWAY_ROOT"); }

    assert!(conf.contains("/__gateway_status"), "missing status internal location");
    assert!(conf.contains("/metrics"), "missing metrics internal location");
}
