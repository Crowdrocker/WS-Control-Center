use serde::Serialize;
use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
};

use crate::steam;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentCheck {
    pub id: String,
    pub label: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mo2Environment {
    pub checks: Vec<EnvironmentCheck>,
    pub proton_tools: Vec<String>,
    pub steam_libraries_scanned: usize,
    pub warnings: Vec<String>,
    pub guidance: Vec<String>,
}

pub fn check_environment(mo2_path: Option<&str>) -> Result<Mo2Environment, String> {
    let steam = steam::scan_steam_library()?;
    let mut warnings = steam.issues;
    let proton_tools = discover_proton_tools(&steam.library_paths, &mut warnings);
    let protontricks = find_protontricks();
    let wine = find_executable(&["wine", "wine64"]);
    let pacman = find_executable(&["pacman"]);
    let mo2 = check_mo2_executable(mo2_path);

    let checks = vec![
        EnvironmentCheck {
            id: "steam".to_string(),
            label: "Steam libraries".to_string(),
            status: if steam.steam_detected {
                "found"
            } else {
                "missing"
            }
            .to_string(),
            detail: if steam.steam_detected {
                format!(
                    "Found Steam and scanned {} library folder(s).",
                    steam.libraries_scanned
                )
            } else {
                "No Steam install was found in the standard Linux locations.".to_string()
            },
        },
        EnvironmentCheck {
            id: "proton".to_string(),
            label: "Proton compatibility tools".to_string(),
            status: if proton_tools.is_empty() {
                "missing"
            } else {
                "found"
            }
            .to_string(),
            detail: if proton_tools.is_empty() {
                "No Proton folders were found in the scanned Steam libraries.".to_string()
            } else {
                format!("Found {} Proton tool(s).", proton_tools.len())
            },
        },
        EnvironmentCheck {
            id: "protontricks".to_string(),
            label: "Protontricks".to_string(),
            status: if protontricks.is_some() {
                "found"
            } else {
                "missing"
            }
            .to_string(),
            detail: protontricks
                .as_ref()
                .map(|path| format!("Found at {}.", path.display()))
                .unwrap_or_else(|| {
                    "No Protontricks executable was found in PATH or Flatpak exports.".to_string()
                }),
        },
        EnvironmentCheck {
            id: "pacman".to_string(),
            label: "Arch package manager".to_string(),
            status: if pacman.is_some() { "found" } else { "missing" }.to_string(),
            detail: pacman
                .as_ref()
                .map(|path| format!("Found at {}. No package commands were run.", path.display()))
                .unwrap_or_else(|| "No pacman executable was found in PATH.".to_string()),
        },
        EnvironmentCheck {
            id: "wine".to_string(),
            label: "System Wine (optional)".to_string(),
            status: if wine.is_some() { "found" } else { "optional" }.to_string(),
            detail: wine
                .as_ref()
                .map(|path| format!("Found at {}.", path.display()))
                .unwrap_or_else(|| {
                    "Not found. MO2 can use the selected Steam Proton prefix instead.".to_string()
                }),
        },
        mo2,
    ];

    warnings.sort();
    warnings.dedup();

    Ok(Mo2Environment {
        checks,
        proton_tools,
        steam_libraries_scanned: steam.libraries_scanned,
        warnings,
        guidance: vec![
            "Install Protontricks from a trusted Arch package source; this app will not run package commands.".to_string(),
            "Install or enable a Proton compatibility tool for the target game in Steam.".to_string(),
            "Use Protontricks to work with the target game's prefix, and keep MO2 and the game in that same prefix.".to_string(),
            "After installing MO2 manually, provide the full path to ModOrganizer.exe above to verify it.".to_string(),
        ],
    })
}

fn discover_proton_tools(libraries: &[String], warnings: &mut Vec<String>) -> Vec<String> {
    let mut candidates = Vec::new();
    for library in libraries {
        let root = PathBuf::from(library);
        candidates.extend([
            root.join("steamapps/common"),
            root.join("steamapps/compatibilitytools.d"),
            root.join("compatibilitytools.d"),
        ]);
    }

    let mut tools = HashSet::new();
    for directory in candidates {
        if !directory.is_dir() {
            continue;
        }

        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) => {
                warnings.push(format!(
                    "Could not read Proton tools in '{}': {error}",
                    directory.display()
                ));
                continue;
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    warnings.push(format!(
                        "Could not read an entry in '{}': {error}",
                        directory.display()
                    ));
                    continue;
                }
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.to_ascii_lowercase().contains("proton") && entry.path().is_dir() {
                tools.insert(name);
            }
        }
    }

    let mut tools: Vec<_> = tools.into_iter().collect();
    tools.sort_by_key(|name| name.to_ascii_lowercase());
    tools
}

