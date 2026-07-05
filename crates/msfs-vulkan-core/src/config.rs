use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Preset {
    Quality,
    #[default]
    Balanced,
    Performance,
}

impl Preset {
    pub fn environment(self) -> BTreeMap<String, String> {
        match self {
            Preset::Quality => BTreeMap::from([
                ("VKD3D_CONFIG".into(), "dxr11".into()),
                ("VKD3D_FEATURE_LEVEL".into(), "12_2".into()),
                ("VKD3D_DEBUG".into(), "info".into()),
                ("DXVK_LOG_LEVEL".into(), "info".into()),
                ("DXVK_STATE_CACHE".into(), "1".into()),
            ]),
            Preset::Balanced => BTreeMap::from([
                ("VKD3D_CONFIG".into(), "dxr11".into()),
                ("VKD3D_FEATURE_LEVEL".into(), "12_2".into()),
                ("VKD3D_DEBUG".into(), "none".into()),
                ("DXVK_LOG_LEVEL".into(), "none".into()),
                ("DXVK_STATE_CACHE".into(), "1".into()),
            ]),
            Preset::Performance => BTreeMap::from([
                ("VKD3D_CONFIG".into(), "nodxr,no_upload_hvv".into()),
                ("VKD3D_FEATURE_LEVEL".into(), "12_1".into()),
                ("VKD3D_DEBUG".into(), "none".into()),
                ("DXVK_LOG_LEVEL".into(), "none".into()),
                ("DXVK_STATE_CACHE".into(), "1".into()),
            ]),
        }
    }
}

pub const CURRENT_SCHEMA_VERSION: u32 = 1;
pub const CONFIG_FILE_NAME: &str = "msfs-vulkan.toml";
pub const DEFAULT_EXECUTABLE: &str = "FlightSimulator2024.exe";
pub const DEFAULT_VKD3D_REPO: &str = "HansKristian-Work/vkd3d-proton";
pub const DEFAULT_DXVK_REPO: &str = "doitsujin/dxvk";

/// Repository choices exposed by the GUI. Custom values remain supported through TOML.
pub const VKD3D_REPOSITORY_PRESETS: &[(&str, &str)] =
    &[("Official VKD3D-Proton", DEFAULT_VKD3D_REPO)];

/// Repository choices exposed by the GUI. Custom values remain supported through TOML.
pub const DXVK_REPOSITORY_PRESETS: &[(&str, &str)] = &[("Official DXVK", DEFAULT_DXVK_REPO)];

/// Resolve the app-local data directory used for persistent config, state, logs, and runtime files.
///
/// # Errors
///
/// Returns an error when the platform has no local application data directory.
pub fn app_data_dir() -> Result<PathBuf> {
    let project_dirs = ProjectDirs::from("dev", "msfs-vulkan", "msfs-vulkan")
        .ok_or_else(|| anyhow!("could not determine the local application data directory"))?;
    Ok(project_dirs.data_local_dir().to_path_buf())
}

/// Resolve the persistent default config path.
///
/// # Errors
///
/// Returns an error when the platform has no local application data directory.
pub fn default_config_path() -> Result<PathBuf> {
    Ok(app_data_dir()?.join(CONFIG_FILE_NAME))
}

pub fn legacy_config_path() -> PathBuf {
    PathBuf::from(CONFIG_FILE_NAME)
}

