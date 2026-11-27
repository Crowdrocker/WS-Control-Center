use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    env, fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
};
use tauri::Manager;

const MAX_FILES: usize = 100;
const MAX_FILE_BYTES: usize = 8 * 1024 * 1024;
const MAX_TOTAL_BYTES: usize = 24 * 1024 * 1024;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DotfileSettings {
    pub repository: String,
    pub files: Vec<DotfileEntry>,
}

fn is_executable(metadata: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        false
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DotfileEntry {
    pub relative_path: String,
    pub exists: bool,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupPreview {
    pub repository: String,
    pub default_branch: String,
    pub github_user: String,
    pub is_private: bool,
    pub files: Vec<BackupFile>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupFile {
    pub source_path: String,
    pub target_path: String,
    pub size_bytes: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupResult {
    pub repository: String,
    pub branch: String,
    pub commit_sha: String,
    pub files_backed_up: usize,
}

#[derive(Debug)]
struct PendingFile {
    relative_path: String,
    contents: Vec<u8>,
    executable: bool,
}

pub fn get_settings(app: &tauri::AppHandle) -> Result<DotfileSettings, String> {
    let mut settings = read_settings(&config_path(app)?)?;
    refresh_entries(&mut settings)?;
    Ok(settings)
}

pub fn save_repository(
    app: &tauri::AppHandle,
    repository: &str,
) -> Result<DotfileSettings, String> {
    let repository = validate_repository(repository)?;
    let path = config_path(app)?;
    let mut settings = read_settings(&path)?;
    settings.repository = repository;
    write_settings(&path, &settings)?;
    refresh_entries(&mut settings)?;
    Ok(settings)
}

pub fn add_file(app: &tauri::AppHandle, input: &str) -> Result<DotfileSettings, String> {
    let path = config_path(app)?;
    let mut settings = read_settings(&path)?;
    let relative_path = relative_dotfile_path(&home_directory()?, input)?;
    if settings
        .files
        .iter()
        .any(|entry| entry.relative_path == relative_path)
    {
        return Err("That file is already on the backup list.".to_string());
    }
    if settings.files.len() >= MAX_FILES {
        return Err(format!("The backup list is limited to {MAX_FILES} files."));
    }

    settings.files.push(DotfileEntry {
        relative_path,
        exists: true,
        size_bytes: None,
    });
    settings
        .files
        .sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    write_settings(&path, &settings)?;
    refresh_entries(&mut settings)?;
    Ok(settings)
}

pub fn remove_file(app: &tauri::AppHandle, relative_path: &str) -> Result<DotfileSettings, String> {
    let path = config_path(app)?;
    let mut settings = read_settings(&path)?;
    let old_len = settings.files.len();
    settings
        .files
        .retain(|entry| entry.relative_path != relative_path);
    if settings.files.len() == old_len {
        return Err("That file is not on the backup list.".to_string());
    }
    write_settings(&path, &settings)?;
    refresh_entries(&mut settings)?;
    Ok(settings)
}

pub fn preview(app: &tauri::AppHandle) -> Result<BackupPreview, String> {
    let settings = read_settings(&config_path(app)?)?;
    let repository = validate_repository(&settings.repository)?;
    let files = read_selected_files(&settings.files)?;
    let (github_user, repo) = github_context(&repository)?;

    Ok(BackupPreview {
        repository,
        default_branch: repo.default_branch,
        github_user,
        is_private: repo.private,
        files: files
            .iter()
            .map(|file| BackupFile {
                source_path: file.relative_path.clone(),
                target_path: format!("dotfiles/{}", file.relative_path),
                size_bytes: file.contents.len() as u64,
            })
            .collect(),
    })
}

pub fn backup(
    app: &tauri::AppHandle,
    allow_public_repository: bool,
) -> Result<BackupResult, String> {
    let settings = read_settings(&config_path(app)?)?;
    let repository = validate_repository(&settings.repository)?;
    let files = read_selected_files(&settings.files)?;
    let (_, repo) = github_context(&repository)?;
    if !repo.private && !allow_public_repository {
        return Err("The repository is public. Review the visibility warning and confirm before backing up.".to_string());
    }

    let head = get_branch_head(&repository, &repo.default_branch)?;
    let base_commit = gh_api_get(&format!("repos/{repository}/git/commits/{}", head.sha))?;
    let base_tree = string_field(&base_commit, &["tree", "sha"])?;

    let mut tree_entries = Vec::with_capacity(files.len());
    for file in &files {
        let blob = gh_api(
            "POST",
            &format!("repos/{repository}/git/blobs"),
            Some(&json!({
                "content": STANDARD.encode(&file.contents),
                "encoding": "base64"
            })),
        )?;
        tree_entries.push(json!({
            "path": format!("dotfiles/{}", file.relative_path),
            "mode": if file.executable { "100755" } else { "100644" },
            "type": "blob",
            "sha": string_field(&blob, &["sha"])?
        }));
    }

    let tree = gh_api(
        "POST",
        &format!("repos/{repository}/git/trees"),
        Some(&json!({
            "base_tree": base_tree,
            "tree": tree_entries
        })),
    )?;
    let tree_sha = string_field(&tree, &["sha"])?;
    let commit = gh_api(
        "POST",
        &format!("repos/{repository}/git/commits"),
        Some(&json!({
            "message": "Back up selected dotfiles",
            "tree": tree_sha,
            "parents": [head.sha]
        })),
    )?;
    let commit_sha = string_field(&commit, &["sha"])?;
    let branch_ref = format!(
        "repos/{repository}/git/refs/heads/{}",
        encode_github_path(&repo.default_branch)
    );
    gh_api(
        "PATCH",
        &branch_ref,
        Some(&json!({
            "sha": commit_sha,
            "force": false
        })),
    )?;

    Ok(BackupResult {
        repository,
        branch: repo.default_branch,
        commit_sha,
        files_backed_up: files.len(),
    })
}

fn config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let directory = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("Could not determine the app config directory: {error}"))?;
    fs::create_dir_all(&directory)
        .map_err(|error| format!("Could not create the app config directory: {error}"))?;
    Ok(directory.join("dotfile-backup.json"))
}

fn read_settings(path: &Path) -> Result<DotfileSettings, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(DotfileSettings::default());
        }
        Err(error) => {
            return Err(format!(
                "Could not read dotfile backup settings '{}': {error}",
                path.display()
            ));
        }
    };

    let settings: DotfileSettings = serde_json::from_str(&contents)
        .map_err(|error| format!("Dotfile backup settings are invalid: {error}"))?;
    if settings.files.len() > MAX_FILES {
        return Err(format!(
            "Dotfile backup settings contain more than {MAX_FILES} files."
        ));
    }
    let mut paths = HashSet::new();
    if settings
        .files
        .iter()
        .any(|entry| !paths.insert(entry.relative_path.as_str()))
    {
        return Err("Dotfile backup settings contain duplicate file paths.".to_string());
    }
    Ok(settings)
}

