use std::fs::{self, File};
use std::io::{self, Cursor, Read};
use std::path::Path;

use anyhow::{Context, Result, bail};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use tar::Archive;
use ureq;
use zstd::stream::read::Decoder as ZstdDecoder;

#[derive(Deserialize)]
struct GitHubRelease {
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

const RUNTIME_MANIFEST_FILE: &str = "runtime-manifest.json";

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
struct RuntimeManifest {
    vkd3d_repo: Option<String>,
    dxvk_repo: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeComponent {
    Vkd3d,
    Dxvk,
}

/// Ensures the Vulkan runtime files are available in the configured runtime directory.
///
/// # Errors
///
/// Returns an error if the runtime directory cannot be created, or if a runtime
/// component cannot be downloaded, read, extracted, or written.
pub fn ensure_runtime(config: &crate::Config) -> Result<()> {
    fs::create_dir_all(&config.payload_dir).context("failed to create payload directory")?;

    let manifest_path = config.payload_dir.join(RUNTIME_MANIFEST_FILE);
    let mut manifest = read_runtime_manifest(&manifest_path)?;

    let mut has_vkd3d = false;
    let mut has_dxvk = false;
    let mut missing_vkd3d = false;
    let mut missing_dxvk = false;

    for mapping in &config.files {
        let path = config.payload_dir.join(&mapping.source);
        match component_for_path(&mapping.source) {
            Some(RuntimeComponent::Vkd3d) => {
                has_vkd3d = true;
                missing_vkd3d |= !path.exists();
            }
            Some(RuntimeComponent::Dxvk) => {
                has_dxvk = true;
                missing_dxvk |= !path.exists();
            }
            None => {}
        }
    }

    let vkd3d_source_changed =
        has_vkd3d && manifest.vkd3d_repo.as_deref() != Some(config.vkd3d_repo.as_str());
    let dxvk_source_changed =
        has_dxvk && manifest.dxvk_repo.as_deref() != Some(config.dxvk_repo.as_str());

    if missing_vkd3d || vkd3d_source_changed {
        download_and_extract(&config.vkd3d_repo, true, &config.payload_dir)?;
        manifest.vkd3d_repo = Some(config.vkd3d_repo.clone());
    }
    if missing_dxvk || dxvk_source_changed {
        download_and_extract(&config.dxvk_repo, false, &config.payload_dir)?;
        manifest.dxvk_repo = Some(config.dxvk_repo.clone());
    }

    write_runtime_manifest(&manifest_path, &manifest)
}

fn component_for_path(path: &Path) -> Option<RuntimeComponent> {
    let name = path.to_string_lossy().to_lowercase();
    if name.contains("d3d12") {
        Some(RuntimeComponent::Vkd3d)
    } else if name.contains("dxgi") || name.contains("d3d11") {
        Some(RuntimeComponent::Dxvk)
    } else {
        None
    }
}

fn read_runtime_manifest(path: &Path) -> Result<RuntimeManifest> {
    if !path.is_file() {
        return Ok(RuntimeManifest::default());
    }

    let bytes = fs::read(path)
        .with_context(|| format!("failed to read runtime manifest {}", path.display()))?;
    serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to parse runtime manifest {}", path.display()))
}

fn write_runtime_manifest(path: &Path, manifest: &RuntimeManifest) -> Result<()> {
    let bytes =
        serde_json::to_vec_pretty(manifest).context("failed to serialize runtime manifest")?;
    fs::write(path, bytes)
        .with_context(|| format!("failed to write runtime manifest {}", path.display()))
}

/// Downloads the latest release archive from a GitHub repository and extracts the required DLLs.
///
/// # Errors
///
/// Returns an error if the GitHub release cannot be fetched or parsed, if no
/// matching archive asset is found, or if the archive cannot be downloaded,
/// read, decoded, or extracted.
pub fn download_and_extract(repo: &str, is_vkd3d: bool, payload_dir: &Path) -> Result<()> {
    let api_url = format!("https://api.github.com/repos/{repo}/releases/latest");

    // Fetch release info
    let response = ureq::get(&api_url)
        .set("User-Agent", "msfs-vulkan-downloader")
        .call()
        .with_context(|| format!("failed to fetch latest release from {api_url}"))?;

    if response.status() != 200 {
        bail!("GitHub API returned status {}", response.status());
    }

    let release: GitHubRelease = response
        .into_json()
        .context("failed to parse GitHub release JSON")?;

    // Find the right asset
    let asset = release
        .assets
        .into_iter()
        .find(|a| {
            if is_vkd3d {
                a.name.ends_with(".tar.zst")
            } else {
                a.name.ends_with(".tar.gz")
            }
        })
        .context(format!("No matching archive asset found in {repo}"))?;

    // Download the archive
    let response = ureq::get(&asset.browser_download_url)
        .set("User-Agent", "msfs-vulkan-downloader")
        .call()
        .with_context(|| format!("failed to download {}", asset.browser_download_url))?;

    let mut reader = response.into_reader();
    let mut buffer = Vec::new();
    reader
        .read_to_end(&mut buffer)
        .context("failed to read download stream")?;

    // Ensure payload directory exists
    fs::create_dir_all(payload_dir).context("failed to create payload directory")?;

    // Extract specific files from archive
    let cursor = Cursor::new(buffer);

    if is_vkd3d {
        let decoder = ZstdDecoder::new(cursor).context("failed to initialize zstd decoder")?;
        let mut archive = Archive::new(decoder);
        extract_x64_dlls(&mut archive, payload_dir)?;
    } else {
        let decoder = GzDecoder::new(cursor);
        let mut archive = Archive::new(decoder);
        extract_x64_dlls(&mut archive, payload_dir)?;
    }

    Ok(())
}

fn extract_x64_dlls<R: Read>(archive: &mut Archive<R>, payload_dir: &Path) -> Result<()> {
    for entry in archive
        .entries()
        .context("failed to read archive entries")?
    {
        let mut entry = entry.context("failed to read archive entry")?;
        let path = entry
            .path()
            .context("failed to read entry path")?
            .to_path_buf();

        let path_str = path.to_string_lossy().to_lowercase();
        // We only want the 64-bit DLLs
        let is_dll = path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("dll"));

        if path_str.contains("x64") && is_dll {
            if let Some(filename) = path.file_name() {
                let target_path = payload_dir.join(filename);
                let mut file = File::create(&target_path)
                    .with_context(|| format!("failed to create {}", target_path.display()))?;
                io::copy(&mut entry, &mut file)
                    .with_context(|| format!("failed to write {}", target_path.display()))?;
            }
        }
    }
    Ok(())
}
