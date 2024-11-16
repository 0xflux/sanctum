use core::{ffi::c_void, ptr::null_mut};

use alloc::string::{String, ToString};
use serde_json::to_value;
use shared_no_std::{constants::{SanctumVersion, PIPE_NAME}, ioctl::SancIoctlPing, ipc::CommandRequest};
use wdk::{nt_success, println};
use wdk_sys::{ntddk::{RtlCopyMemoryNonTemporal, ZwCreateFile, ZwWriteFile}, FILE_ATTRIBUTE_NORMAL, FILE_OPEN, FILE_SHARE_WRITE, FILE_SYNCHRONOUS_IO_ALERT, FILE_SYNCHRONOUS_IO_NONALERT, GENERIC_WRITE, HANDLE, HANDLE_PTR, IO_STACK_LOCATION, IO_STATUS_BLOCK, NTSTATUS, OBJECT_ATTRIBUTES, PIRP, STATUS_BUFFER_ALL_ZEROS, STATUS_BUFFER_TOO_SMALL, STATUS_SUCCESS, STATUS_UNSUCCESSFUL, _IO_STACK_LOCATION, _REG_NOTIFY_CLASS::RegNtPreUnLoadKey};

use crate::{ffi::InitializeObjectAttributes, utils::{check_driver_version, ToUnicodeString}};

struct IoctlBuffer {
    len: u32,
    buf: *mut c_void,
    p_stack_location: *mut _IO_STACK_LOCATION,
    pirp: PIRP,
}


impl IoctlBuffer {

    /// Creates a new instance of the IOCTL buffer type
    fn new(
        p_stack_location: *mut _IO_STACK_LOCATION,
        pirp: PIRP
    ) -> Self {
        IoctlBuffer {
            len: 0,
            buf: null_mut(),
            p_stack_location,
            pirp
        }
    }

    /// Converts the input buffer from the IO Manager into a valid utf8 string.
    fn get_buf_to_str(
        &mut self,
    ) -> Result<&str, NTSTATUS> {

        // first initialise the fields with buf and len
        self.receive()?;

        // construct the message from the pointer (ascii &[u8])
        let input_buffer = unsafe {core::slice::from_raw_parts(self.buf as *const u8, self.len as usize)};
        if input_buffer.is_empty() { 
            println!("[sanctum] [-] Error reading string passed to PING IOCTL");
            return Err(STATUS_UNSUCCESSFUL);
        }

        let input_buffer = core::str::from_utf8(input_buffer).unwrap();

        // this does not result in a dangling reference as we are referring to memory owned by Self, we are returning 
        // a slice of that memory.
        Ok(input_buffer)
    }

    /// Receives raw data from the IO Manager and checks the validity of the data. If the data was valid, it will set the member 
    /// fields for the length, buffer, and raw pointers to the required structs. 
    /// 
    /// If you want to get a string out of an ioctl buffer, it would be better to call get_buf_to_str.
    /// 
    /// # Returns
    /// 
    /// Success: a IoctlBuffer which will hold the length and a pointer to the buffer
    /// 
    /// Error: NTSTATUS
    fn receive(
        &mut self,
    ) -> Result<(), NTSTATUS> {
    
        // length of in buffer
        let input_len: u32 = unsafe {(*self.p_stack_location).Parameters.DeviceIoControl.InputBufferLength};
        if input_len == 0 { 
            println!("[sanctum] [-] IOCTL PING input length invalid.");
            return Err(STATUS_BUFFER_TOO_SMALL) 
        };
    
        // For METHOD_BUFFERED, the driver should use the buffer pointed to by Irp->AssociatedIrp.SystemBuffer as the output buffer.
        let input_buffer: *mut c_void = unsafe {(*self.pirp).AssociatedIrp.SystemBuffer};
        if input_buffer.is_null() { 
            println!("[sanctum] [-] Input buffer is null.");
            return Err(STATUS_BUFFER_ALL_ZEROS) 
        };
    
        // validate the pointer
        if input_buffer.is_null() {
            println!("[sanctum] [-] IOCTL input buffer was null.");
            return Err(STATUS_UNSUCCESSFUL);
        }
    
        self.len = input_len;
        self.buf = input_buffer;
    
        Ok(())
    }


