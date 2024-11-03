use tauri::State;
use um_engine::{SanctumSettings, UmEngine};
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


#[tauri::command]
pub fn settings_update_settings(
    settings: String,
    engine: State<'_, Arc<UmEngine>>,
) -> Result<String, ()> {

    let engine = Arc::clone(&engine);

    let settings: SanctumSettings = serde_json::from_str(&settings).unwrap();

    println!("Received settings: {:?}", settings);

    engine.settings_update_settings(settings);

    // get the settings
    let settings_string = serde_json::to_string(&engine.sanctum_settings).unwrap();

    Ok(settings_string)
}