// Sanctum Windows Kernel Mode Driver (WDM) written in Rust
// Date: 12/10/2024
// Author: flux
//      GH: https://github.com/0xflux
//      Blog: https://fluxsec.red/

#![no_std]
extern crate alloc;

#[cfg(not(test))]
extern crate wdk_panic;

use ffi::IoGetCurrentIrpStackLocation;
use shared::constants::{DEVICE_NAME_PATH, SYMBOLIC_NAME_PATH};
use utils::{ToUnicodeString, ToWindowsUnicodeString};
use wdk::println;
#[cfg(not(test))]
use wdk_alloc::WdkAllocator;

mod ffi;
mod utils;

use wdk_sys::{
    ntddk::{IoCreateSymbolicLink, IoDeleteSymbolicLink},
    DEVICE_OBJECT, DRIVER_OBJECT, IRP, IRP_MJ_DEVICE_CONTROL, NTSTATUS, PCUNICODE_STRING, PIRP,
    STATUS_SUCCESS, STATUS_UNSUCCESSFUL,
};

#[cfg(not(test))]
#[global_allocator]
static GLOBAL_ALLOCATOR: WdkAllocator = WdkAllocator;

/// DriverEntry is required to start the driver, and acts as the main entrypoint
/// for our driver.
#[export_name = "DriverEntry"] // WDF expects a symbol with the name DriverEntry
pub unsafe extern "system" fn driver_entry(
    driver: &mut DRIVER_OBJECT,
    _registry_path: PCUNICODE_STRING,
) -> NTSTATUS {
    println!("[sanctum] [i] Starting Sanctum driver...");

    //
    // Configure the drivers callbacks
    //
    driver.DriverUnload = Some(driver_exit);
    driver.MajorFunction[IRP_MJ_DEVICE_CONTROL as usize] = Some(handle_ioctl);

    //
    // Configure the device symbolic links so we can access it from usermode
    //
    let mut device_name = DEVICE_NAME_PATH
        .to_u16_vec()
        .to_windows_unicode_string()
        .expect("[sanctum] [-] unable to encode string to unicode.");

    let mut symbolic_link = SYMBOLIC_NAME_PATH
        .to_u16_vec()
        .to_windows_unicode_string()
        .expect("[sanctum] [-] unable to encode string to unicode.");

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
