use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamScanResult {
    pub steam_detected: bool,
    pub libraries_scanned: usize,
    pub library_paths: Vec<String>,
    pub games: Vec<SteamGame>,
    pub issues: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamGame {
    pub app_id: String,
    pub name: String,
    pub library_path: String,
    pub install_dir: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamLaunchResult {
    pub process_id: u32,
}

#[derive(Debug, PartialEq)]
enum VdfToken {
    Value(String),
    Open,
    Close,
}

pub fn scan_steam_library() -> Result<SteamScanResult, String> {
    #[cfg(target_os = "linux")]
    {
        scan_linux_steam_library()
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err("Steam library scanning is currently supported on Linux only.".to_string())
    }
}

pub fn launch_steam_game(input: &str) -> Result<SteamLaunchResult, String> {
    #[cfg(target_os = "linux")]
    {
        let app_id = validate_app_id(input)?;
        let scan = scan_steam_library()?;
        ensure_installed_game(&scan, &app_id)?;

        let uri = format!("steam://rungameid/{app_id}");
        let process = std::process::Command::new("xdg-open")
            .arg(uri)
            .spawn()
            .map_err(|error| {
                format!("Could not ask the desktop to open Steam for app {app_id}: {error}")
            })?;

        Ok(SteamLaunchResult {
            process_id: process.id(),
        })
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = input;
        Err("Steam game launching is currently supported on Linux only.".to_string())
    }
}

fn ensure_installed_game(scan: &SteamScanResult, app_id: &str) -> Result<(), String> {
    if scan.games.iter().any(|game| game.app_id == app_id) {
        Ok(())
    } else {
        Err("This game is not currently listed as installed in a Steam library.".to_string())
    }
}

fn validate_app_id(input: &str) -> Result<String, String> {
    let app_id = input
        .trim()
        .parse::<u32>()
        .map_err(|_| "Steam app ID must be a positive numeric ID.".to_string())?;
    if app_id == 0 {
        return Err("Steam app ID must be a positive numeric ID.".to_string());
    }
    Ok(app_id.to_string())
}

#[cfg(target_os = "linux")]
fn scan_linux_steam_library() -> Result<SteamScanResult, String> {
    use std::{env, path::PathBuf};

    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "Could not determine the current user's home directory.".to_string())?;
    scan_steam_home(&home)
}

#[cfg(target_os = "linux")]
fn scan_steam_home(home: &std::path::Path) -> Result<SteamScanResult, String> {
    use std::{collections::HashSet, fs, path::PathBuf};

    let candidates = [
        home.join(".steam/steam"),
        home.join(".steam/root"),
        home.join(".local/share/Steam"),
        home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
        home.join("snap/steam/common/.local/share/Steam"),
    ];

    let mut issues = Vec::new();
    let mut seen_libraries = HashSet::new();
    let mut libraries = Vec::new();

    for candidate in candidates {
        if !candidate.join("steamapps").is_dir() {
            continue;
        }

        match candidate.canonicalize() {
            Ok(path) if seen_libraries.insert(path.clone()) => libraries.push(path),
            Ok(_) => {}
            Err(error) => issues.push(format!(
                "Could not access Steam installation '{}': {error}",
                candidate.display()
            )),
        }
    }

    let steam_detected = !libraries.is_empty();
    let roots = libraries.clone();

    for root in roots {
        let config_path = root.join("steamapps/libraryfolders.vdf");
        if !config_path.is_file() {
            continue;
        }

        let contents = match fs::read_to_string(&config_path) {
            Ok(contents) => contents,
            Err(error) => {
                issues.push(format!(
                    "Could not read Steam library list '{}': {error}",
                    config_path.display()
                ));
                continue;
            }
        };

        let tokens = match tokenize_vdf(&contents) {
            Ok(tokens) => tokens,
            Err(error) => {
                issues.push(format!(
                    "Could not parse Steam library list '{}': {error}",
                    config_path.display()
                ));
                continue;
            }
        };

        for value in values_at_depth(&tokens, "path", 2) {
            let library = PathBuf::from(value);
            if !library.is_absolute() {
                issues.push(format!(
                    "Ignored a non-absolute Steam library path in '{}'.",
                    config_path.display()
                ));
                continue;
            }
            if !library.is_dir() {
                issues.push(format!(
                    "Configured Steam library '{}' is unavailable.",
                    library.display()
                ));
                continue;
            }

            match library.canonicalize() {
                Ok(path) => {
                    if seen_libraries.insert(path.clone()) {
                        libraries.push(path);
                    }
                }
                Err(error) => issues.push(format!(
                    "Could not access Steam library '{}': {error}",
                    library.display()
                )),
            }
        }
    }

    let mut games_by_id = std::collections::HashMap::new();
    let mut libraries_scanned = 0;
    let mut scanned_library_paths = Vec::new();

    for library in &libraries {
        let steamapps = library.join("steamapps");
        if !steamapps.is_dir() {
            continue;
        }

        let entries = match fs::read_dir(&steamapps) {
            Ok(entries) => entries,
            Err(error) => {
                issues.push(format!(
                    "Could not read Steam library '{}': {error}",
                    steamapps.display()
                ));
                continue;
            }
        };
        libraries_scanned += 1;
        scanned_library_paths.push(library.display().to_string());

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    issues.push(format!(
                        "Could not read an entry in '{}': {error}",
                        steamapps.display()
                    ));
                    continue;
                }
            };

            let path = entry.path();
            let Some(filename) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !filename.starts_with("appmanifest_")
                || path.extension().and_then(|extension| extension.to_str()) != Some("acf")
            {
                continue;
            }

            let contents = match fs::read_to_string(&path) {
                Ok(contents) => contents,
                Err(error) => {
                    issues.push(format!(
                        "Could not read Steam app manifest '{}': {error}",
                        path.display()
                    ));
                    continue;
                }
            };
            let tokens = match tokenize_vdf(&contents) {
                Ok(tokens) => tokens,
                Err(error) => {
                    issues.push(format!(
                        "Could not parse Steam app manifest '{}': {error}",
                        path.display()
                    ));
                    continue;
                }
            };

            let Some(app_id) = first_value_at_depth(&tokens, "appid", 1) else {
                issues.push(format!(
                    "Steam app manifest '{}' has no app ID.",
                    path.display()
                ));
                continue;
            };
            let app_id = match validate_app_id(&app_id) {
                Ok(app_id) => app_id,
                Err(_) => {
                    issues.push(format!(
                        "Steam app manifest '{}' has an invalid app ID.",
                        path.display()
                    ));
                    continue;
                }
            };
            let Some(name) = first_value_at_depth(&tokens, "name", 1) else {
                issues.push(format!(
                    "Steam app manifest '{}' has no app name.",
                    path.display()
                ));
                continue;
            };
            let Some(state_flags) = first_value_at_depth(&tokens, "StateFlags", 1)
                .and_then(|flags| flags.parse::<u32>().ok())
            else {
                issues.push(format!(
                    "Steam app manifest '{}' has invalid installation state.",
                    path.display()
                ));
                continue;
            };

            if state_flags & 4 == 0 {
                continue;
            }

            let install_dir = first_value_at_depth(&tokens, "installdir", 1);
            let game = SteamGame {
                app_id: app_id.clone(),
                name,
                library_path: library.display().to_string(),
                install_dir,
            };
            games_by_id.entry(app_id).or_insert(game);
        }
    }

    let mut games: Vec<_> = games_by_id.into_values().collect();
    games.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.app_id.cmp(&right.app_id))
    });

    Ok(SteamScanResult {
        steam_detected,
        libraries_scanned,
        library_paths: scanned_library_paths,
        games,
        issues,
    })
}

