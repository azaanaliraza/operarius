use tauri::{AppHandle, command, Emitter, State};
use reqwest::{Client, header};
use std::path::PathBuf;
use std::fs::{self, OpenOptions, File};
use std::io::Write;
use futures_util::StreamExt;
use std::time::Instant;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use serde::Serialize;
use std::process::Command;
use dirs;

// ─── Data Types ──────────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct DownloadProgress {
    pub progress: u32,
    pub speed_mb: f64,
    pub downloaded_gb: f64,
}

#[derive(Serialize)]
pub struct HardwareInfo {
    pub cpu_brand: String,
    pub cpu_cores: usize,
    pub ram_gb: u64,
    pub storage_free_gb: u64,
    pub recommended_model: String,
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Returns ~/Documents/Operarius
fn get_base_dir(_app: &AppHandle) -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    Ok(home.join("Documents").join("Operarius"))
}

/// Returns the path to the Python 3 binary in the isolated runtime
fn get_python_bin(base: &PathBuf) -> PathBuf {
    base.join("runtime").join("python").join("bin").join("python3")
}

/// Scans ~/Documents/Operarius/models/ and returns the first .gguf file found
fn find_default_model(base: &PathBuf) -> String {
    let models_dir = base.join("models");
    if let Ok(entries) = fs::read_dir(&models_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".gguf") {
                    return entry.path().to_string_lossy().to_string();
                }
            }
        }
    }
    // Fallback
    models_dir.join("Llama-3.2-3B-Instruct-Q4_K_M.gguf").to_string_lossy().to_string()
}

/// Returns the default embedding model path
fn get_embedding_path(base: &PathBuf) -> String {
    base.join("embeddings")
        .join("nomic-embed-text-v1.5.Q4_K_M.gguf")
        .to_string_lossy()
        .to_string()
}

// ─── System & Hardware ───────────────────────────────────────────────────────

