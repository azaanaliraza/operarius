use std::process::{Child, Command, Stdio};
use std::fs;
use std::sync::{Mutex, OnceLock};
use std::path::PathBuf;
use std::collections::HashMap;
use std::time::Duration;
use tauri::{AppHandle, Manager, Emitter};
use tokio::time::sleep;

static SUPERVISOR: OnceLock<Mutex<HashMap<&'static str, Child>>> = OnceLock::new();

fn supervisor_map() -> &'static Mutex<HashMap<&'static str, Child>> {
    SUPERVISOR.get_or_init(|| Mutex::new(HashMap::new()))
}

fn get_binary_path(app: &AppHandle, name: &str) -> PathBuf {
    // 1. Try resource dir (Production)
    if let Some(res_dir) = app.path().resource_dir().ok() {
        let bundled = res_dir.join(name);
        if bundled.exists() { return bundled; }
    }
    
    // 2. Try development path
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if cwd.ends_with("src-tauri") {
        cwd.join(name.strip_prefix("src-tauri/").unwrap_or(name))
    } else {
        cwd.join("src-tauri").join(name)
    }
}

pub const LLAMA_PORT: u16 = 8080;
pub const EMBEDDING_PORT: u16 = 8081;
pub const HERMES_PORT: u16 = 8989;

pub fn find_free_port() -> u16 {
    LLAMA_PORT
}

pub fn get_current_port() -> u16 {
    LLAMA_PORT
}


static LLAMA_IGNITING: OnceLock<Mutex<bool>> = OnceLock::new();

pub fn start_llama(app: &AppHandle, model_path: &str, _port: u16) -> Result<(), String> {
    let mut map = supervisor_map().lock().unwrap();
    let igniting = LLAMA_IGNITING.get_or_init(|| Mutex::new(false));

    // 1. Port Guard: If port 8080 is busy and we DON'T own it, someone else is running.
    if std::net::TcpListener::bind(("127.0.0.1", LLAMA_PORT)).is_err() {
        if map.contains_key("llama") {
            println!("[SUPERVISOR] Llama Server is active and owned — skipping.");
            return Ok(());
        } else {
            println!("[SUPERVISOR] WARNING: Port {} is busy but not owned. Attempting Cleanse...", LLAMA_PORT);
            let _ = Command::new("pkill").arg("-9").arg("-f").arg("llama-server").output();
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }

    {
        let mut lock = igniting.lock().unwrap();
        if *lock {
            println!("[SUPERVISOR] Llama Brain ignition in progress — skipping.");
            return Ok(());
        }
        *lock = true;
    }

    let bin = get_binary_path(app, "bin/llama-server");
    let lib_dir = get_binary_path(app, "bin/llama/lib");

    println!("[SUPERVISOR] Igniting Llama Brain on port {}...", LLAMA_PORT);
    println!("[SYSTEM] Llama URI: http://127.0.0.1:{}", LLAMA_PORT);
    
    let child = Command::new(&bin)
        .env("DYLD_LIBRARY_PATH", &lib_dir)
        .args(&[
            "--model", model_path, 
            "--port", &LLAMA_PORT.to_string(), 
            "--ctx-size", "65536", 
            "--parallel", "1",
            "--flash-attn", "on",
            "--jinja",
            "--host", "127.0.0.1"
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| {
            let mut lock = igniting.lock().unwrap();
            *lock = false;
            format!("Llama failed: {}", e)
        })?;

    map.insert("llama", child);
    Ok(())
}

pub async fn wait_for_llama(app: &AppHandle) {
    let url = format!("http://127.0.0.1:{}/health", LLAMA_PORT);
    let client = reqwest::Client::new();
    
    println!("[SUPERVISOR] Waiting for Llama Brain to become CONSCIOUS...");
    for i in 0..60 {
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                println!("[SUPERVISOR] Llama Brain is ACTIVE after {}s.", i);
                let _ = app.emit("brain-status", serde_json::json!({ "ready": true, "context": 131072 }));
                let igniting = LLAMA_IGNITING.get_or_init(|| Mutex::new(false));
                *igniting.lock().unwrap() = false;
                return;
            }
        }
        sleep(Duration::from_secs(1)).await;
    }
    println!("[SUPERVISOR] WARNING: Llama Brain is taking too long. Gateway ignition may fail.");
}

fn ensure_hermes_runtime(app: &AppHandle) -> Result<PathBuf, String> {
    let source = get_binary_path(app, "bin/hermes");
    let target = crate::services::setup::get_base_dir().join("runtime").join("hermes");

    if !target.exists() {
        println!("[SUPERVISOR] Evacuating Hermes to Sanctuary: {:?}", target);
        let _ = fs::create_dir_all(target.parent().unwrap());
        
        // Use cp -R for deep mirror
        let _ = Command::new("cp")
            .arg("-R")
            .arg(&source)
            .arg(&target)
            .output();
    }
    Ok(target)
}

