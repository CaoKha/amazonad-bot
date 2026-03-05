use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use tracing::warn;

use crate::models::MonitorState;

pub struct StateManager {
    path: PathBuf,
}

impl StateManager {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Load the previous monitor state from disk.
    ///
    /// Returns `Ok(None)` if the file is missing or contains invalid JSON
    /// (treated as first run). Only returns `Err` on unexpected I/O failures
    /// other than "not found".
    pub fn load(&self) -> Result<Option<MonitorState>> {
        let data = match fs::read_to_string(&self.path) {
            Ok(data) => data,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => {
                return Err(e)
                    .with_context(|| format!("Failed to read state file: {}", self.path.display()))
            }
        };

        match serde_json::from_str::<MonitorState>(&data) {
            Ok(state) => Ok(Some(state)),
            Err(e) => {
                warn!(
                    "Corrupt state file at {}: {}. Treating as first run.",
                    self.path.display(),
                    e
                );
                Ok(None)
            }
        }
    }

    /// Save the current state atomically (write to .tmp then rename).
    pub fn save(&self, state: &MonitorState) -> Result<()> {
        let tmp = self.path.with_extension("json.tmp");
        let json =
            serde_json::to_string_pretty(state).context("Failed to serialize monitor state")?;

        fs::write(&tmp, &json)
            .with_context(|| format!("Failed to write temp state file: {}", tmp.display()))?;

        fs::rename(&tmp, &self.path).with_context(|| {
            format!(
                "Failed to rename {} to {}",
                tmp.display(),
                self.path.display()
            )
        })?;

        Ok(())
    }
}
