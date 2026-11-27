mod launcher;
mod steam;

#[tauri::command]
fn scan_steam_library() -> Result<steam::SteamScanResult, String> {
    steam::scan_steam_library()
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
            list_launcher_apps,
            add_launcher_app,
            remove_launcher_app,
            launch_launcher_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running WS Control Center");
}