fn write_settings(path: &Path, settings: &DotfileSettings) -> Result<(), String> {
    let contents = serde_json::to_string_pretty(settings)
        .map_err(|error| format!("Could not encode dotfile backup settings: {error}"))?;
    fs::write(path, contents).map_err(|error| {
        format!(
            "Could not save dotfile backup settings '{}': {error}",
            path.display()
        )
    })
}

fn refresh_entries(settings: &mut DotfileSettings) -> Result<(), String> {
    let home = home_directory()?;
    for entry in &mut settings.files {
        let relative = validate_relative_path(&entry.relative_path)?;
        let path = home.join(relative);
        match path.canonicalize() {
            Ok(canonical) if canonical.starts_with(&home) => {
                let metadata = fs::metadata(&canonical).map_err(|error| {
                    format!(
                        "Could not inspect selected file '{}': {error}",
                        entry.relative_path
                    )
                })?;
                entry.exists = metadata.is_file();
                entry.size_bytes = entry.exists.then_some(metadata.len());
            }
            Ok(_) => {
                entry.exists = false;
                entry.size_bytes = None;
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                ) =>
            {
                entry.exists = false;
                entry.size_bytes = None;
            }
            Err(error) => {
                return Err(format!(
                    "Could not inspect selected file '{}': {error}",
                    entry.relative_path
                ));
            }
        }
    }
    Ok(())
}

fn home_directory() -> Result<PathBuf, String> {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "The current user's home directory could not be determined.".to_string())?;
    home.canonicalize()
        .map_err(|error| format!("Could not access the home directory: {error}"))
}

fn relative_dotfile_path(home: &Path, input: &str) -> Result<String, String> {
    let input = input.trim();
    let path = Path::new(input);
    if !path.is_absolute() {
        return Err("Enter the full absolute path to a file in your home directory.".to_string());
    }
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("Could not access selected file '{input}': {error}"))?;
    if !canonical.starts_with(home) {
        return Err("Only files inside your home directory can be added.".to_string());
    }
    let metadata = fs::metadata(&canonical).map_err(|error| {
        format!(
            "Could not inspect selected file '{}': {error}",
            canonical.display()
        )
    })?;
    if !metadata.is_file() {
        return Err("Select a regular file, not a directory.".to_string());
    }
    if metadata.len() > MAX_FILE_BYTES as u64 {
        return Err(format!(
            "Selected files must be no larger than {} MiB.",
            MAX_FILE_BYTES / (1024 * 1024)
        ));
    }
    let relative = canonical
        .strip_prefix(home)
        .map_err(|error| format!("Could not make the selected path relative: {error}"))?;
    path_to_string(relative)
}

