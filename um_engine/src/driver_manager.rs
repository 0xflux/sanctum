use std::ptr::null_mut;

use shared::constants::{DEVICE_NAME_PATH, SVC_NAME, SYMBOLIC_NAME_PATH, SYS_INSTALL_RELATIVE_LOC};
use windows::{core::{Error, PCWSTR}, Win32::{self, Foundation::{GetLastError, ERROR_ACCESS_DENIED, ERROR_DUPLICATE_SERVICE_NAME, ERROR_SERVICE_EXISTS, UNICODE_STRING}, System::Services::{CloseServiceHandle, ControlService, CreateServiceW, DeleteService, OpenSCManagerW, OpenServiceW, StartServiceW, SC_HANDLE, SC_MANAGER_ALL_ACCESS, SERVICE_ALL_ACCESS, SERVICE_CONTROL_STOP, SERVICE_DEMAND_START, SERVICE_ERROR_NORMAL, SERVICE_KERNEL_DRIVER, SERVICE_STATUS}}};

use crate::strings::{ToUnicodeString, ToWindowsUnicodeString};


/// The SanctumDriverManager holds key information to be shared between
/// modules which relates to uniquely identifiable attributes such as its name 
/// and other critical settings.
pub struct SanctumDriverManager {
    pub device_name_path: UNICODE_STRING,
    pub symbolic_link: UNICODE_STRING,
    svc_path: PCWSTR,
    svc_name: PCWSTR,
}

impl SanctumDriverManager {
    /// Generate a new instance of the driver manager, which initialises the device name path and symbolic link path
    pub fn new() -> SanctumDriverManager {

        // 
        // Generate the UNICODE_STRING values for the device and symbolic name
        //
        let device_name_path = DEVICE_NAME_PATH.to_u16_vec().to_windows_unicode_string().unwrap();
        let symbolic_link = SYMBOLIC_NAME_PATH.to_u16_vec().to_windows_unicode_string().unwrap();
        let install_path = SYS_INSTALL_RELATIVE_LOC.to_u16_vec();
        let svc_name = SVC_NAME.to_u16_vec();


        SanctumDriverManager {
            device_name_path,
            symbolic_link,
            svc_path: PCWSTR::from_raw(install_path.as_ptr()),
            svc_name: PCWSTR::from_raw(svc_name.as_ptr()),
        }
    }
    

    /// Command for the driver manager to install the driver on the target device.
    /// 
    /// # Panics
    /// 
    /// This function will panic if it was unable to open the service manager or install the driver
    /// in most cases. ERROR_SERVICE_EXISTS, ERROR_DUPLICATE_SERVICE_NAME will not panic.
    pub fn install_driver(&self, driver_manager: &SanctumDriverManager) {
        
        // 
        // Create a new ScDbMgr to hold the handle of the result of the OpenSCManagerW call.
        //
        let mut sc_mgr = ServiceInterface::new();
        sc_mgr.open_service_manager_w(SC_MANAGER_ALL_ACCESS);

        //
        // Install the driver on the device
        // 
        let handle = unsafe {
            match CreateServiceW(
                sc_mgr.sc_db_handle.unwrap(), 
                driver_manager.svc_name,  // service name
                driver_manager.svc_name,  // display name
                SERVICE_ALL_ACCESS, 
                SERVICE_KERNEL_DRIVER, 
                SERVICE_DEMAND_START, 
                SERVICE_ERROR_NORMAL, 
                driver_manager.svc_path,
                None, 
                None, 
                None, 
                None, 
                None,
            ) {
                Ok(h) => h,
                Err(e) => {
                    let le = GetLastError();

                    match le {
                        ERROR_DUPLICATE_SERVICE_NAME => {
                            eprintln!("[-] Unable to create service, duplicate service name found.");
                            return;
                        },
                        ERROR_SERVICE_EXISTS => {
                            eprintln!("[-] Unable to create service, service already exists.");
                            return;
                        }
                        _ => {
                            // anything else
                            panic!("[-] Unable to create service. Error: {e}");
                        }
                    } // close match last err

                },
            } // close match handle result
        };

        println!("[+] Driver successfully installed.");

        //
        // At this point, we should have the handle, and we can close it.
        //

        if !handle.is_invalid() {
            if let Err(e) = unsafe { CloseServiceHandle(handle) } {
                eprintln!("[-] Unable to close handle after installing service. Error: {e}");
            }
        }
    }


    /// Start the driver.
    /// 
    /// # Panics
    /// 
    /// Function will panic if it cannot open a handle to the SC Manager
    pub fn start_driver(&self, driver_manager: &SanctumDriverManager) {
        // 
        // Create a new ScDbMgr to hold the handle of the result of the OpenSCManagerW call.
        //
        let mut sc_mgr = ServiceInterface::new();
        sc_mgr.open_service_manager_w(SC_MANAGER_ALL_ACCESS);

        // get a handle to sanctum service
        if let Err(e) = sc_mgr.get_handle_to_sanctum_svc(driver_manager) {
            eprintln!("[-] Failed to get handle to the Sanctum service when attempting to start it. {e}");
            return;
        }

        unsafe {
            if let Err(e) = StartServiceW(
                sc_mgr.sanctum_handle.unwrap(), 
                None,
            ){
                eprintln!("[-] Failed to start service. {e}. Handle: {:?}", sc_mgr.sc_db_handle.unwrap());
            };
        };

        println!("[+] Driver started successfully.");
    }


