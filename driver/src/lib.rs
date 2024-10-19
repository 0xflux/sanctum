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

use ffi::IoGetCurrentIrpStackLocation;
use ioctls::ioctl_handler_ping;
use shared::{constants::{DOS_DEVICE_NAME, NT_DEVICE_NAME}, ioctl::SANC_IOCTL_PING};
use utils::{ToUnicodeString, ToWindowsUnicodeString};
use wdk::{nt_success, println};
#[cfg(not(test))]
use wdk_alloc::WdkAllocator;

mod ffi;
mod utils;
mod ioctls;

use wdk_sys::{
    ntddk::{IoCreateDevice, IoCreateSymbolicLink, IoDeleteDevice, IoDeleteSymbolicLink, IofCompleteRequest, RtlCopyMemoryNonTemporal, RtlCopyString}, DEVICE_OBJECT, DO_BUFFERED_IO, DRIVER_OBJECT, FILE_DEVICE_SECURE_OPEN, FILE_DEVICE_UNKNOWN, IO_NO_INCREMENT, IRP, IRP_MJ_CLOSE, IRP_MJ_CREATE, IRP_MJ_DEVICE_CONTROL, NTSTATUS, PCUNICODE_STRING, PDEVICE_OBJECT, PIRP, PUNICODE_STRING, STATUS_BUFFER_ALL_ZEROS, STATUS_BUFFER_OVERFLOW, STATUS_SUCCESS, STATUS_UNSUCCESSFUL, _IO_STACK_LOCATION
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

    let status = configure_driver(driver, registry_path as *mut _);

    status
}


/// This deals with setting up the driver and any callbacks / configurations required
/// for its operation and lifetime.
pub unsafe extern "C" fn configure_driver(
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

    // Specifies the type of buffering that is used by the I/O manager for I/O requests that are sent to the device stack.
    (*device_object).Flags |= DO_BUFFERED_IO;

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
unsafe extern "C" fn handle_ioctl(device: *mut DEVICE_OBJECT, pirp: PIRP) -> NTSTATUS {
    let p_stack_location: *mut _IO_STACK_LOCATION = IoGetCurrentIrpStackLocation(pirp);

    if p_stack_location.is_null() {
        println!("[sanctum] [-] Unable to get stack location for IRP.");
        return STATUS_UNSUCCESSFUL;
    }

    println!("[sanctum] [+] Found the stack location! {:p}", p_stack_location);

    let control_code = (*p_stack_location).Parameters.DeviceIoControl.IoControlCode; // IOCTL code

    // process the IOCTL based on its code, note that the functions implementing IOCTL's should
    // contain detailed error messages within the functions, returning a Result<(), NTSTATUS> this will
    // assist debugging exactly where an error has occurred, and not printing it at this level prevents
    // duplication.
    //
    // we still require calling IofCompleteRequest to return the IRP to the I/O manager otherwise we risk
    // causing the driver to hang.
    let result: NTSTATUS = match control_code {
        SANC_IOCTL_PING => {
            if let Err(e) = ioctl_handler_ping(device, p_stack_location, pirp){
                println!("[sanctum] [-] Error: {e}");
                e
            } else {
                println!("[sanctum] [i] IOCTL complete.");
                STATUS_SUCCESS
            }
        },
        _ => {
            println!("[sanctum] [-] IOCTL control code: {} not implemented.", control_code);
            STATUS_UNSUCCESSFUL
        }
    };

    // indicates that the caller has completed all processing for a given I/O request and 
    // is returning the given IRP to the I/O manager
    // https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-iocompleterequest
    IofCompleteRequest(pirp, IO_NO_INCREMENT as i8);
    
    result
}