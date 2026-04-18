use tauri::{AppHandle, command, Emitter};
use reqwest::Client;
use std::path::PathBuf;
use std::fs::{self, File};
use std::io::Write;
use futures_util::StreamExt;
use std::time::Instant;
use sqlx::sqlite::SqlitePoolOptions;
use tauri_plugin_keyring::KeyringExt;
use serde::Serialize;
use std::process::Command;
use dirs;

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

fn get_base_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    let base = home.join("Documents").join("Operarius");
    Ok(base)
}

#[command]
pub async fn ensure_data_folder(app: AppHandle) -> Result<String, String> {
    let base = get_base_dir(&app)?;
    fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    Ok(base.to_string_lossy().to_string())
}

#[command]
pub fn scan_hardware() -> Result<HardwareInfo, String> {
    // 1. Get CPU Brand (Native macOS sysctl)
    let cpu_brand = Command::new("sysctl")
        .args(&["-n", "machdep.cpu.brand_string"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "Apple Silicon".to_string());

    // 2. Get CPU Cores
    let cpu_cores = Command::new("sysctl")
        .args(&["-n", "hw.ncpu"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<usize>().unwrap_or(8))
        .unwrap_or(8);

    // 3. Get Total RAM (Native hw.memsize)
    let ram_bytes = Command::new("sysctl")
        .args(&["-n", "hw.memsize"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<u64>().unwrap_or(8589934592))
        .unwrap_or(8589934592);
    let ram_gb = ram_bytes / 1024 / 1024 / 1024;

    // 4. Get Storage Free Space (df)
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
        "Phi-4 Mini Q4_K_M (~0.8 GB)"
    } else if ram_gb <= 24 {
        "Gemma 4 E2B Q4_K_M (~1.4 GB)"
    } else {
        "Gemma 4 E4B Q4_K_M (~2.8 GB)"
    };

    println!("[HARDWARE] Native Scan: {} | {}GB RAM | {}GB Free", cpu_brand, ram_gb, storage_free_gb);

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

#[command]
pub async fn get_models_dir(app: AppHandle) -> Result<String, String> {
    let path = get_base_dir(&app)?.join("models");
    fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[command]
pub async fn save_token(app: AppHandle, token: String) -> Result<(), String> {
    app.keyring().set_password("operarius_auth", "user_token", &token)
        .map_err(|e| e.to_string())
}

#[command]
pub async fn get_token(app: AppHandle) -> Result<String, String> {
    let token = app.keyring().get_password("operarius_auth", "user_token")
        .map_err(|e| e.to_string())?;
    token.ok_or_else(|| "No token found".to_string())
}

#[command]
pub async fn complete_local_setup(
    app: AppHandle, 
    model_repo: String, 
    model_file: String
) -> Result<String, String> {
    let base = get_base_dir(&app)?;
    fs::create_dir_all(&base).map_err(|e| format!("IO: {}", e))?;
    download_model(app.clone(), model_repo, model_file).await?;
    download_embedding_model(app.clone()).await?;
    setup_database(app.clone()).await?;
    Ok("Success".to_string())
}

#[command]
pub async fn download_model(
    app: AppHandle, 
    repo: String, 
    filename: String
) -> Result<String, String> {
    let client = Client::builder().user_agent("Operarius/1.0").build().map_err(|e| e.to_string())?;
    let path = get_base_dir(&app)?.join("models").join(&filename);
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;

    if path.exists() && fs::metadata(&path).map(|m| m.len()).unwrap_or(0) > 50_000_000 {
        app.emit("download-complete", ()).map_err(|e| e.to_string())?;
        return Ok(path.to_string_lossy().to_string());
    }

    let url = format!("https://huggingface.co/{}/resolve/main/{}", repo, filename);
    let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() { return Err(format!("CDN: {}", resp.status())); }

    let total = resp.content_length().ok_or("No size")?;
    let mut file = File::create(&path).map_err(|e| e.to_string())?;
    let mut stream = resp.bytes_stream();
    let mut downloaded = 0;
    let mut last_emit = Instant::now();
    let mut speed_start = Instant::now();
    let mut speed_bytes = 0;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;
        speed_bytes += chunk.len() as u64;
        file.write_all(&chunk).map_err(|e| e.to_string())?;

        if last_emit.elapsed().as_millis() >= 150 {
            let speed = (speed_bytes as f64 / speed_start.elapsed().as_secs_f64().max(0.001)) / 1_048_576.0;
            app.emit("download-progress", DownloadProgress { progress: ((downloaded as f64 / total as f64) * 100.0) as u32, speed_mb: speed, downloaded_gb: downloaded as f64 / 1_073_741_824.0 }).ok();
            speed_bytes = 0;
            speed_start = Instant::now();
            last_emit = Instant::now();
        }
    }
    if downloaded < total { return Err("Truncated".to_string()); }
    app.emit("download-complete", ()).ok();
    Ok(path.to_string_lossy().to_string())
}

#[command]
pub async fn setup_database(app: AppHandle) -> Result<String, String> {
    let path = get_base_dir(&app)?.join("db").join("operarius.db");
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    if !path.exists() { File::create(&path).map_err(|e| e.to_string())?; }
    let pool = SqlitePoolOptions::new().connect(&format!("sqlite:{}", path.to_string_lossy())).await.map_err(|e| e.to_string())?;
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS agents (id TEXT PRIMARY KEY, name TEXT, yaml TEXT, created_at DATETIME DEFAULT CURRENT_TIMESTAMP);
        CREATE TABLE IF NOT EXISTS memory (id INTEGER PRIMARY KEY, agent_id TEXT, key TEXT, value TEXT, learned_at DATETIME DEFAULT CURRENT_TIMESTAMP);
        CREATE TABLE IF NOT EXISTS rag_documents (id INTEGER PRIMARY KEY, content TEXT, embedding BLOB, source TEXT);
    "#).execute(&pool).await.map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[command]
pub async fn download_embedding_model(app: AppHandle) -> Result<String, String> {
    let client = Client::new();
    let filename = "nomic-embed-text-v1.5.Q4_K_M.gguf";
    let path = get_base_dir(&app)?.join("embeddings").join(filename);
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    if path.exists() && fs::metadata(&path).map(|m| m.len()).unwrap_or(0) > 10_000_000 { return Ok(path.to_string_lossy().to_string()); }
    let resp = client.get(format!("https://huggingface.co/nomic-ai/nomic-embed-text-v1.5-GGUF/resolve/main/{}", filename)).send().await.map_err(|e| e.to_string())?;
    let total = resp.content_length().ok_or("No size")?;
    let mut file = File::create(&path).map_err(|e| e.to_string())?;
    let mut downloaded = 0;
    let mut stream = resp.bytes_stream();
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
    }
    if downloaded < total { return Err("Truncated".to_string()); }
    Ok(path.to_string_lossy().to_string())
}

#[command]
pub async fn launch_hermes(app: AppHandle, model_path: String, embedding_path: String) -> Result<String, String> {
    use tauri_plugin_shell::ShellExt;
    let base = get_base_dir(&app)?.join("hermes");
    fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    let sidecar = app.shell().sidecar("hermes").map_err(|e| e.to_string())?.args(&["--model", &model_path, "--embedding", &embedding_path, "--data-dir", &base.to_string_lossy()]);
    sidecar.spawn().map_err(|e| e.to_string())?;
    Ok("Active".to_string())
}
