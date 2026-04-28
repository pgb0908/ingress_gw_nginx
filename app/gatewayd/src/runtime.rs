use crate::models::{LoadResult, ReloadStatus, ValidationResult, ValidationSnapshot};
use crate::nginx::NginxManager;
use crate::revision::load_revision_bundle;
use crate::state::{load_state, save_state};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub struct GatewayRuntime {
    nginx: NginxManager,
}

impl GatewayRuntime {
    pub fn new() -> Self {
        Self {
            nginx: NginxManager::new(),
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

fn validate_bundle(revision_path: &Path, nginx: &NginxManager) -> Result<ValidationResult> {
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