pub fn start_hermes(app: &AppHandle) {
    let mut map = supervisor_map().lock().unwrap();
    
    // 1. Global Cleanse ALWAYS
    let _ = Command::new("pkill").arg("-9").arg("-f").arg("hermes").output();

    if map.contains_key("hermes") {
        println!("[SUPERVISOR] Hermes Gateway is ALREADY OWNED — skipping ignition.");
        return;
    }

    let runtime_dir = match ensure_hermes_runtime(app) {
        Ok(dir) => dir,
        Err(e) => {
            println!("[SUPERVISOR] Sanctuary evacuation failed: {}", e);
            return;
        }
    };

    let bin = runtime_dir.join("hermes");
    let home_dir = crate::services::setup::get_base_dir().join("hermes");
    let _ = fs::create_dir_all(&home_dir);

    println!("[SUPERVISOR] Igniting Hermes Gateway in: {:?}", home_dir);
    println!("[SYSTEM] Hermes Handshake: http://127.0.0.1:{} (Custom Provider)", LLAMA_PORT);
    
    let mut cmd = Command::new(&bin);
    cmd.current_dir(&runtime_dir); // Anchor to Sanctuary
    cmd.env("HERMES_HOME", &home_dir);
    cmd.env("GATEWAY_ALLOW_ALL_USERS", "true");
    
    // Custom Intelligence Handshake (Primary only)
    cmd.env("HERMES_PROVIDER", "custom");
    cmd.env("CUSTOM_MODEL_BASE_URL", format!("http://127.0.0.1:{}/v1", LLAMA_PORT));
    cmd.env("CUSTOM_MODEL_NAME", "llama");
    cmd.env("HERMES_AGENT_CONTEXT_WINDOW", "65536");
    cmd.env("HERMES_MAX_ITERATIONS", "6");
    
    // Fallback Protection
    cmd.env("HERMES_NO_FALLBACK", "true");
    cmd.env("OPENROUTER_API_KEY", "disabled");
    cmd.env("MISTRAL_API_KEY", "disabled");
    

    cmd.arg("gateway").arg("run").arg("--replace");
    
    match cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit()).spawn() {
        Ok(child) => {
            println!("[SUPERVISOR] Hermes Gateway IGNITED.");
            map.insert("hermes", child);
        }
        Err(e) => println!("[SUPERVISOR] Hermes ignition FAILED: {}", e),
    }
}

pub fn start_embedding(app: &AppHandle, model_path: &str) -> Result<(), String> {
    let mut map = supervisor_map().lock().unwrap();

    if map.contains_key("embedding") {
        return Ok(());
    }

    if std::net::TcpListener::bind(("127.0.0.1", EMBEDDING_PORT)).is_err() {
        let _ = Command::new("pkill").arg("-9").arg("-f").arg(format!("--port {}", EMBEDDING_PORT)).output();
    }

    let bin = get_binary_path(app, "bin/llama-server");
    let lib_dir = get_binary_path(app, "bin/llama/lib");

    println!("[SUPERVISOR] Igniting Embedding Server on port {}...", EMBEDDING_PORT);

    let child = Command::new(&bin)
        .env("DYLD_LIBRARY_PATH", &lib_dir)
        .args(&[
            "--model", model_path,
            "--port", &EMBEDDING_PORT.to_string(),
            "--ctx-size", "8192",
            "--embedding",
            "--pooling", "mean",
            "--host", "127.0.0.1"
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("Embedding server failed: {}", e))?;

    map.insert("embedding", child);
    Ok(())
}

pub fn monitor(app: AppHandle, model: String, port: u16, embedding_model: Option<String>) {
    tauri::async_runtime::spawn(async move {
        loop {
            sleep(Duration::from_secs(10)).await; // Less aggressive checking
            let mut map = supervisor_map().lock().unwrap();
            
            // Check Llama
            if map.get_mut("llama").map_or(true, |c| c.try_wait().ok().flatten().is_some()) {
                println!("[SUPERVISOR] Llama restoring...");
                map.remove("llama");
                drop(map);
                let _ = start_llama(&app, &model, port);
                if let Some(ref em) = embedding_model {
                    let _ = start_embedding(&app, em);
                }
            }
        }
    });
}

pub fn stop_all() {
    let mut map = supervisor_map().lock().unwrap();
    for (_, mut child) in map.drain() {
        let _ = child.kill();
    }
    let _ = Command::new("pkill").arg("-9").arg("llama-server").output();
    let _ = Command::new("pkill").arg("-9").arg("-f").arg("hermes").output();
}
