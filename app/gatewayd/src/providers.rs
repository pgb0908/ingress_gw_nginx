use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Default, Deserialize)]
pub struct SecretConfig {
    #[serde(default)]
    pub api_keys: HashMap<String, String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct RateLimitConfig {
    #[serde(default)]
    pub default: RateLimitRule,
    #[serde(default)]
    pub limits: HashMap<String, RateLimitRule>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RateLimitRule {
    #[serde(default = "default_requests")]
    pub requests: u64,
    #[serde(default = "default_window_seconds")]
    pub window_seconds: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RateLimitState {
    #[serde(default)]
    pub buckets: HashMap<String, BucketState>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BucketState {
    pub window_start: u64,
    pub count: u64,
}

#[derive(Debug, Serialize)]
pub struct RateLimitDecision {
    pub allowed: bool,
    pub limit: u64,
    pub remaining: u64,
    pub window_seconds: u64,
}

pub fn load_secret(path: &Path, tenant_id: &str) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let payload: SecretConfig = serde_json::from_slice(&fs::read(path)?)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(payload.api_keys.get(tenant_id).cloned())
}

pub fn consume_rate_limit(
    config_path: &Path,
    state_path: &Path,
    tenant_id: &str,
    service_id: &str,
    route_id: &str,
    now_epoch_seconds: u64,
) -> Result<RateLimitDecision> {
    let config = if config_path.exists() {
        serde_json::from_slice::<RateLimitConfig>(&fs::read(config_path)?)
            .with_context(|| format!("failed to parse {}", config_path.display()))?
    } else {
        RateLimitConfig::default()
    };

    let mut state = if state_path.exists() {
        serde_json::from_slice::<RateLimitState>(&fs::read(state_path)?)
            .with_context(|| format!("failed to parse {}", state_path.display()))?
    } else {
        RateLimitState::default()
    };

    let key = format!("{tenant_id}:{service_id}:{route_id}");
    let rule = config
        .limits
        .get(&key)
        .or_else(|| config.limits.get(&format!("{tenant_id}:{service_id}")))
        .or_else(|| config.limits.get(tenant_id))
        .cloned()
        .unwrap_or_else(|| {
            if config.default.requests == 0 {
                RateLimitRule::default()
            } else {
                config.default.clone()
            }
        });

    let bucket = state.buckets.entry(key).or_default();
    if now_epoch_seconds.saturating_sub(bucket.window_start) >= rule.window_seconds {
        bucket.window_start = now_epoch_seconds;
        bucket.count = 0;
    }
    if bucket.window_start == 0 {
        bucket.window_start = now_epoch_seconds;
    }

    let allowed = bucket.count < rule.requests;
    if allowed {
        bucket.count += 1;
    }
    let remaining = rule.requests.saturating_sub(bucket.count);
    let limit = rule.requests;
    let window_seconds = rule.window_seconds;

    if let Some(parent) = state_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(state_path, serde_json::to_vec_pretty(&state)?)?;

    Ok(RateLimitDecision {
        allowed,
        limit,
        remaining,
        window_seconds,
    })
}

fn default_requests() -> u64 {
    60
}

fn default_window_seconds() -> u64 {
    60
}