fn tokenize_vdf(text: &str) -> Result<Vec<VdfToken>, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut chars = text.chars().peekable();
    let mut tokens = Vec::new();
    let mut depth = 0usize;

    while let Some(character) = chars.next() {
        match character {
            character if character.is_whitespace() => {}
            '/' if chars.peek() == Some(&'/') => {
                for comment_character in chars.by_ref() {
                    if comment_character == '\n' {
                        break;
                    }
                }
            }
            '{' => {
                depth += 1;
                tokens.push(VdfToken::Open);
            }
            '}' => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| "Unexpected closing brace.".to_string())?;
                tokens.push(VdfToken::Close);
            }
            '"' => {
                let mut value = String::new();
                let mut escaped = false;
                let mut closed = false;

                for character in chars.by_ref() {
                    if escaped {
                        match character {
                            '"' | '\\' => value.push(character),
                            other => {
                                value.push('\\');
                                value.push(other);
                            }
                        }
                        escaped = false;
                    } else {
                        match character {
                            '\\' => escaped = true,
                            '"' => {
                                closed = true;
                                break;
                            }
                            other => value.push(other),
                        }
                    }
                }

                if escaped || !closed {
                    return Err("Unterminated quoted value.".to_string());
                }
                tokens.push(VdfToken::Value(value));
            }
            first => {
                let mut value = String::from(first);
                while let Some(&next) = chars.peek() {
                    if next.is_whitespace() || next == '{' || next == '}' {
                        break;
                    }
                    value.push(chars.next().expect("peeked character exists"));
                }
                tokens.push(VdfToken::Value(value));
            }
        }
    }

    if depth != 0 {
        return Err("Unclosed brace.".to_string());
    }
    Ok(tokens)
}

