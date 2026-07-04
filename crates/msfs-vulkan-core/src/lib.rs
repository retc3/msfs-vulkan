//! Reversible deployment and launch primitives for MSFS 2024 Vulkan experiments.

pub mod config;
pub mod deployment;
pub mod discovery;
pub mod download;
pub mod launch;
pub mod state;

pub use config::{Config, FileMapping, Preset};
pub use deployment::{Deployment, DeploymentStatus, FileStatus};
pub use discovery::{GameInstallation, discover_installations};
pub use launch::{LaunchOptions, LaunchResult, launch};
