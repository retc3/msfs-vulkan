use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::Config;
use crate::deployment::{Deployment, DeploymentStatus};

#[derive(Debug, Clone, Default)]
pub struct LaunchOptions {
    pub arguments: Vec<String>,
    pub wait: bool,
    pub allow_uninstalled: bool,
    /// Drop msfs-vulkan-debug.conf next to the game exe so the tailored vkd3d/dxvk
    /// builds write full logs (env vars can't be passed to MSFS). Only meaningful
    /// with debug-capable sources; the GUI gates on that.
    pub debug: bool,
}

/// Name of the env-var-free debug trigger file read by the tailored vkd3d/dxvk builds.
pub const DEBUG_CONF_NAME: &str = "msfs-vulkan-debug.conf";

#[derive(Debug, Clone, Serialize)]
pub struct LaunchResult {
    pub process_id: u32,
    pub exit_code: Option<i32>,
    pub stdout_log: PathBuf,
    pub stderr_log: PathBuf,
    /// Directory the tailored builds were told to write vkd3d.log / dxvk.log into,
    /// when launched with debugging enabled.
    pub debug_log_dir: Option<PathBuf>,
}

/// Launch the configured simulator executable with translation-layer environment variables.
///
/// # Errors
///
/// Returns an error when deployment verification, log creation, or process launch fails.
pub fn launch(config: &Config, options: &LaunchOptions) -> Result<LaunchResult> {
    let deployment = Deployment::new(config)?;
    if !options.allow_uninstalled
        && !matches!(deployment.status()?, DeploymentStatus::Installed { .. })
    {
        bail!("translation layer is not installed and healthy; run install first");
    }

    let executable = config.executable_path();
    if !executable.is_file() {
        bail!("game executable was not found: {}", executable.display());
    }

    let store = deployment.store();
    fs::create_dir_all(store.log_dir())?;
    fs::create_dir_all(store.cache_dir())?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before the Unix epoch")?
        .as_secs();
    let stdout_log = store.log_dir().join(format!("{stamp}-stdout.log"));
    let stderr_log = store.log_dir().join(format!("{stamp}-stderr.log"));

    // Env-var-free debug trigger: MSFS won't inherit VKD3D_DEBUG/DXVK_LOG_LEVEL,
    // so the tailored builds instead read msfs-vulkan-debug.conf next to the exe.
    // Write a fresh per-launch log dir and point the conf at it; clean up on non-debug
    // launches so a stale conf never forces logging.
    let debug_conf = config.game_dir.join(DEBUG_CONF_NAME);
    let debug_log_dir = if options.debug {
        let dir = store.log_dir().join(format!("{stamp}-debug"));
        fs::create_dir_all(&dir)?;
        fs::write(&debug_conf, format!("log_dir={}\n", dir.display())).with_context(|| {
            format!("failed to write debug trigger {}", debug_conf.display())
        })?;
        Some(dir)
    } else {
        if debug_conf.exists() {
            let _ = fs::remove_file(&debug_conf);
        }
        None
    };

    let stdout = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&stdout_log)?;
    let stderr = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&stderr_log)?;

    let mut command = Command::new(&executable);
    command
        .current_dir(&config.game_dir)
        .args(&options.arguments)
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    for (key, value) in &config.environment {
        command.env(key, value);
    }
    command.env("VKD3D_SHADER_CACHE_PATH", store.cache_dir());

    let mut child = command
        .spawn()
        .with_context(|| format!("failed to launch {}", executable.display()))?;
    let process_id = child.id();
    let exit_code = if options.wait {
        child
            .wait()
            .context("failed while waiting for MSFS 2024")?
            .code()
    } else {
        None
    };
    Ok(LaunchResult {
        process_id,
        exit_code,
        stdout_log,
        stderr_log,
        debug_log_dir,
    })
}
