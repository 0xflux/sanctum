//! Driver service controls

use std::ptr::null_mut;
use shared_std::driver_manager::DriverState;
use windows::{
    core::{Error, PCWSTR},
    Win32::{
        Foundation::{
            GetLastError, ERROR_DUPLICATE_SERVICE_NAME, ERROR_SERVICE_EXISTS,
            GENERIC_READ, GENERIC_WRITE,
        },
        Storage::FileSystem::{
            CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_NONE, OPEN_EXISTING
        },
        System::
            Services::{
                CloseServiceHandle, ControlService, CreateServiceW, DeleteService, OpenSCManagerW,
                OpenServiceW, StartServiceW, SC_HANDLE, SC_MANAGER_ALL_ACCESS, SERVICE_ALL_ACCESS,
                SERVICE_CONTROL_STOP, SERVICE_DEMAND_START, SERVICE_ERROR_NORMAL,
                SERVICE_KERNEL_DRIVER, SERVICE_STATUS,
            },
    },
};


use crate::{driver_manager::DriverHandleRaii, utils::log::{Log, LogLevel}};

use super::driver_manager::SanctumDriverManager;
impl SanctumDriverManager {
    /// Command for the driver manager to install the driver on the target device.
    ///
    /// # Panics
    ///
    /// This function will panic if it was unable to open the service manager or install the driver
    /// in most cases. ERROR_SERVICE_EXISTS, ERROR_DUPLICATE_SERVICE_NAME will not panic.
    pub fn install_driver(&mut self) {
        //
        // Create a new ScDbMgr to hold the handle of the result of the OpenSCManagerW call.
        //
        let mut sc_mgr = ServiceControlManager::new();
        sc_mgr.open_service_manager_w(SC_MANAGER_ALL_ACCESS);

        //
        // Install the driver on the device
        //
        let handle = unsafe {
            match CreateServiceW(
                sc_mgr.mgr_handle.unwrap(),
                PCWSTR::from_raw(self.svc_name.as_ptr()), // service name
                PCWSTR::from_raw(self.svc_name.as_ptr()), // display name
                SERVICE_ALL_ACCESS,
                SERVICE_KERNEL_DRIVER,
                SERVICE_DEMAND_START,
                SERVICE_ERROR_NORMAL,
                PCWSTR::from_raw(self.svc_path.as_ptr()),
                None,
                None,
                None,
                None,
                None,
            ) {
                Ok(h) => {
                    if h.is_invalid() {
                        let msg = format!("Handle returned is invalid when attempting to install the service. Error code: {:?}", GetLastError());
                        self.update_state_msg(msg);
                    }

                    h
                }
                Err(e) => {
                    let le = GetLastError();

                    match le {
                        ERROR_DUPLICATE_SERVICE_NAME => {
                            let msg = format!(
                                "Unable to create service, duplicate service name found."
                            );
                            self.update_state_msg(msg);
                            return;
                        }
                        ERROR_SERVICE_EXISTS => {
                            self.state = DriverState::Installed("".to_string());
                            return;
                        }
                        _ => {
                            // anything else
                            let msg = format!(
                                "Unable to create service. Error: {e}. Svc path: {}",
                                String::from_utf16_lossy(self.svc_path.as_slice())
                            );
                            self.update_state_msg(msg);
                            return;
                        }
                    } // close match last err
                }
            } // close match handle result
        };

        self.log.log(LogLevel::Success, "Driver successfully installed");
        self.state = DriverState::Installed("".to_string());

        //
        // At this point, we should have the handle, and we can close it.
        //

        if !handle.is_invalid() {
            if let Err(e) = unsafe { CloseServiceHandle(handle) } {
                self.log.log(LogLevel::Error, &format!("[-] Unable to close handle after installing service. Error: {e}"));
            }
        }
    }


    /// Updates the state in place without modifying the actual state of the driver, but allows for passing
    /// an error string back to the GUI, this way, if the driver manager encounters an error, but this error
    /// doesn't change the state of the driver, we can communicate this to the user without altering the state.
    /// 
    /// Only use this in cases where the state doesn't change to something new, but you wish to emit a string. Usually an
    /// error message.
    fn update_state_msg(&mut self, new_message: String) {
        match self.state {
            DriverState::Uninstalled(ref mut msg) |
            DriverState::Installed(ref mut msg) |
            DriverState::Started(ref mut msg) |
            DriverState::Stopped(ref mut msg) => {
                *msg = new_message;
            }
        };
    }

