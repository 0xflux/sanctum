use std::sync::Arc;

use tauri::State;
use um_engine::UmEngine;

#[derive(serde::Serialize, serde::Deserialize)]
enum Response {
    Ok(String),
    Err(String),
}

/// Install the driver on the host machine
#[tauri::command]
pub async fn driver_install_driver(
    engine: State<'_, Arc<UmEngine>>,
) -> Result<String, ()> {  

    let state= engine.driver_install_driver();

    let state_string = serde_json::to_string(&state).unwrap();

    Ok(state_string)
}

/// Uninstall the driver on the host machine
#[tauri::command]
pub async fn driver_uninstall_driver(
    engine: State<'_, Arc<UmEngine>>,
) -> Result<String, ()> {  

    let state= engine.driver_uninstall_driver();

    let state_string = serde_json::to_string(&state).unwrap();

    Ok(state_string)
}


#[tauri::command]
pub async fn driver_start_driver(
    engine: State<'_, Arc<UmEngine>>,
) -> Result<String, ()> {

    let state = engine.driver_start_driver();

    let state_string = serde_json::to_string(&state).unwrap();
        
    Ok(state_string)
}


#[tauri::command]
pub async fn driver_stop_driver(
    engine: State<'_, Arc<UmEngine>>,
) -> Result<String, ()> {

    let state = engine.driver_stop_driver();

    let state_string = serde_json::to_string(&state).unwrap();
        
    Ok(state_string)
}


#[tauri::command]
pub async fn driver_check_state(
    engine: State<'_, Arc<UmEngine>>,
) -> Result<String, ()> {
    let state = engine.driver_get_state();

    let state_string = serde_json::to_string(&state).unwrap();
        
    Ok(state_string)
}