use std::sync::Arc;

use tauri::State;
use um_engine::UmEngine;

/// Install the driver on the host machine
#[tauri::command]
pub async fn driver_install_driver(
    engine: State<'_, Arc<UmEngine>>,
) -> Result<String, ()> {  

    let engine = Arc::clone(&engine);

    let state= engine.driver_install_driver();

    let state_string = serde_json::to_string(&state).unwrap();

    Ok(state_string)
}

/// Uninstall the driver on the host machine
#[tauri::command]
pub async fn driver_uninstall_driver(
    engine: State<'_, Arc<UmEngine>>,
) -> Result<String, ()> {  

    let engine = Arc::clone(&engine);

    let state= engine.driver_uninstall_driver();

    let state_string = serde_json::to_string(&state).unwrap();

    Ok(state_string)
}