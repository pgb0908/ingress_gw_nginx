use gatewayd::models::{ReloadStatus, RuntimeState, ValidationSnapshot};
use gatewayd::state::{load_state, save_state};
use std::sync::Mutex;
use tempfile::TempDir;

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn load_state_no_file_returns_default() {
    let _g = ENV_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    unsafe { std::env::set_var("GATEWAY_ROOT", dir.path()); }

    let state = load_state().unwrap();
    unsafe { std::env::remove_var("GATEWAY_ROOT"); }

    assert!(state.current_revision.is_none());
    assert!(state.current_revision_path.is_none());
    assert!(state.last_reload_status.is_none());
    assert_eq!(state.metrics.gateway_reload_total, 0);
}

#[test]
fn save_and_load_state_roundtrip() {
    let _g = ENV_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    unsafe { std::env::set_var("GATEWAY_ROOT", dir.path()); }

    let mut state = RuntimeState::default();
    state.current_revision = Some("rev-abc".to_string());
    state.current_revision_path = Some("/data/revisions/rev-abc".to_string());
    state.metrics.gateway_reload_total = 7;
    state.metrics.gateway_reload_failures_total = 2;
    state.metrics.gateway_requests_total = 1024;
    state.last_reload_status = Some(ReloadStatus {
        success: true,
        message: "ok".to_string(),
    });

    save_state(&state).unwrap();
    let loaded = load_state().unwrap();
    unsafe { std::env::remove_var("GATEWAY_ROOT"); }

    assert_eq!(loaded.current_revision.as_deref(), Some("rev-abc"));
    assert_eq!(loaded.current_revision_path.as_deref(), Some("/data/revisions/rev-abc"));
    assert_eq!(loaded.metrics.gateway_reload_total, 7);
    assert_eq!(loaded.metrics.gateway_reload_failures_total, 2);
    assert_eq!(loaded.metrics.gateway_requests_total, 1024);
    let reload = loaded.last_reload_status.as_ref().unwrap();
    assert!(reload.success);
    assert_eq!(reload.message, "ok");
}

#[test]
fn save_state_produces_valid_json_file() {
    let _g = ENV_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    unsafe { std::env::set_var("GATEWAY_ROOT", dir.path()); }

    let mut state = RuntimeState::default();
    state.current_revision = Some("json-check".to_string());
    state.metrics.gateway_reload_total = 3;
    save_state(&state).unwrap();

    let state_file = dir.path().join("runtime/dataplane/state.json");
    assert!(state_file.exists(), "state.json should be created");

    let content = std::fs::read_to_string(&state_file).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    unsafe { std::env::remove_var("GATEWAY_ROOT"); }

    assert_eq!(parsed["current_revision"], "json-check");
    assert_eq!(parsed["metrics"]["gateway_reload_total"], 3);
}

#[test]
fn save_state_multiple_times_last_write_wins() {
    let _g = ENV_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    unsafe { std::env::set_var("GATEWAY_ROOT", dir.path()); }

    let mut state = RuntimeState::default();
    state.current_revision = Some("rev-first".to_string());
    save_state(&state).unwrap();

    state.current_revision = Some("rev-second".to_string());
    state.metrics.gateway_reload_total = 1;
    save_state(&state).unwrap();

    let loaded = load_state().unwrap();
    unsafe { std::env::remove_var("GATEWAY_ROOT"); }

    assert_eq!(loaded.current_revision.as_deref(), Some("rev-second"));
    assert_eq!(loaded.metrics.gateway_reload_total, 1);
}

#[test]
fn save_state_persists_validation_snapshot() {
    let _g = ENV_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    unsafe { std::env::set_var("GATEWAY_ROOT", dir.path()); }

    let mut state = RuntimeState::default();
    state.last_validation = Some(ValidationSnapshot {
        revision: "snap-rev".to_string(),
        valid: false,
        errors: vec!["missing router".to_string()],
        warnings: vec![],
    });
    save_state(&state).unwrap();

    let loaded = load_state().unwrap();
    unsafe { std::env::remove_var("GATEWAY_ROOT"); }

    let snap = loaded.last_validation.as_ref().unwrap();
    assert_eq!(snap.revision, "snap-rev");
    assert!(!snap.valid);
    assert_eq!(snap.errors, ["missing router"]);
}
