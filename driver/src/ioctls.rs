use core::ffi::c_void;

use wdk::println;
use wdk_sys::{ntddk::RtlCopyMemoryNonTemporal, DEVICE_OBJECT, NTSTATUS, PIRP, STATUS_BUFFER_ALL_ZEROS, STATUS_BUFFER_TOO_SMALL, STATUS_SUCCESS, STATUS_UNSUCCESSFUL, _IO_STACK_LOCATION};

pub fn ioctl_handler_ping(
    _device: *mut DEVICE_OBJECT, 
    p_stack_location: *mut _IO_STACK_LOCATION,
    pirp: PIRP,
) -> Result<(), NTSTATUS> {

    // length of in buffer
    let input_len = unsafe {(*p_stack_location).Parameters.DeviceIoControl.InputBufferLength};
    if input_len == 0 { 
        println!("[sanctum] [-] IOCTL PING input length invalid.");
        return Err(STATUS_BUFFER_TOO_SMALL) 
    };

    // For METHOD_BUFFERED, the driver should use the buffer pointed to by Irp->AssociatedIrp.SystemBuffer as the output buffer.
    let input_buffer = unsafe {(*pirp).AssociatedIrp.SystemBuffer};
    if input_buffer.is_null() { 
        println!("[sanctum] [-] Input buffer is null.");
        return Err(STATUS_BUFFER_ALL_ZEROS) 
    };

    // validate the pointer
    if input_buffer.is_null() {
        println!("[sanctum] [-] IOCTL input buffer was null.");
        return Err(STATUS_UNSUCCESSFUL);
    }

    // construct the message from the pointer (ascii &[u8])
    let input_buffer = unsafe {core::slice::from_raw_parts(input_buffer as *const u8, input_len as usize)};
    if input_buffer.is_empty() { 
        println!("[sanctum] [-] Error reading string passed to PING IOCTL");
        return Err(STATUS_UNSUCCESSFUL);
    }

    let input_buffer = core::str::from_utf8(input_buffer).unwrap();
    println!("[sanctum] [+] Input buffer: {:?}", input_buffer);

    // 
    // At this point our input buffers are validated.
    //

    // handled the request successfully
    unsafe {(*pirp).IoStatus.__bindgen_anon_1.Status = STATUS_SUCCESS};

    // response back to userland
    let response = "Msg received!".as_bytes();
    let response_len = response.len();
    unsafe {(*pirp).IoStatus.Information = response_len as u64};

    println!("[sanctum] [i] Sending back to userland {:?}", core::str::from_utf8(response).unwrap());
    println!("[sanctum] [i] System buffer location: {:p}", unsafe {(*pirp).AssociatedIrp.SystemBuffer});

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