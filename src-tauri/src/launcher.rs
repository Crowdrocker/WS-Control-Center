use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use tauri::Manager;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LauncherApp {
    pub name: String,
    pub executable: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchResult {
    pub process_id: u32,
}

pub fn list_apps(app: &tauri::AppHandle) -> Result<Vec<LauncherApp>, String> {
    read_apps(&config_path(app)?)
}

pub fn add_app(
    app: &tauri::AppHandle,
    name: &str,
    executable: &str,
) -> Result<Vec<LauncherApp>, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control) {
        return Err("Enter an app name between 1 and 80 characters.".to_string());
    }

    let executable = canonical_executable(executable)?;
    let executable = executable
        .to_str()
        .ok_or_else(|| "The executable path is not valid UTF-8.".to_string())?
        .to_string();
    let path = config_path(app)?;
    let mut apps = read_apps(&path)?;

    if apps.iter().any(|entry| entry.executable == executable) {
        return Err("That executable is already in your launcher.".to_string());
    }

    apps.push(LauncherApp {
        name: name.to_string(),
        executable,
    });
    sort_apps(&mut apps);
    write_apps(&path, &apps)?;
    Ok(apps)
}

pub fn remove_app(app: &tauri::AppHandle, executable: &str) -> Result<Vec<LauncherApp>, String> {
    let path = config_path(app)?;
    let mut apps = read_apps(&path)?;
    let original_length = apps.len();
    apps.retain(|entry| entry.executable != executable);
    if apps.len() == original_length {
        return Err("That app is not in your launcher.".to_string());
    }
    write_apps(&path, &apps)?;
    Ok(apps)
}

pub fn launch_app(app: &tauri::AppHandle, executable: &str) -> Result<LaunchResult, String> {
    let path = canonical_executable(executable)?;
    let stored_path = path
        .to_str()
        .ok_or_else(|| "The executable path is not valid UTF-8.".to_string())?;
    let apps = read_apps(&config_path(app)?)?;
    if !apps.iter().any(|entry| entry.executable == stored_path) {
        return Err("Add this executable to your launcher before starting it.".to_string());
    }

    let child = std::process::Command::new(&path)
        .spawn()
        .map_err(|error| format!("Could not start '{}': {error}", path.display()))?;
    Ok(LaunchResult {
        process_id: child.id(),
    })
}

fn config_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let directory = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("Could not determine the app config directory: {error}"))?;
    fs::create_dir_all(&directory)
        .map_err(|error| format!("Could not create the app config directory: {error}"))?;
    Ok(directory.join("launcher-apps.json"))
}

fn read_apps(path: &Path) -> Result<Vec<LauncherApp>, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(format!(
                "Could not read launcher config '{}': {error}",
                path.display()
            ));
        }
    };

    let mut apps: Vec<LauncherApp> = serde_json::from_str(&contents)
        .map_err(|error| format!("Launcher config '{}' is invalid: {error}", path.display()))?;
    sort_apps(&mut apps);
    Ok(apps)
}

fn write_apps(path: &Path, apps: &[LauncherApp]) -> Result<(), String> {
    let contents = serde_json::to_string_pretty(apps)
        .map_err(|error| format!("Could not encode launcher config: {error}"))?;
    fs::write(path, contents).map_err(|error| {
        format!(
            "Could not save launcher config '{}': {error}",
            path.display()
        )
    })
}

fn sort_apps(apps: &mut [LauncherApp]) {
    apps.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.executable.cmp(&right.executable))
    });
}

fn canonical_executable(input: &str) -> Result<std::path::PathBuf, String> {
    let input = input.trim();
    let path = Path::new(input);
    if !path.is_absolute() {
        return Err("Enter the full absolute path to the executable.".to_string());
    }

    let path = path
        .canonicalize()
        .map_err(|error| format!("Could not access executable '{input}': {error}"))?;
    let metadata = fs::metadata(&path)
        .map_err(|error| format!("Could not inspect executable '{}': {error}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!("'{}' is not a file.", path.display()));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(format!("'{}' is not marked executable.", path.display()));
        }
    }

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempConfig(std::path::PathBuf);

    impl TempConfig {
        fn new() -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let directory = std::env::temp_dir().join(format!(
                "ws-control-center-launcher-test-{}-{unique}",
                std::process::id()
            ));
            fs::create_dir_all(&directory).unwrap();
            Self(directory.join("launcher-apps.json"))
        }
    }

    impl Drop for TempConfig {
        fn drop(&mut self) {
            if let Some(directory) = self.0.parent() {
                let _ = fs::remove_dir_all(directory);
            }
        }
    }

    #[test]
    fn rejects_relative_executable_paths() {
        assert_eq!(
            canonical_executable("usr/bin/firefox").unwrap_err(),
            "Enter the full absolute path to the executable."
        );
    }

    #[test]
    fn accepts_the_current_executable_as_a_valid_target() {
        let current_executable = std::env::current_exe().unwrap();
        let canonical = canonical_executable(current_executable.to_str().unwrap()).unwrap();
        assert_eq!(canonical, current_executable.canonicalize().unwrap());
    }

    #[test]
    fn sorts_launcher_entries_by_name() {
        let mut apps = vec![
            LauncherApp {
                name: "Zulu".to_string(),
                executable: "/usr/bin/zulu".to_string(),
            },
            LauncherApp {
                name: "alpha".to_string(),
                executable: "/usr/bin/alpha".to_string(),
            },
        ];
        sort_apps(&mut apps);
        assert_eq!(apps[0].name, "alpha");
        assert_eq!(apps[1].name, "Zulu");
    }

    #[test]
    fn persists_launcher_entries_as_json() {
        let config = TempConfig::new();
        let entries = vec![LauncherApp {
            name: "Browser".to_string(),
            executable: "/usr/bin/browser".to_string(),
        }];

        write_apps(&config.0, &entries).unwrap();

        assert_eq!(read_apps(&config.0).unwrap(), entries);
    }
}