fn validate_relative_path(input: &str) -> Result<&Path, String> {
    let path = Path::new(input);
    if input.trim().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!("Invalid home-relative dotfile path '{input}'."));
    }
    Ok(path)
}

fn path_to_string(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| "The selected file path must be valid UTF-8.".to_string())
}

fn validate_repository(input: &str) -> Result<String, String> {
    let repository = input.trim();
    let parts: Vec<_> = repository.split('/').collect();
    if parts.len() != 2
        || parts.iter().any(|part| {
            part.is_empty()
                || part.len() > 100
                || *part == "."
                || *part == ".."
                || !part.bytes().all(is_repo_name_byte)
        })
    {
        return Err("Enter a GitHub repository as owner/repository.".to_string());
    }
    Ok(repository.to_string())
}

fn is_repo_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')
}

#[derive(Deserialize)]
struct GitHubUser {
    login: String,
}

#[derive(Deserialize)]
struct GitHubRepository {
    private: bool,
    default_branch: String,
}

struct BranchHead {
    sha: String,
}

fn github_context(repository: &str) -> Result<(String, GitHubRepository), String> {
    let user: GitHubUser = gh_api_get("user")?;
    let repo: GitHubRepository = gh_api_get(&format!("repos/{repository}"))?;
    if repo.default_branch.trim().is_empty() {
        return Err("The GitHub repository has no default branch.".to_string());
    }
    Ok((user.login, repo))
}

fn get_branch_head(repository: &str, branch: &str) -> Result<BranchHead, String> {
    let endpoint = format!(
        "repos/{repository}/git/ref/heads/{}",
        encode_github_path(branch)
    );
    let response: Value = gh_api_get(&endpoint)?;
    Ok(BranchHead {
        sha: string_field(&response, &["object", "sha"])?,
    })
}

fn gh_api_get<T: for<'de> Deserialize<'de>>(endpoint: &str) -> Result<T, String> {
    let response = gh_api("GET", endpoint, None)?;
    serde_json::from_value(response)
        .map_err(|error| format!("Could not parse GitHub API response: {error}"))
}

fn gh_api(method: &str, endpoint: &str, payload: Option<&Value>) -> Result<Value, String> {
    let encoded_payload = payload
        .map(serde_json::to_vec)
        .transpose()
        .map_err(|error| format!("Could not encode GitHub request: {error}"))?;

    let mut command = Command::new("gh");
    command
        .arg("api")
        .arg("--hostname")
        .arg("github.com")
        .env("GH_PROMPT_DISABLED", "1")
        .env("GH_NO_UPDATE_NOTIFIER", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if encoded_payload.is_some() {
        command
            .arg("--method")
            .arg(method)
            .arg(endpoint)
            .arg("--input")
            .arg("-");
        command.stdin(Stdio::piped());
    } else {
        command.arg(endpoint);
    }

    let mut child = command.spawn().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            "GitHub CLI (gh) was not found. Install it and sign in before using dotfile backup."
                .to_string()
        } else {
            format!("Could not start GitHub CLI: {error}")
        }
    })?;
    if let Some(encoded) = encoded_payload {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| "Could not open GitHub CLI request input.".to_string())?;
        if let Err(error) = stdin.write_all(&encoded) {
            drop(stdin);
            let _ = child.wait();
            return Err(format!("Could not send request to GitHub CLI: {error}"));
        }
    }

    let output = child
        .wait_with_output()
        .map_err(|error| format!("Could not wait for GitHub CLI: {error}"))?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if message.is_empty() {
            format!("GitHub API request failed with status {}.", output.status)
        } else {
            format!("GitHub API request failed: {message}")
        });
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("GitHub API returned invalid JSON: {error}"))
}

fn string_field(value: &Value, path: &[&str]) -> Result<String, String> {
    let mut current = value;
    for key in path {
        current = current
            .get(*key)
            .ok_or_else(|| format!("GitHub response did not contain '{}'.", path.join(".")))?;
    }
    current
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| format!("GitHub response field '{}' was not text.", path.join(".")))
}

