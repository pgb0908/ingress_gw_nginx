use crate::models::{MetricsState, RuntimeState};
use crate::paths;
use anyhow::{Context, Result};
use std::fs;

pub fn load_state() -> Result<RuntimeState> {
    let path = paths::state_file();
    if !path.exists() {
        return Ok(RuntimeState {
            metrics: MetricsState::default(),
            ..RuntimeState::default()
        });
    }

    let text = fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let state = serde_json::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(state)
}

pub fn save_state(state: &RuntimeState) -> Result<()> {
    paths::ensure_runtime_layout()?;
    let path = paths::state_file();
    fs::write(&path, serde_json::to_vec_pretty(state)?)
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

