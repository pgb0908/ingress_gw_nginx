use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root must exist")
}

pub fn local_dir() -> PathBuf {
    repo_root().join(".local")
}

pub fn run_dir() -> PathBuf {
    repo_root().join(".run")
}

pub fn runtime_config_dir() -> PathBuf {
    repo_root().join("src/runtime-config")
}

pub fn revisions_dir() -> PathBuf {
    runtime_config_dir().join("revisions")
}

pub fn current_link() -> PathBuf {
    runtime_config_dir().join("current")
}

pub fn runtime_dir() -> PathBuf {
    repo_root().join("src/gateway/runtime")
}

pub fn nginx_runtime_dir() -> PathBuf {
    runtime_dir().join("nginx")
}

pub fn nginx_conf_dir() -> PathBuf {
    nginx_runtime_dir().join("conf")
}

pub fn generated_dir() -> PathBuf {
    nginx_runtime_dir().join("generated")
}

pub fn log_dir() -> PathBuf {
    runtime_dir().join("logs")
}

pub fn state_file() -> PathBuf {
    runtime_dir().join("state.json")
}

pub fn rate_limit_state_file() -> PathBuf {
    runtime_dir().join("rate-limit-state.json")
}

pub fn project_nginx_bin() -> PathBuf {
    local_dir().join("wasmx/nginx")
}

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
    Ok(())
}

