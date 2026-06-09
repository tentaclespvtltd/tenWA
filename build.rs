const COMMANDS: &[&str] = &[
    "open_whatsapp",
    "auth_status_update",
    "get_engine_status",
    "save_config_val",
    "send_message",
    "send_message_with_media",
    "logout_whatsapp",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