    /// Sends a str slice &[u8] back to the userland application taking in a &str and making 
    /// the necessary conversions.
    /// 
    /// # Returns
    /// 
    /// Success: ()
    /// 
    /// Error: NTSTATUS
    fn send_str(
        &self,
        input_str: &str,
    ) -> Result<(), NTSTATUS> {

        // handled the request successfully
        unsafe {(*self.pirp).IoStatus.__bindgen_anon_1.Status = STATUS_SUCCESS};

        // response back to userland
        let response = input_str.as_bytes();
        let response_len = response.len();
        unsafe {(*self.pirp).IoStatus.Information = response_len as u64};

        println!("[sanctum] [i] Sending back to userland {:?}", core::str::from_utf8(response).unwrap());

        // Copy the data now into the buffer to send back to usermode.
        // The driver should not write directly to the buffer pointed to by Irp->UserBuffer.
        unsafe {
            if !(*self.pirp).AssociatedIrp.SystemBuffer.is_null() {
                RtlCopyMemoryNonTemporal((*self.pirp).AssociatedIrp.SystemBuffer as *mut c_void, response as *const _ as *mut c_void, response_len as u64);
            } else {
                println!("[sanctum] [-] Error handling IOCTL PING, SystemBuffer was null.");
                return Err(STATUS_UNSUCCESSFUL);
            }
        }

        Ok(())
    }
}

/// Simple IOCTL test ping from usermode
pub fn ioctl_handler_ping(
    p_stack_location: *mut _IO_STACK_LOCATION,
    pirp: PIRP,
) -> Result<(), NTSTATUS> {

    let mut ioctl_buffer = IoctlBuffer::new(p_stack_location, pirp);
    // ioctl_buffer.receive()?;

    let input_buffer = ioctl_buffer.get_buf_to_str()?;
    println!("[sanctum] [+] Input buffer: {:?}", input_buffer);

    // send a str response back to userland
    ioctl_buffer.send_str("Msg received!")?;

    Ok(())
}


pub fn ioctl_handler_ping_return_struct(
    p_stack_location: *mut _IO_STACK_LOCATION,
    pirp: PIRP,
) -> Result<(), NTSTATUS> {

    let mut ioctl_buffer = IoctlBuffer::new(p_stack_location, pirp);
    ioctl_buffer.receive()?; // receive the data

    let input_data = ioctl_buffer.buf as *mut c_void as *mut SancIoctlPing;
    if input_data.is_null() {
        println!("[sanctum] [-] Input struct data in IOCTL PING with struct was null.");
    }

    let input_data = unsafe { &(*input_data) };

    // construct the input str from the array
    let input_str = unsafe { core::slice::from_raw_parts(input_data.version.as_ptr() as *const u8, input_data.str_len) };
    let input_str = match core::str::from_utf8(input_str) {
        Ok(v) => v,
        Err(e) => {
            println!("[sanctum] [-] Error converting input slice to string. {e}");
            return Err(STATUS_UNSUCCESSFUL);
        },
    };

    println!("[sanctum] [+] Input bool: {}, input str: {:#?}", input_data.received, input_str);

    // setup output 
    let msg = b"Msg received from the Kernel!";
    let mut out_buf = SancIoctlPing::new(); 

    if msg.len() > out_buf.capacity {
        println!("[sanctum] [-] Message too large to send back to usermode.");
        return Err(STATUS_UNSUCCESSFUL);
    }

    out_buf.received = true;
    out_buf.version[..msg.len()].copy_from_slice(msg);
    out_buf.str_len = msg.len();

    unsafe { 
        if (*pirp).AssociatedIrp.SystemBuffer.is_null() {
            println!("[sanctum] [-] SystemBuffer is a null pointer.");
            return Err(STATUS_UNSUCCESSFUL);
        }
    }
    let size_of_struct = core::mem::size_of_val(&out_buf) as u64;
    unsafe {(*pirp).IoStatus.Information = size_of_struct};

    unsafe {
        RtlCopyMemoryNonTemporal((*pirp).AssociatedIrp.SystemBuffer, &out_buf as *const _ as *const c_void, size_of_struct)
    };

    Ok(())
}


