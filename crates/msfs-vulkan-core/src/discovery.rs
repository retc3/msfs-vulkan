use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;

const MSFS_2024_APP_ID: &str = "2537590";
const MSFS_2020_APP_ID: &str = "1250410";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GameInstallation {
    pub store: &'static str,
    pub game_dir: PathBuf,
    pub executable: PathBuf,
}

/// Discover accessible Steam and Xbox/MS Store installations on local drives.
///
/// # Errors
///
/// Returns an error when a discovered Steam or Xbox library cannot be inspected.
pub fn discover_installations() -> Result<Vec<GameInstallation>> {
    let mut found = discover_steam_installations()?;
    found.extend(discover_xbox_installations()?);
    found.sort_by(|left, right| left.game_dir.cmp(&right.game_dir));
    found.dedup_by(|left, right| left.game_dir == right.game_dir);
    Ok(found)
}

fn discover_steam_installations() -> Result<Vec<GameInstallation>> {
    let mut steam_roots = BTreeSet::new();
    if let Some(program_files) = env::var_os("ProgramFiles(x86)") {
        steam_roots.insert(PathBuf::from(program_files).join("Steam"));
    }
    if let Some(program_files) = env::var_os("ProgramFiles") {
        steam_roots.insert(PathBuf::from(program_files).join("Steam"));
    }
    if let Some(home) = env::var_os("USERPROFILE") {
        steam_roots.insert(PathBuf::from(home).join("AppData/Local/Steam"));
    }
    discover_from_roots(&steam_roots)
}

fn discover_xbox_installations() -> Result<Vec<GameInstallation>> {
    let mut found = Vec::new();
    for letter in b'A'..=b'Z' {
        let xbox_games = PathBuf::from(format!("{}:\\XboxGames", char::from(letter)));
        if !xbox_games.is_dir() {
            continue;
        }
        for entry in fs::read_dir(&xbox_games)
            .with_context(|| format!("failed to inspect {}", xbox_games.display()))?
        {
            let entry = entry?;
            let game_dir = entry.path().join("Content");
            for exe_name in ["FlightSimulator2024.exe", "FlightSimulator.exe"] {
                let executable = game_dir.join(exe_name);
                if executable.is_file() {
                    found.push(GameInstallation {
                        store: "xbox",
                        game_dir: game_dir.clone(),
                        executable,
                    });
                    break;
                }
            }
        }
    }
    Ok(found)
}

fn discover_from_roots(roots: &BTreeSet<PathBuf>) -> Result<Vec<GameInstallation>> {
    let mut libraries = roots.clone();
    for root in roots {
        let vdf = root.join("steamapps/libraryfolders.vdf");
        if !vdf.is_file() {
            continue;
        }
        let text = fs::read_to_string(&vdf)
            .with_context(|| format!("failed to read {}", vdf.display()))?;
        let tokens = quoted_tokens(&text);
        for pair in tokens.windows(2) {
            if pair[0].eq_ignore_ascii_case("path") {
                libraries.insert(PathBuf::from(&pair[1]));
            }
        }
    }

    let mut found = Vec::new();
    for library in libraries {
        let steamapps = if library.file_name().is_some_and(|name| name == "steamapps") {
            library
        } else {
            library.join("steamapps")
        };
        for app_id in [MSFS_2024_APP_ID, MSFS_2020_APP_ID] {
            let manifest = steamapps.join(format!("appmanifest_{app_id}.acf"));
            if !manifest.is_file() {
                continue;
            }
            let Ok(text) = fs::read_to_string(&manifest) else {
                continue;
            };
            let tokens = quoted_tokens(&text);
            let Some(index) = tokens
                .iter()
                .position(|token| token.eq_ignore_ascii_case("installdir"))
            else {
                continue;
            };
            let Some(install_dir) = tokens.get(index + 1) else {
                continue;
            };
            let game_dir = steamapps.join("common").join(install_dir);
            for exe_name in ["FlightSimulator2024.exe", "FlightSimulator.exe"] {
                let executable = game_dir.join(exe_name);
                if executable.is_file() {
                    found.push(GameInstallation {
                        store: "steam",
                        game_dir: game_dir.clone(),
                        executable,
                    });
                    break;
                }
            }
        }
    }
    found.sort_by(|left, right| left.game_dir.cmp(&right.game_dir));
    found.dedup_by(|left, right| left.game_dir == right.game_dir);
    Ok(found)
}

fn quoted_tokens(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(character) = chars.next() {
        if character != '"' {
            continue;
        }
        let mut token = String::new();
        while let Some(character) = chars.next() {
            match character {
                '"' => break,
                '\\' => match chars.peek().copied() {
                    Some('\\' | '"') => token.push(chars.next().unwrap()),
                    _ => token.push('\\'),
                },
                other => token.push(other),
            }
        }
        tokens.push(token);
    }
    tokens
}

#[allow(dead_code)]
fn is_game_directory(path: &Path) -> bool {
    path.join("FlightSimulator2024.exe").is_file() || path.join("FlightSimulator.exe").is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_vdf_paths() {
        let input = r#""0" { "path" "D:\\SteamLibrary" }"#;
        assert_eq!(quoted_tokens(input), ["0", "path", "D:\\SteamLibrary"]);
    }
}
