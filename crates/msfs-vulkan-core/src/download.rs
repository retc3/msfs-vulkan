use std::fs::{self, File};
use std::io::{self, Cursor, Read};
use std::path::Path;

use anyhow::{Context, Result, bail};
use flate2::read::GzDecoder;
use serde::Deserialize;
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

/// Ensures the Vulkan runtime files are available in the configured runtime directory.
///
/// # Errors
///
/// Returns an error if the runtime directory cannot be created, or if a runtime
/// component cannot be downloaded, read, extracted, or written.
pub fn ensure_runtime(config: &crate::Config) -> Result<()> {
    let mut missing_vkd3d = false;
    let mut missing_dxvk = false;

    for mapping in &config.files {
        let path = config.payload_dir.join(&mapping.source);
        if !path.exists() {
            let name = mapping.source.to_string_lossy().to_lowercase();
            if name.contains("d3d12") {
                missing_vkd3d = true;
            } else if name.contains("dxgi") || name.contains("d3d11") {
                missing_dxvk = true;
            }
        }
    }

    if missing_vkd3d {
        download_and_extract(&config.vkd3d_repo, true, &config.payload_dir)?;
    }
    if missing_dxvk {
        download_and_extract(&config.dxvk_repo, false, &config.payload_dir)?;
    }

    Ok(())
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