/// Checks the compatibility of the driver version with client version. For all intents and purposes this can be 
/// considered the real 'ping' with the current pings being POC for passing data between UM and KM.
pub fn ioctl_check_driver_compatibility(
    p_stack_location: *mut _IO_STACK_LOCATION,
    pirp: PIRP,
) -> Result<(), NTSTATUS> {

    let mut ioctl_buffer = IoctlBuffer::new(p_stack_location, pirp);
    ioctl_buffer.receive()?; // receive the data

    let input_data = ioctl_buffer.buf as *const _ as *const SanctumVersion;
    if input_data.is_null() {
        println!("[sanctum] [-] Error receiving input data for checking driver compatibility.");
        return Err(STATUS_UNSUCCESSFUL);
    }

    // validated the pointer, data should be safe to dereference
    let input_data: &SanctumVersion = unsafe {&*input_data};

    // check whether we are compatible
    let response = check_driver_version(input_data);
    println!("[sanctum] [i] Client version: {}.{}.{}, is compatible with driver version: {}.", input_data.major, input_data.minor, input_data.patch, response);

    // prepare the data
    let res_size = core::mem::size_of_val(&response) as u64;
    unsafe { (*pirp).IoStatus.Information = res_size };

    unsafe {
        RtlCopyMemoryNonTemporal((*pirp).AssociatedIrp.SystemBuffer, &response as *const bool as *const c_void, res_size);
    }

    Ok(())
}

/// Send a message to the usermode engine via its named pipe
pub fn send_msg_via_named_pipe<A>(named_pipe_msg: &str, args: Option<&A>) 
    where A: serde::Serialize {

    println!("[sanctum] [i] About to create named pipe");

    let mut file_handle: *mut HANDLE = null_mut();
    let mut object_attributes = OBJECT_ATTRIBUTES::default();
    let mut pipe_name = PIPE_NAME.clone().to_unicode_string().unwrap();
    let mut oa_handle: HANDLE = null_mut();
    let mut io_status: *mut IO_STATUS_BLOCK = null_mut();

    unsafe { InitializeObjectAttributes(
        &mut object_attributes,
        &mut pipe_name,
        0,
        oa_handle,
        null_mut(),
    ) };

    println!("[sanctum] [i] InitializeObjectAttributes done");

    let status = unsafe { ZwCreateFile(
        file_handle,
        GENERIC_WRITE,
        &mut object_attributes,
        io_status,
        null_mut(),
        FILE_ATTRIBUTE_NORMAL,
        FILE_SHARE_WRITE,
        FILE_OPEN,
        FILE_SYNCHRONOUS_IO_NONALERT,
        null_mut(),
        0,
    ) };

    println!("[sanctum] [i] Status of ZwCreateFile: {status}");
    println!("[sanctum] [i] IO Status: {}", unsafe {(*io_status).__bindgen_anon_1.Status});

    if !nt_success(status) {
        return;
    }

    //
    // We now have a handle to the pipe, so write to the file
    //

    // serialise the args
    let args = match args {
        Some(a) => Some(to_value(a).unwrap()),
        None => None,
    };

    let command = CommandRequest {
        command: named_pipe_msg.to_string(),
        args: args,
    };

    let command_length = size_of_val(&command);

    let status = unsafe { ZwWriteFile(
        file_handle, 
        null_mut(), 
        null_mut(), 
        null_mut(), 
        io_status, 
        command, 
        command_length,
        0, 
        null_mut(),
    ) };

    // let mut bytes_written = 0;
    // let status = unsafe { ZwWriteFile(
    //     file_handle,
    //     None,
    //     None,
    //     None,
    //     &mut IO_STATUS_BLOCK::default(),
    //     message.as_ptr() as _,
    //     message.len() as u32,
    //     None,
    //     None,
    // ) };

    // ZwClose(handle);

}