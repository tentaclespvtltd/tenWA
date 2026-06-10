use tauri::{
    plugin::{Builder, TauriPlugin},
    AppHandle, Emitter, Manager, Runtime, WebviewUrl, WebviewWindowBuilder,
};
use std::sync::Mutex;

pub mod config;
pub mod state;
pub mod commands;
mod injections;

use state::EngineState;

const QR_OBSERVER: &str = r#"
(function() {
    let lastQrData = null;
    let lastStatus = null;

    function tryNotify() {
        const invoke = window.__TAURI_INTERNALS__?.invoke;
        if (!invoke) return;

        // Find the QR canvas reliably
        const qrCanvas = document.querySelector('canvas[aria-label="Scan me!"]') || document.querySelector('div[data-ref] canvas') || document.querySelector('canvas');
        
        // Ensure the canvas is actually the QR code (typically it's the only canvas, or a large one)
        if (qrCanvas && qrCanvas.width > 100) {
            const dataUrl = qrCanvas.toDataURL('image/png');
            if (dataUrl && dataUrl !== lastQrData) {
                lastQrData = dataUrl;
                invoke('plugin:tenwa|auth_status_update', { status: 'QR', payload: dataUrl });
            }
        } else {
            let state = window.AuthStore?.AppState?.state;
            
            // DOM FALLBACK
            if (!state) {
                const chatList = document.querySelector('div[aria-label="Chat list"]');
                const searchBox = document.querySelector('div[title="Search input textbox"]');
                if (chatList || searchBox) {
                    state = 'CONNECTED';
                }
            }

            if (state && state !== lastStatus) {
                lastStatus = state;
                invoke('plugin:tenwa|auth_status_update', { status: state, payload: '' });
            }
        }
    }

    setInterval(tryNotify, 1000);
})();
"#;

pub trait TenwaExt<R: Runtime> {
    fn tenwa_open(&self, visible: Option<bool>) -> Result<(), String>;
    fn tenwa_auth_status_update(&self, status: String, payload: String) -> Result<(), String>;
    fn tenwa_get_status(&self) -> Result<serde_json::Value, String>;
    fn tenwa_save_config_val(&self, key: String, value: String) -> Result<(), String>;
    fn tenwa_send_message(&self, phone: String, message: String) -> Result<(), String>;
    fn tenwa_send_media(
        &self,
        phone: String,
        message: String,
        media_base64: String,
        mime_type: String,
        file_name: String,
    ) -> Result<(), String>;
    fn tenwa_logout(&self) -> Result<(), String>;
}

