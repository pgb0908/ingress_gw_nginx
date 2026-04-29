use crate::models::{DeployResult, LoadResult, ReloadStatus, RuntimeState, ValidationResult, ValidationSnapshot};
use crate::nginx::NginxManager;
use crate::paths;
use crate::revision::load_revision_bundle;
use crate::state::{load_state, save_state};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

// ── DeployError ───────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum DeployError {
    /// HTTP 400: 잘못된 kind, 필드 누락, JSON 파싱 오류
    BadRequest(String),
    /// HTTP 500: 서버 내부 오류
    Internal(anyhow::Error),
}

impl From<anyhow::Error> for DeployError {
    fn from(e: anyhow::Error) -> Self {
        DeployError::Internal(e)
    }
}

pub struct GatewayRuntime {
    nginx: NginxManager,
}

impl GatewayRuntime {
    pub fn new() -> Self {
        Self {
            nginx: NginxManager::new(),
        }
    }

    pub fn deploy_resource(&self, body: &[u8]) -> Result<DeployResult, DeployError> {
        // 1. JSON 파싱 → kind, name 추출
        let raw: serde_json::Value = serde_json::from_slice(body)
            .map_err(|e| DeployError::BadRequest(format!("JSON parse error: {e}")))?;

        let kind = raw["kind"]
            .as_str()
            .ok_or_else(|| DeployError::BadRequest("missing field: kind".to_string()))?
            .to_string();

        let name = raw["metadata"]["name"]
            .as_str()
            .ok_or_else(|| DeployError::BadRequest("missing field: metadata.name".to_string()))?
            .to_string();

        // 2. kind → 파일명 결정
        let filename = kind_to_filename(&kind, &name).map_err(DeployError::BadRequest)?;

        // 3. live 디렉토리 초기화 (필요 시)
        let live_dir = paths::live_dir();
        let mut state = load_state()?;
        ensure_live_dir_initialized(&live_dir, &state)?;

        // 4. 리소스 파일 저장 (pretty-print)
        let pretty = serde_json::to_vec_pretty(&raw).map_err(|e| anyhow::anyhow!(e))?;
        fs::write(live_dir.join(&filename), &pretty).map_err(|e| anyhow::anyhow!(e))?;

        // 5. validate_bundle (기존 함수 재사용)
        let validation = validate_bundle(&live_dir, &self.nginx)?;

        state.last_validation = Some(ValidationSnapshot {
            revision: validation.revision.clone(),
            valid: validation.valid,
            errors: validation.errors.clone(),
            warnings: validation.warnings.clone(),
        });

        // 6. 번들 불완전 → staged (202)
        if !validation.valid {
            save_state(&state)?;
            return Ok(DeployResult {
                kind,
                name,
                status: "staged".to_string(),
                message: "resource saved; bundle not yet complete".to_string(),
                validation: Some(validation),
            });
        }

        // 7. nginx activate
        let conf_path = validation
            .rendered_conf
            .as_ref()
            .map(|s| s.as_str())
            .context("missing rendered conf")?;

        state.metrics.gateway_reload_total += 1;

        match self.nginx.activate(Path::new(conf_path)) {
            Ok(message) => {
                state.current_revision = Some(validation.revision.clone());
                state.current_revision_path = Some(
                    live_dir
                        .canonicalize()
                        .unwrap_or_else(|_| live_dir.clone())
                        .to_string_lossy()
                        .to_string(),
                );
                state.last_reload_status = Some(ReloadStatus { success: true, message: message.clone() });
                save_state(&state)?;
                Ok(DeployResult {
                    kind,
                    name,
                    status: "applied".to_string(),
                    message,
                    validation: Some(validation),
                })
            }
            Err(error) => {
                state.metrics.gateway_reload_failures_total += 1;
                state.last_reload_status = Some(ReloadStatus {
                    success: false,
                    message: error.to_string(),
                });
                save_state(&state)?;
                Ok(DeployResult {
                    kind,
                    name,
                    status: "failed".to_string(),
                    message: error.to_string(),
                    validation: Some(validation),
                })
            }
        }
    }

