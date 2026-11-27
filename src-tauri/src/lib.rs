mod dotfiles;
mod launcher;
mod mo2;
mod obs;
mod ollama;
mod steam;
mod system;

#[tauri::command]
fn scan_steam_library() -> Result<steam::SteamScanResult, String> {
    steam::scan_steam_library()
}

#[tauri::command]
fn launch_steam_game(app_id: String) -> Result<steam::SteamLaunchResult, String> {
    steam::launch_steam_game(&app_id)
}

#[tauri::command]
fn check_mo2_environment(mo2_path: Option<String>) -> Result<mo2::Mo2Environment, String> {
    mo2::check_environment(mo2_path.as_deref())
}

#[tauri::command]
fn read_system_status() -> Result<system::SystemStatus, String> {
    system::read_status()
}

#[tauri::command]
async fn list_ollama_models() -> Result<Vec<ollama::OllamaModel>, String> {
    ollama::list_models().await
}

#[tauri::command]
async fn chat_with_ollama(
    model: String,
    messages: Vec<ollama::ChatMessage>,
) -> Result<String, String> {
    ollama::chat(&model, &messages).await
}

#[tauri::command]
async fn read_obs_status(port: u16, password: Option<String>) -> Result<obs::ObsStatus, String> {
    tauri::async_runtime::spawn_blocking(move || obs::read_status(port, password.as_deref()))
        .await
        .map_err(|error| format!("OBS status task failed: {error}"))?
}

#[tauri::command]
fn get_dotfile_settings(app: tauri::AppHandle) -> Result<dotfiles::DotfileSettings, String> {
    dotfiles::get_settings(&app)
}

#[tauri::command]
fn save_dotfile_repository(
    app: tauri::AppHandle,
    repository: String,
) -> Result<dotfiles::DotfileSettings, String> {
    dotfiles::save_repository(&app, &repository)
}

#[tauri::command]
fn add_dotfile_to_backup(
    app: tauri::AppHandle,
    path: String,
) -> Result<dotfiles::DotfileSettings, String> {
    dotfiles::add_file(&app, &path)
}

#[tauri::command]
fn remove_dotfile_from_backup(
    app: tauri::AppHandle,
    relative_path: String,
) -> Result<dotfiles::DotfileSettings, String> {
    dotfiles::remove_file(&app, &relative_path)
}

#[tauri::command]
async fn preview_dotfile_backup(app: tauri::AppHandle) -> Result<dotfiles::BackupPreview, String> {
    tauri::async_runtime::spawn_blocking(move || dotfiles::preview(&app))
        .await
        .map_err(|error| format!("Dotfile backup preview task failed: {error}"))?
}

#[tauri::command]
async fn backup_dotfiles(
    app: tauri::AppHandle,
    allow_public_repository: bool,
) -> Result<dotfiles::BackupResult, String> {
    tauri::async_runtime::spawn_blocking(move || dotfiles::backup(&app, allow_public_repository))
        .await
        .map_err(|error| format!("Dotfile backup task failed: {error}"))?
}

#[tauri::command]
fn list_launcher_apps(app: tauri::AppHandle) -> Result<Vec<launcher::LauncherApp>, String> {
    launcher::list_apps(&app)
}

#[tauri::command]
fn add_launcher_app(
    app: tauri::AppHandle,
    name: String,
    executable: String,
) -> Result<Vec<launcher::LauncherApp>, String> {
    launcher::add_app(&app, &name, &executable)
}

#[tauri::command]
fn remove_launcher_app(
    app: tauri::AppHandle,
    executable: String,
) -> Result<Vec<launcher::LauncherApp>, String> {
    launcher::remove_app(&app, &executable)
}

#[tauri::command]
fn launch_launcher_app(
    app: tauri::AppHandle,
    executable: String,
) -> Result<launcher::LaunchResult, String> {
    launcher::launch_app(&app, &executable)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            scan_steam_library,
            launch_steam_game,
            check_mo2_environment,
            read_system_status,
            list_ollama_models,
            chat_with_ollama,
            read_obs_status,
            get_dotfile_settings,
            save_dotfile_repository,
            add_dotfile_to_backup,
            remove_dotfile_from_backup,
            preview_dotfile_backup,
            backup_dotfiles,
            list_launcher_apps,
            add_launcher_app,
            remove_launcher_app,
            launch_launcher_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running WS Control Center");
}
