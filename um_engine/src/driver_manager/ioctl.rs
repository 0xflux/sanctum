//! IOCTL functions for communicating with the driver from usermode.

use super::driver_manager::SanctumDriverManager;
use core::str;
use std::{ffi::c_void, slice::from_raw_parts};
use shared_no_std::{
    constants::VERSION_CLIENT,
    ioctl::{DriverMessages, SancIoctlPing, SANC_IOCTL_CHECK_COMPATIBILITY, SANC_IOCTL_DRIVER_GET_MESSAGES, SANC_IOCTL_DRIVER_GET_MESSAGE_LEN, SANC_IOCTL_PING, SANC_IOCTL_PING_WITH_STRUCT},
};
use windows::Win32::System::IO::DeviceIoControl;

impl SanctumDriverManager {
    /// Checks the driver compatibility between the driver and user mode applications. 
    /// 
    /// # Panics
    /// 
    /// This function will panic if it cannot obtain a handle to the driver to communicate with it.
    /// 
    /// # Returns
    /// 
    /// If they are not compatible the driver will return false, otherwise it will return true.
    pub(super) fn ioctl_check_driver_compatibility(&mut self) -> bool {
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

        response
    }

    /// Ping the driver from usermode
    pub fn ioctl_ping_driver(&mut self) -> String {
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
                return "".to_string();
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
            return "".to_string();
        }

        println!("[+] Driver IOCTL sent. Bytes returned: {bytes_returned}");

        // parse out the result
        if let Ok(response) = str::from_utf8(&response[..bytes_returned as usize]) {
            println!("[+] IOCTL - Bytes returned: {bytes_returned} response: {:#?}", response);
            return response.to_string();
        } else {
            println!("[-] Error parsing response as UTF-8");
            return "".to_string();
        }
    }


    /// Pings the driver with a struct as its message
    pub fn ioctl_get_driver_messages(&mut self) {
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

        // todo this will possibly quickly grow so it should recursively keep draining the messages from the kernel until
        // it has enough space, or to do this another way? maybe a pre-check to get the size before allocating?
        // the driver could store the data into a temp buffer until the next call which will then send the data of size x,
        // when it stores into the temp buffer it can reset the 'live' vec in the driver?

        let mut size_of_kernel_msg: usize = 0;
        let mut bytes_returned: u32 = 0;

        let result = unsafe {
            DeviceIoControl(
                self.handle_via_path.handle.unwrap(),
                SANC_IOCTL_DRIVER_GET_MESSAGE_LEN,
                None,
                0u32,
                Some(&mut size_of_kernel_msg as *mut _ as *mut _),
                size_of::<usize>() as u32,
                Some(&mut bytes_returned),
                None,
            )
        };
        if let Err(e) = result {
            eprintln!("[-] Error with calling SANC_IOCTL_DRIVER_GET_MESSAGE_LEN. {e}. Size of kernel msg: {}", size_of_kernel_msg);
            return;
        }

        if size_of_kernel_msg == 0 {
            return;
        }

        let mut response: Vec<u8> = vec![0; size_of_kernel_msg];
        let mut bytes_returned: u32 = 0;

        // attempt the call
        let result = unsafe {
            DeviceIoControl(
                self.handle_via_path.handle.unwrap(),
                SANC_IOCTL_DRIVER_GET_MESSAGES,
                None,
                0u32,
                Some(response.as_mut_ptr() as *mut c_void),
                size_of_kernel_msg as u32,
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

        let response_serialised = match serde_json::from_slice::<DriverMessages>(&response) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[-] Could not serialise response from driver messages. {e} Got: {:?}", response);
                return;
            },
        };

        println!("[i] Response serialised: {:?}", response_serialised);

        // todo something with the data

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
}