use reqwest::Client;
use serde_json::Value;
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::time::sleep;

static TELEGRAM_BRIDGE_RUNNING: AtomicBool = AtomicBool::new(false);

pub fn start(pool: SqlitePool, app: AppHandle) {
    if TELEGRAM_BRIDGE_RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }

    tauri::async_runtime::spawn(async move {
        println!("[TELEGRAM_BRIDGE] Starting direct Telegram bridge");
        crate::commands::emit_agent_log(
            &app,
            Some(&pool),
            "info",
            "telegram",
            "Telegram bridge started",
            serde_json::json!({})
        ).await;

        let client = match Client::builder().timeout(Duration::from_secs(45)).build() {
            Ok(c) => c,
            Err(e) => {
                println!("[TELEGRAM_BRIDGE] Failed to build HTTP client: {}", e);
                crate::commands::emit_agent_log(
                    &app,
                    Some(&pool),
                    "error",
                    "telegram",
                    "Failed to build Telegram HTTP client",
                    serde_json::json!({ "error": e.to_string() })
                ).await;
                TELEGRAM_BRIDGE_RUNNING.store(false, Ordering::SeqCst);
                return;
            }
        };

        let mut offset: i64 = 0;

        loop {
            let token = read_telegram_token();
            if token.is_none() {
                sleep(Duration::from_secs(5)).await;
                continue;
            }
            let token = token.unwrap_or_default();

            let updates_url = format!(
                "https://api.telegram.org/bot{}/getUpdates?timeout=25&offset={}",
                token,
                offset + 1
            );

            let updates = match client.get(&updates_url).send().await {
                Ok(resp) => match resp.json::<Value>().await {
                    Ok(json) => json,
                    Err(e) => {
                        println!("[TELEGRAM_BRIDGE] Failed to parse updates: {}", e);
                        sleep(Duration::from_secs(2)).await;
                        continue;
                    }
                },
                Err(e) => {
                    println!("[TELEGRAM_BRIDGE] getUpdates failed: {}", e);
                    sleep(Duration::from_secs(2)).await;
                    continue;
                }
            };

            let Some(results) = updates.get("result").and_then(|v| v.as_array()) else {
                sleep(Duration::from_millis(500)).await;
                continue;
            };

            for update in results {
                if let Some(update_id) = update.get("update_id").and_then(|v| v.as_i64()) {
                    if update_id > offset {
                        offset = update_id;
                    }
                }

                let Some(message) = update.get("message") else {
                    continue;
                };

                if message
                    .get("from")
                    .and_then(|f| f.get("is_bot"))
                    .and_then(|b| b.as_bool())
                    .unwrap_or(false)
                {
                    continue;
                }

                let chat_id = message
                    .get("chat")
                    .and_then(|c| c.get("id"))
                    .and_then(|id| id.as_i64());
                let user_id = message
                    .get("from")
                    .and_then(|f| f.get("id"))
                    .and_then(|id| id.as_i64())
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "telegram-user".to_string());
                let text = message
                    .get("text")
                    .and_then(|t| t.as_str())
                    .map(|s| s.trim().to_string());

                let caption = message
                    .get("caption")
                    .and_then(|t| t.as_str())
                    .map(|s| s.trim().to_string());

                let (Some(chat_id), maybe_text) = (chat_id, text.clone().or(caption.clone())) else {
                    continue;
                };

                let mut indexed_file_note: Option<String> = None;
                let mut uploaded_file_name: Option<String> = None;

                if let Some(file_id) = message
                    .get("document")
                    .and_then(|d| d.get("file_id"))
                    .and_then(|id| id.as_str())
                {
                    if let Ok(downloaded_path) = download_telegram_file(&client, &token, file_id).await {
                        uploaded_file_name = downloaded_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .map(|s| s.to_string());
                        let _ = crate::commands::ensure_embedding_runtime(&app).await;
                        let indexed_id = crate::commands::ingest_file_for_rag(
                            &pool,
                            &downloaded_path,
                            Some(user_id.clone()),
                            Some("telegram".to_string()),
                        )
                        .await;

                        if let Ok(doc_id) = indexed_id {
                            indexed_file_note = Some(format!(
                                    "File indexed successfully (id: {}). You can now ask questions and mention the file name for best accuracy.",
                                doc_id
                            ));
                        }
                    }
                } else if let Some(photo_arr) = message.get("photo").and_then(|p| p.as_array()) {
                    if let Some(file_id) = photo_arr
                        .iter()
                        .filter_map(|p| p.get("file_id").and_then(|id| id.as_str()))
                        .last()
                    {
                        if let Ok(downloaded_path) = download_telegram_file(&client, &token, file_id).await {
                            uploaded_file_name = downloaded_path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .map(|s| s.to_string());
                            let _ = crate::commands::ensure_embedding_runtime(&app).await;
                            let indexed_id = crate::commands::ingest_file_for_rag(
                                &pool,
                                &downloaded_path,
                                Some(user_id.clone()),
                                Some("telegram".to_string()),
                            )
                            .await;

                            if let Ok(doc_id) = indexed_id {
                                indexed_file_note = Some(format!(
                                    "Image indexed successfully (id: {}). You can now ask questions and mention the file name for best accuracy.",
                                    doc_id
                                ));
                                crate::commands::emit_agent_log(
                                    &app,
                                    Some(&pool),
                                    "info",
                                    "telegram",
                                    "Telegram document indexed",
                                    serde_json::json!({
                                        "chat_id": chat_id,
                                        "file_name": uploaded_file_name,
                                        "document_id": doc_id,
                                    })
                                ).await;
                            }
                        }
                    }
                }

                let text = maybe_text.unwrap_or_else(|| "".to_string());
                let stored_message = if !text.trim().is_empty() {
                    text.clone()
                } else if let Some(name) = uploaded_file_name.clone() {
                    format!("Uploaded file: {}", name)
                } else {
                    "Uploaded file".to_string()
                };

                println!("[TELEGRAM_BRIDGE] inbound chat_id={} text={}", chat_id, text);
                crate::commands::emit_agent_log(
                    &app,
                    Some(&pool),
                    "info",
                    "telegram",
                    "Telegram message received",
                    serde_json::json!({
                        "chat_id": chat_id,
                        "has_text": !text.trim().is_empty(),
                        "has_attachment": uploaded_file_name.is_some(),
                    })
                ).await;

                let response = if text.is_empty() {
                    indexed_file_note.unwrap_or_else(|| {
                        "File received. Ask me a question about it to use the indexed content.".to_string()
                    })
                } else {
                    let mut base_response = match query_local_model(&client, &pool, chat_id, &text).await {
                        Ok(r) => r,
                        Err(e) => {
                            println!("[TELEGRAM_BRIDGE] model error: {}", e);
                            "I hit a technical snag. Please try again in a moment.".to_string()
                        }
                    };

                    if let Some(note) = indexed_file_note {
                        base_response = format!("{}\n\n{}", note, base_response);
                    }
                    base_response
                };

                let send_url = format!("https://api.telegram.org/bot{}/sendMessage", token);
                let send_body = serde_json::json!({
                    "chat_id": chat_id,
                    "text": response,
                });

                if let Err(e) = client.post(&send_url).json(&send_body).send().await {
                    println!("[TELEGRAM_BRIDGE] sendMessage failed: {}", e);
                    crate::commands::emit_agent_log(
                        &app,
                        Some(&pool),
                        "warning",
                        "telegram",
                        "Telegram response send failed",
                        serde_json::json!({
                            "chat_id": chat_id,
                            "error": e.to_string(),
                        })
                    ).await;
                }

                let event_type = if stored_message.starts_with("Uploaded file:") {
                    Some("file_upload")
                } else {
                    None
                };
                let event_payload = if let Some(name) = uploaded_file_name.clone() {
                    serde_json::json!({
                        "filename": name,
                        "status": "indexed",
                        "source": "telegram"
                    })
                    .to_string()
                } else {
                    "".to_string()
                };

                if let Err(e) = sqlx::query(
                    "INSERT INTO chat_history (id, user_id, session_id, message_content, response_content, created_at, platform, chat_id, event_type, event_payload)
                     VALUES (?, ?, ?, ?, ?, datetime('now'), ?, ?, ?, ?)"
                )
                .bind(uuid::Uuid::new_v4().to_string())
                .bind(user_id)
                .bind(format!("telegram-{}", chat_id))
                .bind(stored_message)
                .bind(send_body["text"].as_str().unwrap_or_default())
                .bind("telegram")
                .bind(chat_id.to_string())
                .bind(event_type)
                .bind(event_payload)
                .execute(&pool)
                .await
                {
                    println!("[TELEGRAM_BRIDGE] failed to persist chat history: {}", e);
                    crate::commands::emit_agent_log(
                        &app,
                        Some(&pool),
                        "error",
                        "telegram",
                        "Failed to persist Telegram chat history",
                        serde_json::json!({
                            "chat_id": chat_id,
                            "error": e.to_string(),
                        })
                    ).await;
                } else {
                    crate::commands::emit_agent_log(
                        &app,
                        Some(&pool),
                        "info",
                        "telegram",
                        "Telegram conversation synced",
                        serde_json::json!({
                            "chat_id": chat_id,
                            "has_file": uploaded_file_name.is_some(),
                        })
                    ).await;
                    app.emit("chat-history-updated", serde_json::json!({ "platform": "telegram" }))
                        .ok();
                }
            }
        }
    });
}

