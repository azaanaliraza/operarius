mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_keyring::init())
        .invoke_handler(tauri::generate_handler![
            commands::ensure_data_folder,
            commands::scan_hardware,
            commands::check_system_capability,
            commands::download_model,
            commands::setup_database,
            commands::download_embedding_model,
            commands::launch_hermes,
            commands::complete_local_setup,
            commands::get_models_dir,
            commands::save_token,
            commands::get_token
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