fn find_protontricks() -> Option<PathBuf> {
    find_executable(&["protontricks"]).or_else(|| {
        let app_id = "com.github.Matoking.protontricks";
        let home = env::var_os("HOME").map(PathBuf::from)?;
        let candidates = [
            home.join(".local/share/flatpak/exports/bin").join(app_id),
            Path::new("/var/lib/flatpak/exports/bin").join(app_id),
        ];
        candidates.into_iter().find(|path| is_executable(path))
    })
}

fn find_executable(names: &[&str]) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .flat_map(|directory| names.iter().map(move |name| directory.join(name)))
        .find(|candidate| is_executable(candidate))
}

fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn check_mo2_executable(input: Option<&str>) -> EnvironmentCheck {
    let Some(input) = input.map(str::trim).filter(|path| !path.is_empty()) else {
        return EnvironmentCheck {
            id: "mo2".to_string(),
            label: "ModOrganizer.exe".to_string(),
            status: "not_checked".to_string(),
            detail: "Provide its full path to check whether the file exists.".to_string(),
        };
    };

    let path = Path::new(input);
    if !path.is_absolute() {
        return EnvironmentCheck {
            id: "mo2".to_string(),
            label: "ModOrganizer.exe".to_string(),
            status: "missing".to_string(),
            detail: "Use a full absolute path to ModOrganizer.exe.".to_string(),
        };
    }

    let status = match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => match path.file_name().and_then(|name| name.to_str())
        {
            Some(name) if name.eq_ignore_ascii_case("ModOrganizer.exe") => {
                match path.canonicalize() {
                    Ok(path) => EnvironmentCheck {
                        id: "mo2".to_string(),
                        label: "ModOrganizer.exe".to_string(),
                        status: "found".to_string(),
                        detail: format!("File exists at {}.", path.display()),
                    },
                    Err(error) => EnvironmentCheck {
                        id: "mo2".to_string(),
                        label: "ModOrganizer.exe".to_string(),
                        status: "missing".to_string(),
                        detail: format!("Could not resolve the selected path: {error}"),
                    },
                }
            }
            _ => EnvironmentCheck {
                id: "mo2".to_string(),
                label: "ModOrganizer.exe".to_string(),
                status: "missing".to_string(),
                detail: "The selected file is not named ModOrganizer.exe.".to_string(),
            },
        },
        Ok(_) => EnvironmentCheck {
            id: "mo2".to_string(),
            label: "ModOrganizer.exe".to_string(),
            status: "missing".to_string(),
            detail: "The selected path is not a file.".to_string(),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => EnvironmentCheck {
            id: "mo2".to_string(),
            label: "ModOrganizer.exe".to_string(),
            status: "missing".to_string(),
            detail: "No file exists at the selected path.".to_string(),
        },
        Err(error) => EnvironmentCheck {
            id: "mo2".to_string(),
            label: "ModOrganizer.exe".to_string(),
            status: "missing".to_string(),
            detail: format!("Could not inspect the selected path: {error}"),
        },
    };
    status
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mo2_path_check_requires_an_absolute_path_to_the_expected_executable() {
        assert_eq!(check_mo2_executable(None).status, "not_checked");
        assert_eq!(
            check_mo2_executable(Some("ModOrganizer.exe")).status,
            "missing"
        );
    }

    #[test]
    fn mo2_path_check_accepts_only_an_existing_mod_organizer_executable() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = env::temp_dir().join(format!(
            "ws-control-center-mo2-path-test-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let executable = root.join("ModOrganizer.exe");
        fs::write(&executable, b"test file").unwrap();
        let other_file = root.join("other.exe");
        fs::write(&other_file, b"test file").unwrap();

        assert_eq!(check_mo2_executable(executable.to_str()).status, "found");
        assert_eq!(check_mo2_executable(other_file.to_str()).status, "missing");

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn proton_tool_discovery_finds_only_matching_directories() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = env::temp_dir().join(format!(
            "ws-control-center-mo2-test-{}-{unique}",
            std::process::id()
        ));
        let common = root.join("steamapps/common");
        fs::create_dir_all(common.join("Proton - Experimental")).unwrap();
        fs::create_dir_all(common.join("GE-Proton9-1")).unwrap();
        fs::create_dir_all(common.join("Some Game")).unwrap();

        let mut warnings = Vec::new();
        let tools = discover_proton_tools(&[root.to_string_lossy().into_owned()], &mut warnings);

        assert_eq!(
            tools,
            vec![
                "GE-Proton9-1".to_string(),
                "Proton - Experimental".to_string()
            ]
        );
        assert!(warnings.is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}
