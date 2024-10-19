// Sanctum Windows Kernel Mode Driver (WDM) written in Rust
// Date: 12/10/2024
// Author: flux
//      GH: https://github.com/0xflux
//      Blog: https://fluxsec.red/

#![no_std]
extern crate alloc;

#[cfg(not(test))]
extern crate wdk_panic;

use core::{ffi::c_void, ptr::null_mut};

use ffi::{IoGetCurrentIrpStackLocation, RtlCopyMemory};
use shared::constants::{DOS_DEVICE_NAME, NT_DEVICE_NAME};
use utils::{ToUnicodeString, ToWindowsUnicodeString};
use wdk::{nt_success, println};
#[cfg(not(test))]
use wdk_alloc::WdkAllocator;

mod ffi;
mod utils;

use wdk_sys::{
    ntddk::{IoCreateDevice, IoCreateSymbolicLink, IoDeleteDevice, IoDeleteSymbolicLink, IofCompleteRequest, RtlCopyMemoryNonTemporal, RtlCopyString}, DEVICE_OBJECT, DRIVER_OBJECT, FILE_DEVICE_SECURE_OPEN, FILE_DEVICE_UNKNOWN, IO_NO_INCREMENT, IRP, IRP_MJ_CLOSE, IRP_MJ_CREATE, IRP_MJ_DEVICE_CONTROL, IRP_MJ_WRITE, NTSTATUS, PCUNICODE_STRING, PDEVICE_OBJECT, PIRP, PUNICODE_STRING, STATUS_BUFFER_ALL_ZEROS, STATUS_BUFFER_OVERFLOW, STATUS_SUCCESS, STATUS_UNSUCCESSFUL
};

#[cfg(not(test))]
#[global_allocator]
static GLOBAL_ALLOCATOR: WdkAllocator = WdkAllocator;

/// DriverEntry is required to start the driver, and acts as the main entrypoint
/// for our driver.
#[export_name = "DriverEntry"] // WDF expects a symbol with the name DriverEntry
pub unsafe extern "system" fn driver_entry(
    driver: &mut DRIVER_OBJECT,
    registry_path: PCUNICODE_STRING,
) -> NTSTATUS {
    println!("[sanctum] [i] Starting Sanctum driver...");

    // let mut driver_name = DRIVER_NAME.to_u16_vec().to_windows_unicode_string().unwrap();
    // let status = IoCreateDriver(
    //     &mut driver_name,
    //     Some(sanctum_entry),
    // );
    // if !nt_success(status) {
    //     println!("[sanctum] [-] Error with IoCreateDriver. Exiting");
    //     return status;
    // }

    let status = sanctum_entry(driver, registry_path as *mut _);

    status
}


/// This deals with setting up the driver and any callbacks / configurations required
/// for its operation and lifetime.
pub unsafe extern "C" fn sanctum_entry(
    driver: *mut DRIVER_OBJECT,
    _registry_path: PUNICODE_STRING,
) -> NTSTATUS {
    println!("[sanctum] [i] running sanctum_entry...");

    //
    // Configure the strings
    //
    let mut dos_name = DOS_DEVICE_NAME
        .to_u16_vec()
        .to_windows_unicode_string()
        .expect("[sanctum] [-] unable to encode string to unicode.");

    let mut nt_name = NT_DEVICE_NAME
        .to_u16_vec()
        .to_windows_unicode_string()
        .expect("[sanctum] [-] unable to encode string to unicode.");

    //
    // Create the device
    //
    let mut device_object: PDEVICE_OBJECT = null_mut();
    let res = IoCreateDevice(
        driver,
        0,
        &mut nt_name,
        FILE_DEVICE_UNKNOWN, // If a type of hardware does not match any of the defined types, specify a value of either FILE_DEVICE_UNKNOWN
        FILE_DEVICE_SECURE_OPEN,
        0,
        &mut device_object,
    );
    if !nt_success(res) {
        println!("[sanctum] [-] Unable to create device via IoCreateDevice. Failed with code: {res}.");
        return res;
    }

    //
    // Configure the drivers callbacks
    //
    (*driver).MajorFunction[IRP_MJ_CREATE as usize] = Some(sanctum_create_close); // todo can authenticate requests coming from x
    (*driver).MajorFunction[IRP_MJ_CLOSE as usize] = Some(sanctum_create_close);
    // (*driver).MajorFunction[IRP_MJ_WRITE as usize] = Some(handle_ioctl);
    (*driver).MajorFunction[IRP_MJ_DEVICE_CONTROL as usize] = Some(handle_ioctl);
    (*driver).DriverUnload = Some(driver_exit);

    //
    // Create the symbolic link
    //
    let res = IoCreateSymbolicLink(&mut dos_name, &mut nt_name);
    if res != 0 {
        println!("[sanctum] [-] Failed to create driver symbolic link. Error: {res}");

        driver_exit(driver); // cleanup any resources before returning
        return STATUS_UNSUCCESSFUL;
    }

    STATUS_SUCCESS
}

