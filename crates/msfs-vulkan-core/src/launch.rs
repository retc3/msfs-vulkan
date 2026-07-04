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
}

#[derive(Debug, Clone, Serialize)]
pub struct LaunchResult {
    pub process_id: u32,
    pub exit_code: Option<i32>,
    pub stdout_log: PathBuf,
    pub stderr_log: PathBuf,
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
    })
}
