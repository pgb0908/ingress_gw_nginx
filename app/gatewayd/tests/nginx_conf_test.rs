use gatewayd::models::*;
use gatewayd::nginx::NginxManager;
use std::collections::HashMap;
use std::sync::Mutex;
use tempfile::TempDir;

static ENV_LOCK: Mutex<()> = Mutex::new(());

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
                target_ref: TargetRef { kind: "Listener".to_string(), name: "lst".to_string() },
                rules: vec![RouterRule {
                    path: "^/api(/.*)?$".to_string(),
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
        plugin_chain: vec!["tenant-filter".to_string(), "auth-filter".to_string()],
    }
}

#[test]
fn build_conf_wasm_block_declares_all_five_filters() {
    let _g = ENV_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    unsafe { std::env::set_var("GATEWAY_ROOT", dir.path()); }

    let conf = NginxManager::new().build_conf(&make_bundle(dir.path())).unwrap();
    unsafe { std::env::remove_var("GATEWAY_ROOT"); }

    assert!(conf.contains("wasm {"), "missing wasm block");
    for filter in ["tenant_filter", "auth_filter", "header_filter", "rate_limit_filter", "observe_filter"] {
        assert!(conf.contains(&format!("module {filter}")), "missing module declaration for {filter}");
    }
}

#[test]
fn build_conf_upstream_block_matches_service_targets() {
    let _g = ENV_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    unsafe { std::env::set_var("GATEWAY_ROOT", dir.path()); }

    let conf = NginxManager::new().build_conf(&make_bundle(dir.path())).unwrap();
    unsafe { std::env::remove_var("GATEWAY_ROOT"); }

    assert!(conf.contains("upstream svc_api-svc"), "missing upstream block");
    assert!(conf.contains("127.0.0.1:9000 weight=100"), "missing upstream server directive");
}

#[test]
fn build_conf_server_listen_and_server_name() {
    let _g = ENV_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    unsafe { std::env::set_var("GATEWAY_ROOT", dir.path()); }

    let conf = NginxManager::new().build_conf(&make_bundle(dir.path())).unwrap();
    unsafe { std::env::remove_var("GATEWAY_ROOT"); }

    assert!(conf.contains("listen 8080"), "missing listen directive");
    assert!(conf.contains("server_name example.com"), "missing server_name");
}

#[test]
fn build_conf_location_has_proxy_wasm_chain_and_proxy_pass() {
    let _g = ENV_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    unsafe { std::env::set_var("GATEWAY_ROOT", dir.path()); }

    let conf = NginxManager::new().build_conf(&make_bundle(dir.path())).unwrap();
    unsafe { std::env::remove_var("GATEWAY_ROOT"); }

    assert!(conf.contains("proxy_wasm tenant_filter"), "missing tenant_filter directive");
    assert!(conf.contains("proxy_wasm auth_filter"), "missing auth_filter directive");
    assert!(conf.contains("proxy_wasm observe_filter"), "missing observe_filter directive");
    assert!(conf.contains("proxy_pass http://svc_api-svc"), "missing proxy_pass");
}

#[test]
fn build_conf_empty_allowed_hostnames_uses_catch_all() {
    let _g = ENV_LOCK.lock().unwrap();
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
    let _g = ENV_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    unsafe { std::env::set_var("GATEWAY_ROOT", dir.path()); }

    let conf = NginxManager::new().build_conf(&make_bundle(dir.path())).unwrap();
    unsafe { std::env::remove_var("GATEWAY_ROOT"); }

    assert!(conf.contains("test-rev-001"), "revision should appear in header_filter config");
}

#[test]
fn build_conf_metrics_and_status_internal_locations_present() {
    let _g = ENV_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    unsafe { std::env::set_var("GATEWAY_ROOT", dir.path()); }

    let conf = NginxManager::new().build_conf(&make_bundle(dir.path())).unwrap();
    unsafe { std::env::remove_var("GATEWAY_ROOT"); }

    assert!(conf.contains("/__gateway_status"), "missing status internal location");
    assert!(conf.contains("/metrics"), "missing metrics internal location");
}
