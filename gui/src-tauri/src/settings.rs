use tauri::State;
use um_engine::UmEngine;
use std::sync::Arc;

#[tauri::command]
pub fn settings_load_page_state(
    engine: State<'_, Arc<UmEngine>>,
) -> Result<String, ()> {

    let engine = Arc::clone(&engine);

    // get the settings
    let settings_string = serde_json::to_string(&engine.sanctum_settings).unwrap();

    Ok(settings_string)
}