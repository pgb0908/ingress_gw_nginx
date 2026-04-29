use gatewayd::revision::load_revision_bundle;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn write_file(dir: &Path, name: &str, content: &str) {
    fs::write(dir.join(name), content).unwrap();
}

fn setup_minimal_bundle(dir: &Path, revision: &str) {
    write_file(dir, "revision.json", &format!(
        r#"{{"revision":"{r}","created_at":"2026-01-01T00:00:00Z","runtime_compat":"test","plugins":[]}}"#,
        r = revision
    ));
    write_file(dir, "gateway.json", r#"{"metadata":{"name":"gw"},"spec":{}}"#);
    write_file(dir, "listener.json", r#"{"metadata":{"name":"lst"},"spec":{"protocol":"HTTP","port":8080}}"#);
    write_file(dir, "plugin-chain.json", r#"{"plugins":["tenant-filter","auth-filter"]}"#);
    write_file(dir, "router-main.json", r#"{"metadata":{"name":"main-route"},"spec":{"targetRef":{"kind":"Listener","name":"lst"},"rules":[{"path":"^/api(/.*)?$","methods":["GET"]}],"config":{"destinations":[{"destinationRef":{"kind":"Service","name":"svc"},"weight":100}]}}}"#);
    write_file(dir, "service-main.json", r#"{"metadata":{"name":"svc"},"spec":{"loadBalancing":{"targets":[{"host":"127.0.0.1","port":8000,"weight":100}]}}}"#);
}

#[test]
fn load_valid_bundle_returns_all_fields() {
    let dir = TempDir::new().unwrap();
    setup_minimal_bundle(dir.path(), "rev-001");

    let bundle = load_revision_bundle(dir.path()).unwrap();

    assert_eq!(bundle.manifest.revision, "rev-001");
    assert_eq!(bundle.listener.spec.port, 8080);
    assert_eq!(bundle.routers.len(), 1);
    assert_eq!(bundle.routers[0].metadata.name, "main-route");
    assert!(bundle.services.contains_key("svc"));
    assert_eq!(bundle.services["svc"].spec.load_balancing.targets[0].port, 8000);
    assert_eq!(bundle.plugin_chain, ["tenant-filter", "auth-filter"]);
}

#[test]
fn load_bundle_missing_revision_json_returns_error() {
    let dir = TempDir::new().unwrap();
    write_file(dir.path(), "gateway.json", r#"{"metadata":{"name":"gw"},"spec":{}}"#);

    let err = load_revision_bundle(dir.path()).unwrap_err();
    assert!(
        err.to_string().contains("revision.json"),
        "expected error to mention revision.json, got: {err}"
    );
}

#[test]
fn load_bundle_invalid_json_returns_error() {
    let dir = TempDir::new().unwrap();
    write_file(dir.path(), "revision.json", "not { valid json");
    write_file(dir.path(), "gateway.json", r#"{"metadata":{"name":"gw"},"spec":{}}"#);

    let err = load_revision_bundle(dir.path()).unwrap_err();
    assert!(err.to_string().contains("revision.json"));
}

#[test]
fn load_bundle_multiple_routers_sorted_alphabetically() {
    let dir = TempDir::new().unwrap();
    setup_minimal_bundle(dir.path(), "rev-002");
    write_file(dir.path(), "router-users.json", r#"{"metadata":{"name":"users-route"},"spec":{"targetRef":{"kind":"Listener","name":"lst"},"rules":[{"path":"^/users(/.*)?$"}],"config":{"destinations":[{"destinationRef":{"kind":"Service","name":"svc"},"weight":100}]}}}"#);

    let bundle = load_revision_bundle(dir.path()).unwrap();

    assert_eq!(bundle.routers.len(), 2);
    assert_eq!(bundle.routers[0].metadata.name, "main-route");
    assert_eq!(bundle.routers[1].metadata.name, "users-route");
}

#[test]
fn load_bundle_multiple_services_loaded_into_hashmap() {
    let dir = TempDir::new().unwrap();
    setup_minimal_bundle(dir.path(), "rev-003");
    write_file(dir.path(), "service-users.json", r#"{"metadata":{"name":"users-svc"},"spec":{"loadBalancing":{"targets":[{"host":"10.0.0.2","port":8001,"weight":50}]}}}"#);

    let bundle = load_revision_bundle(dir.path()).unwrap();

    assert_eq!(bundle.services.len(), 2);
    assert!(bundle.services.contains_key("svc"));
    assert!(bundle.services.contains_key("users-svc"));
    assert_eq!(bundle.services["users-svc"].spec.load_balancing.targets[0].port, 8001);
}

#[test]
fn load_bundle_empty_plugin_chain_is_ok() {
    let dir = TempDir::new().unwrap();
    setup_minimal_bundle(dir.path(), "rev-004");
    write_file(dir.path(), "plugin-chain.json", r#"{"plugins":[]}"#);

    let bundle = load_revision_bundle(dir.path()).unwrap();
    assert!(bundle.plugin_chain.is_empty());
}

#[test]
fn load_bundle_with_plugins_in_manifest() {
    let dir = TempDir::new().unwrap();
    write_file(dir.path(), "revision.json", r#"{"revision":"rev-plugins","created_at":"2026-01-01T00:00:00Z","runtime_compat":"test","plugins":[{"name":"tenant-filter","version":"1.0.0","wasm_path":"plugins/tenant-filter.wasm","sha256":"abc","failure_mode":"fail-close","hooks":["on_request_headers"]}]}"#);
    write_file(dir.path(), "gateway.json", r#"{"metadata":{"name":"gw"},"spec":{}}"#);
    write_file(dir.path(), "listener.json", r#"{"metadata":{"name":"lst"},"spec":{"protocol":"HTTP","port":8080}}"#);
    write_file(dir.path(), "plugin-chain.json", r#"{"plugins":["tenant-filter"]}"#);
    write_file(dir.path(), "router-main.json", r#"{"metadata":{"name":"r"},"spec":{"targetRef":{"kind":"Listener","name":"lst"},"rules":[{"path":"^/"}],"config":{"destinations":[{"destinationRef":{"kind":"Service","name":"svc"},"weight":100}]}}}"#);
    write_file(dir.path(), "service-main.json", r#"{"metadata":{"name":"svc"},"spec":{"loadBalancing":{"targets":[{"host":"127.0.0.1","port":8000}]}}}"#);

    let bundle = load_revision_bundle(dir.path()).unwrap();

    assert_eq!(bundle.manifest.plugins.len(), 1);
    assert_eq!(bundle.manifest.plugins[0].name, "tenant-filter");
    assert_eq!(bundle.manifest.plugins[0].failure_mode, "fail-close");
}