#[command]
pub fn scan_hardware() -> Result<HardwareInfo, String> {
    let cpu_brand = Command::new("sysctl")
        .args(&["-n", "machdep.cpu.brand_string"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "Apple Silicon".to_string());

    let cpu_cores = Command::new("sysctl")
        .args(&["-n", "hw.ncpu"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<usize>().unwrap_or(8))
        .unwrap_or(8);

    let ram_bytes = Command::new("sysctl")
        .args(&["-n", "hw.memsize"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<u64>().unwrap_or(8_589_934_592))
        .unwrap_or(8_589_934_592);
    let ram_gb = ram_bytes / 1024 / 1024 / 1024;

    let storage_free_gb = Command::new("df")
        .args(&["-g", "/"])
        .output()
        .map(|o| {
            let out = String::from_utf8_lossy(&o.stdout);
            let lines: Vec<&str> = out.lines().collect();
            if lines.len() > 1 {
                let parts: Vec<&str> = lines[1].split_whitespace().collect();
                if parts.len() > 3 {
                    return parts[3].parse::<u64>().unwrap_or(50);
                }
            }
            50
        })
        .unwrap_or(50);

    let recommended = if ram_gb <= 12 {
        "Phi-4 Mini"
    } else if ram_gb <= 24 {
        "Gemma 4 E2B"
    } else {
        "Gemma 4 E4B"
    };

    Ok(HardwareInfo {
        cpu_brand: cpu_brand.replace("Apple ", "M-"),
        cpu_cores,
        ram_gb,
        storage_free_gb,
        recommended_model: recommended.to_string(),
    })
}

#[command]
pub fn check_system_capability() -> Result<HardwareInfo, String> {
    scan_hardware()
}

// ─── Folder & DB Setup ──────────────────────────────────────────────────────

#[command]
pub async fn ensure_data_folder(app: AppHandle) -> Result<String, String> {
    let base = get_base_dir(&app)?;
    for sub in &["models", "embeddings", "hermes", "knowledge", "db"] {
        fs::create_dir_all(base.join(sub)).map_err(|e| e.to_string())?;
    }
    Ok(base.to_string_lossy().to_string())
}

#[command]
pub async fn get_models_dir(app: AppHandle) -> Result<String, String> {
    let path = get_base_dir(&app)?.join("models");
    fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[command]
pub async fn check_model_exists(app: AppHandle, filename: String) -> Result<bool, String> {
    let path = get_base_dir(&app)?.join("models").join(&filename);
    Ok(path.exists() && fs::metadata(&path).map(|m| m.len() > 1_000_000).unwrap_or(false))
}

#[command]
pub async fn setup_database(app: AppHandle) -> Result<String, String> {
    let path = get_base_dir(&app)?.join("db").join("operarius.db");
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    if !path.exists() {
        let _ = File::create(&path);
    }
    let pool = SqlitePoolOptions::new()
        .connect(&format!("sqlite:{}", path.to_string_lossy()))
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS secrets (key TEXT PRIMARY KEY, value TEXT);",
    )
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

// ─── Model Download ─────────────────────────────────────────────────────────

#[command]
pub async fn download_model(app: AppHandle, repo: String, filename: String) -> Result<String, String> {
    let client = Client::builder()
        .user_agent("Operarius/1.0")
        .build()
        .map_err(|e| e.to_string())?;
    let path = get_base_dir(&app)?.join("models").join(&filename);
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;

    let url = format!("https://huggingface.co/{}/resolve/main/{}", repo, filename);
    let head_resp = client.head(&url).send().await.map_err(|e| e.to_string())?;
    let total_size = head_resp.content_length().ok_or("Could not determine model size")?;

    let local_size = if path.exists() {
        fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
    } else {
        0
    };

    if local_size >= total_size && total_size > 0 {
        app.emit("download-complete", ()).ok();
        return Ok(path.to_string_lossy().to_string());
    }

    let mut request = client.get(&url);
    if local_size > 0 {
        request = request.header(header::RANGE, format!("bytes={}-", local_size));
    }
    let resp = request.send().await.map_err(|e| e.to_string())?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    let mut stream = resp.bytes_stream();
    let mut downloaded = local_size;
    let mut last_emit = Instant::now();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        if last_emit.elapsed().as_millis() >= 200 {
            let progress = ((downloaded as f64 / total_size as f64) * 100.0) as u32;
            app.emit(
                "download-progress",
                DownloadProgress {
                    progress: progress.min(100),
                    speed_mb: 0.0,
                    downloaded_gb: downloaded as f64 / 1_073_741_824.0,
                },
            )
            .ok();
            last_emit = Instant::now();
        }
    }

    app.emit("download-complete", ()).ok();
    Ok(path.to_string_lossy().to_string())
}

#[command]
pub async fn download_embedding_model(app: AppHandle) -> Result<String, String> {
    let client = Client::new();
    let filename = "nomic-embed-text-v1.5.Q4_K_M.gguf";
    let path = get_base_dir(&app)?.join("embeddings").join(filename);
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;

    if path.exists() && fs::metadata(&path).map(|m| m.len()).unwrap_or(0) > 10_000_000 {
        return Ok(path.to_string_lossy().to_string());
    }

    let resp = client
        .get(format!(
            "https://huggingface.co/nomic-ai/nomic-embed-text-v1.5-GGUF/resolve/main/{}",
            filename
        ))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let mut file = File::create(&path).map_err(|e| e.to_string())?;
    let mut stream = resp.bytes_stream();
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| e.to_string())?;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
    }
    Ok(path.to_string_lossy().to_string())
}

#[command]
pub async fn complete_local_setup(app: AppHandle, model_repo: String, model_file: String) -> Result<String, String> {
    download_model(app.clone(), model_repo, model_file).await?;
    download_embedding_model(app.clone()).await?;
    setup_database(app.clone()).await?;
    Ok("Success".to_string())
}

// ─── Inference Server ────────────────────────────────────────────────────────

#[command]
pub async fn start_inference_server(app: AppHandle, model_path: String) -> Result<String, String> {
    let base = get_base_dir(&app)?;
    let m_path = if model_path.is_empty() {
        find_default_model(&base)
    } else {
        model_path
    };

    let client = Client::new();

    // Use a lockfile to prevent duplicate starts within the same app session
    let lock_path = base.join(".inference_lock");
    if lock_path.exists() {
        // Check if server is actually alive
        if let Ok(resp) = client.get("http://127.0.0.1:8080/v1/models").send().await {
            if resp.status().is_success() {
                println!("[OPERARIUS] Inference server already running (locked)");
                return Ok("Server already active".to_string());
            }
        }
        // Lock exists but server is dead — clean up and restart
        let _ = fs::remove_file(&lock_path);
    }

    // Kill ALL old server processes to reclaim port 8080
    let _ = Command::new("pkill").arg("-9").arg("-f").arg("llama_cpp.server").output();
    let _ = Command::new("pkill").arg("-9").arg("-f").arg("llama-server").output();
    std::thread::sleep(std::time::Duration::from_millis(1000));

    let python_bin = get_python_bin(&base);
    println!("[OPERARIUS] Starting inference server: {} (n_ctx=16384)", m_path);

    let _ = Command::new(python_bin.to_string_lossy().to_string())
        .args(&[
            "-m", "llama_cpp.server",
            "--model", &m_path,
            "--port", "8080",
            "--n_gpu_layers", "-1",
            "--n_ctx", "16384",
            "--host", "127.0.0.1",
        ])
        .spawn()
        .map_err(|e| format!("Failed to start inference server: {}", e))?;

    // Wait for server to become ready (up to 20 seconds — model loading takes time)
    for i in 0..40 {
        std::thread::sleep(std::time::Duration::from_millis(500));
        if let Ok(resp) = client.get("http://127.0.0.1:8080/v1/models").send().await {
            if resp.status().is_success() {
                // Write lockfile so subsequent calls skip the kill+restart
                let _ = fs::write(&lock_path, format!("{}", std::process::id()));
                println!("[OPERARIUS] Inference server ready after {}ms", (i + 1) * 500);
                return Ok("Server started".to_string());
            }
        }
    }

    Ok("Server starting (may need a moment to load model)".to_string())
}

// ─── Hermes Gateway (for Telegram & messaging platforms) ─────────────────────

#[command]
pub async fn launch_hermes(
    app: AppHandle,
    state_pool: State<'_, SqlitePool>,
    model_path: String,
    embedding_path: String,
) -> Result<String, String> {
    use tauri_plugin_shell::ShellExt;

    let base = get_base_dir(&app)?;
    let hermes_home = base.join("hermes");
    fs::create_dir_all(&hermes_home).map_err(|e| e.to_string())?;

    // 1. Kill ALL old gateway/hermes processes aggressively
    let _ = Command::new("pkill").arg("-9").arg("-f").arg("hermes").output();
    let _ = Command::new("pkill").arg("-9").arg("-f").arg("hermes-aarch64").output();
    let _ = Command::new("pkill").arg("-9").arg("-f").arg("gateway").output();
    // Wait for Telegram bot token to be released
    std::thread::sleep(std::time::Duration::from_millis(2000));

    // 2. Resolve model & embedding paths
    let m_path = if model_path.is_empty() { find_default_model(&base) } else { model_path };
    let e_path = if embedding_path.is_empty() { get_embedding_path(&base) } else { embedding_path };

    // 3. Start inference server (idempotent — skips if already running)
    let _ = start_inference_server(app.clone(), m_path.clone()).await;

    // 4. Check for Telegram token
    let mut telegram_token = String::new();
    if let Ok(row) = sqlx::query("SELECT value FROM secrets WHERE key = 'Telegram'")
        .fetch_optional(&*state_pool)
        .await
    {
        if let Some(r) = row {
            telegram_token = sqlx::Row::get(&r, "value");
        }
    }

    // 5. Write .env — override OpenRouter base URL to point to our local server
    let env_content = format!(
        "OPENROUTER_API_KEY=not-needed\n\
         OPENROUTER_BASE_URL=http://127.0.0.1:8080/v1\n\
         GATEWAY_ALLOW_ALL_USERS=true\n\
         GATEWAY_PORT=8989\n\
         {}",
        if !telegram_token.is_empty() {
            format!("TELEGRAM_BOT_TOKEN={}", telegram_token)
        } else {
            String::new()
        }
    );
    fs::write(hermes_home.join(".env"), env_content.trim()).ok();

    // 6. Write config.yaml — use openrouter provider with local URL override
    let platforms_section = if !telegram_token.is_empty() {
        format!(
            r#"platforms:
  telegram:
    enabled: true
    token: "{}"
    allowed_users: ["*"]"#,
            telegram_token
        )
    } else {
        String::new()
    };

    let config_content = format!(
        r#"---
model:
  provider: "openrouter"
  default: "{model_id}"

embedding:
  provider: "local"
  model_path: "{e_path}"
  dimension: 768

context_compression:
  enabled: false

# Disable all toolsets to bypass tool injection and function calling.
# The 3B model struggles with complex multi-tool JSON schemas.
platform_toolsets:
  telegram:
    - no_mcp
    - none
  cli:
    - no_mcp
    - none

{platforms}

gateway:
  allow_all_users: true
  api:
    enabled: true
    port: 8989
"#,
        model_id = m_path,
        e_path = e_path,
        platforms = platforms_section,
    );
    fs::write(hermes_home.join("config.yaml"), config_content).ok();

    println!("[OPERARIUS] Hermes config written to {}", hermes_home.display());

    // 7. Spawn Hermes gateway as background sidecar
    let (mut rx, _child) = app
        .shell()
        .sidecar("hermes")
        .map_err(|e| e.to_string())?
        .args(&["gateway", "run", "--replace"])
        .env("HERMES_HOME", hermes_home.to_string_lossy().to_string())
        .current_dir(hermes_home.clone())
        .spawn()
        .map_err(|e| e.to_string())?;

    tauri::async_runtime::spawn(async move {
        use tauri_plugin_shell::process::CommandEvent;
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    println!("[HERMES] {}", String::from_utf8_lossy(&line).trim())
                }
                CommandEvent::Stderr(line) => {
                    let msg = String::from_utf8_lossy(&line).trim().to_string();
                    if !msg.is_empty() {
                        println!("[HERMES ERR] {}", msg);
                    }
                }
                _ => {}
            }
        }
    });

    Ok("Hermes gateway started".to_string())
}