/// Resolve the default persistent runtime payload directory.
///
/// # Errors
///
/// Returns an error when the platform has no local application data directory.
pub fn default_payload_dir() -> Result<PathBuf> {
    Ok(app_data_dir()?.join("runtime").join("x64"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub schema_version: u32,
    pub game_dir: PathBuf,
    pub payload_dir: PathBuf,
    #[serde(default = "default_vkd3d_repo")]
    pub vkd3d_repo: String,
    #[serde(default = "default_dxvk_repo")]
    pub dxvk_repo: String,
    #[serde(default = "default_executable")]
    pub executable: PathBuf,
    #[serde(default = "default_files")]
    pub files: Vec<FileMapping>,
    #[serde(default = "default_environment")]
    pub environment: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileMapping {
    pub source: PathBuf,
    pub target: PathBuf,
}

impl Config {
    pub fn new(game_dir: PathBuf, payload_dir: PathBuf) -> Self {
        let executable = if game_dir.join("FlightSimulator2024.exe").is_file() {
            PathBuf::from("FlightSimulator2024.exe")
        } else if game_dir.join("FlightSimulator.exe").is_file() {
            PathBuf::from("FlightSimulator.exe")
        } else {
            PathBuf::from(DEFAULT_EXECUTABLE)
        };
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            game_dir,
            payload_dir,
            vkd3d_repo: DEFAULT_VKD3D_REPO.to_string(),
            dxvk_repo: DEFAULT_DXVK_REPO.to_string(),
            executable,
            files: default_files(),
            environment: default_environment(),
        }
    }

    /// Load and validate a TOML configuration, resolving relative base paths.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read, parsed, or validated.
    pub fn load(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)
            .with_context(|| format!("failed to read config {}", path.display()))?;
        let mut config: Self = toml::from_str(&text)
            .with_context(|| format!("failed to parse config {}", path.display()))?;
        let base = path.parent().unwrap_or_else(|| Path::new("."));
        config.resolve_relative_to(base);
        config.validate()?;
        Ok(config)
    }

    /// Validate and save this configuration as TOML.
    ///
    /// # Errors
    ///
    /// Returns an error when validation, serialization, or writing fails.
    pub fn save(&self, path: &Path) -> Result<()> {
        self.validate()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let text = toml::to_string_pretty(self).context("failed to serialize configuration")?;
        fs::write(path, text).with_context(|| format!("failed to write {}", path.display()))
    }

    pub fn executable_path(&self) -> PathBuf {
        self.game_dir.join(&self.executable)
    }

    pub fn resolve_relative_to(&mut self, base: &Path) {
        if self.game_dir.is_relative() {
            self.game_dir = base.join(&self.game_dir);
        }
        if self.payload_dir.is_relative() {
            self.payload_dir = base.join(&self.payload_dir);
        }
    }

    /// Validate schema, mappings, executable, and environment variable names.
    ///
    /// # Errors
    ///
    /// Returns an error for unsupported schemas or unsafe manifest values.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != CURRENT_SCHEMA_VERSION {
            bail!(
                "unsupported config schema {}; expected {}",
                self.schema_version,
                CURRENT_SCHEMA_VERSION
            );
        }
        if self.files.is_empty() {
            bail!("configuration contains no file mappings");
        }
        validate_relative_path(&self.executable, "executable")?;
        let mut targets = BTreeSet::new();
        for mapping in &self.files {
            validate_relative_path(&mapping.source, "mapping source")?;
            validate_relative_path(&mapping.target, "mapping target")?;
            let normalized = mapping.target.to_string_lossy().to_lowercase();
            if !targets.insert(normalized) {
                bail!("duplicate mapping target: {}", mapping.target.display());
            }
        }
        for key in self.environment.keys() {
            if key.is_empty() || key.contains('=') || key.contains('\0') {
                bail!("invalid environment variable name {key:?}");
            }
        }
        Ok(())
    }
}

fn validate_relative_path(path: &Path, label: &str) -> Result<()> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        bail!(
            "{label} must be a non-empty relative path: {}",
            path.display()
        );
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        bail!("{label} escapes its base directory: {}", path.display());
    }
    Ok(())
}

fn default_executable() -> PathBuf {
    PathBuf::from(DEFAULT_EXECUTABLE)
}

fn default_vkd3d_repo() -> String {
    DEFAULT_VKD3D_REPO.to_string()
}

fn default_dxvk_repo() -> String {
    DEFAULT_DXVK_REPO.to_string()
}

fn default_files() -> Vec<FileMapping> {
    ["d3d12.dll", "d3d12core.dll", "d3d11.dll", "dxgi.dll"]
        .into_iter()
        .map(|name| FileMapping {
            source: PathBuf::from(name),
            target: PathBuf::from(name),
        })
        .collect()
}

fn default_environment() -> BTreeMap<String, String> {
    Preset::Balanced.environment()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_parent_components() {
        let mut config = Config::new(PathBuf::from("game"), PathBuf::from("payload"));
        config.files[0].target = PathBuf::from("../d3d12.dll");
        assert!(config.validate().is_err());
    }

    #[test]
    fn config_round_trip() {
        let mut config = Config::new(PathBuf::from("C:/game"), PathBuf::from("runtime"));
        config.vkd3d_repo = "example/vkd3d-custom".to_owned();
        config.dxvk_repo = "example/dxvk-custom".to_owned();
        let text = toml::to_string(&config).unwrap();
        let decoded: Config = toml::from_str(&text).unwrap();
        assert_eq!(decoded.files, config.files);
        assert_eq!(decoded.vkd3d_repo, config.vkd3d_repo);
        assert_eq!(decoded.dxvk_repo, config.dxvk_repo);
        assert!(text.contains("vkd3d-repo"));
        assert!(text.contains("dxvk-repo"));
    }

    #[test]
    fn rejects_case_insensitive_duplicate_targets() {
        let mut config = Config::new(PathBuf::from("game"), PathBuf::from("payload"));
        config.files.push(FileMapping {
            source: PathBuf::from("another.dll"),
            target: PathBuf::from("D3D12.DLL"),
        });
        assert!(config.validate().is_err());
    }
}
