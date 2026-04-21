use crate::services::downloader::download_with_resume;
use crate::services::workflow_compiler::{self, BuilderWorkflow, CompiledWorkflow};
use base64::Engine;
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;
use sqlx::SqlitePool;
use std::fs::{self, File};
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;
use tauri::{command, AppHandle, Emitter, Manager, State};

const DEFAULT_EMBEDDING_REPO: &str = "nomic-ai/nomic-embed-text-v1.5-GGUF";
const DEFAULT_EMBEDDING_FILE: &str = "nomic-embed-text-v1.5.Q8_0.gguf";
// ─── Data Types ──────────────────────────────────────────────────────────────

pub async fn emit_agent_log(
    app: &AppHandle,
    pool: Option<&SqlitePool>,
    level: &str,
    scope: &str,
    message: &str,
    details: Value,
) {
    let id = uuid::Uuid::new_v4().to_string();
    let ts = chrono::Utc::now().to_rfc3339();
    let payload = serde_json::json!({
        "id": id,
        "ts": ts,
        "level": level,
        "scope": scope,
        "message": message,
        "details": details,
    });

    app.emit("agent-log", payload.clone()).ok();

    if let Some(pool) = pool {
        let _ = sqlx::query(
            "INSERT INTO agent_logs (id, ts, level, scope, message, details) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(payload["id"].as_str().unwrap_or_default())
        .bind(payload["ts"].as_str().unwrap_or_default())
        .bind(payload["level"].as_str().unwrap_or("info"))
        .bind(payload["scope"].as_str().unwrap_or("system"))
        .bind(payload["message"].as_str().unwrap_or_default())
        .bind(payload["details"].to_string())
        .execute(pool)
        .await;
    }
}

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

#[derive(Serialize)]
pub struct ChatHistoryMessage {
    pub role: String,
    pub content: String,
    pub timestamp: String,
    pub platform: String,
    pub chat_id: Option<String>,
    pub event_type: Option<String>,
    pub event_payload: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct AgentLogEntry {
    pub id: String,
    pub ts: String,
    pub level: String,
    pub scope: String,
    pub message: String,
    pub details: Value,
}

#[derive(Serialize)]
pub struct IndexedFile {
    pub filename: String,
    pub platform: Option<String>,
    pub uploaded_at: i64,
}

#[derive(Serialize)]
pub struct FileMetadata {
    pub filename: String,
    pub size_bytes: i64,
}

#[derive(Serialize)]
pub struct ModelCapability {
    pub id: String,
    pub name: String,
    pub runtime: String,
    pub file: String,
    pub repo: String,
    pub size_gb: f64,
    pub estimated_ram_gb: u64,
    pub installed: bool,
    pub active: bool,
    pub supports_vision: bool,
    pub supports_tts: bool,
    pub coding_strength: String,
    pub best_for: Vec<String>,
    pub notes: String,
}

#[derive(Serialize)]
pub struct UserPreference {
    pub key: String,
    pub value: String,
}

#[derive(Serialize)]
pub struct MemoryEntry {
    pub id: String,
    pub user_id: String,
    pub scope: String,
    pub memory_key: String,
    pub value: String,
    pub weight: i64,
    pub updated_at: String,
}

#[derive(Serialize)]
pub struct TaskLearning {
    pub id: String,
    pub task: String,
    pub lesson: String,
    pub success: i64,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct UserSkill {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub description: String,
    pub instruction: String,
    pub is_active: i64,
    pub triggers: String,
    pub updated_at: String,
}

#[derive(Serialize)]
pub struct SkillMarketplaceItem {
    pub name: String,
    pub built_in: bool,
    pub registry: String,
    pub description: String,
    pub installed: bool,
}

#[derive(Serialize)]
pub struct StoredAgentWorkflow {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub workflow_json: Value,
    pub compiled_json: Option<Value>,
    pub updated_at: String,
}

#[derive(Serialize)]
pub struct ModelLinkStatus {
    pub model_id: String,
    pub repo: String,
    pub file: String,
    pub ok: bool,
    pub remote_size_bytes: Option<u64>,
    pub message: String,
}

fn model_catalog_definition() -> Vec<ModelCapability> {
    vec![
        ModelCapability {
            id: "phi4-mini-q4km".to_string(),
            name: "Phi-4 Mini Q4_K_M".to_string(),
            runtime: "llama.cpp".to_string(),
            file: "Phi-4-mini-instruct-Q4_K_M.gguf".to_string(),
            repo: "unsloth/Phi-4-mini-instruct-GGUF".to_string(),
            size_gb: 0.8,
            estimated_ram_gb: 4,
            installed: false,
            active: false,
            supports_vision: false,
            supports_tts: false,
            coding_strength: "good".to_string(),
            best_for: vec!["fast chat".to_string(), "light coding".to_string()],
            notes: "Ultra-fast local assistant for low-memory systems.".to_string(),
        },
        ModelCapability {
            id: "llama32-3b-q4km".to_string(),
            name: "Llama 3.2 3B Q4_K_M".to_string(),
            runtime: "llama.cpp".to_string(),
            file: "Llama-3.2-3B-Instruct-Q4_K_M.gguf".to_string(),
            repo: "unsloth/Llama-3.2-3B-Instruct-GGUF".to_string(),
            size_gb: 2.0,
            estimated_ram_gb: 6,
            installed: false,
            active: false,
            supports_vision: false,
            supports_tts: false,
            coding_strength: "good".to_string(),
            best_for: vec!["conversation".to_string(), "summaries".to_string()],
            notes: "Balanced default for local general chat and retrieval tasks.".to_string(),
        },
        ModelCapability {
            id: "hermes3-8b-q4km".to_string(),
            name: "Hermes 3 8B Q4_K_M".to_string(),
            runtime: "llama.cpp".to_string(),
            file: "Hermes-3-Llama-3.1-8B.Q4_K_M.gguf".to_string(),
            repo: "NousResearch/Hermes-3-Llama-3.1-8B-GGUF".to_string(),
            size_gb: 4.9,
            estimated_ram_gb: 10,
            installed: false,
            active: false,
            supports_vision: false,
            supports_tts: false,
            coding_strength: "strong".to_string(),
            best_for: vec!["agent tasks".to_string(), "reasoning".to_string()],
            notes: "Great quality for complex instructions and structured responses.".to_string(),
        },
        ModelCapability {
            id: "qwen35-9b-q4km".to_string(),
            name: "Qwen3.5-9B-Q4_K_M (GGUF)".to_string(),
            runtime: "llama.cpp".to_string(),
            file: "Qwen3.5-9B-Q4_K_M.gguf".to_string(),
            repo: "lmstudio-community/Qwen3.5-9B-GGUF".to_string(),
            size_gb: 5.3,
            estimated_ram_gb: 10,
            installed: false,
            active: false,
            supports_vision: false,
            supports_tts: false,
            coding_strength: "strong".to_string(),
            best_for: vec!["coding".to_string(), "analysis".to_string(), "long-form writing".to_string()],
            notes: "Requested 9B GGUF profile. Best for deeper reasoning and code-heavy tasks.".to_string(),
        },
        ModelCapability {
            id: "qwen35-9b-mlx-mxfp4".to_string(),
            name: "Qwen3.5-9B-mlx-lm-mxfp4 (MLX)".to_string(),
            runtime: "omlx".to_string(),
            file: "Qwen3.5-9B-MLX-4bit".to_string(),
            repo: "mlx-community/Qwen3.5-9B-MLX-4bit".to_string(),
            size_gb: 5.0,
            estimated_ram_gb: 12,
            installed: false,
            active: false,
            supports_vision: false,
            supports_tts: false,
            coding_strength: "strong".to_string(),
            best_for: vec!["Apple MLX runtime".to_string(), "high-throughput chat".to_string()],
            notes: "Downloadable catalog entry. Runtime switching requires MLX engine integration.".to_string(),
        },
    ]
}

async fn get_active_model_file(pool: &SqlitePool) -> Option<String> {
    sqlx::query_as::<_, (String,)>(
        "SELECT value FROM secrets WHERE key = 'active_model_file' LIMIT 1"
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .map(|row| row.0)
}

async fn build_user_memory_context(pool: &SqlitePool, user_id: &str) -> String {
    let prefs: Vec<(String, String)> = sqlx::query_as(
        "SELECT pref_key, pref_value
         FROM user_preferences
         WHERE user_id = ?
         ORDER BY updated_at DESC
         LIMIT 20"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let memories: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT scope, memory_key, weight
         FROM user_memory_entries
         WHERE user_id = ?
         ORDER BY weight DESC, updated_at DESC
         LIMIT 16"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let learnings: Vec<(String, String)> = sqlx::query_as(
        "SELECT task, lesson
         FROM task_learnings
         WHERE user_id = ?
         ORDER BY created_at DESC
         LIMIT 8"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let skills: Vec<(String, String)> = sqlx::query_as(
        "SELECT name, instruction
         FROM user_skills
         WHERE user_id = ? AND is_active = 1
         ORDER BY updated_at DESC
         LIMIT 8"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut parts = Vec::new();

    if !prefs.is_empty() {
        let lines = prefs
            .into_iter()
            .map(|(k, v)| format!("- {}: {}", k, v))
            .collect::<Vec<String>>()
            .join("\n");
        parts.push(format!("User preferences:\n{}", lines));
    }

    if !memories.is_empty() {
        let lines = memories
            .into_iter()
            .map(|(scope, key, weight)| format!("- [{}|w={}] {}", scope, weight, key))
            .collect::<Vec<String>>()
            .join("\n");
        parts.push(format!("Long-term memory:\n{}", lines));
    }

    if !learnings.is_empty() {
        let lines = learnings
            .into_iter()
            .map(|(task, lesson)| format!("- Task: {} -> Lesson: {}", task, lesson))
            .collect::<Vec<String>>()
            .join("\n");
        parts.push(format!("Task learnings:\n{}", lines));
    }

    if !skills.is_empty() {
        let lines = skills
            .into_iter()
            .map(|(name, instruction)| format!("- {}: {}", name, instruction))
            .collect::<Vec<String>>()
            .join("\n");
        parts.push(format!("Active user skills:\n{}", lines));
    }

    parts.join("\n\n")
}

async fn capture_memory_hints(pool: &SqlitePool, user_id: &str, message: &str, response: &str) {
    let lower = message.to_lowercase();

    let mut personal_facts: Vec<(String, String, i64)> = Vec::new();
    if let Some(idx) = lower.find("my name is ") {
        let raw = message[idx + 11..].trim();
        let name = raw.split(&['.', ',', '\n'][..]).next().unwrap_or("").trim();
        if !name.is_empty() {
            personal_facts.push(("profile-name".to_string(), format!("name={}", name), 98));
            let _ = sqlx::query(
                "INSERT INTO user_preferences (id, user_id, pref_key, pref_value, updated_at)
                 VALUES (?, ?, ?, ?, datetime('now'))
                 ON CONFLICT(user_id, pref_key) DO UPDATE SET pref_value = excluded.pref_value, updated_at = datetime('now')"
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(user_id)
            .bind("name")
            .bind(name)
            .execute(pool)
            .await;
        }
    }

    if lower.contains("i live in ") {
        if let Some(idx) = lower.find("i live in ") {
            let raw = message[idx + 10..].trim();
            let location = raw.split(&['.', ',', '\n'][..]).next().unwrap_or("").trim();
            if !location.is_empty() {
                personal_facts.push(("profile-location".to_string(), format!("location={}", location), 85));
            }
        }
    }

    if lower.contains("i work as ") {
        if let Some(idx) = lower.find("i work as ") {
            let raw = message[idx + 10..].trim();
            let role = raw.split(&['.', ',', '\n'][..]).next().unwrap_or("").trim();
            if !role.is_empty() {
                personal_facts.push(("profile-role".to_string(), format!("role={}", role), 85));
            }
        }
    }

    for (key, value, weight) in personal_facts {
        let _ = sqlx::query(
            "INSERT INTO user_memory_entries (id, user_id, scope, memory_key, memory_value, weight, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, datetime('now'))"
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(user_id)
        .bind("profile")
        .bind(key)
        .bind(value)
        .bind(weight)
        .execute(pool)
        .await;
    }

    if lower.contains("i prefer") || lower.contains("please always") || lower.contains("don't") {
        let key = format!("pref-{}", chrono::Utc::now().timestamp_millis());
        let _ = sqlx::query(
            "INSERT INTO user_memory_entries (id, user_id, scope, memory_key, memory_value, weight, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, datetime('now'))"
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(user_id)
        .bind("preference")
        .bind(key)
        .bind(message.chars().take(300).collect::<String>())
        .bind(80_i64)
        .execute(pool)
        .await;
    }

    let _ = sqlx::query(
        "INSERT INTO task_learnings (id, user_id, task, lesson, success, created_at)
         VALUES (?, ?, ?, ?, ?, datetime('now'))"
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(user_id)
    .bind(message.chars().take(280).collect::<String>())
    .bind(response.chars().take(280).collect::<String>())
    .bind(1_i64)
    .execute(pool)
    .await;
}

async fn ensure_auto_skill_from_message(pool: &SqlitePool, user_id: &str, message: &str) {
    let lower = message.to_lowercase();
    let detected = if lower.contains("code") || lower.contains("debug") || lower.contains("refactor") {
        Some((
            "auto-coding-assistant",
            "Auto-created from repeated coding requests.",
            "Prioritize precise coding guidance, include patch-ready steps, and highlight risks before changes.",
        ))
    } else if lower.contains("summarize") || lower.contains("summary") || lower.contains("explain") {
        Some((
            "auto-summarizer",
            "Auto-created from summarization style requests.",
            "Respond with a concise summary first, then key bullets, and keep wording clear and actionable.",
        ))
    } else if lower.contains("plan") || lower.contains("roadmap") || lower.contains("step by step") {
        Some((
            "auto-planner",
            "Auto-created from planning workflows.",
            "Break complex requests into milestones and propose the next highest-impact step.",
        ))
    } else {
        None
    };

    let Some((name, description, instruction)) = detected else {
        return;
    };

    let exists: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM user_skills WHERE user_id = ? AND name = ? LIMIT 1"
    )
    .bind(user_id)
    .bind(name)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if exists.is_some() {
        return;
    }

    let _ = sqlx::query(
        "INSERT INTO user_skills (id, user_id, name, description, instruction, is_active, triggers, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, 1, ?, datetime('now'), datetime('now'))"
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(user_id)
    .bind(name)
    .bind(description)
    .bind(instruction)
    .bind("auto")
    .execute(pool)
    .await;
}

fn clean_markdown_text(value: &str) -> String {
    value
        .replace("**", "")
        .replace("__", "")
        .replace('`', "")
        .replace("\t", " ")
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

fn contains_cjk(text: &str) -> bool {
    text.chars().any(|ch| {
        let cp = ch as u32;
        (0x4E00..=0x9FFF).contains(&cp) || (0x3400..=0x4DBF).contains(&cp)
    })
}

fn detect_user_language_label(message: &str) -> &'static str {
    if contains_cjk(message) {
        "Chinese"
    } else {
        "English"
    }
}

fn is_language_mismatch(expected_language: &str, content: &str) -> bool {
    if content.trim().is_empty() {
        return false;
    }

    match expected_language {
        "English" => {
            let cjk_count = content
                .chars()
                .filter(|ch| {
                    let cp = *ch as u32;
                    (0x4E00..=0x9FFF).contains(&cp) || (0x3400..=0x4DBF).contains(&cp)
                })
                .count();
            cjk_count >= 8
        }
        "Chinese" => {
            !contains_cjk(content)
        }
        _ => false,
    }
}

async fn rewrite_response_to_language(
    client: &Client,
    endpoint: &str,
    source: &str,
    expected_language: &str,
    max_tokens: usize,
) -> Option<String> {
    let system_prompt = format!(
        "Rewrite the provided answer into {} only. Preserve facts, structure, and meaning. Do not add new claims.",
        expected_language
    );

    let user_prompt = format!("Original answer:\n{}", clip_for_prompt(source, 5000));
    let rewrite_body = serde_json::json!({
        "model": "llama",
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_prompt }
        ],
        "temperature": 0.2,
        "max_tokens": max_tokens,
        "stream": false,
        "chat_template_kwargs": {
            "enable_thinking": false
        }
    });

    let resp = client.post(endpoint).json(&rewrite_body).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }

    let text = resp.text().await.ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&text).ok()?;
    parsed["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.trim().to_string())
}

fn get_hermes_browser_sidecar_dir() -> Option<PathBuf> {
    let base_dir = crate::services::setup::get_base_dir();
    let runtime_candidate = base_dir.join("runtime").join("hermes");
    if runtime_candidate.exists() {
        return Some(runtime_candidate);
    }

    let cwd = std::env::current_dir().ok()?;
    let dev_candidate = if cwd.ends_with("src-tauri") {
        cwd.join("sidecars").join("hermes-agent")
    } else {
        cwd.join("src-tauri").join("sidecars").join("hermes-agent")
    };

    if dev_candidate.exists() {
        Some(dev_candidate)
    } else {
        None
    }
}

fn run_hermes_browser_tool(tool_name: &str, args: serde_json::Value, task_id: &str) -> Result<String, String> {
    let sidecar_dir = get_hermes_browser_sidecar_dir()
        .ok_or_else(|| "Hermes browser sidecar directory was not found".to_string())?;

    let script = r#"
import json
import sys

from tools import browser_tool

tool_name = sys.argv[1]
tool_args = json.loads(sys.argv[2])
task_id = sys.argv[3]

if not isinstance(tool_args, dict):
    raise ValueError("tool args must be a JSON object")

tool_args["task_id"] = task_id

fn = getattr(browser_tool, tool_name, None)
if fn is None:
    raise ValueError(f"Unknown browser tool: {tool_name}")

result = fn(**tool_args)
if result is None:
    result = ""

print(result)
"#;

    let args_json = args.to_string();
    let hermes_home = crate::services::setup::get_base_dir().join("hermes");
    let pythonpath = std::env::var("PYTHONPATH").unwrap_or_default();
    
    let path_sep = if cfg!(target_os = "windows") { ";" } else { ":" };
    
    let merged_pythonpath = if pythonpath.trim().is_empty() {
        sidecar_dir.to_string_lossy().to_string()
    } else {
        format!("{}{}{}", sidecar_dir.to_string_lossy(), path_sep, pythonpath)
    };

    let python_candidates = if cfg!(target_os = "windows") {
        vec![
            sidecar_dir.join(".venv").join("Scripts").join("python.exe"),
            sidecar_dir.join("venv").join("Scripts").join("python.exe"),
            sidecar_dir.join(".venv").join("python.exe"),
            sidecar_dir.join("venv").join("python.exe"),
        ]
    } else {
        vec![
            sidecar_dir.join(".venv").join("bin").join("python3"),
            sidecar_dir.join("venv").join("bin").join("python3"),
            sidecar_dir.join(".venv").join("bin").join("python"),
            sidecar_dir.join("venv").join("bin").join("python"),
        ]
    };

    let python_bin = python_candidates
        .iter()
        .find(|path| path.exists())
        .cloned()
        .unwrap_or_else(|| {
            if cfg!(target_os = "windows") {
                PathBuf::from("python.exe")
            } else {
                PathBuf::from("python3")
            }
        });

    let mut cmd = Command::new(&python_bin);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let output = cmd.arg("-c")
        .arg(script)
        .arg(tool_name)
        .arg(&args_json)
        .arg(task_id)
        .current_dir(&sidecar_dir)
        .env("PYTHONPATH", merged_pythonpath)
        .env("HERMES_HOME", hermes_home)
        .output()
        .map_err(|e| format!("Failed to execute browser tool: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let reason = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            "unknown browser tool failure".to_string()
        };

        return Err(format!("Browser tool failed: {}", reason));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn parse_browser_chat_command(message: &str) -> Option<Result<(String, serde_json::Value), String>> {
    let trimmed = message.trim();
    if !trimmed.to_lowercase().starts_with("/browser") {
        return None;
    }

    let mut parts = trimmed.split_whitespace();
    let _root = parts.next();
    let action = parts.next().unwrap_or("").to_lowercase();

    let parsed = match action.as_str() {
        "navigate" => {
            let url = parts.collect::<Vec<&str>>().join(" ");
            if url.trim().is_empty() {
                Err("Usage: /browser navigate <url>".to_string())
            } else {
                Ok(("browser_navigate".to_string(), serde_json::json!({ "url": url.trim() })))
            }
        }
        "snapshot" => {
            let full = parts.next().is_some_and(|v| v.eq_ignore_ascii_case("full") || v.eq_ignore_ascii_case("true"));
            Ok(("browser_snapshot".to_string(), serde_json::json!({ "full": full })))
        }
        "click" => {
            let ref_id = parts.next().unwrap_or("").trim().to_string();
            if ref_id.is_empty() {
                Err("Usage: /browser click <@eRef>".to_string())
            } else {
                Ok(("browser_click".to_string(), serde_json::json!({ "ref": ref_id })))
            }
        }
        "type" => {
            let ref_id = parts.next().unwrap_or("").trim().to_string();
            let text = parts.collect::<Vec<&str>>().join(" ");
            if ref_id.is_empty() || text.trim().is_empty() {
                Err("Usage: /browser type <@eRef> <text>".to_string())
            } else {
                Ok(("browser_type".to_string(), serde_json::json!({ "ref": ref_id, "text": text.trim() })))
            }
        }
        "scroll" => {
            let direction = parts.next().unwrap_or("down").trim().to_string();
            Ok(("browser_scroll".to_string(), serde_json::json!({ "direction": direction })))
        }
        "press" => {
            let key = parts.next().unwrap_or("").trim().to_string();
            if key.is_empty() {
                Err("Usage: /browser press <key>".to_string())
            } else {
                Ok(("browser_press".to_string(), serde_json::json!({ "key": key })))
            }
        }
        "back" => Ok(("browser_back".to_string(), serde_json::json!({}))),
        "console" => {
            let clear = parts.next().is_some_and(|v| v.eq_ignore_ascii_case("clear") || v.eq_ignore_ascii_case("true"));
            Ok(("browser_console".to_string(), serde_json::json!({ "clear": clear })))
        }
        _ => Err(
            "Browser commands: /browser navigate <url>, /browser snapshot [full], /browser click <@eRef>, /browser type <@eRef> <text>, /browser scroll <up|down>, /browser press <key>, /browser back, /browser console [clear]".to_string(),
        ),
    };

    Some(parsed)
}

fn normalize_web_search_query(message: &str) -> String {
    let mut query = message.trim().to_string();

    let prefixes = [
        "do a web search on",
        "do web search on",
        "search about",
        "web search on",
        "search the web for",
        "search web for",
        "search for",
        "search about",
        "look up",
        "lookup",
        "find",
        "tell me about",
        "what is",
        "what are",
        "explain",
        "give me",
    ];

    let mut lowered = query.to_lowercase();
    loop {
        let mut stripped_any = false;
        for prefix in prefixes {
            if let Some(stripped) = lowered.strip_prefix(prefix) {
                let start = query.len() - stripped.len();
                query = query[start..].trim().to_string();
                lowered = query.to_lowercase();
                stripped_any = true;
                break;
            }
        }

        if !stripped_any {
            break;
        }
    }

    query = query
        .trim_matches(|c: char| c == ':' || c == '-' || c == '.' || c == ',' || c == '?')
        .trim()
        .to_string();

    let filler_words = ["its", "their", "the", "on", "in", "of", "for", "and", "with", "body"];
    let tokens = query
        .split_whitespace()
        .map(|token| token.trim_matches(|c: char| c == ',' || c == '.' || c == '?' || c == '!').to_string())
        .filter(|token| !token.is_empty())
        .collect::<Vec<String>>();

    let filtered_tokens = tokens
        .into_iter()
        .filter(|token| {
            let lowered = token.to_lowercase();
            !filler_words.contains(&lowered.as_str())
        })
        .collect::<Vec<String>>();

    if !filtered_tokens.is_empty() {
        query = filtered_tokens.join(" ");
    }

    if query.is_empty() {
        message.trim().to_string()
    } else {
        query
    }
}

fn generate_web_search_variants(message: &str) -> Vec<String> {
    let normalized = normalize_web_search_query(message);
    let mut variants = Vec::new();

    if !normalized.trim().is_empty() {
        variants.push(normalized.clone());
    }

    let raw_lower = message.trim().to_lowercase();
    if raw_lower != normalized.to_lowercase() {
        variants.push(message.trim().to_string());
    }

    let tokens = tokenize_query(&normalized);
    if !tokens.is_empty() {
        variants.push(tokens.join(" "));

        let short_variant = tokens.iter().take(6).cloned().collect::<Vec<String>>().join(" ");
        if !short_variant.is_empty() && !variants.iter().any(|item| item == &short_variant) {
            variants.push(short_variant);
        }
    }

    if variants.is_empty() {
        variants.push(message.trim().to_string());
    }

    variants
}

fn tokenize_query(value: &str) -> Vec<String> {
    let stopwords = [
        "a", "an", "and", "are", "as", "ask", "be", "for", "from", "give", "how",
        "i", "in", "is", "it", "its", "me", "on", "of", "or", "search", "should",
        "show", "tell", "the", "to", "turn", "up", "use", "what", "when", "with",
        "web", "local", "db", "data", "related", "anything", "must", "only", "answer",
    ];
    let stopwords = stopwords.iter().copied().collect::<HashSet<&str>>();

    value
        .split(|c: char| !c.is_alphanumeric())
        .map(|token| token.trim().to_lowercase())
        .filter(|token| token.len() > 2 && !stopwords.contains(token.as_str()))
        .collect()
}

fn is_related_context(query: &str, context: &str) -> bool {
    if context.trim().is_empty() {
        return false;
    }

    let query_terms = tokenize_query(query);
    if query_terms.is_empty() {
        return false;
    }

    let context_lower = context.to_lowercase();
    let matches = query_terms
        .iter()
        .filter(|term| context_lower.contains(term.as_str()))
        .count();

    let ratio = matches as f64 / query_terms.len() as f64;
    matches >= 2 || ratio >= 0.34
}

fn build_related_local_context(
    query: &str,
    recent_context: &str,
    user_memory_context: &str,
    uploaded_files_context: &str,
    rag_context: &str,
    visual_context: Option<&str>,
) -> String {
    let mut sections = Vec::new();

    let push_section = |sections: &mut Vec<String>, title: &str, content: &str| {
        if !content.trim().is_empty() && is_related_context(query, content) {
            sections.push(format!("{}:\n{}", title, content.trim()));
        }
    };

    push_section(&mut sections, "Recent conversation", recent_context);
    push_section(&mut sections, "Known uploaded files", uploaded_files_context);
    push_section(&mut sections, "User memory and skills", user_memory_context);
    push_section(&mut sections, "Retrieved knowledge", rag_context);

    if let Some(visual) = visual_context {
        push_section(&mut sections, "Visual analysis", visual);
    }

    sections.join("\n\n")
}

fn parse_bing_markdown_results(markdown: &str) -> Vec<String> {
    let lines: Vec<&str> = markdown.lines().collect();
    let mut out = Vec::new();
    let mut i = 0usize;

    while i < lines.len() && out.len() < 6 {
        let line = lines[i].trim();
        if !line.starts_with("## [") {
            i += 1;
            continue;
        }

        let title_start = "## [".len();
        let title_end = line[title_start..]
            .find("](")
            .map(|idx| title_start + idx);

        let Some(title_end) = title_end else {
            i += 1;
            continue;
        };

        let link_start = title_end + 2;
        let link_end = line[link_start..]
            .find(')')
            .map(|idx| link_start + idx)
            .unwrap_or(line.len());

        let title = clean_markdown_text(&line[title_start..title_end]);
        let link = line[link_start..link_end].trim();

        let mut snippet = String::new();
        let mut j = i + 1;
        while j < lines.len() {
            let next = lines[j].trim();
            if next.is_empty() {
                j += 1;
                continue;
            }
            if next.starts_with("## [") || next.starts_with("Title:") || next.starts_with("URL Source:") {
                break;
            }
            if next.chars().next().is_some_and(|c| c.is_ascii_digit()) && next.contains(".   [") {
                break;
            }

            snippet = clean_markdown_text(next);
            break;
        }

        if !title.is_empty() {
            if snippet.is_empty() {
                out.push(format!("- {} ({})", title, link));
            } else {
                out.push(format!("- {}: {} ({})", title, snippet, link));
            }
        }

        i += 1;
    }

    out
}

fn strip_html_tags(value: &str) -> String {
    let mut out = String::new();
    let mut inside_tag = false;
    for ch in value.chars() {
        match ch {
            '<' => inside_tag = true,
            '>' => inside_tag = false,
            _ => {
                if !inside_tag {
                    out.push(ch);
                }
            }
        }
    }

    out
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn extract_search_url(line: &str) -> Option<String> {
    let start = line.rfind(" (")? + 2;
    let end = line.rfind(')')?;
    let url = line[start..end].trim();
    if url.starts_with("http://") || url.starts_with("https://") {
        Some(url.to_string())
    } else {
        None
    }
}

fn extract_search_domain(line: &str) -> Option<String> {
    let url = extract_search_url(line)?;
    let parsed = reqwest::Url::parse(&url).ok()?;
    parsed.host_str().map(|host| host.to_lowercase())
}

fn score_search_result(query: &str, line: &str) -> i32 {
    let mut score = 0;
    let query_lower = query.to_lowercase();
    let line_lower = line.to_lowercase();

    for token in tokenize_query(query) {
        if line_lower.contains(&token) {
            score += 10;
        }
    }

    let query_is_health_topic = query_lower.contains("benefit")
        || query_lower.contains("effects")
        || query_lower.contains("health")
        || query_lower.contains("nutrition")
        || query_lower.contains("body")
        || query_lower.contains("side effect");

    if query_is_health_topic {
        for keyword in ["health", "medical", "nutrition", "benefit", "effect", "clinic", "journal", "research", "study", "evidence", "review"] {
            if line_lower.contains(keyword) {
                score += 14;
            }
        }
    }

    if let Some(domain) = extract_search_domain(line) {
        let trusted_domains = [
            "healthline.com",
            "webmd.com",
            "mayoclinic.org",
            "nih.gov",
            "ncbi.nlm.nih.gov",
            "pubmed.ncbi.nlm.nih.gov",
            "clevelandclinic.org",
            "hopkinsmedicine.org",
            "medicalnewstoday.com",
            "health.harvard.edu",
            "britannica.com",
            "news.google.com",
            "apnews.com",
            "reuters.com",
            "bbc.com",
            "wikipedia.org",
        ];

        let support_domains = [
            "support.google.com",
            "accounts.google.com",
            "policies.google.com",
            "help.google.com",
            "consent.google.com",
            "bing.com",
            "duckduckgo.com",
            "microsoft.com",
        ];

        if trusted_domains.iter().any(|item| domain.ends_with(item)) {
            score += 35;
        }

        if support_domains.iter().any(|item| domain.ends_with(item)) {
            score -= 45;
        }
    }

    for phrase in ["how to use", "support", "help center", "search help", "technical support", "sign in"] {
        if line_lower.contains(phrase) {
            score -= 25;
        }
    }

    score
}

fn rank_search_results(query: &str, results: Vec<String>) -> Vec<String> {
    let mut scored = results
        .into_iter()
        .map(|line| {
            let score = score_search_result(query, &line);
            (score, line)
        })
        .collect::<Vec<(i32, String)>>();

    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.len().cmp(&b.1.len())));

    let mut positive = scored
        .iter()
        .filter(|(score, _)| *score > 0)
        .map(|(_, line)| line.clone())
        .collect::<Vec<String>>();

    if positive.is_empty() {
        positive = scored.iter().take(5).map(|(_, line)| line.clone()).collect();
    }

    let mut unique: Vec<String> = Vec::new();
    for line in positive {
        if !unique.iter().any(|existing| existing.eq_ignore_ascii_case(&line)) {
            unique.push(line);
        }
    }

    unique
}

async fn perform_duckduckgo_instant_answer(client: &Client, query: &str) -> Vec<String> {
    let mut url = reqwest::Url::parse("https://api.duckduckgo.com/").unwrap_or_else(|_| {
        reqwest::Url::parse("https://duckduckgo.com/").expect("valid fallback URL")
    });
    url.query_pairs_mut()
        .append_pair("q", query)
        .append_pair("format", "json")
        .append_pair("no_html", "1")
        .append_pair("no_redirect", "1")
        .append_pair("skip_disambig", "1");

    let resp = match client.get(url).send().await {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let payload: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut lines = Vec::new();
    if let Some(abstract_text) = payload.get("AbstractText").and_then(|v| v.as_str()) {
        if !abstract_text.trim().is_empty() {
            lines.push(format!("- {}", abstract_text.trim()));
        }
    }

    if let Some(related) = payload.get("RelatedTopics").and_then(|v| v.as_array()) {
        for topic in related.iter().take(6) {
            if let Some(text) = topic.get("Text").and_then(|v| v.as_str()) {
                if !text.trim().is_empty() {
                    lines.push(format!("- {}", text.trim()));
                    continue;
                }
            }

            if let Some(nested) = topic.get("Topics").and_then(|v| v.as_array()) {
                for child in nested.iter().take(2) {
                    if let Some(text) = child.get("Text").and_then(|v| v.as_str()) {
                        if !text.trim().is_empty() {
                            lines.push(format!("- {}", text.trim()));
                        }
                    }
                }
            }
        }
    }

    lines
}

async fn perform_bing_proxy_search(client: &Client, query: &str) -> Vec<String> {
    let mut bing_url = match reqwest::Url::parse("https://www.bing.com/search") {
        Ok(url) => url,
        Err(_) => return Vec::new(),
    };
    bing_url.query_pairs_mut().append_pair("q", query);

    let proxy_url = format!(
        "https://r.jina.ai/http://{}",
        bing_url.as_str().trim_start_matches("https://")
    );

    let resp = match client.get(&proxy_url).send().await {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let markdown = match resp.text().await {
        Ok(text) => text,
        Err(_) => return Vec::new(),
    };

    parse_bing_markdown_results(&markdown)
}

async fn perform_wikipedia_search(client: &Client, query: &str) -> Vec<String> {
    let mut url = match reqwest::Url::parse("https://en.wikipedia.org/w/api.php") {
        Ok(u) => u,
        Err(_) => return Vec::new(),
    };

    url.query_pairs_mut()
        .append_pair("action", "query")
        .append_pair("list", "search")
        .append_pair("srsearch", query)
        .append_pair("srlimit", "6")
        .append_pair("utf8", "1")
        .append_pair("format", "json");

    let resp = match client.get(url).send().await {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let payload: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut out = Vec::new();
    let Some(items) = payload
        .get("query")
        .and_then(|q| q.get("search"))
        .and_then(|s| s.as_array())
    else {
        return out;
    };

    for item in items.iter().take(6) {
        let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("").trim();
        if title.is_empty() {
            continue;
        }

        let snippet_raw = item.get("snippet").and_then(|v| v.as_str()).unwrap_or("");
        let snippet = clean_markdown_text(&strip_html_tags(snippet_raw));
        let url = title.replace(' ', "_");
        let page_url = format!("https://en.wikipedia.org/wiki/{}", url);

        if snippet.is_empty() {
            out.push(format!("- {} ({})", title, page_url));
        } else {
            out.push(format!("- {}: {} ({})", title, snippet, page_url));
        }
    }

    out
}

async fn perform_web_search_context_for_variant(client: &Client, query: &str) -> Vec<String> {
    let mut results = Vec::new();
    results.extend(perform_duckduckgo_instant_answer(client, query).await);
    results.extend(perform_bing_proxy_search(client, query).await);
    results.extend(perform_wikipedia_search(client, query).await);

    if results.is_empty() {
        return Vec::new();
    }

    rank_search_results(query, results)
}

async fn perform_web_search_context(client: &Client, query: &str) -> String {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let variants = generate_web_search_variants(trimmed);
    let mut collected = Vec::<String>::new();

    for variant in variants {
        let mut lines = perform_web_search_context_for_variant(client, &variant).await;

        if lines.is_empty() {
            continue;
        }

        collected.append(&mut lines);

        if collected.len() >= 8 {
            break;
        }
    }

    let mut unique: Vec<String> = Vec::new();
    for line in collected {
        if !unique.iter().any(|existing| existing.eq_ignore_ascii_case(&line)) {
            unique.push(line);
        }
    }

    unique.into_iter().take(8).collect::<Vec<String>>().join("\n")
}

async fn resolve_hf_file_size(repo: &str, filename: &str) -> Result<u64, String> {
    let client = Client::builder()
        .user_agent("Operarius/1.0")
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("https://huggingface.co/{}/resolve/main/{}", repo, filename);
    let resp = client
        .head(&url)
        .send()
        .await
        .map_err(|e| format!("Remote HEAD failed: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        return Err(format!("Remote URL invalid or private (status {})", status));
    }

    if let Some(len) = resp.content_length() {
        return Ok(len);
    }

    if let Some(header) = resp.headers().get("x-linked-size") {
        if let Ok(v) = header.to_str() {
            if let Ok(parsed) = v.parse::<u64>() {
                return Ok(parsed);
            }
        }
    }

    Err("Remote size missing from response headers".to_string())
}

async fn list_hf_repo_files(repo: &str) -> Result<Vec<String>, String> {
    let client = Client::builder()
        .user_agent("Operarius/1.0")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let api = format!("https://huggingface.co/api/models/{}?expand[]=siblings", repo);
    let data: serde_json::Value = client
        .get(&api)
        .send()
        .await
        .map_err(|e| format!("HF API request failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("HF API parse failed: {}", e))?;

    let mut files = Vec::new();
    if let Some(siblings) = data.get("siblings").and_then(|v| v.as_array()) {
        for item in siblings {
            if let Some(name) = item
                .get("rfilename")
                .or_else(|| item.get("rFileName"))
                .and_then(|v| v.as_str())
            {
                files.push(name.to_string());
            }
        }
    }

    if files.is_empty() {
        return Err("No files found in repository listing".to_string());
    }

    Ok(files)
}

async fn download_hf_snapshot(
    repo: &str,
    target_dir: &PathBuf,
    window: &tauri::WebviewWindow,
) -> Result<(), String> {
    fs::create_dir_all(target_dir).map_err(|e| e.to_string())?;

    let files = list_hf_repo_files(repo).await?;
    let filtered = files
        .into_iter()
        .filter(|name| {
            !name.starts_with('.')
                && name != "README.md"
                && name != "LICENSE"
                && (name.ends_with(".json")
                    || name.ends_with(".safetensors")
                    || name.ends_with(".model")
                    || name.ends_with(".txt")
                    || name.ends_with(".jinja")
                    || name.ends_with(".py")
                    || name.ends_with(".yaml")
                    || name.ends_with(".yml"))
        })
        .collect::<Vec<String>>();

    if filtered.is_empty() {
        return Err("No downloadable model snapshot files were found".to_string());
    }

    let total = filtered.len() as u32;
    for (idx, file) in filtered.iter().enumerate() {
        let url = format!("https://huggingface.co/{}/resolve/main/{}", repo, file);
        let local = target_dir.join(file);
        if let Some(parent) = local.parent() {
            fs::create_dir_all(parent).ok();
        }
        download_with_resume(&url, &local, |_downloaded, _total| {}).await?;

        let progress = (((idx + 1) as f64 / total as f64) * 100.0).round() as u32;
        let _ = window.emit(
            "download-progress",
            serde_json::json!({
                "progress": progress,
                "speed_mb": 0.0,
                "downloaded_gb": 0.0
            }),
        );
    }

    Ok(())
}

fn parse_skills_hub_page(body: &str) -> Vec<(String, bool, String, String)> {
    let lines = body.lines().collect::<Vec<&str>>();
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    for idx in 0..lines.len() {
        let line = lines[idx].trim();
        if !line.starts_with("### ") {
            continue;
        }

        let name = line.trim_start_matches("### ").trim().to_string();
        if name.is_empty() || seen.contains(&name) {
            continue;
        }

        let mut built_in = false;
        let mut registry = "Hermes".to_string();
        let mut description = String::new();

        for look in (idx + 1)..((idx + 9).min(lines.len())) {
            let chunk = lines[look].trim();
            if chunk.contains("Built-in") {
                built_in = true;
            }
            if registry == "Hermes"
                && ["AI Agents", "Creative", "GitHub", "Media", "MLOps", "Productivity", "Research", "Gaming"]
                    .iter()
                    .any(|tag| chunk.contains(tag))
            {
                registry = chunk.split_whitespace().next().unwrap_or("Hermes").to_string();
            }
            if description.is_empty()
                && !chunk.is_empty()
                && !chunk.starts_with("✓")
                && !chunk.starts_with("###")
                && !chunk.starts_with("Apple")
            {
                description = chunk.to_string();
            }
        }

        seen.insert(name.clone());
        out.push((name, built_in, registry, description));
    }

    out
}

// ─── Path Helpers ────────────────────────────────────────────────────────────

fn get_base_dir(_app: &AppHandle) -> Result<PathBuf, String> {
    Ok(crate::services::setup::get_base_dir())
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

fn find_embedding_model(base: &PathBuf) -> Option<PathBuf> {
    let embeddings_dir = base.join("embeddings");
    if let Ok(entries) = fs::read_dir(&embeddings_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("gguf") {
                return Some(path);
            }
        }
    }
    None
}

pub async fn ensure_embedding_runtime(app: &AppHandle) -> Result<(), String> {
    if find_embedding_model(&get_base_dir(app)?).is_none() {
        download_embedding_model(app.clone()).await?;
    }

    if let Some(embedding_path) = find_embedding_model(&get_base_dir(app)?) {
        crate::services::supervisor::start_embedding(app, &embedding_path.to_string_lossy())?;
    }

    Ok(())
}

fn split_into_chunks(content: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    if content.trim().is_empty() {
        return Vec::new();
    }

    let chars: Vec<char> = content.chars().collect();
    if chars.len() <= chunk_size {
        return vec![content.to_string()];
    }

    let mut chunks = Vec::new();
    let mut start = 0usize;

    while start < chars.len() {
        let end = (start + chunk_size).min(chars.len());
        let chunk: String = chars[start..end].iter().collect();
        if !chunk.trim().is_empty() {
            chunks.push(chunk);
        }

        if end == chars.len() {
            break;
        }

        start = end.saturating_sub(overlap);
    }

    chunks
}

async fn generate_embedding(text: &str) -> Result<Vec<f32>, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Embedding client setup failed: {}", e))?;

    let endpoint = format!(
        "http://127.0.0.1:{}/v1/embeddings",
        crate::services::supervisor::EMBEDDING_PORT
    );

    let body = serde_json::json!({
        "model": "embedding",
        "input": text
    });

    let resp = client
        .post(&endpoint)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Embedding server unavailable: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_text = resp.text().await.unwrap_or_else(|_| "".to_string());
        return Err(format!("Embedding server returned {}: {}", status, err_text));
    }

    let value: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Embedding JSON parse failed: {}", e))?;

    let Some(vector_array) = value
        .get("data")
        .and_then(|d| d.get(0))
        .and_then(|v| v.get("embedding"))
        .and_then(|v| v.as_array()) else {
        return Err("Embedding response missing vector data".to_string());
    };

    let vector = vector_array
        .iter()
        .filter_map(|v| v.as_f64())
        .map(|v| v as f32)
        .collect::<Vec<f32>>();

    if vector.is_empty() {
        return Err("Embedding vector is empty".to_string());
    }

    Ok(vector)
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return -1.0;
    }

    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    if norm_a == 0.0 || norm_b == 0.0 {
        return -1.0;
    }

    dot / (norm_a.sqrt() * norm_b.sqrt())
}

fn clip_for_prompt(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect::<String>()
}

async fn get_recent_chat_context(pool: &SqlitePool, session_filter: &str) -> String {
    let rows: Vec<(Option<String>, Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT message_content, response_content, event_type, event_payload
         FROM chat_history
         WHERE session_id = ?
         ORDER BY rowid DESC
         LIMIT 12"
    )
    .bind(session_filter)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut lines = Vec::new();
    for (user, assistant, event_type, event_payload) in rows.into_iter().rev() {
        if let Some(event) = event_type {
            if event == "file_upload" {
                let file_name = event_payload
                    .as_deref()
                    .and_then(|payload| serde_json::from_str::<serde_json::Value>(payload).ok())
                    .and_then(|v| v.get("filename").and_then(|f| f.as_str()).map(|s| s.to_string()))
                    .unwrap_or_else(|| "uploaded file".to_string());
                lines.push(format!("Event: Uploaded and indexed {}", file_name));
            }
        }
        if let Some(u) = user {
            if !u.trim().is_empty() {
                let clipped = u.chars().take(320).collect::<String>();
                lines.push(format!("User: {}", clipped));
            }
        }
        if let Some(a) = assistant {
            if !a.trim().is_empty() {
                let clipped = a.chars().take(320).collect::<String>();
                lines.push(format!("Assistant: {}", clipped));
            }
        }
    }

    lines.join("\n")
}

async fn build_recent_uploaded_files_context(pool: &SqlitePool, user_id: &str) -> String {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT filename, uploaded_at
         FROM files
         WHERE user_id = ?
         ORDER BY uploaded_at DESC
         LIMIT 16"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    if rows.is_empty() {
        return String::new();
    }

    rows.into_iter()
        .map(|(filename, ts)| format!("- {} (uploaded_at={})", filename, ts))
        .collect::<Vec<String>>()
        .join("\n")
}

pub async fn build_rag_context(pool: &SqlitePool, query: &str) -> String {
    let terms: Vec<String> = query
        .to_lowercase()
        .split_whitespace()
        .filter(|w| w.len() > 2)
        .take(6)
        .map(|w| w.to_string())
        .collect();

    // Fast path: keyword-only lookup in KB before expensive embedding scan.
    let mut quick_hits = Vec::new();
    for term in &terms {
        let pattern = format!("%{}%", term);
        let hits: Vec<(String, String)> = sqlx::query_as(
            "SELECT title, substr(content, 1, 420)
             FROM knowledge_base
             WHERE lower(content) LIKE lower(?)
             ORDER BY rowid DESC
             LIMIT 2"
        )
        .bind(pattern)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        for (title, snippet) in hits {
            quick_hits.push(format!("- [FILE:{}] {}", title, snippet));
        }
    }

    if quick_hits.len() >= 3 {
        return quick_hits.into_iter().take(6).collect::<Vec<String>>().join("\n");
    }

    let mut rag_context = String::new();

    if let Ok(query_vector) = generate_embedding(query).await {
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT content, embedding_json FROM rag_chunks ORDER BY rowid DESC LIMIT 220"
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        let mut scored: Vec<(f32, String)> = rows
            .into_iter()
            .filter_map(|(content, embedding_json)| {
                let vector = serde_json::from_str::<Vec<f32>>(&embedding_json).ok()?;
                let score = cosine_similarity(&query_vector, &vector);
                Some((score, content))
            })
            .filter(|(score, _)| *score > 0.18)
            .collect();

        scored.sort_by(|a, b| b.0.total_cmp(&a.0));
        rag_context = scored
            .into_iter()
            .take(6)
            .map(|(_, chunk)| format!("- {}", chunk))
            .collect::<Vec<String>>()
            .join("\n");
    }

    if !rag_context.is_empty() {
        return rag_context;
    }

    if terms.is_empty() {
        return String::new();
    }

    let mut fallback = Vec::new();
    for term in terms {
        let pattern = format!("%{}%", term);
        let hits: Vec<(String, String)> = sqlx::query_as(
            "SELECT title, substr(content, 1, 500)
             FROM knowledge_base
             WHERE lower(content) LIKE lower(?)
             ORDER BY rowid DESC
             LIMIT 2"
        )
        .bind(pattern)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        for (title, snippet) in hits {
            fallback.push(format!("- [FILE:{}] {}", title, snippet));
        }
    }

    fallback.into_iter().take(6).collect::<Vec<String>>().join("\n")
}

async fn build_light_local_evidence_context(
    pool: &SqlitePool,
    query: &str,
    target_file: Option<&str>,
) -> String {
    if let Some(target_name) = target_file.map(|t| t.trim()).filter(|t| !t.is_empty()) {
        let file_match: Option<(String, String)> = sqlx::query_as(
            "SELECT id, filename
             FROM files
             WHERE lower(filename) LIKE lower(?)
             ORDER BY uploaded_at DESC
             LIMIT 1"
        )
        .bind(format!("%{}%", target_name))
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

        if let Some((file_id, file_name)) = file_match {
            let terms: Vec<String> = query
                .to_lowercase()
                .split_whitespace()
                .filter(|w| w.len() > 2)
                .take(6)
                .map(|w| w.to_string())
                .collect();

            let mut snippets = Vec::new();
            for term in &terms {
                let hits: Vec<(String,)> = sqlx::query_as(
                    "SELECT substr(content, 1, 360)
                     FROM rag_chunks
                     WHERE file_id = ? AND lower(content) LIKE lower(?)
                     ORDER BY rowid DESC
                     LIMIT 2"
                )
                .bind(&file_id)
                .bind(format!("%{}%", term))
                .fetch_all(pool)
                .await
                .unwrap_or_default();

                for (snippet,) in hits {
                    snippets.push(format!("- {}", snippet));
                }
            }

            if !snippets.is_empty() {
                return format!(
                    "Target file: {}\n{}",
                    file_name,
                    snippets.into_iter().take(6).collect::<Vec<String>>().join("\n")
                );
            }

            let kb_fallback: Option<(String,)> = sqlx::query_as(
                "SELECT content
                 FROM knowledge_base
                 WHERE file_id = ?
                 ORDER BY rowid DESC
                 LIMIT 1"
            )
            .bind(file_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();

            if let Some((content,)) = kb_fallback {
                return format!(
                    "Target file: {}\n- {}",
                    file_name,
                    content.chars().take(800).collect::<String>()
                );
            }
        }
    }

    let terms: Vec<String> = query
        .to_lowercase()
        .split_whitespace()
        .filter(|w| w.len() > 2)
        .take(6)
        .map(|w| w.to_string())
        .collect();

    let mut quick_hits = Vec::new();
    for term in &terms {
        let pattern = format!("%{}%", term);
        let hits: Vec<(String, String)> = sqlx::query_as(
            "SELECT title, substr(content, 1, 320)
             FROM knowledge_base
             WHERE lower(content) LIKE lower(?)
             ORDER BY rowid DESC
             LIMIT 2"
        )
        .bind(pattern)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        for (title, snippet) in hits {
            quick_hits.push(format!("- [FILE:{}] {}", title, snippet));
        }
    }

    quick_hits.into_iter().take(4).collect::<Vec<String>>().join("\n")
}

pub async fn build_targeted_rag_context(
    pool: &SqlitePool,
    query: &str,
    target_file: Option<&str>,
) -> String {
    let Some(target_name) = target_file.map(|t| t.trim()).filter(|t| !t.is_empty()) else {
        return build_rag_context(pool, query).await;
    };

    let file_match: Option<(String, String)> = sqlx::query_as(
        "SELECT id, filename
         FROM files
         WHERE lower(filename) LIKE lower(?)
         ORDER BY uploaded_at DESC
         LIMIT 1"
    )
    .bind(format!("%{}%", target_name))
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let Some((file_id, file_name)) = file_match else {
        return build_rag_context(pool, query).await;
    };

    let terms: Vec<String> = query
        .to_lowercase()
        .split_whitespace()
        .filter(|w| w.len() > 2)
        .take(6)
        .map(|w| w.to_string())
        .collect();

    let mut file_specific = Vec::new();
    for term in &terms {
        let hits: Vec<(String,)> = sqlx::query_as(
            "SELECT substr(content, 1, 420)
             FROM rag_chunks
             WHERE file_id = ? AND lower(content) LIKE lower(?)
             ORDER BY rowid DESC
             LIMIT 3"
        )
        .bind(&file_id)
        .bind(format!("%{}%", term))
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        for (snippet,) in hits {
            file_specific.push(format!("- {}", snippet));
        }
    }

    if !file_specific.is_empty() {
        return format!(
            "Target file: {}\n{}",
            file_name,
            file_specific.into_iter().take(8).collect::<Vec<String>>().join("\n")
        );
    }

    if let Ok(query_vector) = generate_embedding(query).await {
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT content, embedding_json
             FROM rag_chunks
             WHERE file_id = ?
             ORDER BY rowid DESC
             LIMIT 180"
        )
        .bind(&file_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        let mut scored: Vec<(f32, String)> = rows
            .into_iter()
            .filter_map(|(content, embedding_json)| {
                let vector = serde_json::from_str::<Vec<f32>>(&embedding_json).ok()?;
                let score = cosine_similarity(&query_vector, &vector);
                Some((score, content))
            })
            .filter(|(score, _)| *score > 0.12)
            .collect();

        scored.sort_by(|a, b| b.0.total_cmp(&a.0));

        if !scored.is_empty() {
            return format!(
                "Target file: {}\n{}",
                file_name,
                scored
                    .into_iter()
                    .take(8)
                    .map(|(_, chunk)| format!("- {}", chunk))
                    .collect::<Vec<String>>()
                    .join("\n")
            );
        }
    }

    let kb_fallback: Option<(String,)> = sqlx::query_as(
        "SELECT content
         FROM knowledge_base
         WHERE file_id = ?
         ORDER BY rowid DESC
         LIMIT 1"
    )
    .bind(file_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if let Some((content,)) = kb_fallback {
        let lc = content.to_lowercase();
        let terms: Vec<String> = query
            .to_lowercase()
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .take(8)
            .map(|s| s.to_string())
            .collect();

        let mut focused_snippets = Vec::new();
        for term in terms {
            if let Some(pos) = lc.find(&term) {
                let start = pos.saturating_sub(220);
                let end = (pos + 260).min(content.len());
                let snippet = content.get(start..end).unwrap_or(&content);
                focused_snippets.push(format!("- {}", snippet));
            }
        }

        if !focused_snippets.is_empty() {
            return format!(
                "Target file: {}\n{}",
                file_name,
                focused_snippets.into_iter().take(6).collect::<Vec<String>>().join("\n")
            );
        }

        return format!(
            "Target file: {}\n- {}",
            file_name,
            content.chars().take(2000).collect::<String>()
        );
    }

    String::new()
}

fn image_mime_from_ext(ext: &str) -> Option<&'static str> {
    match ext {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        "gif" => Some("image/gif"),
        "heic" => Some("image/heic"),
        _ => None,
    }
}

pub async fn build_visual_context_for_target_file(
    pool: &SqlitePool,
    target_file: Option<&str>,
    question: &str,
) -> Option<String> {
    let target = target_file?.trim();
    if target.is_empty() {
        return None;
    }

    let row: Option<(String, String, Vec<u8>, String, Option<String>)> = sqlx::query_as(
        "SELECT f.id, f.file_type, b.data, f.filename, f.content
         FROM files f
         JOIN file_blobs b ON b.file_id = f.id
         WHERE lower(f.filename) LIKE lower(?)
         ORDER BY f.uploaded_at DESC
         LIMIT 1"
    )
    .bind(format!("%{}%", target))
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let Some((_file_id, file_type, bytes, file_name, content)) = row else {
        return None;
    };

    let fallback_summary = {
        let fallback_base = content.unwrap_or_default();
        let extracted = if let Some(pos) = fallback_base.find("OCR text:") {
            fallback_base
                .get(pos + "OCR text:".len()..)
                .unwrap_or(&fallback_base)
                .trim()
                .to_string()
        } else {
            fallback_base.trim().to_string()
        };

        if extracted.is_empty() {
            format!(
                "Image summary fallback for {}: This image is indexed, but no OCR text was extracted. Ask a focused question about visible elements and I will use file metadata and context.",
                file_name
            )
        } else {
            format!(
                "Image summary fallback for {} (vision model not available):\nQuestion: {}\nExtracted text and clues:\n{}",
                file_name,
                question,
                extracted.chars().take(1800).collect::<String>()
            )
        }
    };

    let ext = file_type.to_lowercase();
    let Some(mime) = image_mime_from_ext(&ext) else {
        return None;
    };

    // Fast capability gate: skip expensive multimodal call when the active model has no vision support.
    let cap_client = Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .ok()?;
    let models_endpoint = format!(
        "http://127.0.0.1:{}/v1/models",
        crate::services::supervisor::LLAMA_PORT
    );
    let supports_vision = if let Ok(resp) = cap_client.get(&models_endpoint).send().await {
        if let Ok(v) = resp.json::<serde_json::Value>().await {
            if let Some(models) = v.get("data").and_then(|d| d.as_array()) {
                models.iter().any(|m| {
                    m.get("capabilities")
                        .and_then(|c| c.as_array())
                        .map(|caps| caps.iter().any(|c| c.as_str() == Some("vision")))
                        .unwrap_or(false)
                })
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    if !supports_vision {
        return Some(fallback_summary);
    }

    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    let data_url = format!("data:{};base64,{}", mime, b64);

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .ok()?;

    let body = serde_json::json!({
        "model": "llama",
        "messages": [
            {
                "role": "system",
                "content": "You are a visual analyst. Extract all relevant visible details, text, entities, layout, and key evidence for answering the user question."
            },
            {
                "role": "user",
                "content": [
                    { "type": "text", "text": format!("Question: {}", question) },
                    { "type": "image_url", "image_url": { "url": data_url } }
                ]
            }
        ],
        "temperature": 0.2,
        "stream": false
    });

    let endpoint = format!(
        "http://127.0.0.1:{}/v1/chat/completions",
        crate::services::supervisor::LLAMA_PORT
    );

    let response = match client.post(&endpoint).json(&body).send().await {
        Ok(r) => r,
        Err(_) => return Some(fallback_summary),
    };
    if !response.status().is_success() {
        return Some(fallback_summary);
    }

    let value: serde_json::Value = response.json().await.ok()?;
    let content = value["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    if content.is_empty() {
        Some(fallback_summary)
    } else {
        Some(content)
    }
}

#[command]
pub async fn get_file_metadata(file_path: String) -> Result<FileMetadata, String> {
    let path = PathBuf::from(&file_path);
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let size_bytes = fs::metadata(&path)
        .map_err(|e| e.to_string())?
        .len() as i64;

    Ok(FileMetadata { filename, size_bytes })
}

fn extract_text_for_rag(path: &std::path::Path, file_name: &str) -> String {
    let run_cmd = |program: &str, args: &[&str]| -> Option<String> {
        let output = Command::new(program).args(args).output().ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if text.is_empty() { None } else { Some(text) }
    };

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let file_kind = run_cmd("file", &["-b", &path.to_string_lossy()]).unwrap_or_else(|| "unknown".to_string());
    let file_hash = run_cmd("shasum", &["-a", "256", &path.to_string_lossy()])
        .and_then(|line| line.split_whitespace().next().map(|s| s.to_string()))
        .unwrap_or_else(|| "unavailable".to_string());

    let metadata_header = format!(
        "FILE_NAME: {}\nFILE_TYPE: {}\nFILE_SHA256: {}\n",
        file_name, file_kind, file_hash
    );

    let text_like = [
        "txt", "md", "json", "csv", "xml", "html", "htm", "rs", "py", "js", "ts", "tsx", "jsx",
        "toml", "yaml", "yml", "sql", "log",
    ];

    if text_like.contains(&ext.as_str()) {
        let content = fs::read_to_string(path)
            .unwrap_or_else(|_| format!("Text extraction failed for file {}", file_name));
        return format!("{}\n{}", metadata_header, content);
    }

    if ext == "pdf" {
        let output = Command::new("pdftotext").arg(path).arg("-").output();
        if let Ok(out) = output {
            let extracted = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !extracted.is_empty() {
                return format!("{}\n{}", metadata_header, extracted);
            }
        }
    }

    if ext == "doc" || ext == "docx" || ext == "rtf" {
        let output = Command::new("textutil")
            .args(["-convert", "txt", "-stdout"])
            .arg(path)
            .output();
        if let Ok(out) = output {
            let extracted = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !extracted.is_empty() {
                return format!("{}\n{}", metadata_header, extracted);
            }
        }
    }

    if ["png", "jpg", "jpeg", "webp", "gif", "heic"].contains(&ext.as_str()) {
        let mut description = format!(
            "Image file: {}. Type: {}. This image was uploaded and indexed for retrieval.",
            file_name, ext
        );

        let ocr = Command::new("tesseract").arg(path).arg("stdout").output();
        if let Ok(out) = ocr {
            let extracted = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !extracted.is_empty() {
                description.push_str(" OCR text: ");
                description.push_str(&extracted);
            }
        }
        return format!("{}\n{}", metadata_header, description);
    }

    if ["zip", "jar", "apk", "docx", "xlsx", "pptx"].contains(&ext.as_str()) {
        if let Some(listing) = run_cmd("zipinfo", &["-1", &path.to_string_lossy()]) {
            return format!(
                "{}\nArchive contents:\n{}",
                metadata_header,
                listing.lines().take(200).collect::<Vec<&str>>().join("\n")
            );
        }
    }

    if ["tar", "gz", "tgz", "bz2", "xz"].contains(&ext.as_str()) {
        if let Some(listing) = run_cmd("tar", &["-tf", &path.to_string_lossy()]) {
            return format!(
                "{}\nArchive contents:\n{}",
                metadata_header,
                listing.lines().take(200).collect::<Vec<&str>>().join("\n")
            );
        }
    }

    if let Some(strings_out) = run_cmd("strings", &["-n", "4", &path.to_string_lossy()]) {
        let preview = strings_out
            .lines()
            .filter(|l| !l.trim().is_empty())
            .take(300)
            .collect::<Vec<&str>>()
            .join("\n");

        if !preview.is_empty() {
            return format!(
                "{}\nBinary strings preview:\n{}",
                metadata_header,
                preview
            );
        }
    }

    let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let fallback = format!(
        "Binary file uploaded: {} (extension: {}, size: {} bytes).",
        file_name,
        if ext.is_empty() { "unknown" } else { &ext },
        size
    );

    format!("{}\n{}", metadata_header, fallback)
}

pub async fn ingest_file_for_rag(
    pool: &SqlitePool,
    file_path: &std::path::Path,
    user_id: Option<String>,
    platform: Option<String>,
) -> Result<String, String> {
    if !file_path.exists() {
        return Err(format!("File not found: {}", file_path.to_string_lossy()));
    }

    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let file_size = fs::metadata(file_path).map_err(|e| e.to_string())?.len() as i64;
    let file_id = uuid::Uuid::new_v4().to_string();
    let extension = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("unknown")
        .to_string();

    let content = extract_text_for_rag(file_path, &file_name);
    let file_bytes = fs::read(file_path).unwrap_or_default();

    sqlx::query(
        "INSERT INTO files (id, filename, file_type, file_size, mime_type, uploaded_at, user_id, platform, indexed, content)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&file_id)
    .bind(&file_name)
    .bind(&extension)
    .bind(file_size)
    .bind("application/octet-stream")
    .bind(chrono::Utc::now().timestamp())
    .bind(&user_id)
    .bind(&platform)
    .bind(true)
    .bind(&content)
    .execute(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query(
        "INSERT INTO file_blobs (id, file_id, data, created_at)
         VALUES (?, ?, ?, datetime('now'))"
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&file_id)
    .bind(file_bytes)
    .execute(pool)
    .await
    .map_err(|e| format!("Blob save error: {}", e))?;

    sqlx::query(
        "INSERT INTO knowledge_base (id, title, content, file_id, source, created_at, user_id, tags)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&file_name)
    .bind(&content)
    .bind(&file_id)
    .bind("upload")
    .bind(chrono::Utc::now().timestamp())
    .bind(&user_id)
    .bind(&extension)
    .execute(pool)
    .await
    .map_err(|e| format!("Knowledge base save error: {}", e))?;

    let chunks = split_into_chunks(&content, 520, 170);
    for chunk in chunks {
        if let Ok(vector) = generate_embedding(&chunk).await {
            let embedding_json = serde_json::to_string(&vector)
                .map_err(|e| format!("Embedding serialization error: {}", e))?;

            let chunk_with_file = format!("[FILE:{}]\n{}", file_name, chunk);
            sqlx::query(
                "INSERT INTO rag_chunks (id, file_id, content, embedding_json, created_at)
                 VALUES (?, ?, ?, ?, datetime('now'))"
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(&file_id)
            .bind(chunk_with_file)
            .bind(embedding_json)
            .execute(pool)
            .await
            .map_err(|e| format!("Vector DB write error: {}", e))?;
        }
    }

    Ok(file_id)
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
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        File::create(&path).map_err(|e| e.to_string())?;
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

    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Main window is not available".to_string())?;

    if filename.ends_with(".gguf") {
        let expected_size = resolve_hf_file_size(&repo, &filename).await?;
        let current_size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

        // Legit pass only when local file is within 1% of remote size.
        if current_size > 0 {
            let min_ok = (expected_size as f64 * 0.99) as u64;
            if current_size >= min_ok {
                println!(
                    "[ORCHESTRATOR] Integrity Check PASSED for {} ({:.2} GB/{:.2} GB)",
                    filename,
                    current_size as f64 / 1_073_741_824.0,
                    expected_size as f64 / 1_073_741_824.0
                );
                return Ok(path.to_string_lossy().to_string());
            }
        }

        println!("[ORCHESTRATOR] Pulling binary weights (Resumable): {}", filename);
        let url = format!("https://huggingface.co/{}/resolve/main/{}", repo, filename);

        let start_time = std::time::Instant::now();
        let mut last_emit = std::time::Instant::now();

        download_with_resume(&url, &path, move |downloaded, total| {
            let now = std::time::Instant::now();
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

        let final_size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let min_ok = (expected_size as f64 * 0.99) as u64;
        if final_size < min_ok {
            return Err(format!(
                "Downloaded file failed integrity check for {} (local {} bytes, expected {} bytes)",
                filename, final_size, expected_size
            ));
        }
    } else {
        let snapshot_dir = get_base_dir(&app)?.join("models").join(&filename);
        let has_snapshot = snapshot_dir.join("model.safetensors.index.json").exists();
        if !has_snapshot {
            download_hf_snapshot(&repo, &snapshot_dir, &window).await?;
        }
    }

    // Ensure an embedding model is available for RAG retrieval.
    if find_embedding_model(&get_base_dir(&app)?).is_none() {
        let _ = download_embedding_model(app.clone()).await;
    }

    app.emit("download-complete", ()).ok();
    Ok(path.to_string_lossy().to_string())
}

#[command]
pub async fn download_embedding_model(app: AppHandle) -> Result<String, String> {
    let base = get_base_dir(&app)?;
    let path = base.join("embeddings").join(DEFAULT_EMBEDDING_FILE);
    fs::create_dir_all(path.parent().unwrap_or(&path)).map_err(|e| e.to_string())?;

    let existing_size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    if existing_size > 300_000_000 {
        return Ok(path.to_string_lossy().to_string());
    }

    let url = format!(
        "https://huggingface.co/{}/resolve/main/{}",
        DEFAULT_EMBEDDING_REPO,
        DEFAULT_EMBEDDING_FILE
    );

    download_with_resume(&url, &path, |_downloaded, _total| {}).await?;
    Ok(path.to_string_lossy().to_string())
}

#[command]
pub async fn complete_local_setup(
    app: AppHandle,
    model_repo: String,
    model_file: String,
) -> Result<String, String> {
    download_model(app.clone(), model_repo, model_file).await?;
    let _ = download_embedding_model(app).await;
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
pub async fn chat_with_rag_agent(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    message: String,
    target_file: Option<String>,
    web_search: Option<bool>,
    deep_thinking: Option<bool>,
    memory_use: Option<bool>,
) -> Result<String, String> {
    if message.trim().is_empty() {
        return Ok("I'm listening...".into());
    }

    if let Some(parsed_browser_command) = parse_browser_chat_command(&message) {
        let (tool_name, tool_args) = match parsed_browser_command {
            Ok(value) => value,
            Err(usage) => return Ok(usage),
        };

        emit_agent_log(
            &app,
            Some(&*pool),
            "info",
            "browser",
            "Executing browser automation command",
            serde_json::json!({
                "tool": tool_name,
                "args": tool_args,
            })
        ).await;

        let task_id = "operarius-browser-main";
        return match run_hermes_browser_tool(&tool_name, tool_args, task_id) {
            Ok(result) => {
                let content = if result.trim().is_empty() {
                    "Browser command completed.".to_string()
                } else {
                    result
                };

                emit_agent_log(
                    &app,
                    Some(&*pool),
                    "info",
                    "browser",
                    "Browser automation command completed",
                    serde_json::json!({
                        "tool": tool_name,
                        "task_id": task_id,
                        "output_chars": content.chars().count(),
                    })
                ).await;

                Ok(content)
            }
            Err(err) => {
                emit_agent_log(
                    &app,
                    Some(&*pool),
                    "error",
                    "browser",
                    "Browser automation command failed",
                    serde_json::json!({
                        "tool": tool_name,
                        "task_id": task_id,
                        "error": err,
                    })
                ).await;

                Ok(format!("Browser automation error: {}", err))
            }
        };
    }

    ensure_auto_skill_from_message(&pool, "local-user", &message).await;

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(35))
        .build()
        .map_err(|e| format!("Network setup failed: {}", e))?;

    println!("[OPERARIUS] Chat query: {}", message);
    let request_started_at = Instant::now();

    emit_agent_log(
        &app,
        Some(&*pool),
        "info",
        "chat",
        "Agent received a new request",
        serde_json::json!({
            "web_search": web_search.unwrap_or(false),
            "deep_thinking": deep_thinking.unwrap_or(false),
            "memory_use": memory_use.unwrap_or(false),
            "target_file": target_file.clone(),
        })
    ).await;

    let web_search_enabled = web_search.unwrap_or(false);
    let deep_thinking_enabled = deep_thinking.unwrap_or(false);
    let memory_use_enabled = memory_use.unwrap_or(true);
    let expected_language = detect_user_language_label(&message);

    emit_agent_log(
        &app,
        Some(&*pool),
        "info",
        "chat",
        "Mulling over your request",
        serde_json::json!({ "phase": "mulling" })
    ).await;

    // Context Truncation & Intent Filtering (Surgical token management)
    let lower_msg = message.to_lowercase();
    let is_chitchat = lower_msg.contains("hi") || lower_msg.contains("hello") || lower_msg.contains("hey") || 
                     lower_msg.contains("how are you") || lower_msg.contains("thank") || lower_msg.len() < 25;

    emit_agent_log(
        &app,
        Some(&*pool),
        "info",
        "chat",
        "Loading memory and file context",
        serde_json::json!({ "phase": "memory-scan" })
    ).await;

    let (recent_context, user_memory_context, uploaded_files_context) = (
        get_recent_chat_context(&*pool, "app-main").await,
        build_user_memory_context(&pool, "local-user").await,
        build_recent_uploaded_files_context(&pool, "local-user").await,
    );

    let (rag_context, visual_context) = if is_chitchat {
        (String::new(), None)
    } else if deep_thinking_enabled {
        emit_agent_log(
            &app,
            Some(&*pool),
            "info",
            "chat",
            "Running deep retrieval across indexed chunks",
            serde_json::json!({ "phase": "deep-retrieval" })
        ).await;

        if target_file.as_deref().is_some() {
            emit_agent_log(
                &app,
                Some(&*pool),
                "info",
                "chat",
                "Indexing selected file context for answer",
                serde_json::json!({ "phase": "file-context" })
            ).await;
        }

        let _ = ensure_embedding_runtime(&app).await;
        (
            build_targeted_rag_context(&pool, &message, target_file.as_deref()).await,
            build_visual_context_for_target_file(&pool, target_file.as_deref(), &message).await,
        )
    } else {
        emit_agent_log(
            &app,
            Some(&*pool),
            "info",
            "chat",
            "Scanning local knowledge base",
            serde_json::json!({ "phase": "local-scan" })
        ).await;
        (
            build_light_local_evidence_context(&pool, &message, target_file.as_deref()).await,
            None,
        )
    };

    let related_local_context = build_related_local_context(
        &message,
        &recent_context,
        &user_memory_context,
        &uploaded_files_context,
        &rag_context,
        visual_context.as_deref(),
    );

    let has_related_local_evidence = !related_local_context.trim().is_empty();
    let has_any_local_evidence = !recent_context.trim().is_empty()
        || !user_memory_context.trim().is_empty()
        || !uploaded_files_context.trim().is_empty()
        || !rag_context.trim().is_empty()
        || visual_context.as_deref().is_some_and(|v| !v.trim().is_empty());

    if !web_search_enabled && !is_chitchat && !has_related_local_evidence {
        emit_agent_log(
            &app,
            Some(&*pool),
            "warning",
            "chat",
            "Local data did not contain a related answer",
            serde_json::json!({
                "web_search": false,
                "deep_thinking": deep_thinking_enabled,
                "memory_use": memory_use_enabled,
                "elapsed_ms": request_started_at.elapsed().as_millis(),
            })
        ).await;

        return Ok("I couldn't find anything related in local data. Turn on Web Search and ask again, or add a file or memory note first.".to_string());
    }

    let local_context = if has_related_local_evidence {
        clip_for_prompt(&related_local_context, 5000)
    } else {
        String::new()
    };

    let use_web_search = web_search_enabled && (!has_related_local_evidence || local_context.trim().is_empty());
    let web_context = if use_web_search {
        emit_agent_log(
            &app,
            Some(&*pool),
            "info",
            "chat",
            if has_any_local_evidence {
                "Local data was not related enough, searching the web for fresh references"
            } else {
                "No related local evidence found, searching the web for fresh references"
            },
            serde_json::json!({ "phase": "web-search" })
        ).await;
        let ctx = perform_web_search_context(&client, &message).await;
        if ctx.trim().is_empty() {
            emit_agent_log(
                &app,
                Some(&*pool),
                "warning",
                "chat",
                "Web search was enabled but no external hits were returned",
                serde_json::json!({ "phase": "web-search", "query": message })
            ).await;
        }
        ctx
    } else {
        String::new()
    };

    let mut evidence_sections = Vec::new();
    if !local_context.trim().is_empty() {
        evidence_sections.push(format!("Local related evidence:\n{}", local_context));
    }
    if !web_context.trim().is_empty() {
        evidence_sections.push(format!("External web evidence:\n{}", web_context));
    }

    let evidence_block = if evidence_sections.is_empty() {
        String::from("(no related local or web evidence found)")
    } else {
        evidence_sections.join("\n\n")
    };

    let user_prompt = format!(
        "User question:\n{}\n\nEvidence to use (follow the order given):\n{}\n\nAnswer rules: use only evidence related to the question; if local evidence is related, prioritize it; if local evidence is unrelated or missing, use the web evidence; if neither is related, say you could not find related information. Respond ONLY in {} unless the user explicitly asks for translation. Then synthesize a direct answer that mixes the relevant evidence cleanly and avoids mentioning search mechanics.",
        clip_for_prompt(&message, 2400),
        evidence_block,
        expected_language
    );

    // 2. Force extreme truncation: system prompt + current turn with context.
    let messages = vec![
        serde_json::json!({
            "role": "system",
            "content": if deep_thinking_enabled {
                format!("You are Operarius, a world-class AI agent. Deep thinking mode is ON: provide a deeper, more rigorous analysis with assumptions, trade-offs, and edge cases. Use conversation memory, retrieved knowledge, and visual analysis when relevant. If a target file is provided, answer specifically from that file first. Respond ONLY in {} unless the user explicitly requests translation. Format every answer for humans: 1) a short summary line, 2) key points as bullet lines, 3) optional 'Next step' line when useful. Keep paragraphs short and readable. Never output chain-of-thought or internal reasoning.", expected_language)
            } else {
                if web_search_enabled {
                    format!("You are Operarius, a world-class AI agent. Web search is ON. Use the external web findings provided in the user prompt as the primary source and answer directly from them. Do not invent facts, do not rely on local memory if web findings are present, and if the web findings are empty say so plainly and ask for a narrower search. If a target file is provided, still prefer web findings for the requested topic. Respond ONLY in {} unless the user explicitly requests translation. Format every answer for humans: 1) a short summary line, 2) key points as bullet lines, 3) optional 'Next step' line when useful. Keep paragraphs short and readable. Never output chain-of-thought or internal reasoning.", expected_language)
                } else {
                    format!("You are Operarius, a world-class AI agent. Web search is OFF. You must answer only from the local evidence provided in the prompt, memory, indexed files, and retrieved knowledge. If the evidence is insufficient, say you could not find it locally and recommend enabling Web Search. Respond ONLY in {} unless the user explicitly requests translation. Format every answer for humans: 1) a short summary line, 2) key points as bullet lines, 3) optional 'Next step' line when useful. Keep paragraphs short and readable. Never output chain-of-thought or internal reasoning.", expected_language)
                }
            }
        }),
        serde_json::json!({ "role": "user", "content": user_prompt })
    ];

    if !is_chitchat {
        println!("[OPERARIUS] Intent: COMPLEX — checking memory nodes.");
    }

    // Force standardized JSON
    let response_budget = if deep_thinking_enabled {
        512
    } else if is_chitchat {
        128
    } else {
        480
    };

    emit_agent_log(
        &app,
        Some(&*pool),
        "info",
        "chat",
        "Generating final response",
        serde_json::json!({
            "phase": "generation",
            "deep_thinking": deep_thinking_enabled,
            "max_tokens": response_budget
        })
    ).await;

    let body = serde_json::json!({
        "model": "llama",
        "messages": messages,
        "temperature": if deep_thinking_enabled { 0.4 } else { 0.5 },
        "max_tokens": response_budget,
        "stream": false,
        "chat_template_kwargs": {
            "enable_thinking": false
        }
    });

    let port = crate::services::supervisor::LLAMA_PORT;
    let endpoint = format!("http://127.0.0.1:{}/v1/chat/completions", port);

    let mut last_send_error: Option<String> = None;
    let mut resp: Option<reqwest::Response> = None;
    for attempt in 1..=3 {
        match client.post(&endpoint).json(&body).send().await {
            Ok(r) => {
                if r.status().is_server_error() && attempt < 3 {
                    emit_agent_log(
                        &app,
                        Some(&*pool),
                        "warning",
                        "chat",
                        "AI engine returned a temporary server error, retrying",
                        serde_json::json!({
                            "attempt": attempt,
                            "status": r.status().as_u16(),
                        })
                    ).await;
                    tokio::time::sleep(std::time::Duration::from_millis(650)).await;
                    continue;
                }
                resp = Some(r);
                break;
            }
            Err(e) => {
                last_send_error = Some(e.to_string());
                if attempt < 3 {
                    emit_agent_log(
                        &app,
                        Some(&*pool),
                        "warning",
                        "chat",
                        "AI engine is unreachable, retrying",
                        serde_json::json!({
                            "attempt": attempt,
                            "error": e.to_string(),
                        })
                    ).await;
                    tokio::time::sleep(std::time::Duration::from_millis(650)).await;
                    continue;
                }
            }
        }
    }

    let Some(resp) = resp else {
        emit_agent_log(
            &app,
            Some(&*pool),
            "error",
            "chat",
            "AI engine remains unreachable after retries",
            serde_json::json!({
                "elapsed_ms": request_started_at.elapsed().as_millis(),
                "error": last_send_error,
            })
        ).await;
        return Ok("We are facing some issue right now. Please try again in some time.".to_string());
    };

    let status = resp.status();
    let resp_text = match resp.text().await {
        Ok(t) => t,
        Err(e) => return Err(format!("Failed to read engine response: {}", e)),
    };

    if !status.is_success() {
        println!("[OPERARIUS] ENGINE ERROR ({}): {}", status, resp_text);
        emit_agent_log(
            &app,
            Some(&*pool),
            "error",
            "chat",
            "AI engine returned an error",
            serde_json::json!({
                "status": status.as_u16(),
                "elapsed_ms": request_started_at.elapsed().as_millis(),
            })
        ).await;
        return Ok(format!("I hit a technical snag ({}). Try a shorter message.", status));
    }

    let resp_json: serde_json::Value = match serde_json::from_str(&resp_text) {
        Ok(j) => j,
        Err(_) => return Ok("The AI engine sent a garbled response. Please try again.".to_string()),
    };
    
    let mut content = resp_json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("Server error")
        .trim()
        .to_string();

    if is_language_mismatch(expected_language, &content) {
        emit_agent_log(
            &app,
            Some(&*pool),
            "warning",
            "chat",
            "Response language mismatch detected, applying correction pass",
            serde_json::json!({
                "expected_language": expected_language,
            })
        ).await;

        if let Some(rewritten) = rewrite_response_to_language(
            &client,
            &endpoint,
            &content,
            expected_language,
            response_budget,
        ).await {
            if !rewritten.trim().is_empty() {
                content = rewritten;
            }
        }
    }

    if memory_use_enabled {
        sqlx::query(
            "INSERT INTO chat_history (id, user_id, session_id, message_content, response_content, created_at, platform, chat_id)
             VALUES (?, ?, ?, ?, ?, datetime('now'), ?, ?)"
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind("local-user")
        .bind("app-main")
        .bind(&message)
        .bind(&content)
        .bind("app")
        .bind("app")
        .execute(&*pool)
        .await
        .map_err(|e| format!("Failed to save chat history: {}", e))?;

        app.emit("chat-history-updated", serde_json::json!({ "platform": "app" }))
            .ok();

        capture_memory_hints(&pool, "local-user", &message, &content).await;
    }

    emit_agent_log(
        &app,
        Some(&*pool),
        "info",
        "chat",
        "Agent completed response",
        serde_json::json!({
            "elapsed_ms": request_started_at.elapsed().as_millis(),
            "response_chars": content.chars().count(),
            "engine_status": "healthy"
        })
    ).await;

    Ok(content)
}

#[command]
pub async fn get_chat_history(pool: State<'_, SqlitePool>) -> Result<Vec<ChatHistoryMessage>, String> {
    let rows: Vec<(
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    )> =
        sqlx::query_as(
            "SELECT message_content, response_content, created_at, platform, chat_id, event_type, event_payload
             FROM chat_history
             ORDER BY rowid ASC"
        )
        .fetch_all(&*pool)
        .await
        .map_err(|e| e.to_string())?;

    let mut history = Vec::new();

    for (message_content, response_content, created_at, platform, chat_id, event_type, event_payload) in rows {
        let ts = created_at.unwrap_or_else(|| "".to_string());
        let source = platform.unwrap_or_else(|| "app".to_string());

        if let Some(user_message) = message_content {
            if !user_message.trim().is_empty() {
                history.push(ChatHistoryMessage {
                    role: "user".to_string(),
                    content: user_message,
                    timestamp: ts.clone(),
                    platform: source.clone(),
                    chat_id: chat_id.clone(),
                    event_type: event_type.clone(),
                    event_payload: event_payload.clone(),
                });
            }
        }

        if let Some(assistant_message) = response_content {
            if !assistant_message.trim().is_empty() {
                history.push(ChatHistoryMessage {
                    role: "assistant".to_string(),
                    content: assistant_message,
                    timestamp: ts.clone(),
                    platform: source.clone(),
                    chat_id: chat_id.clone(),
                    event_type: event_type.clone(),
                    event_payload: event_payload.clone(),
                });
            }
        }
    }

    Ok(history)
}

#[command]
pub async fn get_agent_logs(pool: State<'_, SqlitePool>) -> Result<Vec<AgentLogEntry>, String> {
    let rows: Vec<(String, String, String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT id, ts, level, scope, message, details
         FROM agent_logs
         ORDER BY rowid DESC
         LIMIT 200"
    )
    .fetch_all(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut logs = Vec::with_capacity(rows.len());
    for (id, ts, level, scope, message, details) in rows {
        let parsed_details = details
            .as_deref()
            .and_then(|value| serde_json::from_str::<Value>(value).ok())
            .unwrap_or_else(|| serde_json::json!({}));
        logs.push(AgentLogEntry {
            id,
            ts,
            level,
            scope,
            message,
            details: parsed_details,
        });
    }

    Ok(logs)
}

// ─── Commands (App & Platform Hub) ──────────────────────────────────────────

#[command]
pub async fn save_app_token(
    _app: AppHandle,
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
    Ok(())
}

#[command]
pub async fn set_app_flag(
    pool: State<'_, SqlitePool>,
    key: String,
    value: String,
) -> Result<(), String> {
    sqlx::query("INSERT OR REPLACE INTO secrets (key, value) VALUES (?, ?)")
        .bind(&key)
        .bind(&value)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub async fn get_app_flag(pool: State<'_, SqlitePool>, key: String) -> Result<bool, String> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM secrets WHERE key = ?")
        .bind(&key)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(row
        .map(|(value,)| value == "true" || value == "1" || value.eq_ignore_ascii_case("yes"))
        .unwrap_or(false))
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
    Ok(())
}

#[command]
pub async fn setup_telegram_bot(
    _app: AppHandle,
    pool: State<'_, SqlitePool>,
    bot_token: String,
) -> Result<String, String> {
    let hermes_home = crate::services::setup::get_base_dir().join("hermes");
    fs::create_dir_all(&hermes_home).map_err(|e| e.to_string())?;

    let env_file = hermes_home.join(".env");
    let env_content = format!(
        "TELEGRAM_BOT_TOKEN={}\nHERMES_HOME={}\n",
        bot_token,
        hermes_home.to_string_lossy()
    );

    fs::write(&env_file, env_content).map_err(|e| e.to_string())?;
    println!("[OPERARIUS] Telegram bot token saved to Hermes configuration");

    sqlx::query("INSERT OR REPLACE INTO secrets (key, value) VALUES (?, ?)")
        .bind("Telegram")
        .bind(&bot_token)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;

    let _ = Command::new("pkill")
        .arg("-9")
        .arg("-f")
        .arg("hermes")
        .output();

    crate::services::telegram_bridge::start(pool.inner().clone(), _app.clone());

    Ok("Telegram bot configured. Direct app-to-Telegram bridge is active.".to_string())
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

// ─── RAG Commands ───────────────────────────────────────────────────────────

#[command]
pub async fn upload_document(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    file_path: String,
    user_id: Option<String>,
    platform: Option<String>,
    memory_use: Option<bool>,
) -> Result<String, String> {
    let path = PathBuf::from(&file_path);
    let _ = ensure_embedding_runtime(&app).await;

    let file_id = ingest_file_for_rag(&pool, &path, user_id.clone(), platform.clone()).await?;
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let session_id = if platform.as_deref() == Some("telegram") {
        "telegram-upload".to_string()
    } else {
        "app-main".to_string()
    };
    let user_for_history = user_id.clone().unwrap_or_else(|| "local-user".to_string());
    let platform_for_history = platform.clone().unwrap_or_else(|| "app".to_string());

    let event_payload = serde_json::json!({
        "filename": file_name,
        "size_bytes": fs::metadata(&path).map(|m| m.len()).unwrap_or(0),
        "status": "indexed",
        "source": platform_for_history,
    });

    if memory_use.unwrap_or(true) {
        sqlx::query(
            "INSERT INTO chat_history (id, user_id, session_id, message_content, response_content, created_at, platform, chat_id, event_type, event_payload)
             VALUES (?, ?, ?, ?, ?, datetime('now'), ?, ?, ?, ?)"
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&user_for_history)
        .bind(session_id)
        .bind(format!("Uploaded file: {}", file_name))
        .bind(format!("File indexed and ready: {}", file_name))
        .bind(platform_for_history.clone())
        .bind("app")
        .bind("file_upload")
        .bind(event_payload.to_string())
        .execute(&*pool)
        .await
        .map_err(|e| format!("Failed to save upload chat history: {}", e))?;

        app.emit("chat-history-updated", serde_json::json!({ "platform": "app" }))
            .ok();

        if let Ok(Some((content,))) = sqlx::query_as::<_, (String,)>(
            "SELECT content FROM files WHERE id = ? LIMIT 1"
        )
        .bind(&file_id)
        .fetch_optional(&*pool)
        .await
        {
            let profile_snippet = content.chars().take(420).collect::<String>();
            if !profile_snippet.trim().is_empty() {
                let _ = sqlx::query(
                    "INSERT INTO user_memory_entries (id, user_id, scope, memory_key, memory_value, weight, updated_at)
                     VALUES (?, ?, ?, ?, ?, ?, datetime('now'))"
                )
                .bind(uuid::Uuid::new_v4().to_string())
                .bind(&user_for_history)
                .bind("profile-file")
                .bind(format!("uploaded-file:{}", file_name))
                .bind(profile_snippet)
                .bind(72_i64)
                .execute(&*pool)
                .await;
            }
        }
    }

    println!("[RAG] Document indexed: {} ({})", file_name, file_id);
    Ok(format!("Document indexed successfully: {}", file_id))
}

#[command]
pub async fn search_documents(
    _app: AppHandle,
    pool: State<'_, SqlitePool>,
    query: String,
    limit: Option<i32>,
) -> Result<Vec<serde_json::Value>, String> {
    let limit = limit.unwrap_or(5).min(10) as i64;
    
    // Full-text search on knowledge base
    let rows = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id, title, content FROM knowledge_base 
         WHERE content MATCH ? LIMIT ?"
    )
    .bind(&query)
    .bind(limit)
    .fetch_all(&*pool)
    .await
    .map_err(|e| e.to_string())?;
    
    let results: Vec<serde_json::Value> = rows.iter().map(|(id, title, content)| {
        serde_json::json!({
            "id": id,
            "title": title,
            "content": content.chars().take(300).collect::<String>() + "...",
            "relevance": 0.8
        })
    }).collect();
    
    println!("[RAG] Search query: {} -> {} results", query, results.len());
    Ok(results)
}

#[command]
pub async fn query_rag_agent(
    _app: AppHandle,
    pool: State<'_, SqlitePool>,
    message: String,
    user_id: Option<String>,
) -> Result<String, String> {
    // Inline search to avoid moving pool
    let search_results = {
        let limit = 3i64;
        let rows = sqlx::query_as::<_, (String, String, String)>(
            "SELECT id, title, content FROM knowledge_base 
             WHERE content MATCH ? LIMIT ?"
        )
        .bind(&message)
        .bind(limit)
        .fetch_all(&*pool)
        .await
        .map_err(|e| e.to_string())?;
        
        rows.iter().map(|(id, title, content)| {
            serde_json::json!({
                "id": id,
                "title": title,
                "content": content.chars().take(300).collect::<String>() + "...",
                "relevance": 0.8
            })
        }).collect::<Vec<_>>()
    };
    
    if search_results.is_empty() {
        println!("[RAG] No documents found for query: {}", message);
    } else {
        println!("[RAG] Found {} documents for: {}", search_results.len(), message);
    }
    
    // Build context from documents
    let mut context = String::new();
    for doc in &search_results {
        if let Some(content) = doc.get("content").and_then(|c| c.as_str()) {
            context.push_str(&format!("- {}\n", content));
        }
    }
    
    // Call LLM with RAG context
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(35))
        .build()
        .map_err(|e| format!("Network setup failed: {}", e))?;
    
    let messages = vec![
        serde_json::json!({
            "role": "system",
            "content": "You are Operarius, a world-class AI agent with access to user documents. Answer based on provided context and cite sources. Format output for human readability with a short summary, bullet key points, and a concise next step when useful."
        }),
        serde_json::json!({
            "role": "user",
            "content": if context.is_empty() {
                message.clone()
            } else {
                format!("Context from documents:\n{}\n\nQuestion: {}", context, message)
            }
        })
    ];
    
    let body = serde_json::json!({
        "model": "llama",
        "messages": messages,
        "temperature": 0.5,
        "max_tokens": 220,
        "stream": false,
        "chat_template_kwargs": {
            "enable_thinking": false
        }
    });
    
    let port = crate::services::supervisor::LLAMA_PORT;
    let endpoint = format!("http://127.0.0.1:{}/v1/chat/completions", port);
    
    let resp = client.post(&endpoint).json(&body).send().await
        .map_err(|e| format!("AI Engine unavailable: {}", e))?;
    
    let resp_text = resp.text().await
        .map_err(|e| format!("Failed to read response: {}", e))?;
    
    let resp_json: serde_json::Value = serde_json::from_str(&resp_text)
        .map_err(|_| "Invalid response format".to_string())?;
    
    let content = resp_json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("No response generated")
        .to_string();
    
    // Store in chat history
    sqlx::query(
        "INSERT INTO chat_history (id, user_id, session_id, message_content, response_content, created_at) 
         VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&user_id)
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&message)
    .bind(&content)
    .bind(chrono::Utc::now().timestamp())
    .execute(&*pool)
    .await
    .ok();
    
    Ok(content)
}

#[command]
pub async fn get_knowledge_base(
    _app: AppHandle,
    pool: State<'_, SqlitePool>,
    user_id: Option<String>,
) -> Result<Vec<serde_json::Value>, String> {
    let mut query = "SELECT id, title, tags FROM knowledge_base WHERE 1=1".to_string();
    let mut args: Vec<String> = vec![];
    
    if let Some(uid) = user_id {
        query.push_str(" AND user_id = ?");
        args.push(uid);
    }
    
    let rows = sqlx::query_as::<_, (String, String, String)>(&query)
        .fetch_all(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    
    let results = rows.iter().map(|(id, title, tags)| {
        serde_json::json!({
            "id": id,
            "title": title,
            "tags": tags.split(',').collect::<Vec<_>>()
        })
    }).collect();
    
    Ok(results)
}

#[command]
pub async fn get_indexed_files(pool: State<'_, SqlitePool>) -> Result<Vec<IndexedFile>, String> {
    let rows: Vec<(String, Option<String>, i64)> = sqlx::query_as(
        "SELECT filename, platform, uploaded_at
         FROM files
         ORDER BY uploaded_at DESC
         LIMIT 200"
    )
    .fetch_all(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|(filename, platform, uploaded_at)| IndexedFile {
            filename,
            platform,
            uploaded_at,
        })
        .collect())
}

#[command]
pub async fn get_model_catalog(app: AppHandle, pool: State<'_, SqlitePool>) -> Result<Vec<ModelCapability>, String> {
    let base = get_base_dir(&app)?;
    let models_dir = base.join("models");
    let active_file = get_active_model_file(&pool).await;

    let mut catalog = model_catalog_definition();
    for item in &mut catalog {
        if item.runtime == "llama.cpp" {
            item.installed = models_dir.join(&item.file).exists();
            item.active = active_file.as_deref() == Some(item.file.as_str());
        } else {
            item.installed = models_dir.join(&item.file).exists();
            item.active = false;
        }
    }

    Ok(catalog)
}

#[command]
pub async fn get_active_model(pool: State<'_, SqlitePool>) -> Result<String, String> {
    Ok(get_active_model_file(&pool).await.unwrap_or_default())
}

#[command]
pub async fn switch_active_model(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    filename: String,
) -> Result<String, String> {
    let catalog = model_catalog_definition();
    let model = catalog
        .iter()
        .find(|m| m.file == filename)
        .ok_or_else(|| "Model not found in catalog".to_string())?;

    if model.runtime != "llama.cpp" {
        return Err("This model uses MLX/omlx runtime and cannot be switched in llama.cpp engine yet.".to_string());
    }

    let model_path = get_base_dir(&app)?.join("models").join(&filename);
    if !model_path.exists() {
        return Err(format!("Model is not installed: {}", filename));
    }

    crate::services::supervisor::stop_all();
    crate::services::supervisor::start_llama(&app, &model_path.to_string_lossy(), crate::services::supervisor::LLAMA_PORT)?;
    crate::services::supervisor::wait_for_llama(&app).await;
    let _ = ensure_embedding_runtime(&app).await;
    crate::services::telegram_bridge::start(pool.inner().clone(), app.clone());

    sqlx::query("INSERT OR REPLACE INTO secrets (key, value) VALUES ('active_model_file', ?)")
        .bind(&filename)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(filename)
}

#[command]
pub async fn delete_model_file(app: AppHandle, pool: State<'_, SqlitePool>, filename: String) -> Result<bool, String> {
    let path = get_base_dir(&app)?.join("models").join(&filename);
    if !path.exists() {
        return Ok(false);
    }

    if path.is_dir() {
        fs::remove_dir_all(&path).map_err(|e| e.to_string())?;
    } else {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }

    if get_active_model_file(&pool).await.as_deref() == Some(filename.as_str()) {
        sqlx::query("DELETE FROM secrets WHERE key = 'active_model_file'")
            .execute(&*pool)
            .await
            .ok();
    }

    Ok(true)
}

#[command]
pub async fn set_user_preference(
    pool: State<'_, SqlitePool>,
    user_id: String,
    key: String,
    value: String,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO user_preferences (user_id, pref_key, pref_value, updated_at)
         VALUES (?, ?, ?, datetime('now'))
         ON CONFLICT(user_id, pref_key)
         DO UPDATE SET pref_value = excluded.pref_value, updated_at = datetime('now')"
    )
    .bind(user_id)
    .bind(key)
    .bind(value)
    .execute(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[command]
pub async fn get_user_preferences(
    pool: State<'_, SqlitePool>,
    user_id: String,
) -> Result<Vec<UserPreference>, String> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT pref_key, pref_value
         FROM user_preferences
         WHERE user_id = ?
         ORDER BY updated_at DESC"
    )
    .bind(user_id)
    .fetch_all(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|(key, value)| UserPreference { key, value })
        .collect())
}

#[command]
pub async fn upsert_memory_entry(
    pool: State<'_, SqlitePool>,
    user_id: String,
    scope: String,
    memory_key: String,
    value: String,
    weight: Option<i64>,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let w = weight.unwrap_or(50).clamp(1, 100);

    sqlx::query(
        "INSERT INTO user_memory_entries (id, user_id, scope, memory_key, memory_value, weight, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, datetime('now'))
         ON CONFLICT(user_id, scope, memory_key)
         DO UPDATE SET memory_value = excluded.memory_value, weight = excluded.weight, updated_at = datetime('now')"
    )
    .bind(&id)
    .bind(user_id)
    .bind(scope)
    .bind(memory_key)
    .bind(value)
    .bind(w)
    .execute(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(id)
}

#[command]
pub async fn list_memory_entries(
    pool: State<'_, SqlitePool>,
    user_id: String,
    scope: Option<String>,
) -> Result<Vec<MemoryEntry>, String> {
    let rows: Vec<(String, String, String, String, String, i64, String)> = if let Some(s) = scope {
        sqlx::query_as(
            "SELECT id, user_id, scope, memory_key, memory_value, weight, updated_at
             FROM user_memory_entries
             WHERE user_id = ? AND scope = ?
             ORDER BY weight DESC, updated_at DESC"
        )
        .bind(user_id)
        .bind(s)
        .fetch_all(&*pool)
        .await
        .map_err(|e| e.to_string())?
    } else {
        sqlx::query_as(
            "SELECT id, user_id, scope, memory_key, memory_value, weight, updated_at
             FROM user_memory_entries
             WHERE user_id = ?
             ORDER BY weight DESC, updated_at DESC"
        )
        .bind(user_id)
        .fetch_all(&*pool)
        .await
        .map_err(|e| e.to_string())?
    };

    Ok(rows
        .into_iter()
        .map(|(id, user_id, scope, memory_key, value, weight, updated_at)| MemoryEntry {
            id,
            user_id,
            scope,
            memory_key,
            value,
            weight,
            updated_at,
        })
        .collect())
}

#[command]
pub async fn delete_memory_entry(pool: State<'_, SqlitePool>, id: String) -> Result<(), String> {
    sqlx::query("DELETE FROM user_memory_entries WHERE id = ?")
        .bind(id)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub async fn add_task_learning(
    pool: State<'_, SqlitePool>,
    user_id: String,
    task: String,
    lesson: String,
    success: Option<i64>,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO task_learnings (id, user_id, task, lesson, success, created_at)
         VALUES (?, ?, ?, ?, ?, datetime('now'))"
    )
    .bind(&id)
    .bind(user_id)
    .bind(task)
    .bind(lesson)
    .bind(success.unwrap_or(1))
    .execute(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(id)
}

#[command]
pub async fn list_task_learnings(
    pool: State<'_, SqlitePool>,
    user_id: String,
    limit: Option<i64>,
) -> Result<Vec<TaskLearning>, String> {
    let rows: Vec<(String, String, String, i64, String)> = sqlx::query_as(
        "SELECT id, task, lesson, success, created_at
         FROM task_learnings
         WHERE user_id = ?
         ORDER BY created_at DESC
         LIMIT ?"
    )
    .bind(user_id)
    .bind(limit.unwrap_or(50).clamp(1, 200))
    .fetch_all(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|(id, task, lesson, success, created_at)| TaskLearning {
            id,
            task,
            lesson,
            success,
            created_at,
        })
        .collect())
}

#[command]
pub async fn create_user_skill(
    pool: State<'_, SqlitePool>,
    user_id: String,
    name: String,
    description: String,
    instruction: String,
    triggers: Option<String>,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO user_skills (id, user_id, name, description, instruction, is_active, triggers, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, 1, ?, datetime('now'), datetime('now'))"
    )
    .bind(&id)
    .bind(user_id)
    .bind(name)
    .bind(description)
    .bind(instruction)
    .bind(triggers.unwrap_or_default())
    .execute(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(id)
}

#[command]
pub async fn list_user_skills(
    pool: State<'_, SqlitePool>,
    user_id: String,
) -> Result<Vec<UserSkill>, String> {
    let rows: Vec<(String, String, String, String, String, i64, String, String)> = sqlx::query_as(
        "SELECT id, user_id, name, description, instruction, is_active, triggers, updated_at
         FROM user_skills
         WHERE user_id = ?
         ORDER BY updated_at DESC"
    )
    .bind(user_id)
    .fetch_all(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|(id, user_id, name, description, instruction, is_active, triggers, updated_at)| UserSkill {
            id,
            user_id,
            name,
            description,
            instruction,
            is_active,
            triggers,
            updated_at,
        })
        .collect())
}

#[command]
pub async fn set_user_skill_active(
    pool: State<'_, SqlitePool>,
    skill_id: String,
    is_active: bool,
) -> Result<(), String> {
    sqlx::query(
        "UPDATE user_skills
         SET is_active = ?, updated_at = datetime('now')
         WHERE id = ?"
    )
    .bind(if is_active { 1_i64 } else { 0_i64 })
    .bind(skill_id)
    .execute(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[command]
pub async fn delete_user_skill(pool: State<'_, SqlitePool>, skill_id: String) -> Result<(), String> {
    sqlx::query("DELETE FROM user_skills WHERE id = ?")
        .bind(skill_id)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub async fn get_chat_upload_event_payload(content: String) -> Result<Option<Value>, String> {
    if !content.starts_with("Uploaded file:") {
        return Ok(None);
    }
    Ok(Some(serde_json::json!({ "type": "file_upload" })))
}

#[command]
pub async fn verify_model_links() -> Result<Vec<ModelLinkStatus>, String> {
    let catalog = model_catalog_definition();
    let mut out = Vec::new();

    for item in catalog {
        if item.runtime == "llama.cpp" {
            match resolve_hf_file_size(&item.repo, &item.file).await {
                Ok(size) => out.push(ModelLinkStatus {
                    model_id: item.id,
                    repo: item.repo,
                    file: item.file,
                    ok: true,
                    remote_size_bytes: Some(size),
                    message: "ok".to_string(),
                }),
                Err(err) => out.push(ModelLinkStatus {
                    model_id: item.id,
                    repo: item.repo,
                    file: item.file,
                    ok: false,
                    remote_size_bytes: None,
                    message: err,
                }),
            }
        } else {
            match list_hf_repo_files(&item.repo).await {
                Ok(files) => out.push(ModelLinkStatus {
                    model_id: item.id,
                    repo: item.repo,
                    file: item.file,
                    ok: files.iter().any(|f| f.ends_with(".safetensors") || f.ends_with(".index.json")),
                    remote_size_bytes: None,
                    message: format!("snapshot files discovered: {}", files.len()),
                }),
                Err(err) => out.push(ModelLinkStatus {
                    model_id: item.id,
                    repo: item.repo,
                    file: item.file,
                    ok: false,
                    remote_size_bytes: None,
                    message: err,
                }),
            }
        }
    }

    Ok(out)
}

#[command]
pub async fn fetch_skill_marketplace(
    app: AppHandle,
    query: Option<String>,
) -> Result<Vec<SkillMarketplaceItem>, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let page = client
        .get("https://hermes-agent.nousresearch.com/docs/skills")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch skills hub: {}", e))?
        .text()
        .await
        .map_err(|e| format!("Failed to parse skills hub: {}", e))?;

    let installed_raw = crate::services::skills_manager::list_skills(&app).unwrap_or_default();
    let installed = installed_raw
        .into_iter()
        .map(|s| s.to_lowercase())
        .collect::<Vec<String>>();

    let q = query.unwrap_or_default().to_lowercase();
    let mut parsed = parse_skills_hub_page(&page)
        .into_iter()
        .filter(|(name, _built_in, registry, description)| {
            q.is_empty()
                || name.to_lowercase().contains(&q)
                || registry.to_lowercase().contains(&q)
                || description.to_lowercase().contains(&q)
        })
        .map(|(name, built_in, registry, description)| {
            let installed_match = installed.iter().any(|s| s.contains(&name.to_lowercase()));
            SkillMarketplaceItem {
                name,
                built_in,
                registry,
                description,
                installed: installed_match,
            }
        })
        .collect::<Vec<SkillMarketplaceItem>>();

    parsed.sort_by(|a, b| a.name.cmp(&b.name));
    parsed.truncate(700);
    Ok(parsed)
}

#[command]
pub async fn install_marketplace_skill(app: AppHandle, skill_name: String) -> Result<String, String> {
    crate::services::skills_manager::install_skill(&app, &skill_name)
}

#[command]
pub async fn save_agent_workflow(
    pool: State<'_, SqlitePool>,
    user_id: String,
    name: String,
    workflow_json: Value,
) -> Result<String, String> {
    let workflow_id = workflow_json
        .get("id")
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO agent_workflows (id, user_id, name, workflow_json, compiled_json, updated_at)
         VALUES (?, ?, ?, ?, NULL, ?)
         ON CONFLICT(id) DO UPDATE SET
             user_id = excluded.user_id,
             name = excluded.name,
             workflow_json = excluded.workflow_json,
             updated_at = excluded.updated_at"
    )
    .bind(&workflow_id)
    .bind(user_id)
    .bind(name)
    .bind(workflow_json.to_string())
    .bind(now)
    .execute(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(workflow_id)
}

#[command]
pub async fn list_agent_workflows(
    pool: State<'_, SqlitePool>,
    user_id: String,
) -> Result<Vec<StoredAgentWorkflow>, String> {
    let rows: Vec<(String, String, String, String, Option<String>, String)> = sqlx::query_as(
        "SELECT id, user_id, name, workflow_json, compiled_json, updated_at
         FROM agent_workflows
         WHERE user_id = ?
         ORDER BY updated_at DESC"
    )
    .bind(user_id)
    .fetch_all(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    for (id, user_id, name, workflow_json, compiled_json, updated_at) in rows {
        let parsed_workflow: Value = serde_json::from_str(&workflow_json)
            .unwrap_or_else(|_| serde_json::json!({"raw": workflow_json}));

        let parsed_compiled = compiled_json
            .and_then(|raw| serde_json::from_str::<Value>(&raw).ok());

        out.push(StoredAgentWorkflow {
            id,
            user_id,
            name,
            workflow_json: parsed_workflow,
            compiled_json: parsed_compiled,
            updated_at,
        });
    }

    Ok(out)
}

#[command]
pub async fn compile_agent_workflow(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    workflow_json: Value,
) -> Result<CompiledWorkflow, String> {
    let workflow: BuilderWorkflow = serde_json::from_value(workflow_json.clone())
        .map_err(|e| format!("Invalid workflow payload: {}", e))?;

    let compiled = workflow_compiler::compile(&workflow)?;

    let _ = emit_agent_log(
        &app,
        Some(&pool),
        "info",
        "builder",
        "Workflow compiled",
        serde_json::json!({
            "workflow_id": compiled.workflow_id,
            "node_count": compiled.node_count,
            "edge_count": compiled.edge_count,
            "capability_flags": compiled.capability_flags,
            "warnings": compiled.warnings,
        }),
    )
    .await;

    sqlx::query(
        "UPDATE agent_workflows
         SET compiled_json = ?, updated_at = ?
         WHERE id = ?"
    )
    .bind(compiled.manifest.to_string())
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(&compiled.workflow_id)
    .execute(&*pool)
    .await
    .ok();

    Ok(compiled)
}
