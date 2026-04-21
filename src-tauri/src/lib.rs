use tauri::Manager;
use sqlx::sqlite::SqlitePoolOptions;

pub mod commands;
pub mod services;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_dialog::init())
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

                sqlx::query(
                    "CREATE TABLE IF NOT EXISTS chat_history (
                        id TEXT PRIMARY KEY,
                        user_id TEXT,
                        session_id TEXT,
                        message_content TEXT,
                        response_content TEXT,
                        created_at TEXT,
                        platform TEXT,
                        chat_id TEXT
                    )"
                )
                .execute(&p)
                .await
                .map_err(|e| e.to_string())?;

                sqlx::query(
                    "CREATE TABLE IF NOT EXISTS files (
                        id TEXT PRIMARY KEY,
                        filename TEXT,
                        file_type TEXT,
                        file_size INTEGER,
                        mime_type TEXT,
                        uploaded_at INTEGER,
                        user_id TEXT,
                        platform TEXT,
                        indexed BOOLEAN,
                        content TEXT
                    )"
                )
                .execute(&p)
                .await
                .map_err(|e| e.to_string())?;

                sqlx::query(
                    "CREATE TABLE IF NOT EXISTS knowledge_base (
                        id TEXT PRIMARY KEY,
                        title TEXT,
                        content TEXT,
                        file_id TEXT,
                        source TEXT,
                        created_at INTEGER,
                        user_id TEXT,
                        tags TEXT
                    )"
                )
                .execute(&p)
                .await
                .map_err(|e| e.to_string())?;

                sqlx::query(
                    "CREATE TABLE IF NOT EXISTS rag_chunks (
                        id TEXT PRIMARY KEY,
                        file_id TEXT,
                        content TEXT,
                        embedding_json TEXT,
                        created_at TEXT
                    )"
                )
                .execute(&p)
                .await
                .map_err(|e| e.to_string())?;

                sqlx::query(
                    "CREATE TABLE IF NOT EXISTS file_blobs (
                        id TEXT PRIMARY KEY,
                        file_id TEXT,
                        data BLOB,
                        created_at TEXT
                    )"
                )
                .execute(&p)
                .await
                .map_err(|e| e.to_string())?;

                sqlx::query(
                    "CREATE TABLE IF NOT EXISTS user_preferences (
                        user_id TEXT,
                        pref_key TEXT,
                        pref_value TEXT,
                        updated_at TEXT,
                        PRIMARY KEY (user_id, pref_key)
                    )"
                )
                .execute(&p)
                .await
                .map_err(|e| e.to_string())?;

                sqlx::query(
                    "CREATE TABLE IF NOT EXISTS user_memory_entries (
                        id TEXT PRIMARY KEY,
                        user_id TEXT,
                        scope TEXT,
                        memory_key TEXT,
                        memory_value TEXT,
                        weight INTEGER DEFAULT 50,
                        updated_at TEXT,
                        UNIQUE(user_id, scope, memory_key)
                    )"
                )
                .execute(&p)
                .await
                .map_err(|e| e.to_string())?;

                sqlx::query(
                    "CREATE TABLE IF NOT EXISTS task_learnings (
                        id TEXT PRIMARY KEY,
                        user_id TEXT,
                        task TEXT,
                        lesson TEXT,
                        success INTEGER DEFAULT 1,
                        created_at TEXT
                    )"
                )
                .execute(&p)
                .await
                .map_err(|e| e.to_string())?;

                sqlx::query(
                    "CREATE TABLE IF NOT EXISTS user_skills (
                        id TEXT PRIMARY KEY,
                        user_id TEXT,
                        name TEXT,
                        description TEXT,
                        instruction TEXT,
                        is_active INTEGER DEFAULT 1,
                        triggers TEXT,
                        created_at TEXT,
                        updated_at TEXT
                    )"
                )
                .execute(&p)
                .await
                .map_err(|e| e.to_string())?;

                sqlx::query(
                    "CREATE TABLE IF NOT EXISTS agent_logs (
                        id TEXT PRIMARY KEY,
                        ts TEXT,
                        level TEXT,
                        scope TEXT,
                        message TEXT,
                        details TEXT
                    )"
                )
                .execute(&p)
                .await
                .map_err(|e| e.to_string())?;

                sqlx::query(
                    "CREATE TABLE IF NOT EXISTS agent_workflows (
                        id TEXT PRIMARY KEY,
                        user_id TEXT,
                        name TEXT,
                        workflow_json TEXT,
                        compiled_json TEXT,
                        updated_at TEXT
                    )"
                )
                .execute(&p)
                .await
                .map_err(|e| e.to_string())?;

                // Backward-compatible chat history enrichment for file upload cards.
                let _ = sqlx::query("ALTER TABLE chat_history ADD COLUMN event_type TEXT")
                    .execute(&p)
                    .await;
                let _ = sqlx::query("ALTER TABLE chat_history ADD COLUMN event_payload TEXT")
                    .execute(&p)
                    .await;
                
                println!("[DB] Migration Success: Persistence Node Ready.");

                Ok::<sqlx::Pool<sqlx::Sqlite>, String>(p)
            })?;

            // --- DETERMINISTIC STARTUP SEQUENCE (Non-blocking) ---
            let handle = app.handle().clone();
            let pool_for_bridge = pool.clone();
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

                    // Ensure embedding model exists for RAG indexing/retrieval.
                    let mut embedding_model = std::fs::read_dir(crate::services::setup::get_base_dir().join("embeddings"))
                        .ok()
                        .and_then(|mut entries| entries.find_map(|e| {
                            let path = e.ok()?.path();
                            if path.extension()? == "gguf" { Some(path.to_string_lossy().to_string()) } else { None }
                        }));

                    if embedding_model.is_none() {
                        let _ = crate::commands::download_embedding_model(handle.clone()).await;
                        embedding_model = std::fs::read_dir(crate::services::setup::get_base_dir().join("embeddings"))
                            .ok()
                            .and_then(|mut entries| entries.find_map(|e| {
                                let path = e.ok()?.path();
                                if path.extension()? == "gguf" { Some(path.to_string_lossy().to_string()) } else { None }
                            }));
                    }
                    


                    // 2. Write Hermes configuration
                    let _ = crate::services::config_writer::write_hermes_config("llama", "llama.cpp", None, port);
                    // 3. Ignite Brain and Telegram bridge
                    if crate::services::supervisor::start_llama(&handle, &model_str, port).is_ok() {
                        crate::services::supervisor::wait_for_llama(&handle).await;
                        if let Some(embedding_model) = embedding_model.clone() {
                            let _ = crate::services::supervisor::start_embedding(&handle, &embedding_model);
                        }
                        crate::services::telegram_bridge::start(pool_for_bridge.clone(), handle.clone());
                        crate::services::supervisor::monitor(handle, model_str, port, embedding_model);
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
            commands::get_chat_history,
            commands::get_agent_logs,
            commands::check_model_exists,
            commands::save_app_token,
            commands::set_app_flag,
            commands::get_app_flag,
            commands::get_connected_apps,
            commands::remove_app_token,
            commands::setup_telegram_bot,
            commands::upload_document,
            commands::search_documents,
            commands::query_rag_agent,
            commands::get_knowledge_base,
            commands::get_indexed_files,
            commands::get_file_metadata,
            commands::install_skill,
            commands::list_skills,
            commands::fetch_hf_models,
            commands::fetch_hf_files,
            commands::get_model_catalog,
            commands::get_active_model,
            commands::switch_active_model,
            commands::delete_model_file,
            commands::set_user_preference,
            commands::get_user_preferences,
            commands::upsert_memory_entry,
            commands::list_memory_entries,
            commands::delete_memory_entry,
            commands::add_task_learning,
            commands::list_task_learnings,
            commands::create_user_skill,
            commands::list_user_skills,
            commands::set_user_skill_active,
            commands::delete_user_skill,
            commands::get_chat_upload_event_payload,
            commands::verify_model_links,
            commands::fetch_skill_marketplace,
            commands::install_marketplace_skill,
            commands::save_agent_workflow,
            commands::list_agent_workflows,
            commands::compile_agent_workflow
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
