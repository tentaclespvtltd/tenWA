use tauri::{AppHandle, Manager, Runtime};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

pub fn get_config_path<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.path().app_local_data_dir().ok().map(|p| p.join("config.json"))
}

pub fn read_config_val<R: Runtime>(app: &AppHandle<R>, key: &str) -> Option<String> {
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

pub fn save_config_val<R: Runtime>(
    app: &AppHandle<R>,
    key: String,
    value: String,
) -> Result<(), String> {
    let path = get_config_path(app).ok_or("Failed to get config path")?;
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
