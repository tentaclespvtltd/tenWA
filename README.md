# tenWA Tauri Plugin

A headless WhatsApp Web integration plugin for Tauri v2. It allows launching a WhatsApp Web window, automating interactions (sending text/media), and tracking login/QR status from your frontend without imposing any specific UI.

---

## Installation

### 1. Add to Crate Dependencies
Add the dependency to your `src-tauri/Cargo.toml` dependencies.

#### Option A: CLI Command (Recommended & Easiest)
Run the following command from the root directory of your Tauri project:
```bash
cargo add tauri-plugin-tenwa --git https://github.com/tentaclespvtltd/tenWA.git --manifest-path src-tauri/Cargo.toml
```

#### Option B: Manual Cargo.toml configuration
Add the following line to your `src-tauri/Cargo.toml` under `[dependencies]`:
```toml
tauri-plugin-tenwa = { git = "https://github.com/tentaclespvtltd/tenWA.git" }
```

#### Option C: Local Path Dependency (For development)
If you have cloned the plugin files locally into your project (e.g. inside `src-tauri/plugins/ten-wa`):
```toml
[dependencies]
tauri-plugin-tenwa = { path = "plugins/ten-wa" }
```

### 2. Register the Plugin in Rust
In your main application entry point (e.g. `src-tauri/src/main.rs`), initialize and register the plugin in the Tauri builder:

```rust
fn main() {
    tauri::Builder::default()
        // Register the tenWA plugin
        .plugin(tauri_plugin_tenwa::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 3. Configure Permissions
To allow frontend calls to the plugin commands, define a grouped permission (e.g. `tenWA-allow`) in your app's `src-tauri/permissions/default.toml`:

```toml
[[permission]]
identifier = "tenWA-allow"
description = "Enables all tenWA plugin commands"
permissions = [
  "tenWA:allow-open-whatsapp",
  "tenWA:allow-auth-status-update",
  "tenWA:allow-send-message",
  "tenWA:allow-send-message-with-media",
  "tenWA:allow-logout-whatsapp",
  "tenWA:allow-get-engine-status",
  "tenWA:allow-save-config-val"
]
```

Then add `"tenWA-allow"` to the permissions list in your app capabilities configuration (e.g. `src-tauri/capabilities/default.json`):

```json
{
  "permissions": [
    "core:default",
    "tenWA-allow"
  ]
}
```

---

## Frontend Integration Options

Use the following JavaScript/TypeScript patterns to integrate `tenWA` into your frontend UI.

### 1. Listening to Status & QR Code Updates (Real-Time)
Listen to the `auth_status` event which updates in real-time as the engine detects changes on the WhatsApp Web page.

```typescript
import { listen } from "@tauri-apps/api/event";

// Listen to auth status updates
const unlisten = await listen("auth_status", (event) => {
  const { status, payload } = event.payload as { status: string; payload: string };
  
  if (status === "QR" && payload) {
    // payload is the raw WhatsApp Web QR code string (e.g., to render via canvas or QRCode generator)
    console.log("New QR Code:", payload);
  } else if (status === "CONNECTED") {
    console.log("WhatsApp Web is connected and authenticated.");
  } else {
    // Other statuses: Offline, PAIRING, TIMEOUT, etc.
    console.log("Auth State Changed:", status);
  }
});

// To clean up the listener later (e.g., inside useEffect cleanup):
unlisten();
```

### 2. Checking Current Engine Status Manually
Fetch the current status of the engine (e.g. upon app initialization or page load) to sync state.

```typescript
import { invoke } from "@tauri-apps/api/core";

interface EngineStatus {
  status: string;    // "CONNECTED", "QR", "Offline", etc.
  payload: string;   // QR code data ref if status is "QR", otherwise empty
  started: boolean;  // Whether the engine is active
}

async function checkStatus() {
  try {
    const status: EngineStatus = await invoke("plugin:tenWA|get_engine_status");
    console.log("Current status:", status);
  } catch (error) {
    console.error("Failed to fetch engine status:", error);
  }
}
```

### 3. Starting the Engine (Opening WhatsApp Web Window)
Launches the headless/spooled WhatsApp web window.

```typescript
import { invoke } from "@tauri-apps/api/core";

async function startEngine(showWindow: boolean = false) {
  try {
    // Set visible to true to show the WhatsApp Web window, or false to keep it headless
    await invoke("plugin:tenWA|open_whatsapp", { visible: showWindow });
    console.log("WhatsApp engine successfully initialized");
  } catch (error) {
    console.error("Failed to start engine:", error);
  }
}
```

### 4. Logging Out (Disconnecting)
Triggers standard logout workflows, invalidates the session WebSocket, and closes the browser window after unlinking.

```typescript
import { invoke } from "@tauri-apps/api/core";

async function logout() {
  try {
    await invoke("plugin:tenWA|logout_whatsapp");
    console.log("Successfully logged out and session unlinked");
  } catch (error) {
    console.error("Failed to log out:", error);
  }
}
```

### 5. Sending Text Messages
Send a plain text message to a phone number.

```typescript
import { invoke } from "@tauri-apps/api/core";

async function sendMessage(phoneNumber: string, message: string) {
  try {
    // phoneNumber: "1234567890" (do not include "@c.us" - the backend appends it)
    await invoke("plugin:tenWA|send_message", {
      phone: phoneNumber,
      message: message
    });
    console.log("Message sent successfully");
  } catch (error) {
    console.error("Failed to send message:", error);
  }
}
```

### 6. Sending Media Messages (Images/Videos/Documents)
Send a file via base64 encoding to a phone number.

```typescript
import { invoke } from "@tauri-apps/api/core";

async function sendMediaMessage(
  phoneNumber: string,
  caption: string,
  base64Data: string,
  mimeType: string,
  fileName: string
) {
  try {
    // base64Data: raw base64 data string (e.g. without "data:image/png;base64,")
    await invoke("plugin:tenWA|send_message_with_media", {
      phone: phoneNumber,
      message: caption,
      mediaBase64: base64Data,
      mimeType: mimeType,
      fileName: fileName
    });
    console.log("Media message sent successfully");
  } catch (error) {
    console.error("Failed to send media:", error);
  }
}
```

### 7. Saving Configuration Value
Store engine configuration key-values (e.g. country codes, auto-start delay parameters).

```typescript
import { invoke } from "@tauri-apps/api/core";

async function saveConfig(key: string, value: string) {
  try {
    await invoke("plugin:tenWA|save_config_val", { key, value });
    console.log(`Config ${key} saved successfully`);
  } catch (error) {
    console.error("Failed to save config:", error);
  }
}
```