    /// Start the driver.
    ///
    /// # Panics
    ///
    /// Function will panic if it cannot open a handle to the SC Manager
    pub fn start_driver(&mut self) {

        //
        // Create a new ScDbMgr to hold the handle of the result of the OpenSCManagerW call.
        //
        let mut sc_mgr = ServiceControlManager::new();
        sc_mgr.open_service_manager_w(SC_MANAGER_ALL_ACCESS);

        // get a handle to sanctum service
        if let Err(e) = sc_mgr.get_handle_to_sanctum_svc(self) {
            self.log.log(LogLevel::Error, &format!(
                "Failed to get handle to the Sanctum service when attempting to start it. {e}"
            ));
            let msg = format!("Failed to get handle to the Sanctum service when attempting to start it {}.", e);
            self.state = DriverState::Stopped(msg);
            return;
        }

        unsafe {
            if let Err(e) = StartServiceW(sc_mgr.sanctum_handle.unwrap(), None) {
                self.log.log(LogLevel::Error, &format!(
                    "[-] Failed to start service. {e}. Handle: {:?}.",
                    sc_mgr.mgr_handle.unwrap()
                ));
                
                let msg = format!("Failed to start service. {e}.");
                self.state = DriverState::Stopped(msg);
                return;
            };
        };

        // try to get a handle now the driver has started
        self.init_handle_via_registry();

        // check the driver version is compatible with the engine
        if self.ioctl_check_driver_compatibility() == false {
            self.stop_driver(); // ensure a clean shutdown
            let msg = format!("Driver and client version incompatible. Please ensure you are running the latest version.");
            self.state = DriverState::Stopped(msg);
            return;
        }

        self.state = DriverState::Started("".to_string());

        self.log.log(LogLevel::Success, "Driver started successfully");
    }


    /// Stop the driver
    ///
    /// # Panics
    ///
    /// Function will panic if it cannot open a handle to the SC Manager
    pub fn stop_driver(&mut self) {
        let mut sc_mgr = ServiceControlManager::new();
        sc_mgr.open_service_manager_w(SC_MANAGER_ALL_ACCESS);

        // get a handle to sanctum service
        if let Err(e) = sc_mgr.get_handle_to_sanctum_svc(self) {
            self.log.log(LogLevel::Error, &format!("Failed to get handle to the Sanctum service when attempting to start it. {e}"));
            let msg = format!("Failed to get handle to the Sanctum service when attempting to start it. {e}");
            self.update_state_msg(msg);
            return;
        }

        let mut service_status = SERVICE_STATUS::default();

        if let Err(e) = unsafe {
            ControlService(
                sc_mgr.sanctum_handle.unwrap(),
                SERVICE_CONTROL_STOP,
                &mut service_status,
            )
        } {
            // if was error
            self.log.log(LogLevel::Error, &format!("Failed to stop the service, {e}. Handle: {:?}", sc_mgr.mgr_handle.unwrap()));
            let msg = format!("Failed to stop the service, {e}");
            self.update_state_msg(msg);
            return;
        }

        // if we were successful, delete our local reference to the driver handle
        // todo - possible bug here, making the handle None if there was an error
        // maybe some form of IOCTL conversation to make sure unload is unloading..?
        self.handle_via_path = DriverHandleRaii::default(); // drop will be invoked closing the handle

        self.state = DriverState::Stopped("".to_string());

        self.log.log(LogLevel::Success, "Driver stopped successfully");
    }


    /// Uninstall the driver.
    ///
    /// # Panics
    ///
    /// Function will panic if it cannot open a handle to the SC Manager
    pub fn uninstall_driver(&mut self) {
        let mut sc_mgr = ServiceControlManager::new();
        sc_mgr.open_service_manager_w(SC_MANAGER_ALL_ACCESS);

        // get a handle to sanctum service
        if let Err(e) = sc_mgr.get_handle_to_sanctum_svc(self) {
            self.log.log(LogLevel::Error, &format!("Failed to get handle to the Sanctum service. {e}"));
            let msg = format!("Failed to get handle to the Sanctum service. {e}");
            self.update_state_msg(msg);
            return;
        }

        if let Err(e) = unsafe { DeleteService(sc_mgr.sanctum_handle.unwrap()) } {
            self.log.log(LogLevel::Error, &format!("[-] Failed to uninstall the driver: {e}. Handle: {:?}", sc_mgr.mgr_handle.unwrap()));
            let msg = format!("Failed to uninstall the driver: {e}");
            self.update_state_msg(msg);
            return;
        }

        self.state = DriverState::Uninstalled("".to_string());
        
        self.log.log(LogLevel::Success, "Driver uninstalled successfully");
    }


