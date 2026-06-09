use tauri::{
    plugin::{Builder, TauriPlugin},
    AppHandle, Emitter, Manager, Runtime, State, WebviewUrl, WebviewWindowBuilder,
};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Mutex;

mod injections;

pub struct EngineState {
    pub status: String,
    pub payload: String,
    pub started: bool,
}

fn get_config_path<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.path().app_local_data_dir().ok().map(|p| p.join("config.json"))
}

fn read_config_val<R: Runtime>(app: &AppHandle<R>, key: &str) -> Option<String> {
    let path = get_config_path(app)?;
    if !path.exists() {
        return None;
    }
    let mut file = File::open(path).ok()?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).ok()?;
    let json: serde_json::Value = serde_json::from_str(&contents).ok()?;
    json.get(key).and_then(|v| v.as_str().map(|s| s.to_string()))
}

const QR_OBSERVER: &str = r#"
(function() {
    let lastQr = null;
    let lastStatus = null;

    function tryNotify() {
        const invoke = window.__TAURI_INTERNALS__?.invoke;
        if (!invoke) return;

        const qrCanvas = document.querySelector('div[data-ref] canvas') || document.querySelector('canvas[aria-label="Scan me!"]');
        if (qrCanvas) {
            const container = qrCanvas.closest('[data-ref]');
            const dataRef = container?.getAttribute('data-ref');
            if (dataRef && dataRef !== lastQr) {
                lastQr = dataRef;
                invoke('plugin:tenWA|auth_status_update', { status: 'QR', payload: dataRef });
            }
        } else {
            const state = window.AuthStore?.AppState?.state;
            if (state && state !== lastStatus) {
                lastStatus = state;
                invoke('plugin:tenWA|auth_status_update', { status: state, payload: '' });
            }
        }
    }

    setInterval(tryNotify, 1500);
})();
"#;

#[tauri::command]
async fn open_whatsapp<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, Mutex<EngineState>>,
    visible: Option<bool>,
) -> Result<(), String> {
    {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        s.started = true;
    }

    if app.get_webview_window("whatsapp").is_some() {
        return Ok(());
    }

    let is_visible = visible.unwrap_or(false);

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
        &app,
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

#[tauri::command]
async fn auth_status_update<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, Mutex<EngineState>>,
    status: String,
    payload: String,
) -> Result<(), String> {
    {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        s.status = status.clone();
        s.payload = payload.clone();
        s.started = true;
    }
    app.emit("auth_status", serde_json::json!({
        "status": status,
        "payload": payload
    })).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn get_engine_status(
    state: State<'_, Mutex<EngineState>>,
) -> Result<serde_json::Value, String> {
    let s = state.lock().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "status": s.status,
        "payload": s.payload,
        "started": s.started,
    }))
}

#[tauri::command]
async fn save_config_val<R: Runtime>(
    app: AppHandle<R>,
    key: String,
    value: String,
) -> Result<(), String> {
    let path = get_config_path(&app).ok_or("Failed to get config path")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut json = if path.exists() {
        let mut file = File::open(&path).map_err(|e| e.to_string())?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|e| e.to_string())?;
        serde_json::from_str(&contents).unwrap_or(serde_json::Value::Object(serde_json::Map::new()))
    } else {
        serde_json::Value::Object(serde_json::Map::new())
    };
    
    if let Some(obj) = json.as_object_mut() {
        obj.insert(key, serde_json::Value::String(value));
    }
    
    let mut file = File::create(path).map_err(|e| e.to_string())?;
    let contents = serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?;
    file.write_all(contents.as_bytes()).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn send_message<R: Runtime>(
    app: AppHandle<R>,
    phone: String,
    message: String,
) -> Result<(), String> {
    let window = app
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

#[tauri::command]
async fn send_message_with_media<R: Runtime>(
    app: AppHandle<R>,
    phone: String,
    message: String,
    media_base64: String,
    mime_type: String,
    file_name: String,
) -> Result<(), String> {
    let window = app
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

#[tauri::command]
async fn logout_whatsapp<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, Mutex<EngineState>>,
) -> Result<(), String> {
    {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        s.status = "Offline".to_string();
        s.payload = "".to_string();
        s.started = false;
    }
    if let Some(window) = app.get_webview_window("whatsapp") {
        let js_code = r#"
            (async () => {
                const tryLogout = async () => {
                    // Try Socket/AppState logout (most standard for WA WebSocket)
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

                    // Try Cmd logout
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

                    // Try WAWebAuth / WAWebUserPrefsMeUser
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

        // Wait 3 seconds for unlinking network requests to complete, then close the webview
        let app_clone = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            if let Some(w) = app_clone.get_webview_window("whatsapp") {
                let _ = w.close();
            }
        });
    }
    Ok(())
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("tenWA")
        .invoke_handler(tauri::generate_handler![
            open_whatsapp,
            auth_status_update,
            get_engine_status,
            save_config_val,
            send_message,
            send_message_with_media,
            logout_whatsapp
        ])
        .setup(|app, _api| {
            // Manage internal state
            app.manage(Mutex::new(EngineState {
                status: "Offline".to_string(),
                payload: "".to_string(),
                started: false,
            }));

            // Execute auto-start task if tenWA config is enabled
            let app_handle = app.clone();
            let auto_start = read_config_val(&app_handle, "tenWA").unwrap_or_default() == "true";
            let show_window = read_config_val(&app_handle, "show_whatsapp_window").unwrap_or_default() == "true";
            if auto_start {
                println!("Auto-starting WhatsApp Web engine in a background task...");
                let app_handle_clone = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app_handle_clone.state::<Mutex<EngineState>>();
                    if let Err(e) = open_whatsapp(app_handle_clone.clone(), state, Some(show_window)).await {
                        eprintln!("Failed to auto-start WhatsApp: {}", e);
                    }
                });
            }

            Ok(())
        })
        .build()
}
