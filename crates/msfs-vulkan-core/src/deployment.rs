use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::Config;
use crate::state::{
    DeploymentState, OriginalFile, Phase, StateEntry, StateStore, encode_hex, replace_file,
};

/// The "why is my screen black" fix, explained for people like @Vivloss:
///
/// MSFS 2020 ships an NVIDIA thing called Streamline (sl.interposer.dll).
/// Its whole job is to sit BETWEEN the game and Direct3D so it can do DLSS.
/// Problem: with our setup, "Direct3D" is actually our Vulkan translation
/// layer wearing a trench coat, and when Streamline pokes it with real
/// NVIDIA driver calls it gets nonsense back, gives up, and the game sits
/// on a black screen forever.
///
/// You cannot just delete sl.interposer.dll (the game literally refuses to
/// start without it - yes we tried, rip). Luckily NVIDIA gave it an off
/// switch: a little json file next to the dll that says "please do
/// nothing". So on install we drop that file, and on restore we take it
/// back out (or put back the one that was already there). DLSS options
/// will look greyed out in game while installed - that is expected, not
/// broken.
const SL_INTERPOSER_CONFIG_NAME: &str = "sl.interposer.json";
const SL_INTERPOSER_CONFIG: &str = "{\n  \"enableInterposer\": false\n}\n";

