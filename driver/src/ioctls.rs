use core::{ffi::c_void, ptr::null_mut};

use wdk::println;
use wdk_sys::{ntddk::RtlCopyMemoryNonTemporal, DEVICE_OBJECT, NTSTATUS, PIRP, STATUS_BUFFER_ALL_ZEROS, STATUS_BUFFER_TOO_SMALL, STATUS_SUCCESS, STATUS_UNSUCCESSFUL, _IO_STACK_LOCATION};

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

    fn get_buf_to_str(
        &mut self,
    ) -> Result<&str, NTSTATUS> {

        // first initialise the fields with buf and len
        self.check_ioctl_buffer()?;

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

    /// Checks the validity of data coming in from an IOCTL handler ensuring data isn't null etc. 
    /// 
    /// If you want to get a string out of an ioctl buffer, it would be better to call get_buf_to_str.
    /// 
    /// # Returns
    /// 
    /// Success: a IoctlBuffer which will hold the length and a pointer to the buffer
    /// 
    /// Error: NTSTATUS
    fn check_ioctl_buffer(
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
}

pub fn ioctl_handler_ping(
    _device: *mut DEVICE_OBJECT, 
    p_stack_location: *mut _IO_STACK_LOCATION,
    pirp: PIRP,
) -> Result<(), NTSTATUS> {

    let mut ioctl_buffer = IoctlBuffer::new(p_stack_location, pirp);
    // ioctl_buffer.check_ioctl_buffer()?;

    let input_buffer = ioctl_buffer.get_buf_to_str()?;
    println!("[sanctum] [+] Input buffer: {:?}", input_buffer);

    // handled the request successfully
    unsafe {(*pirp).IoStatus.__bindgen_anon_1.Status = STATUS_SUCCESS};

    // response back to userland
    let response = "Msg received!".as_bytes();
    let response_len = response.len();
    unsafe {(*pirp).IoStatus.Information = response_len as u64};

    println!("[sanctum] [i] Sending back to userland {:?}", core::str::from_utf8(response).unwrap());

    // Copy the data now into the buffer to send back to usermode.
    // The driver should not write directly to the buffer pointed to by Irp->UserBuffer.
    unsafe {
        if !(*pirp).AssociatedIrp.SystemBuffer.is_null() {
            RtlCopyMemoryNonTemporal((*pirp).AssociatedIrp.SystemBuffer as *mut c_void, response as *const _ as *mut c_void, response_len as u64);
        } else {
            println!("[sanctum] [-] Error handling IOCTL PING, SystemBuffer was null.");
            return Err(STATUS_UNSUCCESSFUL);
        }
    }

    Ok(())
}