    /// Gets a handle to the driver via its registry path using CreateFileW. This function
    /// may silently fail if the driver is not installed, or there is some other error.
    ///
    /// If unsuccessful, the handle field will be None; otherwise it will be Some(handle). The handle is managed
    /// by Rust's RAII Drop trait so no requirement to manually close the handle.
    ///
    /// todo better error handling for this fn.
    pub fn init_handle_via_registry(&mut self) -> bool {
        let filename = PCWSTR::from_raw(self.device_um_symbolic_link_name.as_ptr());
        let handle = unsafe {
            CreateFileW(
                filename,
                GENERIC_READ.0 | GENERIC_WRITE.0,
                FILE_SHARE_NONE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
        };

        match handle {
            Ok(h) => self.handle_via_path.handle = Some(h),
            Err(e) => {
                self.log.log(LogLevel::Error, &format!("Unable to get handle to driver via its registry path, error: {e}."));
                return false;
            }
        }

        true
    }
}


/// A custom struct to hold a SC_HANDLE. This struct implements the drop trait so that
/// when it goes out of scope, it will clean up its handle so you do not need to remember
/// to call CloseServiceHandle.
struct ServiceControlManager {
    mgr_handle: Option<SC_HANDLE>,
    sanctum_handle: Option<SC_HANDLE>,
}

impl ServiceControlManager {
    /// Establishes a connection to the service control manager on the computer and opens the specified 
    /// service control manager database.
    ///
    /// # Panics
    ///
    /// If the call to OpenServiceManagerW fails, this will panic.
    fn open_service_manager_w(&mut self, dw_desired_access: u32) {
        self.mgr_handle = unsafe {
            match OpenSCManagerW(None, None, dw_desired_access) {
                Ok(h) => Some(h),
                Err(e) => panic!("[-] Unable to open service manager handle, {e}."),
            }
        }
    }

    /// Attempt to obtain a handle to the Sanctum service. If this is successful the function returns
    /// a Result<()>, and the field sanctum_handle is given the value of the handle.
    ///
    /// The handle will automatically be closed when it goes out of scope as it is implemented in the
    /// drop trait.
    fn get_handle_to_sanctum_svc(
        &mut self,
        driver_manager: &SanctumDriverManager,
    ) -> Result<(), Error> {
        let driver_handle = unsafe {
            OpenServiceW(
                self.mgr_handle.unwrap(),
                PCWSTR::from_raw(driver_manager.svc_name.as_ptr()),
                SERVICE_ALL_ACCESS,
            )
        }?;

        self.sanctum_handle = Some(driver_handle);

        // we return nothing, as the field sanctum_handle is set on success
        Ok(())
    }

    /// Instantiates the ServiceInterface with a null handle.
    fn new() -> ServiceControlManager {
        ServiceControlManager {
            mgr_handle: None,
            sanctum_handle: None,
        }
    }
}

impl Drop for ServiceControlManager {
    /// Automatically close the service handle if it is valid
    fn drop(&mut self) {
        //
        // Close the handle for the SC DB
        //
        if self.mgr_handle.is_none() {
            return;
        }

        let log = Log::init();

        if self.mgr_handle.unwrap().0 != null_mut() {
            if let Err(e) = unsafe { CloseServiceHandle(self.mgr_handle.unwrap()) } {
                log.log(LogLevel::Error, &format!("Unable to close handle after installing service. Error: {e}."));
            }
            self.mgr_handle = None;
        } else {
            log.log(LogLevel::Error, "Unable to close handle, handle was null");
        }

        //
        // Close the handle to the sanctum driver
        //
        if self.sanctum_handle.is_none() {
            return;
        }

        if self.sanctum_handle.unwrap().0 != null_mut() {
            if let Err(e) = unsafe { CloseServiceHandle(self.sanctum_handle.unwrap()) } {
                log.log(LogLevel::Error, &format!("Unable to close handle after installing service. Error: {e}."));
            }
            self.sanctum_handle = None;
        } else {
            log.log(LogLevel::Error, "Unable to close handle, handle was null");
        }
    }
}