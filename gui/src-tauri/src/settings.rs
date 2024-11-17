use serde_json::Value;
use shared_std::settings::SanctumSettings;

use crate::ipc::IpcClient;

#[tauri::command]
pub async fn settings_load_page_state() -> Result<String, ()> {

    match IpcClient::send_ipc::<SanctumSettings, Option<Value>>("settings_load_page_state", None).await {
        Ok(response) => {
            return Ok(
                serde_json::to_string(&response).unwrap()
            )
        },
        Err(e) => {
            eprintln!("[-] Error with IPC: {e}");
            return Ok("IPC error".to_string()); // todo proper error handling
        },
    };
}


#[tauri::command]
pub async fn settings_update_settings(settings: String) -> Result<String, ()> {

    let settings: SanctumSettings = serde_json::from_str(&settings).unwrap();

    match IpcClient::send_ipc::<String, _>("settings_update_settings", Some(settings)).await {
        Ok(response) => {
            return Ok(
                serde_json::to_string(&response).unwrap()
            )
        },
        Err(e) => {
            eprintln!("[-] Error with IPC: {e}");
            return Ok("IPC error".to_string()); // todo proper error handling
        },
    };
}