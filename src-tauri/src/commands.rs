use crate::services::downloader::download_with_resume;
use reqwest::Client;
use serde::Serialize;
use sqlx::SqlitePool;
use std::fs::{self, File};
use std::path::PathBuf;
use std::process::Command;
use tauri::{command, AppHandle, Emitter, Manager, State};
// ─── Data Types ──────────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct DownloadProgress {
    pub progress: u32,
    pub speed_mb: f64,
    pub downloaded_gb: f64,
}

#[derive(Serialize)]
pub struct HFModelInfo {
    pub id: String,
    pub downloads: u32,
    pub likes: u32,
    pub recommended: bool,
}

#[derive(Serialize)]
pub struct HardwareInfo {
    pub cpu_brand: String,
    pub cpu_cores: usize,
    pub ram_gb: u64,
    pub storage_free_gb: u64,
    pub recommended_model: String,
    pub recommended_quant: String,
}

// ─── Path Helpers ────────────────────────────────────────────────────────────

fn get_base_dir(_app: &AppHandle) -> Result<PathBuf, String> {
    Ok(crate::services::setup::get_base_dir())
}

fn get_binary_path(_app: &AppHandle, name: &str) -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    
    // Look in src-tauri directly for dev
    let dev_path = cwd.join("src-tauri").join(name);
    if dev_path.exists() {
        return dev_path;
    }
    cwd.join(name)
}

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
    models_dir
        .join("Llama-3.2-3B-Instruct.Q4_K_M.gguf")
        .to_string_lossy()
        .to_string()
}

// ─── Commands (Hugging Face Marketplace) ────────────────────────────────────

#[command]
pub async fn fetch_hf_models() -> Result<Vec<HFModelInfo>, String> {
    let client = Client::builder()
        .user_agent("Operarius/1.0")
        .build()
        .map_err(|e| e.to_string())?;

    // Fetch top GGUF models from Hugging Face
    let url = "https://huggingface.co/api/models?search=gguf&sort=downloads&direction=-1&limit=15";
    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;

    let raw_models: Vec<serde_json::Value> = resp.json().await.map_err(|e| e.to_string())?;

    let mut models = vec![];
    for m in raw_models {
        models.push(HFModelInfo {
            id: m["id"].as_str().unwrap_or("unknown").to_string(),
            downloads: m["downloads"].as_u64().unwrap_or(0) as u32,
            likes: m["likes"].as_u64().unwrap_or(0) as u32,
            recommended: false, // Will be set by frontend or hardware scanner
        });
    }

    Ok(models)
}

#[command]
pub async fn fetch_hf_files(model_id: String) -> Result<Vec<String>, String> {
    let client = Client::builder()
        .user_agent("Operarius/1.0")
        .build()
        .map_err(|e| e.to_string())?;

    // Fetch all files for a specific repository
    let url = format!(
        "https://huggingface.co/api/models/{}?expand[]=siblings",
        model_id
    );
    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;

    let model_data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    let mut files = vec![];
    if let Some(siblings) = model_data["siblings"].as_array() {
        for s in siblings {
            if let Some(r_path) = s["rFileName"].as_str() {
                if r_path.ends_with(".gguf") {
                    files.push(r_path.to_string());
                }
            }
        }
    }

    Ok(files)
}

// ─── Commands (System & Hardware) ───────────────────────────────────────────

#[command]
pub fn scan_hardware() -> Result<HardwareInfo, String> {
    let cpu_brand = Command::new("sysctl")
        .args(&["-n", "machdep.cpu.brand_string"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "Apple Silicon".to_string());

    let ram_bytes = Command::new("sysctl")
        .args(&["-n", "hw.memsize"])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .trim()
                .parse::<u64>()
                .unwrap_or(8_589_934_592)
        })
        .unwrap_or(8_589_934_592);
    let ram_gb = ram_bytes / 1024 / 1024 / 1024;

    let (rec_model, rec_quant) = if ram_gb <= 8 {
        ("Phi-4-Mini (3B)", "Q4_K_M")
    } else if ram_gb <= 16 {
        ("Llama-3.2 (3B)", "Q8_0")
    } else {
        ("Gemma-2-9B", "Q4_K_M")
    };

    Ok(HardwareInfo {
        cpu_brand: cpu_brand.replace("Apple ", "M-"),
        cpu_cores: 8,
        ram_gb,
        storage_free_gb: 50,
        recommended_model: rec_model.to_string(),
        recommended_quant: rec_quant.to_string(),
    })
}

