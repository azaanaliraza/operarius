use std::fs;

pub fn write_hermes_config(model_id: &str, _provider: &str, _unused: Option<String>, _port: u16) -> Result<std::path::PathBuf, String> {
    let base_dir = crate::services::setup::get_base_dir();
    let config_dir = base_dir.join("config");
    
    fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;

    let config = format!(
r#"---
model:
  provider: "custom"
  default: "{model_id}"
  base_url: "http://127.0.0.1:8080/v1"
  context_length: 65536

memory:
  memory_enabled: false
  user_profile_enabled: false

context:
  max_tokens: 3000

compression:
  enabled: false

agent:
  system_prompt: "You are Operarius, a world-class AI agent. Keep responses helpful, direct, and extremely concise. Do not generate code unless requested."

embedding:
  provider: "local"
  dimension: 768

auxiliary:
  title_generation:
    provider: main
    model: "llama"

rag:
  enabled: false

gateway:
  allow_all_users: true
  api:
    enabled: true
    port: 8989

platform_toolsets:
  telegram: []
"#,
        model_id = model_id
    );


    let config_path = base_dir.join("hermes").join("config.yaml");
    fs::create_dir_all(base_dir.join("hermes")).map_err(|e| e.to_string())?;
    fs::write(&config_path, config).map_err(|e| e.to_string())?;
    println!("[ARCHITECT] Hermes Manifest SECURED at port 8080");
    Ok(config_path)
}
