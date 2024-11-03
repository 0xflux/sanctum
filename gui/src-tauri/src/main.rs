//! Sanctum Tauri based GUI which will allow the user to interact with the kernel and usermode application.

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#[allow(non_snake_case)]

mod antivirus;
mod settings;

use std::sync::Arc;
use antivirus::{scanner_check_page_state, scanner_get_scan_stats, scanner_start_folder_scan, scanner_stop_scan, scanner_start_quick_scan};
use settings::{settings_load_page_state, settings_update_settings};
use um_engine::{SanctumSettings, UmEngine};

#[tokio::main]
async fn main() {

	// the usermode engine will be used as a singleton
	let um_engine = Arc::new(UmEngine::new(SanctumSettings::load()));
	
	tauri::Builder::default()
	.manage(um_engine)
		.invoke_handler(tauri::generate_handler![
			scanner_start_folder_scan, 
			scanner_check_page_state,
			scanner_stop_scan,
			scanner_get_scan_stats,
			scanner_start_quick_scan,
			settings_load_page_state,
			settings_update_settings,
		])
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}