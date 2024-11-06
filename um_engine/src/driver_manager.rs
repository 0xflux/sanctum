use core::str;
use std::{cell::RefCell, ffi::c_void, ptr::null_mut, slice::from_raw_parts};

use shared::{
    constants::{DRIVER_UM_NAME, SANC_SYS_FILE_LOCATION, SVC_NAME, SYS_INSTALL_RELATIVE_LOC, VERSION_CLIENT},
    ioctl::{SancIoctlPing, SANC_IOCTL_CHECK_COMPATIBILITY, SANC_IOCTL_PING, SANC_IOCTL_PING_WITH_STRUCT},
};
use windows::{
    core::{Error, PCWSTR},
    Win32::{
        Foundation::{
            CloseHandle, GetLastError, ERROR_DUPLICATE_SERVICE_NAME, ERROR_SERVICE_EXISTS,
            GENERIC_READ, GENERIC_WRITE, HANDLE, MAX_PATH,
        },
        Storage::FileSystem::{
            CreateFileW, GetFileAttributesW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_NONE, INVALID_FILE_ATTRIBUTES, OPEN_EXISTING
        },
        System::{
            LibraryLoader::GetModuleFileNameW,
            Services::{
                CloseServiceHandle, ControlService, CreateServiceW, DeleteService, OpenSCManagerW,
                OpenServiceW, StartServiceW, SC_HANDLE, SC_MANAGER_ALL_ACCESS, SERVICE_ALL_ACCESS,
                SERVICE_CONTROL_STOP, SERVICE_DEMAND_START, SERVICE_ERROR_NORMAL,
                SERVICE_KERNEL_DRIVER, SERVICE_STATUS,
            },
            IO::DeviceIoControl,
        },
    },
};

use crate::strings::ToUnicodeString;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DriverState {
    Uninstalled,
    Installed,
    Started,
    Stopped,
}

/// The SanctumDriverManager holds key information to be shared between
/// modules which relates to uniquely identifiable attributes such as its name
/// and other critical settings.
/// 
/// # Safety
/// 
/// The structure implements Send and Sync for the Handle stored in DriverHandleRaii. This should be safe as all accesses to the driver handle
/// will live for the lifetime of the object. If the handle could be null, the wrapping Option **should** be None.
pub struct SanctumDriverManager {
    pub device_um_symbolic_link_name: Vec<u16>,
    svc_path: Vec<u16>,
    svc_name: Vec<u16>,
    pub handle_via_path: DriverHandleRaii,
    pub state: RefCell<DriverState>,
}

impl SanctumDriverManager {
    /// Generate a new instance of the driver manager, which initialises the device name path and symbolic link path
    pub fn new() -> SanctumDriverManager {
        //
        // Generate the UNICODE_STRING values for the device and symbolic name
        //
        // let device_name_path = DEVICE_NAME_PATH.to_u16_vec();
        let device_um_symbolic_link_name = DRIVER_UM_NAME.to_u16_vec();

        let svc_path = SANC_SYS_FILE_LOCATION.to_u16_vec();
        let svc_name = SVC_NAME.to_u16_vec();
        let path_as_str = String::from_utf16_lossy(&svc_path);

        // check the sys file exists
        // todo this eventually should be in the actual install directory under Windows
        let x = unsafe { GetFileAttributesW(PCWSTR::from_raw(svc_path.as_ptr())) };
        if x == INVALID_FILE_ATTRIBUTES {
            panic!("[-] Cannot find sys file. Err: {}. Expected at: {}", unsafe {
                GetLastError().0
            }, path_as_str);
        }

        let mut instance = SanctumDriverManager {
            device_um_symbolic_link_name,
            svc_path,
            svc_name,
            handle_via_path: DriverHandleRaii::default(), // set to None
            state: RefCell::new(DriverState::Stopped), // todo will need to check if is installed
        };

        // attempt to initialise a handle to the driver, this may silently fail - and will do so in the case
        // where the driver is not yet installed (or has been uninstalled)
        if instance.init_handle_via_registry() {
            *instance.state.borrow_mut() = DriverState::Started;
        }

        instance
    }


