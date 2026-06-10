use tauri::{AppHandle, Runtime};
use crate::TenwaExt;

#[tauri::command]
pub async fn open_whatsapp<R: Runtime>(
    app: AppHandle<R>,
    visible: Option<bool>,
) -> Result<(), String> {
    app.tenwa_open(visible)
}

#[tauri::command]
pub async fn auth_status_update<R: Runtime>(
    app: AppHandle<R>,
    status: String,
    payload: String,
) -> Result<(), String> {
    app.tenwa_auth_status_update(status, payload)
}

#[tauri::command]
pub async fn get_engine_status<R: Runtime>(
    app: AppHandle<R>,
) -> Result<serde_json::Value, String> {
    app.tenwa_get_status()
}

#[tauri::command]
pub async fn save_config_val<R: Runtime>(
    app: AppHandle<R>,
    key: String,
    value: String,
) -> Result<(), String> {
    app.tenwa_save_config_val(key, value)
}

#[tauri::command]
pub async fn send_message<R: Runtime>(
    app: AppHandle<R>,
    phone: String,
    message: String,
) -> Result<(), String> {
    app.tenwa_send_message(phone, message)
}

#[tauri::command]
pub async fn send_message_with_media<R: Runtime>(
    app: AppHandle<R>,
    phone: String,
    message: String,
    media_base64: String,
    mime_type: String,
    file_name: String,
) -> Result<(), String> {
    app.tenwa_send_media(phone, message, media_base64, mime_type, file_name)
}

#[tauri::command]
pub async fn logout_whatsapp<R: Runtime>(
    app: AppHandle<R>,
) -> Result<(), String> {
    app.tenwa_logout()
}
