use std::fs;

pub fn write_hermes_config(model_id: &str, _provider: &str, telegram_token: Option<String>, _port: u16) -> Result<std::path::PathBuf, String> {
    let base_dir = crate::services::setup::get_base_dir();
    let config_dir = base_dir.join("config");
    
    fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;

    let mut config = format!(
r#"---
model:
  provider: "custom"
  default: "{model_id}"
  base_url: "http://127.0.0.1:8080/v1"
  context_length: 131072

memory:
  max_turns: 5
  summarization_threshold: 10
  short_term_limit: 4000

agent:
  system_prompt: "You are Operarius, a world-class AI agent. Keep responses helpful, direct, and extremely concise. Do not generate code unless requested."

embedding:
  provider: "local"
  dimension: 768

auxiliary:
  provider: "custom"
  base_url: "http://127.0.0.1:8080/v1"
  name: "llama"

gateway:
  allow_all_users: true
  api:
    enabled: true
    port: 8989
"#,
        model_id = model_id
    );

    if let Some(token) = telegram_token {
        if !token.is_empty() {
            config.push_str(&format!(
r#"
platforms:
  telegram:
    enabled: true
    bot_token: "{}"
    allowed_users: ["*"]
"#,
                token
            ));
        }
    }

    let config_path = base_dir.join("hermes").join("config.yaml");
    fs::create_dir_all(base_dir.join("hermes")).map_err(|e| e.to_string())?;
    fs::write(&config_path, config).map_err(|e| e.to_string())?;
    println!("[ARCHITECT] Hermes Manifest SECURED at port 8080");
    Ok(config_path)
}
