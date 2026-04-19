use tauri::Manager;
use sqlx::sqlite::SqlitePoolOptions;

pub mod commands;
pub mod services;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_os::init())
        .setup(|app| {
            // 1. Initialize Ecosystem (Folders)
            let _ = services::setup::init(app.handle());

            // 2. Initialize Database Pool
            let db_path = services::setup::get_base_dir().join("db").join("operarius.db");
            let pool = tauri::async_runtime::block_on(async {
                let p = SqlitePoolOptions::new()
                    .connect(&format!("sqlite:{}?mode=rwc", db_path.to_string_lossy()))
                    .await
                    .map_err(|e| e.to_string())?;
                
                // --- MIGRATIONS ---
                sqlx::query("CREATE TABLE IF NOT EXISTS secrets (key TEXT PRIMARY KEY, value TEXT)")
                    .execute(&p)
                    .await
                    .map_err(|e| e.to_string())?;
                
                sqlx::query("CREATE TABLE IF NOT EXISTS agents (id TEXT PRIMARY KEY, name TEXT, config TEXT)")
                    .execute(&p)
                    .await
                    .map_err(|e| e.to_string())?;

                sqlx::query("CREATE TABLE IF NOT EXISTS sessions (id TEXT PRIMARY KEY, agent_id TEXT, history TEXT)")
                    .execute(&p)
                    .await
                    .map_err(|e| e.to_string())?;
                
                println!("[DB] Migration Success: Persistence Node Ready.");

                Ok::<sqlx::Pool<sqlx::Sqlite>, String>(p)
            })?;

            // --- DETERMINISTIC STARTUP SEQUENCE (Non-blocking) ---
            let handle = app.handle().clone();
            let pool_clone = pool.clone(); 
            tauri::async_runtime::spawn(async move {
                let port = crate::services::supervisor::LLAMA_PORT;
                
                // 1. Auto-discover primary model
                let model_dir = crate::services::setup::get_base_dir().join("models");
                let default_model = std::fs::read_dir(model_dir)
                    .ok()
                    .and_then(|mut entries| entries.find_map(|e| {
                        let path = e.ok()?.path();
                        if path.extension()? == "gguf" { Some(path) } else { None }
                    }));

                if let Some(model_path) = default_model {
                    let model_str = model_path.to_string_lossy().to_string();
                    
                    // 2. Fetch platform token
                    let token: Option<String> = sqlx::query_scalar("SELECT value FROM secrets WHERE key = 'Telegram'")
                        .fetch_optional(&pool_clone)
                        .await
                        .ok()
                        .flatten();

                    // 3. Write Secured Manifest
                    let _ = crate::services::config_writer::write_hermes_config("llama", "llama.cpp", token.clone(), port);

                    // 4. Ignite Brain
                    if crate::services::supervisor::start_llama(&handle, &model_str, port).is_ok() {
                        crate::services::supervisor::wait_for_llama(&handle).await;
                        crate::services::supervisor::start_hermes(&handle, token);
                        crate::services::supervisor::monitor(handle, model_str, port);
                    }
                } else {
                    println!("[ORCHESTRATOR] Brain in standby: No models found.");
                }
            });

            // 4. Force Window Visibility (Mac Stability Fix)
            if let Some(window) = app.get_webview_window("main") {
                window.show().ok();
            }

            app.manage(pool);
            Ok(())
        })
        .on_window_event(|_window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                println!("[ORCHESTRATOR] Performing Graceful Halt...");
                crate::services::supervisor::stop_all();
            }
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
            commands::create_default_rag_agent,
            commands::chat_with_rag_agent,
            commands::check_model_exists,
            commands::save_app_token,
            commands::get_connected_apps,
            commands::remove_app_token,
            commands::install_skill,
            commands::list_skills,
            commands::fetch_hf_models,
            commands::fetch_hf_files
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