    /// Command for the driver manager to install the driver on the target device.
    ///
    /// # Panics
    ///
    /// This function will panic if it was unable to open the service manager or install the driver
    /// in most cases. ERROR_SERVICE_EXISTS, ERROR_DUPLICATE_SERVICE_NAME will not panic.
    pub fn install_driver(&self) {
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
                        panic!("[-] Handle returned is invalid when attempting to install the service. Last error: {:?}", GetLastError());
                    }

                    h
                }
                Err(e) => {
                    let le = GetLastError();

                    match le {
                        ERROR_DUPLICATE_SERVICE_NAME => {
                            eprintln!(
                                "[-] Unable to create service, duplicate service name found."
                            );
                            return;
                        }
                        ERROR_SERVICE_EXISTS => {
                            eprintln!("[-] Unable to create service, service already exists.");
                            return;
                        }
                        _ => {
                            // anything else
                            panic!(
                                "[-] Unable to create service. Error: {e}. Svc path: {}",
                                String::from_utf16_lossy(self.svc_path.as_slice())
                            );
                        }
                    } // close match last err
                }
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
    pub fn start_driver(&mut self) {
        //
        // Create a new ScDbMgr to hold the handle of the result of the OpenSCManagerW call.
        //
        let mut sc_mgr = ServiceControlManager::new();
        sc_mgr.open_service_manager_w(SC_MANAGER_ALL_ACCESS);

        // get a handle to sanctum service
        if let Err(e) = sc_mgr.get_handle_to_sanctum_svc(self) {
            eprintln!(
                "[-] Failed to get handle to the Sanctum service when attempting to start it. {e}"
            );
            return;
        }

        unsafe {
            if let Err(e) = StartServiceW(sc_mgr.sanctum_handle.unwrap(), None) {
                eprintln!(
                    "[-] Failed to start service. {e}. Handle: {:?}.",
                    sc_mgr.mgr_handle.unwrap()
                );
                return;
            };
        };

        // try to get a handle now the driver has started
        self.init_handle_via_registry();

        // check the driver version is compatible with the engine
        if self.ioctl_check_driver_compatibility() == false {
            self.stop_driver(); // ensure a clean shutdown
            // todo replace panic once GUI in
            panic!("[-] Driver and client version incompatible. Please ensure you are running the latest version.");
        }

        println!("[+] Driver started successfully.");
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
            eprintln!(
                "[-] Failed to get handle to the Sanctum service when attempting to start it. {e}"
            );
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
            eprintln!(
                "[-] Failed to stop the service, {e}. Handle: {:?}",
                sc_mgr.mgr_handle.unwrap()
            );
            return;
        }

        // if we were successful, delete our local reference to the driver handle
        // todo - possible bug here, making the handle None if there was an error
        // maybe some form of IOCTL conversation to make sure unload is unloading..?
        self.handle_via_path = DriverHandleRaii::default(); // drop will be invoked closing the handle

        println!("[+] Driver stopped successfully.");
    }


    /// Uninstall the driver.
    ///
    /// # Panics
    ///
    /// Function will panic if it cannot open a handle to the SC Manager
    pub fn uninstall_driver(&self) {
        let mut sc_mgr = ServiceControlManager::new();
        sc_mgr.open_service_manager_w(SC_MANAGER_ALL_ACCESS);

        // get a handle to sanctum service
        if let Err(e) = sc_mgr.get_handle_to_sanctum_svc(self) {
            eprintln!(
                "[-] Failed to get handle to the Sanctum service when attempting to start it. {e}"
            );
            return;
        }

        if let Err(e) = unsafe { DeleteService(sc_mgr.sanctum_handle.unwrap()) } {
            eprintln!(
                "[-] Failed to uninstall the driver: {e}. Handle: {:?}",
                sc_mgr.mgr_handle.unwrap()
            );
            return;
        }

        {
            *self.state.borrow_mut() = DriverState::Uninstalled;
        }
        
        println!("[+] Driver uninstalled successfully.");
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
                eprintln!("[-] Unable to get handle to driver via its registry path, error: {e}");
                return false;
            }
        }

        true
    }


    ///////////////////////////////////////////////////////////////////////////////////////////////////////////////
    /////////////////////////////////////////////////// IOCTLS ////////////////////////////////////////////////////
    ///////////////////////////////////////////////////////////////////////////////////////////////////////////////

    // All IOCTL functions should start with ioctl_

    /// Checks the driver compatibility between the driver and user mode applications. 
    /// 
    /// # Panics
    /// 
    /// This function will panic if it cannot obtain a handle to the driver to communicate with it.
    /// 
    /// # Returns
    /// 
    /// If they are not compatible the driver will return false, otherwise it will return true.
    fn ioctl_check_driver_compatibility(&mut self) -> bool {
        if self.handle_via_path.handle.is_none() {
            // try 1 more time
            self.init_handle_via_registry();
            if self.handle_via_path.handle.is_none() {
                eprintln!("[-] Handle to the driver is not initialised; please ensure you have started / installed the service. \
                    Unable to pass IOCTL. Handle: {:?}. Exiting the driver.", 
                    self.handle_via_path.handle
                );
                
                // stop the driver then panic
                self.stop_driver();

                // todo in the future have some gui option instead of a panic
                panic!("[-] Unable to communicate with the driver to check version compatibility, please try again.");
            }
        }

        let mut response: bool = false;
        let mut bytes_returned: u32 = 0;

        let result = unsafe {
            DeviceIoControl(
                self.handle_via_path.handle.unwrap(),
                SANC_IOCTL_CHECK_COMPATIBILITY,
                Some(&VERSION_CLIENT as *const _ as *const c_void),
                size_of_val(&VERSION_CLIENT) as u32,
                Some(&mut response as *mut _ as *mut c_void),
                size_of_val(&response) as u32,
                Some(&mut bytes_returned),
                None,
            )
        };

        // error checks
        if let Err(e) = result {
            eprintln!("[-] Error fetching version result from driver. {e}");
            return false;
        }
        if bytes_returned == 0 {
            eprintln!("[-] Error fetching version result from driver. Zero bytes returned from the driver.");
            return false;
        }

        println!("[i] Response is: {}", response);

        response
    }

    /// Ping the driver from usermode
    pub fn ioctl_ping_driver(&mut self) {
        //
        // Check the handle to the driver is valid, if not, attempt to initialise it.
        //

        // todo improve how the error handling happens..
        if self.handle_via_path.handle.is_none() {
            // try 1 more time
            self.init_handle_via_registry();
            if self.handle_via_path.handle.is_none() {
                eprintln!("[-] Handle to the driver is not initialised; please ensure you have started / installed the service. \
                    Unable to pass IOCTL. Handle: {:?}", 
                    self.handle_via_path.handle
                );
                return;
            }
        }

        //
        // If we have a handle
        //

        let message = "Hello world".as_bytes();
        const RESP_SIZE: u32 = 256; // todo
        let mut response: [u8; RESP_SIZE as usize] = [0; RESP_SIZE as usize]; // gets mutated in unsafe block
        let mut bytes_returned: u32 = 0;

        // attempt the call
        let result = unsafe {
            // todo implementation for WriteFile
            // WriteFile(
            //     self.handle_via_path.handle.unwrap(), 
            //     Some(message), 
            //     Some(&mut bytes_returned),
            //     None,
            // )
            DeviceIoControl(
                self.handle_via_path.handle.unwrap(),
                SANC_IOCTL_PING,
                Some(message.as_ptr() as *const _),
                message.len() as u32,
                Some(response.as_mut_ptr() as *mut c_void),
                RESP_SIZE,
                Some(&mut bytes_returned),
                None,
            )
        };

        if let Err(e) = result {
            eprintln!("Error from attempting IOCTL call. {e}");
            // no cleanup required, no additional handles or heap objects
            return;
        }

        println!("[+] Driver IOCTL sent. Bytes returned: {bytes_returned}");

        // parse out the result
        if let Ok(response) = str::from_utf8(&response[..bytes_returned as usize]) {
            println!(
                "[+] Bytes returned: {bytes_returned} response: {:#?}",
                response
            );
        } else {
            println!("[-] Error parsing response as UTF-8");
        }
    }


    /// Pings the driver with a struct as its message
    pub fn ioctl_ping_driver_w_struct(&mut self) {
        //
        // Check the handle to the driver is valid, if not, attempt to initialise it.
        //

        // todo improve how the error handling happens..
        if self.handle_via_path.handle.is_none() {
            // try 1 more time
            self.init_handle_via_registry();
            if self.handle_via_path.handle.is_none() {
                eprintln!("[-] Handle to the driver is not initialised; please ensure you have started / installed the service. \
                    Unable to pass IOCTL. Handle: {:?}", 
                    self.handle_via_path.handle
                );
                return;
            }
        }

        //
        // If we have a handle
        //
        let ver = "Hello from usermode!".as_bytes();        
        let mut message = SancIoctlPing::new();
        if ver.len() > message.capacity {
            eprintln!("[-] Message too long for buffer.");
            return;
        }

        // copy the message into the array
        message.version[..ver.len()].copy_from_slice(ver);
        message.str_len = ver.len();
        message.received = true;

        let mut response = SancIoctlPing::new();
        let mut bytes_returned: u32 = 0;

        // attempt the call
        let result = unsafe {
            DeviceIoControl(
                self.handle_via_path.handle.unwrap(),
                SANC_IOCTL_PING_WITH_STRUCT,
                Some(&message as *const _ as *const c_void),
                std::mem::size_of_val(&message) as u32,
                Some(&mut response as *mut _ as *mut c_void),
                std::mem::size_of_val(&response) as u32,
                Some(&mut bytes_returned),
                None,
            )
        };

        if let Err(e) = result {
            eprintln!("[-] Error from attempting IOCTL call. {e}");
            return;
        }

        // parse out the result
        if bytes_returned == 0 {
            eprintln!("[-] No bytes returned from DeviceIOControl");
            return;
        }

        let constructed = unsafe {from_raw_parts(response.version.as_ptr(), response.str_len)};

        println!("[+] Response from driver: {}, {:?}", response.received, std::str::from_utf8(constructed));

    }


    pub fn get_state(&self) -> DriverState {
        *self.state.borrow()
    }
}


