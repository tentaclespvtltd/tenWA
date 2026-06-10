import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export interface WhatsAppStatus {
  status: 'Offline' | 'QR' | 'CONNECTED' | 'authenticated' | string;
  payload: string;
  started: boolean;
}

/**
 * Spawns the background WhatsApp Web spooled browser engine.
 * @param visible Whether to show the browser window (default: false)
 */
export async function openWhatsApp(visible?: boolean): Promise<void> {
  await invoke('plugin:tenwa|open_whatsapp', { visible });
}

/**
 * Fetches the current connection and initialization status of the engine.
 */
export async function getWhatsAppStatus(): Promise<WhatsAppStatus> {
  return await invoke<WhatsAppStatus>('plugin:tenwa|get_engine_status');
}

/**
 * Forces the webview to check and emit the QR code payload.
 */
export async function getQr(): Promise<void> {
  await invoke('plugin:tenwa|get_qr');
}

/**
 * Saves a key-value configuration pair to local config.json.
 */
export async function saveConfigVal(key: string, value: string): Promise<void> {
  await invoke('plugin:tenwa|save_config_val', { key, value });
}

/**
 * Sends a plain text message to a phone number.
 * @param phone Recipient phone number (country code prefix, e.g. "919876543210")
 * @param message Message content
 */
export async function sendWhatsAppMessage(phone: string, message: string): Promise<void> {
  const cleanPhone = phone.replace(/\D/g, '');
  await invoke('plugin:tenwa|send_message', { phone: cleanPhone, message });
}

/**
 * Sends a media message (images, videos, documents) to a phone number.
 * @param phone Recipient phone number
 * @param message Caption message
 * @param mediaBase64 Raw base64 data (without "data:...base64,")
 * @param mimeType Mimetype (e.g. "image/png", "application/pdf")
 * @param fileName Target filename
 */
export async function sendWhatsAppMedia(
  phone: string,
  message: string,
  mediaBase64: string,
  mimeType: string,
  fileName: string
): Promise<void> {
  const cleanPhone = phone.replace(/\D/g, '');
  await invoke('plugin:tenwa|send_message_with_media', {
    phone: cleanPhone,
    message,
    mediaBase64,
    mimeType,
    fileName,
  });
}

/**
 * Logs out and unlinks the current WhatsApp Web session.
 */
export async function logoutWhatsApp(): Promise<void> {
  await invoke('plugin:tenwa|logout_whatsapp');
}

/**
 * Listens to real-time authentication and status updates (such as QR codes).
 * @param callback Callback fired with the status and raw payload
 * @returns Promise resolving to an unlisten cleanup function
 */
export async function onWhatsAppStatusChange(
  callback: (status: string, payload: string) => void
): Promise<() => void> {
  const unlisten = await listen<{ status: string; payload: string }>('auth_status', (event) => {
    callback(event.payload.status, event.payload.payload);
  });
  return unlisten;
}

/**
 * Listens specifically to raw WhatsApp Web QR code string updates.
 * @param callback Callback fired with the raw QR code string
 * @returns Promise resolving to an unlisten cleanup function
 */
export async function onWhatsAppQRChange(
  callback: (qrCode: string) => void
): Promise<() => void> {
  const unlisten = await listen<{ status: string; payload: string }>('auth_status', (event) => {
    if (event.payload.status === 'QR' && event.payload.payload) {
      callback(event.payload.payload);
    }
  });
  return unlisten;
}
