// Sanctum Windows Kernel Mode Driver (WDM) written in Rust
// Date: 12/10/2024
// Author: flux
//      GH: https://github.com/0xflux
//      Blog: https://fluxsec.red/

#![no_std]
extern crate alloc;

#[cfg(not(test))]
extern crate wdk_panic;

use core::ptr::null_mut;

use ffi::{IoCreateDriver, IoGetCurrentIrpStackLocation};
use shared::constants::{DEVICE_NAME_PATH, DRIVER_NAME, SYMBOLIC_NAME_PATH};
use utils::{ToUnicodeString, ToWindowsUnicodeString};
use wdk::{nt_success, println};
#[cfg(not(test))]
use wdk_alloc::WdkAllocator;

mod ffi;
mod utils;

use wdk_sys::{
    ntddk::{IoCreateDevice, IoCreateSymbolicLink, IoDeleteSymbolicLink}, DEVICE_OBJECT, DRIVER_OBJECT, FILE_DEVICE_SECURE_OPEN, FILE_DEVICE_UNKNOWN, IRP, IRP_MJ_DEVICE_CONTROL, NTSTATUS, PCUNICODE_STRING, PDEVICE_OBJECT, PIRP, PUNICODE_STRING, STATUS_SUCCESS, STATUS_UNSUCCESSFUL
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
    let mut device_name = DEVICE_NAME_PATH
        .to_u16_vec()
        .to_windows_unicode_string()
        .expect("[sanctum] [-] unable to encode string to unicode.");

    let mut symbolic_link = SYMBOLIC_NAME_PATH
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
        &mut device_name,
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
    (*driver).DriverUnload = Some(driver_exit);
    (*driver).MajorFunction[IRP_MJ_DEVICE_CONTROL as usize] = Some(handle_ioctl);

    //
    // Create the symbolic link
    //
    if IoCreateSymbolicLink(&mut symbolic_link, &mut device_name) != 0 {
        println!("[sanctum] [-] Failed to create driver symbolic link.");
        return STATUS_UNSUCCESSFUL;
    }

    STATUS_SUCCESS
}

/// Driver unload functions when it is to exit.
///
/// # Safety
///
/// This function makes use of unsafe code.
extern "C" fn driver_exit(_driver: *mut DRIVER_OBJECT) {
    println!("[sanctum] driver unloading...");

    //
    // rm symbolic link
    //
    let mut symbolic_link = SYMBOLIC_NAME_PATH
        .to_u16_vec()
        .to_windows_unicode_string()
        .expect("[sanctum] [-] unable to encode string to unicode.");
    let _ = unsafe { IoDeleteSymbolicLink(&mut symbolic_link) };

    println!("[sanctum] driver unloaded successfully...");
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
        panic!("[-] Unable to get stack location for IRP.");
    }

    println!("[+] Found the stack location! {:p}", p_stack_location);

    0
}