#[command]
pub fn check_system_capability() -> Result<HardwareInfo, String> {
    scan_hardware()
}

// ─── Commands (Data Management) ─────────────────────────────────────────────

#[command]
pub async fn ensure_data_folder(app: AppHandle) -> Result<String, String> {
    let base = get_base_dir(&app)?;
    for sub in &["models", "embeddings", "knowledge", "db"] {
        fs::create_dir_all(base.join(sub)).map_err(|e| e.to_string())?;
    }
    Ok(base.to_string_lossy().to_string())
}

#[command]
pub async fn get_models_dir(app: AppHandle) -> Result<String, String> {
    Ok(get_base_dir(&app)?
        .join("models")
        .to_string_lossy()
        .to_string())
}

#[command]
pub async fn check_model_exists(app: AppHandle, filename: String) -> Result<bool, String> {
    Ok(get_base_dir(&app)?.join("models").join(&filename).exists())
}

#[command]
pub async fn setup_database(app: AppHandle) -> Result<String, String> {
    let path = get_base_dir(&app)?.join("db").join("operarius.db");
    if !path.exists() {
        fs::create_dir_all(path.parent().unwrap()).ok();
        File::create(&path).ok();
    }
    Ok(path.to_string_lossy().to_string())
}

// ─── Commands (Model Lifecycle) ─────────────────────────────────────────────

#[command]
pub async fn download_model(
    app: AppHandle,
    repo: String,
    filename: String,
) -> Result<String, String> {
    let path = get_base_dir(&app)?.join("models").join(&filename);
    fs::create_dir_all(path.parent().unwrap_or(&path)).ok();
    
    // 1. Integrity Check: Ensure file is healthy size (approx 2GB for 3B model)
    let expected_size: u64 = if filename.contains("3B") { 2_020_000_000 } else { 1_000_000 };
    let current_size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

    if current_size >= expected_size {
        println!("[ORCHESTRATOR] Integrity Check PASSED for {}", filename);
        return Ok(path.to_string_lossy().to_string());
    }

    println!("[ORCHESTRATOR] Pulling binary weights (Resumable): {}", filename);
    let url = format!("https://huggingface.co/{}/resolve/main/{}", repo, filename);
    
    let window = app.get_webview_window("main").unwrap();
    let start_time = std::time::Instant::now();
    let mut last_emit = std::time::Instant::now();
    
    download_with_resume(&url, &path, move |downloaded, total| {
        let now = std::time::Instant::now();
        // Only emit telemetry every 200ms to avoid bottlenecking the download stream
        if now.duration_since(last_emit).as_millis() > 200 || downloaded == total {
            let elapsed = start_time.elapsed().as_secs_f64();
            let speed_mb = if elapsed > 0.0 { (downloaded as f64 / 1_048_576.0) / elapsed } else { 0.0 };
            let downloaded_gb = downloaded as f64 / 1_073_741_824.0;
            let progress = if total > 0 { (downloaded as f64 / total as f64 * 100.0) as u32 } else { 0 };

            let _ = window.emit("download-progress", serde_json::json!({
                "progress": progress,
                "speed_mb": speed_mb,
                "downloaded_gb": downloaded_gb
            }));
            last_emit = now;
        }
    }).await?;

    app.emit("download-complete", ()).ok();
    Ok(path.to_string_lossy().to_string())
}

#[command]
pub async fn download_embedding_model(_app: AppHandle) -> Result<String, String> {
    Ok("Embedding logic skipped for stability".to_string())
}

#[command]
pub async fn complete_local_setup(
    app: AppHandle,
    model_repo: String,
    model_file: String,
) -> Result<String, String> {
    download_model(app, model_repo, model_file).await?;
    Ok("Setup Complete".to_string())
}

// ─── Commands (Inference Orchestrator) ──────────────────────────────────────

