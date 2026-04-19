use tauri::AppHandle;

#[deprecated(note = "Please use services::supervisor instead")]
pub fn start_gateway(app: &AppHandle) {
    crate::services::supervisor::start_hermes(app, None);
}

#[deprecated(note = "Please use services::supervisor instead")]
pub fn stop_all() {
    crate::services::supervisor::stop_all();
}