impl Default for SanctumDriverManager {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Send for SanctumDriverManager {}
unsafe impl Sync for SanctumDriverManager {}

pub struct DriverHandleRaii {
    pub handle: Option<HANDLE>,
}

impl Default for DriverHandleRaii {
    fn default() -> Self {
        Self { handle: None }
    }
}


impl Drop for DriverHandleRaii {
    fn drop(&mut self) {
        if self.handle.is_some() && !self.handle.unwrap().is_invalid() {
            println!("[i] Dropping driver handle.");
            let _ = unsafe { CloseHandle(self.handle.unwrap()) };
            self.handle = None;
        }
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

        if self.mgr_handle.unwrap().0 != null_mut() {
            if let Err(e) = unsafe { CloseServiceHandle(self.mgr_handle.unwrap()) } {
                eprintln!("[-] Unable to close handle after installing service. Error: {e}");
            }
            self.mgr_handle = None;
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
        } else {
            eprintln!("[-] Unable to close handle, handle was null!");
        }
    }
}


/// Gets the path to the .sys file on the target device, for the time being this needs to be
/// located in the same folder as where this usermode exe is run from.
fn get_sys_file_path() -> Vec<u16> {
    //
    // A little long winded, but construct the path as a PCWSTR to where the sys driver is
    // this should be bundled into the same location as where the usermode exe is.
    //
    let mut svc_path: Vec<u16> = vec![0u16; MAX_PATH as usize];
    let len = unsafe { GetModuleFileNameW(None, &mut svc_path) };
    if len == 0 {
        eprintln!(
            "[-] Error getting path of module. Win32 Error: {}",
            unsafe { GetLastError().0 }
        );
    } else if len >= MAX_PATH {
        panic!("[-] Path of module is too long. Run from a location with a shorter path.");
    }

    svc_path.truncate(len as usize - 11); // remove um_engine.sys\0
    svc_path.append(&mut SYS_INSTALL_RELATIVE_LOC.to_u16_vec()); // append the .sys file

    svc_path
}