async fn download_telegram_file(client: &Client, token: &str, file_id: &str) -> Result<PathBuf, String> {
    let get_file_url = format!("https://api.telegram.org/bot{}/getFile?file_id={}", token, file_id);
    let info = client
        .get(&get_file_url)
        .send()
        .await
        .map_err(|e| format!("getFile request failed: {}", e))?
        .json::<Value>()
        .await
        .map_err(|e| format!("getFile parse failed: {}", e))?;

    let remote_path = info
        .get("result")
        .and_then(|r| r.get("file_path"))
        .and_then(|p| p.as_str())
        .ok_or_else(|| "Telegram file path missing".to_string())?;

    let download_url = format!("https://api.telegram.org/file/bot{}/{}", token, remote_path);
    let bytes = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| format!("file download failed: {}", e))?
        .bytes()
        .await
        .map_err(|e| format!("file bytes read failed: {}", e))?;

    let file_name = remote_path
        .split('/')
        .last()
        .filter(|name| !name.is_empty())
        .unwrap_or("telegram_file.bin");

    let downloads_dir = crate::services::setup::get_base_dir().join("uploads");
    std::fs::create_dir_all(&downloads_dir).map_err(|e| e.to_string())?;
    let local_path = downloads_dir.join(format!("{}_{}", uuid::Uuid::new_v4(), file_name));

    std::fs::write(&local_path, &bytes).map_err(|e| format!("save downloaded file failed: {}", e))?;
    Ok(local_path)
}