    pub fn load_revision(&self, revision_path: &Path) -> Result<LoadResult> {
        let mut state = load_state()?;

        let validation = validate_bundle(revision_path, &self.nginx)?;
        state.last_validation = Some(ValidationSnapshot {
            revision: validation.revision.clone(),
            valid: validation.valid,
            errors: validation.errors.clone(),
            warnings: validation.warnings.clone(),
        });

        if !validation.valid {
            save_state(&state)?;
            return Ok(LoadResult {
                revision: Some(validation.revision.clone()),
                status: "validation_failed".to_string(),
                message: "revision did not pass validation".to_string(),
                validation: Some(validation),
            });
        }

        let conf_path = validation
            .rendered_conf
            .as_ref()
            .map(|s| s.as_str())
            .context("missing rendered conf")?;

        state.metrics.gateway_reload_total += 1;

        match self.nginx.activate(Path::new(conf_path)) {
            Ok(message) => {
                state.current_revision = Some(validation.revision.clone());
                state.current_revision_path = Some(
                    revision_path
                        .canonicalize()
                        .unwrap_or_else(|_| revision_path.to_path_buf())
                        .to_string_lossy()
                        .to_string(),
                );
                state.last_reload_status = Some(ReloadStatus {
                    success: true,
                    message: message.clone(),
                });
                save_state(&state)?;
                Ok(LoadResult {
                    revision: Some(validation.revision.clone()),
                    status: "loaded".to_string(),
                    message,
                    validation: Some(validation),
                })
            }
            Err(error) => {
                state.metrics.gateway_reload_failures_total += 1;
                state.last_reload_status = Some(ReloadStatus {
                    success: false,
                    message: error.to_string(),
                });
                save_state(&state)?;
                Ok(LoadResult {
                    revision: Some(validation.revision.clone()),
                    status: "reload_failed".to_string(),
                    message: error.to_string(),
                    validation: Some(validation),
                })
            }
        }
    }
}

pub fn validate_bundle(revision_path: &Path, nginx: &NginxManager) -> Result<ValidationResult> {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    for required in ["revision.json", "gateway.json", "listener.json", "plugin-chain.json"] {
        if !revision_path.join(required).exists() {
            errors.push(format!("missing required file: {required}"));
        }
    }
    if !contains_prefixed_file(revision_path, "router-")? {
        errors.push("missing router resource".to_string());
    }
    if !contains_prefixed_file(revision_path, "service-")? {
        errors.push("missing service resource".to_string());
    }

    if !errors.is_empty() {
        return Ok(ValidationResult {
            revision: revision_path
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or("unknown")
                .to_string(),
            valid: false,
            rendered_conf: None,
            errors,
            warnings,
        });
    }

    let bundle = load_revision_bundle(revision_path)?;
    for plugin in &bundle.manifest.plugins {
        if !revision_path.join(&plugin.wasm_path).exists() {
            errors.push(format!("missing wasm module: {}", plugin.wasm_path));
        }
        if plugin.version.starts_with("0.") {
            warnings.push(format!(
                "plugin {} uses pre-1.0 version {}",
                plugin.name, plugin.version
            ));
        }
    }

    let conf_path = nginx.render(&bundle)?;
    if let Err(error) = nginx.validate(&conf_path) {
        errors.push(error.to_string());
    }

    Ok(ValidationResult {
        revision: bundle.manifest.revision,
        valid: errors.is_empty(),
        rendered_conf: Some(conf_path.display().to_string()),
        errors,
        warnings,
    })
}

fn contains_prefixed_file(root: &Path, prefix: &str) -> Result<bool> {
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if let Some(name) = path.file_name().and_then(|v| v.to_str()) {
            if name.starts_with(prefix) && name.ends_with(".json") {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

// ── deploy helpers ────────────────────────────────────────────────────────────

fn kind_to_filename(kind: &str, name: &str) -> Result<String, String> {
    match kind {
        "Gateway"  => Ok("gateway.json".to_string()),
        "Listener" => Ok("listener.json".to_string()),
        "Router"   => Ok(format!("router-{name}.json")),
        "Service"  => Ok(format!("service-{name}.json")),
        "Policy"   => Ok(format!("policy-{name}.json")),
        other      => Err(format!("unsupported kind: {other}")),
    }
}

fn ensure_live_dir_initialized(live_dir: &Path, state: &RuntimeState) -> Result<()> {
    // 이미 초기화된 경우 skip
    if live_dir.join("revision.json").exists() {
        return Ok(());
    }

    fs::create_dir_all(live_dir)?;

    // revision.json 자동 생성
    let epoch_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let revision_id = format!("live-{epoch_secs}");
    let manifest = serde_json::json!({
        "revision": revision_id,
        "created_at": revision_id,
        "runtime_compat": "live",
        "plugins": []
    });
    fs::write(live_dir.join("revision.json"), serde_json::to_vec_pretty(&manifest)?)?;

    // plugin-chain.json 기본값 생성
    let chain = serde_json::json!({
        "plugins": ["tenant-filter", "auth-filter", "header-filter", "rate-limit-filter", "observe-filter"]
    });
    fs::write(live_dir.join("plugin-chain.json"), serde_json::to_vec_pretty(&chain)?)?;

    // 활성 revision의 plugins/, data/ 복사 (wasm 바이너리 재사용)
    if let Some(active_path) = &state.current_revision_path {
        let src = Path::new(active_path);
        for subdir in ["plugins", "data"] {
            let src_sub = src.join(subdir);
            if src_sub.exists() {
                copy_dir_all(&src_sub, &live_dir.join(subdir))?;
            }
        }
    }

    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let dst_path = dst.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir_all(&entry.path(), &dst_path)?;
        } else {
            fs::copy(entry.path(), dst_path)?;
        }
    }
    Ok(())
}