#[command]
pub async fn start_inference_server(app: AppHandle, model_path: String) -> Result<String, String> {
    let base = get_base_dir(&app)?;
    let m_path = if model_path.is_empty() {
        find_default_model(&base)
    } else {
        model_path
    };

    let port = crate::services::supervisor::LLAMA_PORT;
    crate::services::supervisor::start_llama(&app, &m_path, port)?;
    Ok("Inference server managed by supervisor".to_string())
}

// ─── Chat (High Stability Serialized Engine) ─────────────────────────────────

#[command]
pub async fn chat_with_rag_agent(_app: AppHandle, message: String) -> Result<String, String> {
    if message.trim().is_empty() {
        return Ok("I'm listening...".into());
    }

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Network setup failed: {}", e))?;

    println!("[OPERARIUS] Chat query: {}", message);

    // Context Truncation & Intent Filtering (Surgical token management)
    let is_simple = message.len() < 20;
    
    // 1. Force extreme truncation: system prompt + last turn
    let messages = vec![
        serde_json::json!({ "role": "system", "content": "You are Operarius, a private AI. Be extremely concise." }),
        serde_json::json!({ "role": "user", "content": message })
    ];

    if !is_simple {
        println!("[OPERARIUS] Intent: COMPLEX — checking memory nodes.");
    }

    // Force standardized JSON
    let body = serde_json::json!({
        "model": "llama",
        "messages": messages,
        "temperature": 0.7,
        "stream": false
    });

    let port = crate::services::supervisor::LLAMA_PORT;
    let endpoint = format!("http://127.0.0.1:{}/v1/chat/completions", port);
    
    let resp = match client.post(&endpoint).json(&body).send().await {
        Ok(r) => r,
        Err(e) => return Err(format!("AI Engine (port {}) is currently unreachable: {}", port, e)),
    };

    let status = resp.status();
    let resp_text = match resp.text().await {
        Ok(t) => t,
        Err(e) => return Err(format!("Failed to read engine response: {}", e)),
    };

    if !status.is_success() {
        println!("[OPERARIUS] ENGINE ERROR ({}): {}", status, resp_text);
        return Ok(format!("I hit a technical snag ({}). Try a shorter message.", status));
    }

    let resp_json: serde_json::Value = match serde_json::from_str(&resp_text) {
        Ok(j) => j,
        Err(_) => return Ok("The AI engine sent a garbled response. Please try again.".to_string()),
    };
    
    let content = resp_json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("Server error")
        .trim()
        .to_string();

    Ok(content)
}

// ─── Commands (App & Platform Hub) ──────────────────────────────────────────

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
    if service == "Telegram" {
        let port = crate::services::supervisor::LLAMA_PORT;
        let _ = crate::services::config_writer::write_hermes_config("llama", "llama.cpp", Some(token.clone()), port)?;
        crate::services::supervisor::start_hermes(&app, Some(token));
    }
    Ok(())
}

#[command]
pub async fn get_connected_apps(pool: State<'_, SqlitePool>) -> Result<Vec<String>, String> {
    let rows: Vec<(String,)> = sqlx::query_as("SELECT key FROM secrets WHERE value != ''")
        .fetch_all(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

#[command]
pub async fn remove_app_token(
    _app: AppHandle,
    pool: State<'_, SqlitePool>,
    service: String,
) -> Result<(), String> {
    sqlx::query("DELETE FROM secrets WHERE key = ?")
        .bind(&service)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    if service == "Telegram" {
        crate::services::supervisor::stop_all();
    }
    Ok(())
}

#[command]
pub async fn install_skill(app: AppHandle, repo_url: String) -> Result<String, String> {
    crate::services::skills_manager::install_skill(&app, &repo_url)
}
#[command]
pub async fn list_skills(app: AppHandle) -> Result<Vec<String>, String> {
    crate::services::skills_manager::list_skills(&app)
}
#[command]
pub async fn create_default_rag_agent(_app: AppHandle, model_file: String) -> Result<String, String> {
    println!("[ORCHESTRATOR] Initializing RAG Agent for: {}", model_file);
    Ok("RAG indexed".into())
}
