use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;

#[cfg(windows)]
use winreg::{RegKey, enums::*};

const MSFS_2024_APP_ID: &str = "2537590";
const MSFS_2020_APP_ID: &str = "1250410";
const GAME_EXECUTABLES: &[&str] = &["FlightSimulator2024.exe", "FlightSimulator.exe"];
const XBOX_PACKAGE_NAMES: &[&str] = &["Microsoft.Limitless", "Microsoft.FlightSimulator"];
const XBOX_FALLBACK_FOLDERS: &[&str] = &[
    "Microsoft Flight Simulator 2024",
    "Microsoft Flight Simulator",
];

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
    sort_installations(&mut found);
    Ok(found)
}

fn discover_steam_installations() -> Result<Vec<GameInstallation>> {
    discover_from_steam_roots(&steam_roots())
}

fn steam_roots() -> BTreeSet<PathBuf> {
    let mut roots = BTreeSet::new();
    roots.extend(steam_roots_from_registry());
    if let Some(program_files) = env::var_os("ProgramFiles(x86)") {
        roots.insert(PathBuf::from(program_files).join("Steam"));
    }
    if let Some(program_files) = env::var_os("ProgramFiles") {
        roots.insert(PathBuf::from(program_files).join("Steam"));
    }
    if let Some(home) = env::var_os("USERPROFILE") {
        roots.insert(PathBuf::from(home).join("AppData/Local/Steam"));
    }
    roots.extend(drive_roots().into_iter().flat_map(|drive| {
        [
            drive.join("Steam"),
            drive.join("SteamLibrary"),
            drive.join("Games").join("Steam"),
        ]
    }));
    roots
}

#[cfg(windows)]
fn steam_roots_from_registry() -> BTreeSet<PathBuf> {
    let mut roots = BTreeSet::new();
    for (hive, subkey) in [
        (HKEY_CURRENT_USER, r"Software\Valve\Steam"),
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\Valve\Steam"),
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\WOW6432Node\Valve\Steam"),
    ] {
        let Ok(key) = RegKey::predef(hive).open_subkey(subkey) else {
            continue;
        };
        for value_name in ["SteamPath", "InstallPath"] {
            if let Ok(value) = key.get_value::<String, _>(value_name) {
                insert_path_string(&mut roots, &value);
            }
        }
        if let Ok(value) = key.get_value::<String, _>("SteamExe") {
            if let Some(parent) = path_from_string(&value).and_then(|path| parent_path(&path)) {
                roots.insert(parent);
            }
        }
    }
    roots
}

#[cfg(not(windows))]
fn steam_roots_from_registry() -> BTreeSet<PathBuf> {
    BTreeSet::new()
}

#[cfg(windows)]
fn discover_xbox_installations() -> Result<Vec<GameInstallation>> {
    let mut found = Vec::new();
    let metadata = xbox_game_metadata();
    let mut executable_names = executable_name_set();
    let mut folder_names = XBOX_FALLBACK_FOLDERS
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<BTreeSet<_>>();

    for game in &metadata {
        executable_names.extend(game.executable_names.iter().cloned());
        folder_names.extend(game.folder_names.iter().cloned());
    }

    found.extend(discover_xbox_from_library_roots(
        &xbox_library_roots(),
        &folder_names,
        &executable_names,
    )?);

    for root in xbox_package_roots(&metadata)
        .into_iter()
        .filter(|root| !is_windows_apps_path(root))
    {
        add_installation_from_dir(&mut found, "xbox", &root, &executable_names);
        add_installation_from_dir(&mut found, "xbox", &root.join("Content"), &executable_names);
    }

    sort_installations(&mut found);
    Ok(found)
}

#[cfg(not(windows))]
fn discover_xbox_installations() -> Result<Vec<GameInstallation>> {
    Ok(Vec::new())
}

#[cfg(windows)]
#[derive(Debug, Default)]
struct XboxGameMetadata {
    package_full_name: String,
    package_name: Option<String>,
    executable_names: BTreeSet<String>,
    folder_names: BTreeSet<String>,
}

#[cfg(windows)]
fn xbox_game_metadata() -> Vec<XboxGameMetadata> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let Ok(game_config) = hklm.open_subkey(r"SOFTWARE\Microsoft\GamingServices\GameConfig") else {
        return Vec::new();
    };

    let mut games = Vec::new();
    for subkey_name in game_config.enum_keys().filter_map(std::result::Result::ok) {
        let Ok(key) = game_config.open_subkey(&subkey_name) else {
            continue;
        };
        let package_name = key.get_value::<String, _>("Name").ok();
        if !package_name.as_deref().is_some_and(is_target_xbox_package)
            && !is_target_xbox_package(&subkey_name)
        {
            continue;
        }

        let mut game = XboxGameMetadata {
            package_full_name: subkey_name,
            package_name,
            ..XboxGameMetadata::default()
        };

        if let Ok(shell) = key.open_subkey("ShellVisuals") {
            if let Ok(display_name) = shell.get_value::<String, _>("DefaultDisplayName") {
                if !display_name.starts_with("ms-resource:") {
                    game.folder_names.insert(display_name);
                }
            }
        }

        if let Ok(executables) = key.open_subkey("Executable") {
            for executable_key_name in executables.enum_keys().filter_map(std::result::Result::ok) {
                let Ok(executable_key) = executables.open_subkey(executable_key_name) else {
                    continue;
                };
                if let Ok(name) = executable_key.get_value::<String, _>("Name") {
                    if name.to_ascii_lowercase().ends_with(".exe") {
                        game.executable_names.insert(name);
                    }
                }
            }
        }

        games.push(game);
    }
    games
}