impl<R: Runtime> TenwaExt<R> for AppHandle<R> {
    fn tenwa_open(&self, visible: Option<bool>) -> Result<(), String> {
        let state = self.state::<Mutex<EngineState>>();
        {
            let mut s = state.lock().map_err(|e| e.to_string())?;
            s.started = true;
            s.status = "Starting...".to_string();
        }

        let is_visible = visible.unwrap_or(false);

        if let Some(window) = self.get_webview_window("whatsapp") {
            if is_visible {
                let _ = window.show();
                let _ = window.set_focus();
            }
            return Ok(());
        }

        let (os_agent, js_platform) = if cfg!(target_os = "windows") {
            ("Windows NT 10.0; Win64; x64", "Win32")
        } else if cfg!(target_os = "macos") {
            ("Macintosh; Intel Mac OS X 10_15_7", "MacIntel")
        } else {
            ("X11; Linux x86_64", "Linux x86_64")
        };

        let user_agent = format!(
            "Mozilla/5.0 ({}) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/129.0.0.0 Safari/537.36",
            os_agent
        );

        let spoof_script = format!(
            r#"
            (() => {{
                try {{
                    Object.defineProperty(navigator, 'userAgent', {{
                        get: () => "{}"
                    }});
                    Object.defineProperty(navigator, 'platform', {{
                        get: () => "{}"
                    }});
                    Object.defineProperty(navigator, 'vendor', {{
                        get: () => "Google Inc."
                    }});
                }} catch(e) {{
                    console.error("Failed to spoof navigator properties", e);
                }}
            }})();
            "#,
            user_agent, js_platform
        );

        WebviewWindowBuilder::new(
            self,
            "whatsapp",
            WebviewUrl::External("https://web.whatsapp.com".parse().unwrap()),
        )
        .title("WhatsApp Web Engine")
        .inner_size(1000.0, 800.0)
        .user_agent(&user_agent)
        .visible(is_visible)
        .initialization_script(&spoof_script)
        .initialization_script(injections::EXPOSE_AUTH_STORE)
        .initialization_script(injections::LOAD_UTILS)
        .initialization_script(QR_OBSERVER)
        .build()
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    fn tenwa_auth_status_update(&self, status: String, payload: String) -> Result<(), String> {
        let state = self.state::<Mutex<EngineState>>();
        {
            let mut s = state.lock().map_err(|e| e.to_string())?;
            s.status = status.clone();
            s.payload = payload.clone();
            s.started = true;
        }
        self.emit("auth_status", serde_json::json!({
            "status": status,
            "payload": payload
        })).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn tenwa_get_status(&self) -> Result<serde_json::Value, String> {
        let state = self.state::<Mutex<EngineState>>();
        let s = state.lock().map_err(|e| e.to_string())?;
        Ok(serde_json::json!({
            "status": s.status,
            "payload": s.payload,
            "started": s.started,
        }))
    }

    fn tenwa_save_config_val(&self, key: String, value: String) -> Result<(), String> {
        config::save_config_val(self, key, value)
    }

    fn tenwa_send_message(&self, phone: String, message: String) -> Result<(), String> {
        let window = self
            .get_webview_window("whatsapp")
            .ok_or("WhatsApp window not found. Please open it first.")?;

        let js_code = format!(
            r#"
            (async () => {{
                if (window.WWebJS) {{
                    const chat = await window.WWebJS.getChat("{}@c.us", {{ getAsModel: false }});
                    if (chat) {{
                        await window.WWebJS.sendMessage(chat, "{}", {{}});
                    }} else {{
                        console.error("Chat not found");
                    }}
                }} else {{
                    console.error("WWebJS not loaded");
                }}
            }})();
            "#,
            phone,
            message.replace("\"", "\\\"").replace("\n", "\\n")
        );

        window.eval(&js_code).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn tenwa_send_media(
        &self,
        phone: String,
        message: String,
        media_base64: String,
        mime_type: String,
        file_name: String,
    ) -> Result<(), String> {
        let window = self
            .get_webview_window("whatsapp")
            .ok_or("WhatsApp window not found. Please open it first.")?;

        let js_code = format!(
            r#"
            (async () => {{
                if (window.WWebJS) {{
                    const chat = await window.WWebJS.getChat("{}@c.us", {{ getAsModel: false }});
                    if (chat) {{
                        const options = {{
                            caption: "{}",
                            media: {{
                                mimetype: "{}",
                                data: "{}",
                                filename: "{}"
                            }}
                        }};
                        await window.WWebJS.sendMessage(chat, "", options);
                    }} else {{
                        console.error("Chat not found");
                    }}
                }} else {{
                    console.error("WWebJS not loaded");
                }}
            }})();
            "#,
            phone,
            message.replace("\"", "\\\"").replace("\n", "\\n"),
            mime_type,
            media_base64,
            file_name
        );

        window.eval(&js_code).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn tenwa_logout(&self) -> Result<(), String> {
        let state = self.state::<Mutex<EngineState>>();
        {
            let mut s = state.lock().map_err(|e| e.to_string())?;
            s.status = "Offline".to_string();
            s.payload = "".to_string();
            s.started = false;
        }
        if let Some(window) = self.get_webview_window("whatsapp") {
            let js_code = r#"
                (async () => {
                    const tryLogout = async () => {
                        try {
                            if (window.AuthStore && window.AuthStore.AppState && typeof window.AuthStore.AppState.logout === 'function') {
                                await window.AuthStore.AppState.logout();
                                console.log("Logged out via window.AuthStore.AppState.logout()");
                                return true;
                            }
                        } catch(e) { console.error("AppState logout failed:", e); }

                        try {
                            const socket = window.require('WAWebSocketModel')?.Socket;
                            if (socket && typeof socket.logout === 'function') {
                                await socket.logout();
                                console.log("Logged out via WAWebSocketModel.Socket.logout()");
                                return true;
                            }
                        } catch(e) { console.error("Socket logout failed:", e); }

                        try {
                            if (window.AuthStore && window.AuthStore.Cmd && typeof window.AuthStore.Cmd.logout === 'function') {
                                await window.AuthStore.Cmd.logout();
                                console.log("Logged out via window.AuthStore.Cmd.logout()");
                                return true;
                            }
                        } catch(e) { console.error("AuthStore Cmd logout failed:", e); }

                        try {
                            const cmd = window.require('WAWebCmd')?.Cmd;
                            if (cmd && typeof cmd.logout === 'function') {
                                await cmd.logout();
                                console.log("Logged out via WAWebCmd.Cmd.logout()");
                                return true;
                            }
                        } catch(e) { console.error("WAWebCmd logout failed:", e); }

                        try {
                            const auth = window.require('WAWebAuth') || window.require('WAWebUserPrefsMeUser');
                            if (auth && typeof auth.logout === 'function') {
                                await auth.logout();
                                console.log("Logged out via WAWebAuth/WAWebUserPrefsMeUser.logout()");
                                return true;
                            }
                        } catch(e) { console.error("WAWebAuth logout failed:", e); }

                        return false;
                    };

                    const success = await tryLogout();
                    if (!success) {
                        console.error("All programmatic logout methods failed.");
                    }
                })();
            "#;
            window.eval(js_code).map_err(|e| e.to_string())?;

            let app_clone = self.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                if let Some(w) = app_clone.get_webview_window("whatsapp") {
                    let _ = w.close();
                }
            });
        }
        Ok(())
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("tenwa")
        .invoke_handler(tauri::generate_handler![
            commands::open_whatsapp,
            commands::auth_status_update,
            commands::get_engine_status,
            commands::save_config_val,
            commands::send_message,
            commands::send_message_with_media,
            commands::logout_whatsapp
        ])
        .setup(|app, _api| {
            app.manage(Mutex::new(EngineState {
                status: "Offline".to_string(),
                payload: "".to_string(),
                started: false,
            }));

            let app_handle = app.clone();
            let auto_start = config::read_config_val(&app_handle, "tenwa")
                .or_else(|| config::read_config_val(&app_handle, "tenWA"))
                .unwrap_or_default() == "true";
            let show_window = config::read_config_val(&app_handle, "show_whatsapp_window").unwrap_or_default() == "true";
            if auto_start {
                println!("Auto-starting WhatsApp Web engine in a background task...");
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = app_handle.tenwa_open(Some(show_window)) {
                        eprintln!("Failed to auto-start WhatsApp: {}", e);
                    }
                });
            }

            Ok(())
        })
        .build()
}