// ─── RAG Agent Setup ─────────────────────────────────────────────────────────

#[command]
pub async fn create_default_rag_agent(app: AppHandle, _model_file: String) -> Result<String, String> {
    let base = get_base_dir(&app)?;
    let knowledge_dir = base.join("knowledge");
    fs::create_dir_all(&knowledge_dir).map_err(|e| e.to_string())?;

    // Create a welcome document so the knowledge folder isn't empty
    let welcome_path = knowledge_dir.join("welcome.md");
    if !welcome_path.exists() {
        fs::write(
            &welcome_path,
            "# Welcome to Operarius\n\n\
             Drop documents (.txt, .md, .pdf) into this folder to build your knowledge base.\n\n\
             Operarius will use these documents to provide contextual answers to your questions.\n",
        )
        .ok();
    }

    Ok("RAG agent ready".to_string())
}

// ─── Chat (Direct HTTP to local inference server) ────────────────────────────

#[command]
pub async fn chat_with_rag_agent(app: AppHandle, message: String) -> Result<String, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    println!("[OPERARIUS] Chat query: {}", message);

    // 1. Check if inference server is alive
    let models_resp = match client.get("http://127.0.0.1:8080/v1/models").send().await {
        Ok(r) => r,
        Err(_) => {
            // Try to auto-start the server
            let _ = start_inference_server(app.clone(), String::new()).await;
            // Wait and retry
            std::thread::sleep(std::time::Duration::from_secs(2));
            match client.get("http://127.0.0.1:8080/v1/models").send().await {
                Ok(r) => r,
                Err(_) => {
                    return Ok("The AI engine is still starting up. Please wait a moment and try again.".to_string());
                }
            }
        }
    };

    // 2. Get the model ID from the server
    let model_id = match models_resp.json::<serde_json::Value>().await {
        Ok(json) => json["data"][0]["id"]
            .as_str()
            .unwrap_or("default")
            .to_string(),
        Err(_) => "default".to_string(),
    };

    println!("[OPERARIUS] Using model: {}", model_id);

    // 3. Load any knowledge files for RAG context
    let base = get_base_dir(&app)?;
    let knowledge_dir = base.join("knowledge");
    let mut context_parts: Vec<String> = Vec::new();

    if knowledge_dir.exists() {
        if let Ok(entries) = fs::read_dir(&knowledge_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if matches!(ext, "txt" | "md" | "text") {
                            if let Ok(content) = fs::read_to_string(&path) {
                                // Limit each file to 2000 chars to avoid context overflow
                                let truncated = if content.len() > 2000 {
                                    format!("{}...", &content[..2000])
                                } else {
                                    content
                                };
                                context_parts.push(format!(
                                    "--- {} ---\n{}",
                                    path.file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy(),
                                    truncated
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    let system_prompt = if context_parts.is_empty() {
        "You are Operarius, a helpful AI assistant running locally on the user's machine. \
         You are private, fast, and secure. Be concise and helpful."
            .to_string()
    } else {
        format!(
            "You are Operarius, a helpful AI assistant running locally on the user's machine. \
             You have access to the following knowledge documents. Use them to answer questions \
             when relevant.\n\n{}\n\n\
             Be concise and helpful. If the answer is in the documents, cite which document it came from.",
            context_parts.join("\n\n")
        )
    };

    // 4. Send chat completion request directly to llama.cpp server
    let body = serde_json::json!({
        "model": model_id,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": message }
        ],
        "max_tokens": 1024,
        "temperature": 0.7
    });

    let resp = client
        .post("http://127.0.0.1:8080/v1/chat/completions")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to reach AI engine: {}", e))?;

    let status = resp.status();
    let resp_text = resp.text().await.map_err(|e| e.to_string())?;

    if !status.is_success() {
        println!("[OPERARIUS] API error {}: {}", status, resp_text);
        return Ok(format!("Engine error ({}). Please try again.", status));
    }

    let resp_json: serde_json::Value =
        serde_json::from_str(&resp_text).map_err(|e| e.to_string())?;

    let content = resp_json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    if content.is_empty() {
        return Ok("I'm thinking... Please try rephrasing your question.".to_string());
    }

    println!("[OPERARIUS] Response: {}...", &content[..content.len().min(80)]);
    Ok(content)
}

// ─── App Token Management (Telegram, etc.) ───────────────────────────────────

#[command]
pub async fn save_app_token(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    service: String,
    token: String,
) -> Result<(), String> {
    sqlx::query("INSERT OR REPLACE INTO secrets (key, value) VALUES (?, ?)")
        .bind(&service)
        .bind(&token)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;

    // Restart Hermes gateway to pick up the new token
    let _ = launch_hermes(app.clone(), pool, "".to_string(), "".to_string()).await;
    Ok(())
}

#[command]
pub async fn get_connected_apps(pool: State<'_, SqlitePool>) -> Result<Vec<String>, String> {
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT key FROM secrets WHERE value != ''")
            .fetch_all(&*pool)
            .await
            .map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

#[command]
pub async fn remove_app_token(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    service: String,
) -> Result<(), String> {
    sqlx::query("DELETE FROM secrets WHERE key = ?")
        .bind(&service)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;

    // Restart Hermes gateway without the removed token
    let _ = launch_hermes(app.clone(), pool, "".to_string(), "".to_string()).await;
    Ok(())
}