#[cfg(windows)]
fn xbox_package_roots(metadata: &[XboxGameMetadata]) -> BTreeSet<PathBuf> {
    let mut roots = BTreeSet::new();
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let Ok(repository) =
        hklm.open_subkey(r"SOFTWARE\Microsoft\GamingServices\PackageRepository\Root")
    else {
        return roots;
    };

    let packages = metadata
        .iter()
        .flat_map(|game| {
            game.package_name
                .iter()
                .chain(std::iter::once(&game.package_full_name))
        })
        .collect::<BTreeSet<_>>();

    for root_key_name in repository.enum_keys().filter_map(std::result::Result::ok) {
        let Ok(root_key) = repository.open_subkey(root_key_name) else {
            continue;
        };
        for package_key_name in root_key.enum_keys().filter_map(std::result::Result::ok) {
            let Ok(package_key) = root_key.open_subkey(&package_key_name) else {
                continue;
            };
            let package = package_key
                .get_value::<String, _>("Package")
                .unwrap_or(package_key_name);
            if !packages
                .iter()
                .any(|known| package_starts_with(&package, known))
                && !is_target_xbox_package(&package)
            {
                continue;
            }
            if let Ok(root) = package_key.get_value::<String, _>("Root") {
                insert_path_string(&mut roots, &root);
            }
        }
    }
    roots
}

#[cfg(windows)]
fn xbox_library_roots() -> BTreeSet<PathBuf> {
    let mut roots = BTreeSet::new();
    for drive in drive_roots() {
        let marker = drive.join(".GamingRoot");
        if let Ok(bytes) = fs::read(marker) {
            if let Some(path) = gaming_root_relative_path(&bytes) {
                roots.insert(if path.is_absolute() {
                    path
                } else {
                    drive.join(path)
                });
            }
        }
        let default = drive.join("XboxGames");
        if default.is_dir() {
            roots.insert(default);
        }
    }
    roots
}

fn discover_xbox_from_library_roots(
    roots: &BTreeSet<PathBuf>,
    folder_names: &BTreeSet<String>,
    executable_names: &BTreeSet<String>,
) -> Result<Vec<GameInstallation>> {
    let mut found = Vec::new();
    for root in roots {
        for folder in folder_names {
            let game_dir = root.join(folder).join("Content");
            add_installation_from_dir(&mut found, "xbox", &game_dir, executable_names);
            let game_dir = root.join(folder);
            add_installation_from_dir(&mut found, "xbox", &game_dir, executable_names);
        }
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };
        for entry in entries {
            let entry = entry.with_context(|| format!("failed to inspect {}", root.display()))?;
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_dir() {
                continue;
            }
            let game_dir = entry.path().join("Content");
            add_installation_from_dir(&mut found, "xbox", &game_dir, executable_names);
            let game_dir = entry.path();
            add_installation_from_dir(&mut found, "xbox", &game_dir, executable_names);
        }
    }
    sort_installations(&mut found);
    Ok(found)
}

fn discover_from_steam_roots(roots: &BTreeSet<PathBuf>) -> Result<Vec<GameInstallation>> {
    let libraries = steam_libraries_from_roots(roots)?;
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
            add_installation_from_dir(&mut found, "steam", &game_dir, &executable_name_set());
        }
    }
    sort_installations(&mut found);
    Ok(found)
}