    /// Stop the driver
    /// 
    /// # Panics
    /// 
    /// Function will panic if it cannot open a handle to the SC Manager
    pub fn stop_driver(&self, driver_manager: &SanctumDriverManager) {
        let mut sc_mgr = ServiceInterface::new();
        sc_mgr.open_service_manager_w(SC_MANAGER_ALL_ACCESS);

        // get a handle to sanctum service
        if let Err(e) = sc_mgr.get_handle_to_sanctum_svc(driver_manager) {
            eprintln!("[-] Failed to get handle to the Sanctum service when attempting to start it. {e}");
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
            eprintln!("[-] Failed to stop the service, {e}. Handle: {:?}", sc_mgr.sc_db_handle.unwrap());
        }

        println!("[+] Driver stopped successfully.");

    }


    /// Uninstall the driver.
    ///
    /// # Panics
    /// 
    /// Function will panic if it cannot open a handle to the SC Manager
    pub fn uninstall_driver(&self, driver_manager: &SanctumDriverManager) {
        let mut sc_mgr = ServiceInterface::new();
        sc_mgr.open_service_manager_w(SC_MANAGER_ALL_ACCESS);

        // get a handle to sanctum service
        if let Err(e) = sc_mgr.get_handle_to_sanctum_svc(driver_manager) {
            eprintln!("[-] Failed to get handle to the Sanctum service when attempting to start it. {e}");
            return;
        }

        if let Err(e) = unsafe { DeleteService(sc_mgr.sanctum_handle.unwrap())} {
            eprintln!("[-] Failed to uninstall the driver: {e}. Handle: {:?}", sc_mgr.sc_db_handle.unwrap());
        }

        println!("[+] Driver uninstalled successfully.");
    }
}


/// A custom struct to hold a SC_HANDLE. This struct implements the drop trait so that
/// when it goes out of scope, it will clean up its handle so you do not need to remember 
/// to call CloseServiceHandle.
struct ServiceInterface { 
    sc_db_handle: Option<SC_HANDLE>,
    sanctum_handle: Option<SC_HANDLE>,
}

impl ServiceInterface {
    /// Open a handle to the SC Manager, storing the resulting handle in the instance.
    /// 
    /// # Panics
    /// 
    /// If the call to OpenServiceManagerW fails, this will panic.
    fn open_service_manager_w(
        &mut self,
        dw_desired_access: u32,
    ) {
        self.sc_db_handle = unsafe {
            match OpenSCManagerW(
               None, 
               None, 
               dw_desired_access,
           ) {
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
    fn get_handle_to_sanctum_svc(&mut self, driver_manager: &SanctumDriverManager) -> Result<(), Error> {
        let driver_handle = unsafe { OpenServiceW(
            self.sc_db_handle.unwrap(), 
            driver_manager.svc_name, 
            SERVICE_ALL_ACCESS) 
        }?;

        self.sanctum_handle = Some(driver_handle);

        // we return nothing, as the field sanctum_handle is set on success
        Ok(())
       }


    /// Instantiates the ServiceInterface with a null handle.
    fn new() -> ServiceInterface {
        ServiceInterface { 
            sc_db_handle: None,
            sanctum_handle: None,
        }
    }
}

impl Drop for ServiceInterface {
    /// Automatically close the service handle if it is valid
    fn drop(&mut self) {
        //
        // Close the handle for the SC DB
        //
        if self.sc_db_handle.is_none() {
            return;
        }

        if self.sc_db_handle.unwrap().0 != null_mut() {
            if let Err(e) = unsafe { CloseServiceHandle(self.sc_db_handle.unwrap()) } {
                eprintln!("[-] Unable to close handle after installing service. Error: {e}");
            }
            self.sc_db_handle = None;
            println!("[i] Service manager handle dropped.");
        } else {
            eprintln!("[-] Unable to close handle, handle was null!");
        }


        //
        // Close the handle to the sanctum driver
        //
        if self.sanctum_handle.is_none() {
            return;
        }

        if self.sanctum_handle.unwrap().0 != null_mut() {
            if let Err(e) = unsafe { CloseServiceHandle(self.sanctum_handle.unwrap()) } {
                eprintln!("[-] Unable to close handle after installing service. Error: {e}");
            }
            self.sanctum_handle = None;
            println!("[i] Sanctum driver handle dropped.");
        } else {
            eprintln!("[-] Unable to close handle, handle was null!");
        }
    }
}