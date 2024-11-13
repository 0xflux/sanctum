//! Sanctum Tauri based GUI which will allow the user to interact with the kernel and usermode application.

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#[allow(non_snake_case)]

mod antivirus;
mod settings;
mod driver_controller;
mod ipc;

use antivirus::{scanner_check_page_state, scanner_get_scan_stats, scanner_start_folder_scan, scanner_stop_scan, scanner_start_quick_scan};
use driver_controller::{driver_check_state, driver_install_driver, driver_start_driver, driver_stop_driver, driver_uninstall_driver};
use settings::{settings_load_page_state, settings_update_settings};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {


	Ok(
		tauri::Builder::default()
			.invoke_handler(tauri::generate_handler![
				scanner_start_folder_scan, 
				scanner_check_page_state,
				scanner_stop_scan,
				scanner_get_scan_stats,
				scanner_start_quick_scan,
				settings_load_page_state,
				settings_update_settings,
				driver_install_driver,
				driver_uninstall_driver,
				driver_start_driver,
				driver_stop_driver,
				driver_check_state,
			])
			.run(tauri::generate_context!())
			.expect("error while running tauri application")
	)
}