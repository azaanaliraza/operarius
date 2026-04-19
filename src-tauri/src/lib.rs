use tauri::Manager;
use sqlx::sqlite::SqlitePoolOptions;
use std::fs;
use dirs;

mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_os::init())
        .setup(|app| {
            let home = dirs::home_dir().ok_or("Home dir not found")?;
            let db_dir = home.join("Documents").join("Operarius").join("db");
            fs::create_dir_all(&db_dir)?;

            let db_path = db_dir.join("operarius.db");
            if !db_path.exists() {
                let _ = fs::File::create(&db_path);
            }

            let pool = tauri::async_runtime::block_on(async {
                let pool = SqlitePoolOptions::new()
                    .connect(&format!("sqlite:{}?mode=rwc", db_path.to_string_lossy()))
                    .await?;

                // Ensure the secrets table exists on startup
                sqlx::query(
                    "CREATE TABLE IF NOT EXISTS secrets (key TEXT PRIMARY KEY, value TEXT);"
                )
                .execute(&pool)
                .await?;

                Ok::<_, sqlx::Error>(pool)
            }).map_err(|e: sqlx::Error| e.to_string())?;

            app.manage(pool);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::ensure_data_folder,
            commands::scan_hardware,
            commands::check_system_capability,
            commands::get_models_dir,
            commands::complete_local_setup,
            commands::download_model,
            commands::setup_database,
            commands::download_embedding_model,
            commands::start_inference_server,
            commands::launch_hermes,
            commands::create_default_rag_agent,
            commands::chat_with_rag_agent,
            commands::check_model_exists,
            commands::save_app_token,
            commands::get_connected_apps,
            commands::remove_app_token,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
