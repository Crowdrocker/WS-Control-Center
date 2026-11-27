mod steam;

#[tauri::command]
fn scan_steam_library() -> Result<steam::SteamScanResult, String> {
    steam::scan_steam_library()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![scan_steam_library])
        .run(tauri::generate_context!())
        .expect("error while running WS Control Center");
}
