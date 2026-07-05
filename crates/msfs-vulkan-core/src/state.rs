use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Phase {
    Installing,
    Installed,
    Restoring,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentState {
    pub schema_version: u32,
    pub game_dir: PathBuf,
    pub phase: Phase,
    pub entries: Vec<StateEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateEntry {
    pub target: PathBuf,
    pub installed_sha256: String,
    pub original: Option<OriginalFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginalFile {
    pub backup_name: String,
    pub sha256: String,
}

#[derive(Debug, Clone)]
pub struct StateStore {
    root: PathBuf,
}

impl StateStore {
    /// Resolve the per-game state directory below local application data.
    ///
    /// # Errors
    ///
    /// Returns an error when the platform has no local application data directory.
    pub fn for_game(game_dir: &Path) -> Result<Self> {
        Ok(Self::under(&crate::config::app_data_dir()?, game_dir))
    }

    /// Return game directories with saved deployment state.
    ///
    /// # Errors
    ///
    /// Returns an error when local application data or a saved deployment state cannot be read.
    pub fn known_game_dirs() -> Result<Vec<PathBuf>> {
        let root = crate::config::app_data_dir()?.join("profiles");
        known_game_dirs_under(&root)
    }

    pub fn under(base: &Path, game_dir: &Path) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(game_dir.to_string_lossy().to_lowercase().as_bytes());
        let id = encode_hex(&hasher.finalize());
        Self {
            root: base.join("profiles").join(&id[..16]),
        }
    }

    pub fn state_path(&self) -> PathBuf {
        self.root.join("state.json")
    }

    pub fn backup_dir(&self) -> PathBuf {
        self.root.join("backups")
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.root.join("cache")
    }

    pub fn log_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    /// Read deployment state when it exists.
    ///
    /// # Errors
    ///
    /// Returns an error when state cannot be read or decoded.
    pub fn load(&self) -> Result<Option<DeploymentState>> {
        let path = self.state_path();
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(read_state(&path)?))
    }

    /// Atomically persist deployment state.
    ///
    /// # Errors
    ///
    /// Returns an error when serialization or an I/O operation fails.
    pub fn save(&self, state: &DeploymentState) -> Result<()> {
        fs::create_dir_all(&self.root)
            .with_context(|| format!("failed to create state directory {}", self.root.display()))?;
        let bytes =
            serde_json::to_vec_pretty(state).context("failed to serialize deployment state")?;
        let temporary = self.root.join("state.json.tmp");
        fs::write(&temporary, bytes)
            .with_context(|| format!("failed to write temporary state {}", temporary.display()))?;
        replace_file(&temporary, &self.state_path())
    }

    /// Remove completed deployment state and backups while preserving logs and caches.
    ///
    /// # Errors
    ///
    /// Returns an error when state or backups cannot be removed.
    pub fn remove(&self) -> Result<()> {
        let state_path = self.state_path();
        if state_path.exists() {
            fs::remove_file(&state_path)
                .with_context(|| format!("failed to remove {}", state_path.display()))?;
        }
        let backup_dir = self.backup_dir();
        if backup_dir.exists() {
            fs::remove_dir_all(&backup_dir)
                .with_context(|| format!("failed to remove {}", backup_dir.display()))?;
        }
        Ok(())
    }
}

fn known_game_dirs_under(root: &Path) -> Result<Vec<PathBuf>> {
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut game_dirs = Vec::new();
    for entry in
        fs::read_dir(root).with_context(|| format!("failed to inspect {}", root.display()))?
    {
        let entry = entry.with_context(|| format!("failed to inspect {}", root.display()))?;
        let state_path = entry.path().join("state.json");
        if !state_path.is_file() {
            continue;
        }
        game_dirs.push(read_state(&state_path)?.game_dir);
    }
    game_dirs.sort();
    game_dirs.dedup();
    Ok(game_dirs)
}

fn read_state(path: &Path) -> Result<DeploymentState> {
    let bytes = fs::read(path)
        .with_context(|| format!("failed to read deployment state {}", path.display()))?;
    serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to parse deployment state {}", path.display()))
}

pub(crate) fn encode_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(output, "{byte:02x}").expect("writing to a String cannot fail");
    }
    output
}

pub(crate) fn replace_file(source: &Path, target: &Path) -> Result<()> {
    let displaced = target.with_extension(format!(
        "{}.msfs-vulkan-old-{}",
        target
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("file"),
        std::process::id()
    ));
    if target.exists() {
        fs::rename(target, &displaced).with_context(|| {
            format!(
                "failed to move existing file {} to {}",
                target.display(),
                displaced.display()
            )
        })?;
    }
    if let Err(error) = fs::rename(source, target) {
        if displaced.exists() {
            let _ = fs::rename(&displaced, target);
        }
        return Err(error).with_context(|| {
            format!(
                "failed to move {} to {}",
                source.display(),
                target.display()
            )
        });
    }
    if displaced.exists() {
        fs::remove_file(&displaced)
            .with_context(|| format!("failed to remove {}", displaced.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_game_dirs_from_saved_deployment_states() {
        let temp = tempfile::tempdir().unwrap();
        let game_dir = temp.path().join("game");
        let store = StateStore::under(temp.path(), &game_dir);
        store
            .save(&DeploymentState {
                schema_version: 1,
                game_dir: game_dir.clone(),
                phase: Phase::Installed,
                entries: Vec::new(),
            })
            .unwrap();

        let found = known_game_dirs_under(&temp.path().join("profiles")).unwrap();

        assert_eq!(found, [game_dir]);
    }
}
