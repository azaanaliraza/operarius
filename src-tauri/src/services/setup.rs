use std::fs;
use std::path::PathBuf;
use tauri::AppHandle;

pub fn init(_app: &AppHandle) -> Result<(), String> {
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    let base = home.join("Documents").join("Operarius");

    let subdirs = ["models", "embeddings", "knowledge", "db", "logs", "hermes"];

    for sub in &subdirs {
        let path = base.join(sub);
        if !path.exists() {
            fs::create_dir_all(&path).map_err(|e| format!("Failed to create {}: {}", sub, e))?;
        }
    }

    println!("[SETUP] Operarius ecosystem initialized successfully at {:?}", base);
    Ok(())
}

pub fn get_base_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join("Documents").join("Operarius")
}