fn steam_libraries_from_roots(roots: &BTreeSet<PathBuf>) -> Result<BTreeSet<PathBuf>> {
    let mut libraries = roots.clone();
    for root in roots {
        for vdf in [
            root.join("steamapps").join("libraryfolders.vdf"),
            root.join("libraryfolders.vdf"),
        ] {
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
    }
    Ok(libraries)
}

fn add_installation_from_dir(
    found: &mut Vec<GameInstallation>,
    store: &'static str,
    game_dir: &Path,
    executable_names: &BTreeSet<String>,
) {
    for exe_name in executable_names {
        let executable = game_dir.join(exe_name);
        if executable.is_file() {
            found.push(GameInstallation {
                store,
                game_dir: game_dir.to_path_buf(),
                executable,
            });
            break;
        }
    }
}

fn executable_name_set() -> BTreeSet<String> {
    GAME_EXECUTABLES
        .iter()
        .map(|name| (*name).to_owned())
        .collect()
}

fn sort_installations(found: &mut Vec<GameInstallation>) {
    found.sort_by(|left, right| {
        left.game_dir
            .cmp(&right.game_dir)
            .then_with(|| left.store.cmp(right.store))
    });
    found.dedup_by(|left, right| left.game_dir == right.game_dir);
}

fn insert_path_string(paths: &mut BTreeSet<PathBuf>, value: &str) {
    if let Some(path) = path_from_string(value) {
        paths.insert(path);
    }
}

fn path_from_string(value: &str) -> Option<PathBuf> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let trimmed = trimmed.strip_prefix(r"\\?\").unwrap_or(trimmed);
    Some(PathBuf::from(trimmed))
}

fn parent_path(path: &Path) -> Option<PathBuf> {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(Path::to_path_buf)
}

fn is_windows_apps_path(path: &Path) -> bool {
    path.components().any(|component| {
        component
            .as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case("WindowsApps")
    })
}

#[cfg(windows)]
fn is_target_xbox_package(package: &str) -> bool {
    XBOX_PACKAGE_NAMES
        .iter()
        .any(|target| package_starts_with(package, target))
}

#[cfg(windows)]
fn package_starts_with(package: &str, target: &str) -> bool {
    package.eq_ignore_ascii_case(target)
        || package
            .get(..target.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(target))
            && package.as_bytes().get(target.len()) == Some(&b'_')
}

fn gaming_root_relative_path(bytes: &[u8]) -> Option<PathBuf> {
    let start = if bytes.starts_with(b"RGBX") && bytes.len() >= 8 {
        8
    } else {
        0
    };
    let mut code_units = Vec::new();
    for chunk in bytes[start..].chunks_exact(2) {
        let code_unit = u16::from_le_bytes([chunk[0], chunk[1]]);
        if code_unit == 0 {
            break;
        }
        code_units.push(code_unit);
    }
    if code_units.is_empty() {
        return None;
    }
    let path = String::from_utf16(&code_units).ok()?;
    path_from_string(&path)
}

fn drive_roots() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        (b'A'..=b'Z')
            .map(|letter| PathBuf::from(format!("{}:\\", char::from(letter))))
            .filter(|path| path.is_dir())
            .collect()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
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
    GAME_EXECUTABLES
        .iter()
        .any(|executable| path.join(executable).is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_vdf_paths() {
        let input = r#""0" { "path" "D:\\SteamLibrary" }"#;
        assert_eq!(quoted_tokens(input), ["0", "path", "D:\\SteamLibrary"]);
    }

    #[test]
    fn parses_gaming_root_path() {
        let mut bytes = b"RGBX\x01\0\0\0".to_vec();
        bytes.extend("Games\\Xbox".encode_utf16().flat_map(u16::to_le_bytes));
        bytes.extend([0, 0]);

        assert_eq!(
            gaming_root_relative_path(&bytes),
            Some(PathBuf::from("Games\\Xbox"))
        );
    }

    #[test]
    fn discovers_steam_installation_in_custom_library() {
        let temp = tempfile::tempdir().unwrap();
        let steam_root = temp.path().join("client");
        let library = temp.path().join("custom-steam-library");
        let game_dir = library
            .join("steamapps")
            .join("common")
            .join("MicrosoftFlightSimulator");
        fs::create_dir_all(steam_root.join("steamapps")).unwrap();
        fs::create_dir_all(&game_dir).unwrap();
        fs::write(game_dir.join("FlightSimulator2024.exe"), b"exe").unwrap();
        fs::write(
            steam_root.join("steamapps").join("libraryfolders.vdf"),
            format!(
                r#""libraryfolders" {{ "1" {{ "path" "{}" }} }}"#,
                library.display().to_string().replace('\\', "\\\\")
            ),
        )
        .unwrap();
        fs::write(
            library
                .join("steamapps")
                .join(format!("appmanifest_{MSFS_2024_APP_ID}.acf")),
            r#""AppState" { "installdir" "MicrosoftFlightSimulator" }"#,
        )
        .unwrap();

        let found = discover_from_steam_roots(&BTreeSet::from([steam_root])).unwrap();

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].store, "steam");
        assert_eq!(found[0].game_dir, game_dir);
    }

    #[test]
    fn discovers_xbox_installation_in_custom_library_root() {
        let temp = tempfile::tempdir().unwrap();
        let library = temp.path().join("not-xboxgames");
        let game_dir = library.join("Any Folder Name").join("Content");
        fs::create_dir_all(&game_dir).unwrap();
        fs::write(game_dir.join("FlightSimulator2024.exe"), b"exe").unwrap();

        let found = discover_xbox_from_library_roots(
            &BTreeSet::from([library]),
            &BTreeSet::new(),
            &executable_name_set(),
        )
        .unwrap();

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].store, "xbox");
        assert_eq!(found[0].game_dir, game_dir);
    }

    #[cfg(windows)]
    #[test]
    fn identifies_windows_apps_package_paths() {
        assert!(is_windows_apps_path(Path::new(
            r"C:\Program Files\WindowsApps\Microsoft.Limitless_1.0.0.0_x64__8wekyb3d8bbwe"
        )));
        assert!(!is_windows_apps_path(Path::new(
            r"D:\Games\Microsoft Flight Simulator 2024\Content"
        )));
    }
}