fn values_at_depth(tokens: &[VdfToken], field: &str, target_depth: usize) -> Vec<String> {
    let mut values = Vec::new();
    let mut depth = 0usize;
    let mut index = 0;

    while index < tokens.len() {
        match &tokens[index] {
            VdfToken::Value(key) => {
                if let Some(VdfToken::Value(value)) = tokens.get(index + 1) {
                    if depth == target_depth && key == field {
                        values.push(value.clone());
                    }
                    index += 2;
                    continue;
                }
                if matches!(tokens.get(index + 1), Some(VdfToken::Open)) {
                    depth += 1;
                    index += 2;
                    continue;
                }
                index += 1;
            }
            VdfToken::Open => {
                depth += 1;
                index += 1;
            }
            VdfToken::Close => {
                depth = depth.saturating_sub(1);
                index += 1;
            }
        }
    }

    values
}

fn first_value_at_depth(tokens: &[VdfToken], field: &str, depth: usize) -> Option<String> {
    values_at_depth(tokens, field, depth).into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    struct TempHome(PathBuf);

    impl TempHome {
        fn new() -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "ws-control-center-steam-test-{}-{unique}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempHome {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn reads_library_paths_from_nested_vdf_entries() {
        let vdf = r#"
            "libraryfolders"
            {
                "0" { "path" "/home/player/.local/share/Steam" }
                "1" { "path" "/mnt/Games/Steam Library" }
            }
        "#;
        let tokens = tokenize_vdf(vdf).unwrap();
        assert_eq!(
            values_at_depth(&tokens, "path", 2),
            vec![
                "/home/player/.local/share/Steam".to_string(),
                "/mnt/Games/Steam Library".to_string()
            ]
        );
    }

    #[test]
    fn reads_manifest_fields_at_the_root_and_unescapes_values() {
        let vdf = r#"
            "AppState"
            {
                "appid" "12345"
                "name" "Game \"With Quotes\""
                "StateFlags" "4"
                "nested" { "name" "Not the app name" }
            }
        "#;
        let tokens = tokenize_vdf(vdf).unwrap();
        assert_eq!(
            first_value_at_depth(&tokens, "appid", 1).as_deref(),
            Some("12345")
        );
        assert_eq!(
            first_value_at_depth(&tokens, "name", 1).as_deref(),
            Some("Game \"With Quotes\"")
        );
        assert_eq!(
            first_value_at_depth(&tokens, "StateFlags", 1).as_deref(),
            Some("4")
        );
    }

    #[test]
    fn rejects_malformed_vdf() {
        assert!(tokenize_vdf(r#""root" { "key" "value""#).is_err());
        assert!(tokenize_vdf(r#""root" }"#).is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn scans_installed_games_across_configured_libraries_without_duplicates() {
        let home = TempHome::new();
        let steam = home.0.join(".local/share/Steam");
        let extra_library = home.0.join("Games/SteamLibrary");
        let steamapps = steam.join("steamapps");
        let extra_steamapps = extra_library.join("steamapps");
        fs::create_dir_all(&steamapps).unwrap();
        fs::create_dir_all(&extra_steamapps).unwrap();

        fs::write(
            steamapps.join("libraryfolders.vdf"),
            format!(
                r#""libraryfolders" {{
                    "0" {{ "path" "{}" }}
                    "1" {{ "path" "{}" }}
                }}"#,
                steam.display(),
                extra_library.display()
            ),
        )
        .unwrap();
        fs::write(
            steamapps.join("appmanifest_100.acf"),
            r#""AppState" { "appid" "100" "name" "Alpha" "StateFlags" "4" "installdir" "Alpha" }"#,
        )
        .unwrap();
        fs::write(
            steamapps.join("appmanifest_200.acf"),
            r#""AppState" { "appid" "200" "name" "Not Installed" "StateFlags" "2" }"#,
        )
        .unwrap();
        fs::write(
            extra_steamapps.join("appmanifest_300.acf"),
            r#""AppState" { "appid" "300" "name" "Beta" "StateFlags" "4" "installdir" "Beta" }"#,
        )
        .unwrap();

        let scan = scan_steam_home(&home.0).unwrap();

        assert!(scan.steam_detected);
        assert_eq!(scan.libraries_scanned, 2);
        assert_eq!(scan.library_paths.len(), 2);
        assert_eq!(scan.games.len(), 2);
        assert_eq!(scan.games[0].name, "Alpha");
        assert_eq!(scan.games[1].name, "Beta");
        assert!(ensure_installed_game(&scan, "100").is_ok());
        assert!(ensure_installed_game(&scan, "200").is_err());
        assert!(scan.issues.is_empty());
    }

    #[test]
    fn validates_steam_app_ids_before_building_a_launch_request() {
        assert_eq!(validate_app_id("730").unwrap(), "730");
        assert_eq!(validate_app_id(" 730 ").unwrap(), "730");
        assert!(validate_app_id("0").is_err());
        assert!(validate_app_id("730;echo unsafe").is_err());
        assert!(validate_app_id("999999999999").is_err());
    }
}
