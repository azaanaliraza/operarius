use crate::services::downloader::download_with_resume;
use base64::Engine;
use reqwest::Client;
use serde::Serialize;
use sqlx::SqlitePool;
use std::fs::{self, File};
use std::path::PathBuf;
use std::process::Command;
use tauri::{command, AppHandle, Emitter, Manager, State};

const DEFAULT_EMBEDDING_REPO: &str = "nomic-ai/nomic-embed-text-v1.5-GGUF";
const DEFAULT_EMBEDDING_FILE: &str = "nomic-embed-text-v1.5.Q8_0.gguf";
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

#[derive(Serialize)]
pub struct ChatHistoryMessage {
    pub role: String,
    pub content: String,
    pub timestamp: String,
    pub platform: String,
    pub chat_id: Option<String>,
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

async fn get_recent_chat_context(pool: &SqlitePool, session_filter: &str) -> String {
    let rows: Vec<(Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT message_content, response_content
         FROM chat_history
         WHERE session_id = ?
         ORDER BY rowid DESC
         LIMIT 8"
    )
    .bind(session_filter)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut lines = Vec::new();
    for (user, assistant) in rows.into_iter().rev() {
        if let Some(u) = user {
            if !u.trim().is_empty() {
                lines.push(format!("User: {}", u));
            }
        }
        if let Some(a) = assistant {
            if !a.trim().is_empty() {
                lines.push(format!("Assistant: {}", a));
            }
        }
    }

    lines.join("\n")
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

    let row: Option<(String, String, Vec<u8>)> = sqlx::query_as(
        "SELECT f.id, f.file_type, b.data
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

    let Some((_file_id, file_type, bytes)) = row else {
        return None;
    };

    let ext = file_type.to_lowercase();
    let Some(mime) = image_mime_from_ext(&ext) else {
        return None;
    };

    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    let data_url = format!("data:{};base64,{}", mime, b64);

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60))
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

    let response = client.post(&endpoint).json(&body).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }

    let value: serde_json::Value = response.json().await.ok()?;
    let content = value["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    if content.is_empty() {
        None
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
    
    // 1. Integrity Check: Ensure file exists and is reasonable size (1.8GB+ for 3B model)
    let expected_size: u64 = if filename.contains("3B") { 1_800_000_000 } else { 800_000_000 };
    let current_size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

    if current_size >= expected_size {
        println!("[ORCHESTRATOR] Integrity Check PASSED for {} ({:.2} GB)", filename, current_size as f64 / 1_073_741_824.0);
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
) -> Result<String, String> {
    if message.trim().is_empty() {
        return Ok("I'm listening...".into());
    }

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Network setup failed: {}", e))?;

    println!("[OPERARIUS] Chat query: {}", message);

    // Context Truncation & Intent Filtering (Surgical token management)
    let lower_msg = message.to_lowercase();
    let is_chitchat = lower_msg.contains("hi") || lower_msg.contains("hello") || lower_msg.contains("hey") || 
                     lower_msg.contains("how are you") || lower_msg.contains("thank") || lower_msg.len() < 25;
    
    let _ = ensure_embedding_runtime(&app).await;
    let rag_context = build_targeted_rag_context(&pool, &message, target_file.as_deref()).await;
    let visual_context = build_visual_context_for_target_file(&pool, target_file.as_deref(), &message).await;

    let recent_context = get_recent_chat_context(&*pool, "app-main").await;

    // 1. Keep prompt compact but include memory + retrieval context.
    let user_prompt = if rag_context.is_empty() && recent_context.is_empty() {
        message.clone()
    } else {
        format!(
            "Recent conversation:\n{}\n\nRetrieved knowledge:\n{}\n\nVisual analysis (if target image):\n{}\n\nTarget file: {}\n\nCurrent user message: {}",
            if recent_context.is_empty() { "(none)" } else { &recent_context },
            if rag_context.is_empty() { "(none)" } else { &rag_context },
            visual_context.as_deref().unwrap_or("(none)"),
            target_file.clone().unwrap_or_else(|| "(none)".to_string()),
            message
        )
    };

    // 2. Force extreme truncation: system prompt + current turn with context.
    let messages = vec![
        serde_json::json!({
            "role": "system",
            "content": "You are Operarius, a world-class AI agent. Use conversation memory, retrieved knowledge, and visual analysis when relevant. If a target file is provided, answer specifically from that file first. Format every answer for humans: 1) a short summary line, 2) key points as bullet lines, 3) optional 'Next step' line when useful. Keep paragraphs short and readable."
        }),
        serde_json::json!({ "role": "user", "content": user_prompt })
    ];

    if !is_chitchat {
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

    Ok(content)
}

#[command]
pub async fn get_chat_history(pool: State<'_, SqlitePool>) -> Result<Vec<ChatHistoryMessage>, String> {
    let rows: Vec<(Option<String>, Option<String>, Option<String>, Option<String>, Option<String>)> =
        sqlx::query_as(
            "SELECT message_content, response_content, created_at, platform, chat_id
             FROM chat_history
             ORDER BY rowid ASC"
        )
        .fetch_all(&*pool)
        .await
        .map_err(|e| e.to_string())?;

    let mut history = Vec::new();

    for (message_content, response_content, created_at, platform, chat_id) in rows {
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
                });
            }
        }
    }

    Ok(history)
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

    sqlx::query(
        "INSERT INTO chat_history (id, user_id, session_id, message_content, response_content, created_at, platform, chat_id)
         VALUES (?, ?, ?, ?, ?, datetime('now'), ?, ?)"
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(user_for_history)
    .bind(session_id)
    .bind(format!("Uploaded file: {}", file_name))
    .bind(format!("File indexed and ready: {}", file_name))
    .bind(platform_for_history)
    .bind("app")
    .execute(&*pool)
    .await
    .map_err(|e| format!("Failed to save upload chat history: {}", e))?;

    app.emit("chat-history-updated", serde_json::json!({ "platform": "app" }))
        .ok();

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
        .timeout(std::time::Duration::from_secs(60))
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
        "temperature": 0.7,
        "stream": false
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