fn encode_github_path(path: &str) -> String {
    let mut encoded = String::with_capacity(path.len());
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~' | b'/') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn read_selected_files(entries: &[DotfileEntry]) -> Result<Vec<PendingFile>, String> {
    if entries.is_empty() {
        return Err("Add at least one file to the backup list first.".to_string());
    }
    let home = home_directory()?;
    let mut files = Vec::with_capacity(entries.len());
    let mut estimated_total_bytes = 0usize;
    let mut total_bytes = 0usize;
    for entry in entries {
        let relative = validate_relative_path(&entry.relative_path)?;
        let path = home
            .join(relative)
            .canonicalize()
            .map_err(|error| format!("Could not access '{}': {error}", entry.relative_path))?;
        if !path.starts_with(&home) {
            return Err(format!(
                "'{}' no longer points to a file inside your home directory.",
                entry.relative_path
            ));
        }
        let metadata = fs::metadata(&path)
            .map_err(|error| format!("Could not inspect '{}': {error}", entry.relative_path))?;
        if !metadata.is_file() {
            return Err(format!("'{}' is not a regular file.", entry.relative_path));
        }
        if metadata.len() > MAX_FILE_BYTES as u64 {
            return Err(format!(
                "'{}' exceeds the {} MiB per-file limit.",
                entry.relative_path,
                MAX_FILE_BYTES / (1024 * 1024)
            ));
        }
        estimated_total_bytes += metadata.len() as usize;
        if estimated_total_bytes > MAX_TOTAL_BYTES {
            return Err(format!(
                "Selected files exceed the {} MiB total backup limit.",
                MAX_TOTAL_BYTES / (1024 * 1024)
            ));
        }
        let file = fs::File::open(&path)
            .map_err(|error| format!("Could not read '{}': {error}", entry.relative_path))?;
        let mut contents = Vec::new();
        file.take(MAX_FILE_BYTES as u64 + 1)
            .read_to_end(&mut contents)
            .map_err(|error| format!("Could not read '{}': {error}", entry.relative_path))?;
        if contents.len() > MAX_FILE_BYTES {
            return Err(format!(
                "'{}' exceeds the {} MiB per-file limit.",
                entry.relative_path,
                MAX_FILE_BYTES / (1024 * 1024)
            ));
        }
        total_bytes += contents.len();
        if total_bytes > MAX_TOTAL_BYTES {
            return Err(format!(
                "Selected files exceed the {} MiB total backup limit.",
                MAX_TOTAL_BYTES / (1024 * 1024)
            ));
        }
        files.push(PendingFile {
            relative_path: entry.relative_path.clone(),
            contents,
            executable: is_executable(&metadata),
        });
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempHome(PathBuf);

    impl TempHome {
        fn new() -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = env::temp_dir().join(format!(
                "ws-control-center-dotfiles-test-{}-{unique}",
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
    fn repository_validation_accepts_only_owner_and_repo() {
        assert_eq!(validate_repository("  owner/repo  ").unwrap(), "owner/repo");
        assert!(validate_repository("owner/repo/path").is_err());
        assert!(validate_repository("../repo").is_err());
        assert!(validate_repository("./repo").is_err());
        assert!(validate_repository("owner;rm/repo").is_err());
    }

    #[test]
    fn backup_settings_persist_repository_and_selected_paths() {
        let home = TempHome::new();
        let path = home.0.join("dotfile-backup.json");
        let settings = DotfileSettings {
            repository: "owner/dotfiles".to_string(),
            files: vec![DotfileEntry {
                relative_path: ".zshrc".to_string(),
                exists: true,
                size_bytes: Some(12),
            }],
        };

        write_settings(&path, &settings).unwrap();
        let loaded = read_settings(&path).unwrap();

        assert_eq!(loaded.repository, settings.repository);
        assert_eq!(loaded.files.len(), 1);
        assert_eq!(loaded.files[0].relative_path, ".zshrc");
    }

    #[test]
    fn selected_paths_must_be_regular_files_inside_home() {
        let home = TempHome::new();
        let selected = home.0.join(".zshrc");
        fs::write(&selected, "export EDITOR=vim").unwrap();
        assert_eq!(
            relative_dotfile_path(&home.0, selected.to_str().unwrap()).unwrap(),
            ".zshrc"
        );

        let directory = home.0.join(".config");
        fs::create_dir(&directory).unwrap();
        assert!(relative_dotfile_path(&home.0, directory.to_str().unwrap()).is_err());

        let outside = env::temp_dir().join(format!("outside-dotfile-{}", std::process::id()));
        fs::write(&outside, "not a dotfile").unwrap();
        assert!(relative_dotfile_path(&home.0, outside.to_str().unwrap()).is_err());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&outside, home.0.join("external-link")).unwrap();
            assert!(
                relative_dotfile_path(&home.0, home.0.join("external-link").to_str().unwrap())
                    .is_err()
            );
        }
        fs::remove_file(outside).unwrap();
    }

    #[test]
    fn github_paths_encode_spaces_but_preserve_directory_separators() {
        assert_eq!(
            encode_github_path("dotfiles/.config/My App/settings"),
            "dotfiles/.config/My%20App/settings"
        );
    }
}