/// Driver unload functions when it is to exit.
///
/// # Safety
///
/// This function makes use of unsafe code.
extern "C" fn driver_exit(driver: *mut DRIVER_OBJECT) {

    // rm symbolic link
    let mut device_name = DOS_DEVICE_NAME
        .to_u16_vec()
        .to_windows_unicode_string()
        .expect("[sanctum] [-] unable to encode string to unicode.");
    let _ = unsafe { IoDeleteSymbolicLink(&mut device_name) };

    // delete the device
    unsafe { IoDeleteDevice((*driver).DeviceObject);}

    println!("[sanctum] driver unloaded successfully...");
}


unsafe extern "C" fn sanctum_create_close(_device: *mut DEVICE_OBJECT, pirp: PIRP) -> NTSTATUS {
    
    (*pirp).IoStatus.__bindgen_anon_1.Status = STATUS_SUCCESS;
    (*pirp).IoStatus.Information = 0;
    IofCompleteRequest(pirp, IO_NO_INCREMENT as i8);

    println!("[sanctum] [i] IRP received...");
    
    STATUS_SUCCESS
}

/// Device IOCTL input handler.
///
/// This function will process IOCTL commands as they come into the driver and executing the relevant actions.
///
/// # Arguments
///
/// - '_device': Unused
/// - 'irp': A pointer to the I/O request packet (IRP) that contains information about the request
unsafe extern "C" fn handle_ioctl(_device: *mut DEVICE_OBJECT, pirp: PIRP) -> NTSTATUS {
    let p_stack_location = IoGetCurrentIrpStackLocation(pirp);

    if p_stack_location.is_null() {
        println!("[sanctum] [-] Unable to get stack location for IRP.");
        return STATUS_UNSUCCESSFUL;
    }

    println!("[sanctum] [+] Found the stack location! {:p}", p_stack_location);

    let control_code = (*p_stack_location).Parameters.DeviceIoControl.IoControlCode; // IOCTL code
    let input_len = (*p_stack_location).Parameters.DeviceIoControl.InputBufferLength; // length of in buffer
    let input_buffer = (*p_stack_location).Parameters.DeviceIoControl.Type3InputBuffer; // ptr to the buffer

    // validate the pointer
    if input_buffer.is_null() {
        println!("[sanctum] [-] IOCTL input buffer was null.");
        return STATUS_UNSUCCESSFUL;
    }

    // construct the message from the pointer (ascii &[u8])
    let input_buffer = core::slice::from_raw_parts(input_buffer as *const u8, input_len as usize);
    let input_buffer = core::str::from_utf8(input_buffer).unwrap();
    println!("[sanctum] [+] Input buffer: {:?}, control code: {}", input_buffer, control_code);

    // handled the request successfully
    (*pirp).IoStatus.__bindgen_anon_1.Status = STATUS_SUCCESS;

    // response back to userland
    let response = "Msg received!".as_bytes();
    let response_len = response.len();
    (*pirp).IoStatus.Information = response_len as u64; // potentially unsafe for 32-bit

    println!("[sanctum] [i] Sending back to userland {:?}", core::str::from_utf8(response).unwrap());
    println!("[i] buffs: {:p}, {:p}", (*pirp).UserBuffer, response as *const _ as *mut c_void);

    // copy the message into the UserBuffer location
    // WARNING - SAFETY
    // The driver should not write directly to the buffer pointed to by Irp->UserBuffer
    // https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/ns-wdm-_irp
    // todo - update this to something more appropriate, writing to AssociatedIrp.SystemBuffer blue screens
    // the pointer is invalid..? works for now in this simple case :~)
    // this is a BAD example of how to do this, but for now (for my purposes), it is fine.
    RtlCopyMemoryNonTemporal((*pirp).UserBuffer as *mut c_void, response as *const _ as *mut c_void, response_len as u64);
    
    // complete the request
    IofCompleteRequest(pirp, IO_NO_INCREMENT as i8);
    
    // return success
    STATUS_SUCCESS
}