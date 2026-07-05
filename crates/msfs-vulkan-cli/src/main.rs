use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use msfs_vulkan_core::{
    Config, Deployment, DeploymentStatus, LaunchOptions, Preset, discover_installations, launch,
    state::StateStore,
};

#[derive(Debug, Parser)]
#[command(name = "msfs-vulkan", version, about)]
struct Cli {
    /// Configuration file used by commands that operate on the game.
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum PresetArg {
    Quality,
    Balanced,
    Performance,
}

impl From<PresetArg> for Preset {
    fn from(arg: PresetArg) -> Self {
        match arg {
            PresetArg::Quality => Preset::Quality,
            PresetArg::Balanced => Preset::Balanced,
            PresetArg::Performance => Preset::Performance,
        }
    }
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Find Microsoft Flight Simulator installations.
    Discover(OutputArgs),
    /// Create a configuration file.
    Init(InitArgs),
    /// Report Vulkan devices and baseline VKD3D-Proton requirements.
    Probe(OutputArgs),
    /// Install the configured translation DLLs with verified backups.
    Install,
    /// Show whether the translation layer is installed and unchanged.
    Status(OutputArgs),
    /// Launch MSFS 2024 with the configured environment.
    Run(RunArgs),
    /// Restore original files, or remove files that did not previously exist.
    Restore(RestoreArgs),
}

#[derive(Debug, Args)]
struct OutputArgs {
    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct InitArgs {
    /// MSFS 2024 directory containing FlightSimulator2024.exe. Auto-detected when omitted.
    #[arg(long)]
    game_dir: Option<PathBuf>,
    /// Directory containing x64 d3d12.dll, d3d12core.dll, and dxgi.dll.
    #[arg(long, default_value = "runtime/x64")]
    payload_dir: PathBuf,
    /// Replace an existing configuration file.
    #[arg(long)]
    force: bool,
    /// Apply a configuration preset.
    #[arg(long, value_enum)]
    preset: Option<PresetArg>,
    /// Set a custom VKD3D-Proton repository.
    #[arg(long)]
    vkd3d_repo: Option<String>,
    /// Set a custom DXVK repository.
    #[arg(long)]
    dxvk_repo: Option<String>,
}

#[derive(Debug, Args)]
struct RunArgs {
    /// Wait for the simulator process and report its exit code.
    #[arg(long)]
    wait: bool,
    /// Permit a baseline launch without an installed translation layer.
    #[arg(long)]
    allow_uninstalled: bool,
    /// Arguments passed to FlightSimulator2024.exe.
    #[arg(last = true)]
    arguments: Vec<String>,
}

#[derive(Debug, Args)]
struct RestoreArgs {
    /// Restore even when installed DLLs were modified or removed after installation.
    #[arg(long)]
    force: bool,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let config_path = resolve_config_path(cli.config)?;
    match cli.command {
        Command::Discover(output) => discover(output.json),
        Command::Init(args) => init(&config_path, args),
        Command::Probe(output) => probe(output.json),
        Command::Install => {
            let config = load_config_or_recover(&config_path)?;
            println!("Ensuring translation runtime is downloaded...");
            msfs_vulkan_core::download::ensure_runtime(&config)?;
            Deployment::new(&config)?.install()?;
            println!(
                "installed translation layer into {}",
                config.game_dir.display()
            );
            Ok(())
        }
        Command::Status(output) => {
            let config = load_config_or_recover(&config_path)?;
            let status = Deployment::new(&config)?.status()?;
            print_status(&status, output.json)
        }
        Command::Run(args) => {
            let config = load_config_or_recover(&config_path)?;
            let result = launch(
                &config,
                &LaunchOptions {
                    arguments: args.arguments,
                    wait: args.wait,
                    allow_uninstalled: args.allow_uninstalled,
                },
            )?;
            println!("started process {}", result.process_id);
            if let Some(code) = result.exit_code {
                println!("exit code: {code}");
            }
            println!("stdout: {}", result.stdout_log.display());
            println!("stderr: {}", result.stderr_log.display());
            Ok(())
        }
        Command::Restore(args) => {
            let config = load_config_or_recover(&config_path)?;
            Deployment::new(&config)?.restore(args.force)?;
            println!("restored original files in {}", config.game_dir.display());
            Ok(())
        }
    }
}

fn resolve_config_path(path: Option<PathBuf>) -> Result<PathBuf> {
    path.map_or_else(msfs_vulkan_core::config::default_config_path, Ok)
}

fn load_config_or_recover(path: &Path) -> Result<Config> {
    if path.is_file() {
        return Config::load(path);
    }

    let default_path = msfs_vulkan_core::config::default_config_path()?;
    let legacy_path = msfs_vulkan_core::config::legacy_config_path();
    if path == default_path && legacy_path.is_file() {
        let config = Config::load(&legacy_path)?;
        config
            .save(path)
            .with_context(|| format!("failed to migrate configuration to {}", path.display()))?;
        return Ok(config);
    }

    if path == default_path {
        let config = config_from_saved_deployment()?
            .with_context(|| format!("configuration not found: {}", path.display()))?;
        config
            .save(path)
            .with_context(|| format!("failed to recover configuration to {}", path.display()))?;
        return Ok(config);
    }

    Config::load(path)
}

fn config_from_saved_deployment() -> Result<Option<Config>> {
    match StateStore::known_game_dirs()?.as_slice() {
        [] => Ok(None),
        [game_dir] => Ok(Some(Config::new(
            game_dir.clone(),
            msfs_vulkan_core::config::default_payload_dir()?,
        ))),
        _ => {
            bail!("multiple saved deployments found; provide --config or run init")
        }
    }
}

fn discover(json: bool) -> Result<()> {
    let installations = discover_installations()?;
    if json {
        println!("{}", serde_json::to_string_pretty(&installations)?);
    } else if installations.is_empty() {
        println!("no installation was found");
    } else {
        for installation in installations {
            println!(
                "{}: {}",
                installation.store,
                installation.game_dir.display()
            );
        }
    }
    Ok(())
}

fn init(path: &std::path::Path, args: InitArgs) -> Result<()> {
    if path.exists() && !args.force {
        bail!(
            "configuration already exists at {}; use --force to replace it",
            path.display()
        );
    }
    let game_dir = if let Some(game_dir) = args.game_dir {
        game_dir
    } else {
        let installations = discover_installations()?;
        match installations.as_slice() {
            [installation] => installation.game_dir.clone(),
            [] => bail!("no installation found; provide --game-dir"),
            _ => bail!("multiple installations found; provide --game-dir"),
        }
    };
    let mut config = Config::new(game_dir, args.payload_dir);
    if let Some(preset_arg) = args.preset {
        config.environment = Preset::from(preset_arg).environment();
    }
    if let Some(vkd3d_repo) = args.vkd3d_repo {
        config.vkd3d_repo = vkd3d_repo;
    }
    if let Some(dxvk_repo) = args.dxvk_repo {
        config.dxvk_repo = dxvk_repo;
    }
    config
        .save(path)
        .with_context(|| format!("failed to initialize {}", path.display()))?;
    println!("wrote {}", path.display());
    Ok(())
}

fn probe(json: bool) -> Result<()> {
    let report = msfs_vulkan_vulkan::probe()?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    println!("Vulkan loader: {}", report.loader_api_version);
    if report.devices.is_empty() {
        println!("no Vulkan physical devices found");
    }
    for device in report.devices {
        println!();
        println!("{}", device.name);
        println!("  type: {}", device.device_type);
        println!("  Vulkan: {}", device.api_version);
        println!(
            "  device-local memory: {} MiB",
            device.device_local_memory_mib
        );
        println!("  graphics queue: {}", yes_no(device.graphics_queue));
        println!("  swapchain: {}", yes_no(device.swapchain));
        println!(
            "  descriptor indexing: {}",
            yes_no(device.descriptor_indexing)
        );
        println!(
            "  update-after-bind descriptor limit: {}",
            device.max_update_after_bind_descriptors
        );
        println!(
            "  ray tracing pipeline: {}",
            yes_no(device.ray_tracing_pipeline)
        );
        println!(
            "  acceleration structure: {}",
            yes_no(device.acceleration_structure)
        );
        println!(
            "  baseline VKD3D-Proton candidate: {}",
            yes_no(device.basic_vkd3d_candidate)
        );
    }
    Ok(())
}

fn print_status(status: &DeploymentStatus, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(status)?);
        return Ok(());
    }
    match status {
        DeploymentStatus::NotInstalled => println!("not installed"),
        DeploymentStatus::Installed { files } => {
            println!("installed and verified");
            for file in files {
                println!("  {:?}: {}", file.condition, file.target.display());
            }
        }
        DeploymentStatus::Drifted { phase, files } => {
            println!("deployment requires attention (phase: {phase:?})");
            for file in files {
                println!("  {:?}: {}", file.condition, file.target.display());
            }
        }
    }
    Ok(())
}

const fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}