fn read_telegram_token() -> Option<String> {
    let env_path = crate::services::setup::get_base_dir().join("hermes").join(".env");
    let content = std::fs::read_to_string(env_path).ok()?;

    for line in content.lines() {
        if let Some(value) = line.strip_prefix("TELEGRAM_BOT_TOKEN=") {
            let token = value.trim();
            if !token.is_empty() {
                return Some(token.to_string());
            }
        }
    }

    None
}

async fn query_local_model(
    client: &Client,
    pool: &SqlitePool,
    chat_id: i64,
    message: &str,
) -> Result<String, String> {
    let rows: Vec<(Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT message_content, response_content
         FROM chat_history
         WHERE session_id = ?
         ORDER BY rowid DESC
         LIMIT 8"
    )
    .bind(format!("telegram-{}", chat_id))
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut history = Vec::new();
    for (u, a) in rows.into_iter().rev() {
        if let Some(user_message) = u {
            if !user_message.trim().is_empty() {
                history.push(format!("User: {}", user_message));
            }
        }
        if let Some(assistant_message) = a {
            if !assistant_message.trim().is_empty() {
                history.push(format!("Assistant: {}", assistant_message));
            }
        }
    }

    let rag_context = crate::commands::build_rag_context(pool, message).await;

    let user_prompt = format!(
        "Recent conversation:\n{}\n\nRetrieved knowledge:\n{}\n\nCurrent user message: {}",
        if history.is_empty() { "(none)".to_string() } else { history.join("\n") },
        if rag_context.is_empty() { "(none)".to_string() } else { rag_context },
        message
    );

    let body = serde_json::json!({
        "model": "llama",
        "messages": [
            { "role": "system", "content": "You are Operarius, a world-class AI agent. Use recent conversation context to answer consistently. Format every answer for humans: one short summary line, then bullet key points, then a next step if useful." },
            { "role": "user", "content": user_prompt }
        ],
        "temperature": 0.7,
        "stream": false
    });

    let endpoint = format!(
        "http://127.0.0.1:{}/v1/chat/completions",
        crate::services::supervisor::LLAMA_PORT
    );

    let resp = client
        .post(&endpoint)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("AI engine unreachable: {}", e))?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read model response: {}", e))?;

    if !status.is_success() {
        return Err(format!("Model returned {}: {}", status, text));
    }

    let json: Value = serde_json::from_str(&text)
        .map_err(|_| "Model returned invalid JSON".to_string())?;

    Ok(json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("I could not generate a response right now.")
        .trim()
        .to_string())
}
