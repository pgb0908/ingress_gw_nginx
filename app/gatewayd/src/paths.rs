use anyhow::{Context, Result};
use std::{env, fs};
use std::path::PathBuf;

fn gateway_root() -> PathBuf {
    if let Ok(val) = env::var("GATEWAY_ROOT") {
        return PathBuf::from(val)
            .canonicalize()
            .expect("GATEWAY_ROOT must point to an existing directory");
    }
    let exe = env::current_exe()
        .expect("cannot determine current executable path");
    exe.parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .and_then(|p| p.canonicalize().ok())
        .expect("cannot resolve gateway root; set GATEWAY_ROOT")
}

pub fn local_dir() -> PathBuf            { gateway_root().join("env/local") }
pub fn run_dir() -> PathBuf              { gateway_root().join("runtime/process") }
pub fn fixtures_dir() -> PathBuf         { gateway_root().join("fixtures") }
pub fn runtime_dir() -> PathBuf          { gateway_root().join("runtime/dataplane") }
pub fn nginx_runtime_dir() -> PathBuf    { runtime_dir().join("nginx") }
pub fn nginx_conf_dir() -> PathBuf       { nginx_runtime_dir().join("conf") }
pub fn generated_dir() -> PathBuf        { nginx_runtime_dir().join("generated") }
pub fn log_dir() -> PathBuf              { runtime_dir().join("logs") }
pub fn state_file() -> PathBuf           { runtime_dir().join("state.json") }
pub fn rate_limit_state_file() -> PathBuf { runtime_dir().join("rate-limit-state.json") }

pub fn project_nginx_bin() -> PathBuf {
    if let Ok(val) = env::var("GATEWAY_NGINX_BIN") {
        return PathBuf::from(val);
    }
    gateway_root().join("bin/nginx")
}

const MIME_TYPES: &str = include_str!("mime.types");

pub fn ensure_runtime_layout() -> Result<()> {
    for path in [
        run_dir(),
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
