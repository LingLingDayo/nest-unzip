mod commands;
mod extractor;
mod types;
mod utils;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::detect_tools,
            commands::run_depth_extraction,
            commands::extract_archive,
            commands::scan_archives,
            commands::trash_path,
            commands::delete_path,
            commands::path_exists,
            commands::scan_dir_entries
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
