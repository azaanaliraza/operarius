use serde::Serialize;
use std::process::Command;
use std::fs;
use dirs;

#[derive(Serialize, Clone)]
pub struct ModelMetadata {
    pub id: String,
    pub name: String,
    pub size_gb: f64,
    pub speed_tps: u32,
    pub use_case: String,
    pub downloaded: bool,
    pub recommended_reason: String,
}

#[derive(Serialize)]
pub struct SystemSpecs {
    pub chip: String,
    pub ram_gb: u32,
    pub free_disk_gb: u32,
    pub gpu: String,
    pub recommended_id: String,
    pub models: Vec<ModelMetadata>,
}

pub fn model_database() -> Vec<ModelMetadata> {
    let home = dirs::home_dir().unwrap_or_default();
    let models_dir = home.join("Documents").join("Operarius").join("models");
    
    let mut catalog = vec![
        ModelMetadata {
            id: "phi-4-mini".to_string(),
            name: "Phi-4 Mini (3.8B)".to_string(),
            size_gb: 2.4,
            speed_tps: 180,
            use_case: "Fast & Precise Coding".to_string(),
            downloaded: false,
            recommended_reason: "Optimized for M-series entry level chips.".to_string(),
        },
        ModelMetadata {
            id: "hermes-3-8b".to_string(),
            name: "Hermes 3 (8B)".to_string(),
            size_gb: 4.9,
            speed_tps: 85,
            use_case: "Smart Personal Assistant".to_string(),
            downloaded: false,
            recommended_reason: "Best balance of intelligence and speed.".to_string(),
        },
        ModelMetadata {
            id: "gemma-2-9b".to_string(),
            name: "Gemma 2 (9B)".to_string(),
            size_gb: 5.4,
            speed_tps: 60,
            use_case: "Deep Creative Thinking".to_string(),
            downloaded: false,
            recommended_reason: "High reasoning capabilities for 16GB+ RAM.".to_string(),
        },
    ];

    // Check disk for existence
    for m in &mut catalog {
        let path = models_dir.join(format!("{}.gguf", m.id));
        m.downloaded = path.exists();
    }

    catalog
}

#[tauri::command]
pub fn scan_system() -> Result<SystemSpecs, String> {
    // 1. Get Chip
    let chip = Command::new("sysctl")
        .args(&["-n", "machdep.cpu.brand_string"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "Apple Silicon".to_string());

    // 2. Get RAM
    let ram_bytes = Command::new("sysctl")
        .args(&["-n", "hw.memsize"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<u64>().unwrap_or(8589934592))
        .unwrap_or(8589934592);
    let ram_gb = (ram_bytes / 1024 / 1024 / 1024) as u32;

    // 3. Get Storage
    let free_disk_gb = Command::new("df")
        .args(&["-g", "/"]) // -g for gigabytes
        .output()
        .map(|o| {
            let out = String::from_utf8_lossy(&o.stdout);
            let lines: Vec<&str> = out.lines().collect();
            if lines.len() > 1 {
                let parts: Vec<&str> = lines[1].split_whitespace().collect();
                if parts.len() > 3 {
                    return parts[3].parse::<u32>().unwrap_or(50);
                }
            }
            50
        })
        .unwrap_or(50);

    // 4. Recommendation Logic
    let recommended_id = if ram_gb <= 8 {
        "phi-4-mini"
    } else if ram_gb <= 16 {
        "hermes-3-8b"
    } else {
        "gemma-2-9b"
    };

    Ok(SystemSpecs {
        chip,
        ram_gb,
        free_disk_gb,
        gpu: "Apple Metal".to_string(),
        recommended_id: recommended_id.to_string(),
        models: model_database(),
    })
}