#[derive(Debug)]
pub struct Deployment<'a> {
    config: &'a Config,
    store: StateStore,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum DeploymentStatus {
    NotInstalled,
    Installed {
        files: Vec<FileStatus>,
    },
    Drifted {
        phase: Phase,
        files: Vec<FileStatus>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct FileStatus {
    pub target: PathBuf,
    pub condition: FileCondition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FileCondition {
    Installed,
    Missing,
    Modified,
}

impl<'a> Deployment<'a> {
    /// Construct a deployment controller and its per-game state location.
    ///
    /// # Errors
    ///
    /// Returns an error when the local application data directory is unavailable.
    pub fn new(config: &'a Config) -> Result<Self> {
        Ok(Self {
            config,
            store: StateStore::for_game(&config.game_dir)?,
        })
    }

    #[cfg(test)]
    fn with_store(config: &'a Config, store: StateStore) -> Self {
        Self { config, store }
    }

    /// Back up configured targets and install every payload file.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid inputs, existing state, failed verification, or I/O failure.
    pub fn install(&self) -> Result<()> {
        self.preflight()?;
        if self.store.load()?.is_some() {
            bail!("a deployment state already exists; run status or restore first");
        }

        fs::create_dir_all(self.store.backup_dir()).with_context(|| {
            format!(
                "failed to create backup directory {}",
                self.store.backup_dir().display()
            )
        })?;

        let mut state = DeploymentState {
            schema_version: 1,
            game_dir: self.config.game_dir.clone(),
            phase: Phase::Installing,
            entries: Vec::with_capacity(self.config.files.len()),
        };

        for (index, mapping) in self.config.files.iter().enumerate() {
            let source = self.config.payload_dir.join(&mapping.source);
            let target = self.config.game_dir.join(&mapping.target);
            let original = if target.exists() {
                let backup_name = format!("{index:02}.backup");
                let backup = self.store.backup_dir().join(&backup_name);
                copy_and_verify(&target, &backup)?;
                Some(OriginalFile {
                    backup_name,
                    sha256: sha256_file(&target)?,
                })
            } else {
                None
            };

            state.entries.push(StateEntry {
                target: mapping.target.clone(),
                installed_sha256: sha256_file(&source)?,
                original,
            });
        }

        // Streamline kill switch: recorded like any other installed file so
        // status verification and restore treat it uniformly.
        if let Some(target_name) = self.streamline_config_target() {
            let target = self.config.game_dir.join(&target_name);
            let original = if target.exists() {
                let backup_name = format!("{:02}.backup", state.entries.len());
                let backup = self.store.backup_dir().join(&backup_name);
                copy_and_verify(&target, &backup)?;
                Some(OriginalFile {
                    backup_name,
                    sha256: sha256_file(&target)?,
                })
            } else {
                None
            };
            state.entries.push(StateEntry {
                target: target_name,
                installed_sha256: sha256_bytes(SL_INTERPOSER_CONFIG.as_bytes()),
                original,
            });
        }
        self.store.save(&state)?;

        for mapping in &self.config.files {
            let source = self.config.payload_dir.join(&mapping.source);
            let target = self.config.game_dir.join(&mapping.target);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            let temporary = temporary_sibling(&target);
            copy_and_verify(&source, &temporary)?;
            replace_file(&temporary, &target)?;
        }

        if let Some(target_name) = self.streamline_config_target() {
            let target = self.config.game_dir.join(&target_name);
            let temporary = temporary_sibling(&target);
            fs::write(&temporary, SL_INTERPOSER_CONFIG)
                .with_context(|| format!("failed to write {}", temporary.display()))?;
            replace_file(&temporary, &target)?;
        }

        state.phase = Phase::Installed;
        self.store.save(&state)
    }

    /// Restore original files and remove targets that were originally absent.
    ///
    /// # Errors
    ///
    /// Returns an error when state or backups are invalid, installed files drifted without
    /// `force`, or an I/O operation fails.
    pub fn restore(&self, force: bool) -> Result<()> {
        let Some(mut state) = self.store.load()? else {
            bail!(
                "nothing is installed for {}",
                self.config.game_dir.display()
            );
        };
        ensure_state_matches(&state, &self.config.game_dir)?;

        if !force {
            let drifted: Vec<_> = self
                .file_statuses(&state)?
                .into_iter()
                .filter(|status| status.condition != FileCondition::Installed)
                .collect();
            if !drifted.is_empty() {
                let paths = drifted
                    .iter()
                    .map(|status| status.target.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                bail!("installed files changed or disappeared ({paths}); use --force to restore");
            }
        }

        state.phase = Phase::Restoring;
        self.store.save(&state)?;
        for entry in &state.entries {
            let target = self.config.game_dir.join(&entry.target);
            if let Some(original) = &entry.original {
                let backup = self.store.backup_dir().join(&original.backup_name);
                let actual = sha256_file(&backup).with_context(|| {
                    format!("original backup is unavailable: {}", backup.display())
                })?;
                if actual != original.sha256 {
                    bail!("backup checksum mismatch for {}", entry.target.display());
                }
                let temporary = temporary_sibling(&target);
                copy_and_verify(&backup, &temporary)?;
                replace_file(&temporary, &target)?;
            } else if target.exists() {
                fs::remove_file(&target)
                    .with_context(|| format!("failed to remove {}", target.display()))?;
            }
        }
        self.store.remove()
    }

    /// Inspect saved state and verify installed target checksums.
    ///
    /// # Errors
    ///
    /// Returns an error when state cannot be read or a target cannot be hashed.
    pub fn status(&self) -> Result<DeploymentStatus> {
        let Some(state) = self.store.load()? else {
            return Ok(DeploymentStatus::NotInstalled);
        };
        ensure_state_matches(&state, &self.config.game_dir)?;
        let files = self.file_statuses(&state)?;
        if state.phase == Phase::Installed
            && files
                .iter()
                .all(|status| status.condition == FileCondition::Installed)
        {
            Ok(DeploymentStatus::Installed { files })
        } else {
            Ok(DeploymentStatus::Drifted {
                phase: state.phase,
                files,
            })
        }
    }

    pub(crate) fn store(&self) -> &StateStore {
        &self.store
    }

    /// Where the Streamline off switch goes, or None when it is not needed.
    /// Only MSFS 2020 (FlightSimulator.exe) gets it, and only when the game
    /// actually ships sl.interposer.dll. 2024 is left alone on purpose.
    fn streamline_config_target(&self) -> Option<PathBuf> {
        let is_msfs2020 = self
            .config
            .executable
            .file_name()
            .is_some_and(|name| name.eq_ignore_ascii_case("FlightSimulator.exe"));
        (is_msfs2020 && self.config.game_dir.join("sl.interposer.dll").is_file())
            .then(|| PathBuf::from(SL_INTERPOSER_CONFIG_NAME))
    }

    fn preflight(&self) -> Result<()> {
        if !self.config.game_dir.is_dir() {
            bail!(
                "game directory does not exist: {}",
                self.config.game_dir.display()
            );
        }
        let executable = self.config.executable_path();
        if !executable.is_file() {
            bail!("game executable was not found: {}", executable.display());
        }
        for mapping in &self.config.files {
            let source = self.config.payload_dir.join(&mapping.source);
            if !source.is_file() {
                bail!("payload file is missing: {}", source.display());
            }
            let target = self.config.game_dir.join(&mapping.target);
            if target.is_dir() {
                bail!("deployment target is a directory: {}", target.display());
            }
        }
        Ok(())
    }

    fn file_statuses(&self, state: &DeploymentState) -> Result<Vec<FileStatus>> {
        state
            .entries
            .iter()
            .map(|entry| {
                let target = self.config.game_dir.join(&entry.target);
                let condition = if !target.is_file() {
                    FileCondition::Missing
                } else if sha256_file(&target)? == entry.installed_sha256 {
                    FileCondition::Installed
                } else {
                    FileCondition::Modified
                };
                Ok(FileStatus {
                    target: entry.target.clone(),
                    condition,
                })
            })
            .collect()
    }
}

fn ensure_state_matches(state: &DeploymentState, game_dir: &Path) -> Result<()> {
    if state.schema_version != 1 {
        bail!(
            "unsupported deployment state schema {}",
            state.schema_version
        );
    }
    if state.game_dir != game_dir {
        bail!(
            "deployment state belongs to {}, not {}",
            state.game_dir.display(),
            game_dir.display()
        );
    }
    Ok(())
}

fn temporary_sibling(target: &Path) -> PathBuf {
    let name = target
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("payload");
    target.with_file_name(format!(".{name}.msfs-vulkan-new-{}", std::process::id()))
}

fn copy_and_verify(source: &Path, target: &Path) -> Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, target).with_context(|| {
        format!(
            "failed to copy {} to {}",
            source.display(),
            target.display()
        )
    })?;
    let source_hash = sha256_file(source)?;
    let target_hash = sha256_file(target)?;
    if source_hash != target_hash {
        let _ = fs::remove_file(target);
        bail!("checksum mismatch after copying to {}", target.display());
    }
    Ok(())
}

fn sha256_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    encode_hex(&hasher.finalize())
}

/// Compute a lowercase SHA-256 digest for a file.
///
/// # Errors
///
/// Returns an error when the file cannot be opened or read.
pub fn sha256_file(path: &Path) -> Result<String> {
    let file = File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024];
    loop {
        let count = reader
            .read(&mut buffer)
            .with_context(|| format!("failed to read {}", path.display()))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(encode_hex(&hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::TempDir;

    use super::*;

    fn fixture() -> (TempDir, Config) {
        let temp = TempDir::new().unwrap();
        let game = temp.path().join("game");
        let payload = temp.path().join("payload");
        fs::create_dir_all(&game).unwrap();
        fs::create_dir_all(&payload).unwrap();
        fs::write(game.join("FlightSimulator2024.exe"), b"exe").unwrap();
        fs::write(game.join("d3d12.dll"), b"original").unwrap();
        for name in ["d3d12.dll", "d3d12core.dll", "d3d11.dll", "dxgi.dll"] {
            fs::write(payload.join(name), format!("translated-{name}")).unwrap();
        }
        (temp, Config::new(game, payload))
    }

    #[test]
    fn install_and_restore_are_reversible() {
        let (temp, config) = fixture();
        let store = StateStore::under(&temp.path().join("state"), &config.game_dir);
        let deployment = Deployment::with_store(&config, store);

        deployment.install().unwrap();
        assert!(matches!(
            deployment.status().unwrap(),
            DeploymentStatus::Installed { .. }
        ));
        assert_eq!(
            fs::read(config.game_dir.join("d3d12.dll")).unwrap(),
            b"translated-d3d12.dll"
        );

        deployment.restore(false).unwrap();
        assert_eq!(
            fs::read(config.game_dir.join("d3d12.dll")).unwrap(),
            b"original"
        );
        assert!(!config.game_dir.join("d3d12core.dll").exists());
        assert!(matches!(
            deployment.status().unwrap(),
            DeploymentStatus::NotInstalled
        ));
    }

    #[test]
    fn restore_refuses_modified_installed_file() {
        let (temp, config) = fixture();
        let store = StateStore::under(&temp.path().join("state"), &config.game_dir);
        let deployment = Deployment::with_store(&config, store);
        deployment.install().unwrap();

        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(config.game_dir.join("dxgi.dll"))
            .unwrap();
        file.write_all(b"changed").unwrap();

        assert!(deployment.restore(false).is_err());
        deployment.restore(true).unwrap();
    }

    #[test]
    fn msfs2020_gets_streamline_kill_switch_and_restore_removes_it() {
        let temp = TempDir::new().unwrap();
        let game = temp.path().join("game");
        let payload = temp.path().join("payload");
        fs::create_dir_all(&game).unwrap();
        fs::create_dir_all(&payload).unwrap();
        // 2020 exe (no 2024 exe present) plus the Streamline dll the game ships.
        fs::write(game.join("FlightSimulator.exe"), b"exe").unwrap();
        fs::write(game.join("sl.interposer.dll"), b"streamline").unwrap();
        for name in ["d3d12.dll", "d3d12core.dll", "d3d11.dll", "dxgi.dll"] {
            fs::write(payload.join(name), format!("translated-{name}")).unwrap();
        }
        let config = Config::new(game, payload);
        let store = StateStore::under(&temp.path().join("state"), &config.game_dir);
        let deployment = Deployment::with_store(&config, store);

        deployment.install().unwrap();
        let written = fs::read_to_string(config.game_dir.join("sl.interposer.json")).unwrap();
        assert!(written.contains("\"enableInterposer\": false"));
        assert!(matches!(
            deployment.status().unwrap(),
            DeploymentStatus::Installed { .. }
        ));

        deployment.restore(false).unwrap();
        assert!(!config.game_dir.join("sl.interposer.json").exists());
        // The game's own dll is untouched.
        assert!(config.game_dir.join("sl.interposer.dll").exists());
    }
}
