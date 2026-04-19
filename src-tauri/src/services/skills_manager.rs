use std::process::Command;
use tauri::AppHandle;
use std::path::PathBuf;

fn get_hermes_bin(app: &AppHandle) -> PathBuf {
    use tauri::Manager;
    app.path().resource_dir().unwrap_or_default()
        .join("hermes-aarch64-apple-darwin")
}

pub fn install_skill(app: &AppHandle, repo_url: &str) -> Result<String, String> {
    let bin = get_hermes_bin(app);
    let output = Command::new(bin)
        .arg("skills")
        .arg("install")
        .arg(repo_url)
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

pub fn list_skills(app: &AppHandle) -> Result<Vec<String>, String> {
    let bin = get_hermes_bin(app);
    let output = Command::new(bin)
        .arg("skills")
        .arg("list")
        .output()
        .map_err(|e| e.to_string())?;

    let text = String::from_utf8_lossy(&output.stdout);
    let skills: Vec<String> = text.lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect();
    
    Ok(skills)
}
