// Sanctum Windows Kernel Mode Driver written in Rust
// Date: 12/10/2024
// Author: flux
//      GH: https://github.com/0xflux
//      Blog: https://fluxsec.red/

#![no_std]
extern crate alloc;

#[cfg(not(test))]
extern crate wdk_panic;

use core::core_callback_notify_ps;
use ::core::{ptr::null_mut, sync::atomic::{AtomicPtr, Ordering}};

use alloc::{boxed::Box, format};
use ffi::IoGetCurrentIrpStackLocation;
use device_comms::{ioctl_check_driver_compatibility, ioctl_handler_get_kernel_msg_len, ioctl_handler_ping, ioctl_handler_ping_return_struct, ioctl_handler_send_kernel_msgs_to_userland, DriverMessagesCache, DriverMessagesWithMutex};
use shared_no_std::{constants::{DOS_DEVICE_NAME, NT_DEVICE_NAME, VERSION_DRIVER}, ioctl::{SANC_IOCTL_CHECK_COMPATIBILITY, SANC_IOCTL_DRIVER_GET_MESSAGES, SANC_IOCTL_DRIVER_GET_MESSAGE_LEN, SANC_IOCTL_PING, SANC_IOCTL_PING_WITH_STRUCT}};
use utils::{ToU16Vec, ToUnicodeString};
use wdk::{nt_success, println};
use wdk_sys::{
    ntddk::{IoCreateDevice, IoCreateSymbolicLink, IoDeleteDevice, IoDeleteSymbolicLink, IofCompleteRequest, PsSetCreateProcessNotifyRoutineEx}, DEVICE_OBJECT, DO_BUFFERED_IO, DRIVER_OBJECT, FALSE, FILE_DEVICE_SECURE_OPEN, FILE_DEVICE_UNKNOWN, IO_NO_INCREMENT, IRP_MJ_CLOSE, IRP_MJ_CREATE, IRP_MJ_DEVICE_CONTROL, NTSTATUS, PCUNICODE_STRING, PDEVICE_OBJECT, PIRP, PUNICODE_STRING, STATUS_SUCCESS, STATUS_UNSUCCESSFUL, TRUE, _IO_STACK_LOCATION
};

mod ffi;
mod utils;
mod device_comms;
mod core;

use wdk_alloc::WdkAllocator;
#[global_allocator]
static GLOBAL_ALLOCATOR: WdkAllocator = WdkAllocator;

/// An atomic pointer to the DriverMessagesWithSpinLock struct so that it can be used anywhere in the 
/// kernel.
static DRIVER_MESSAGES: AtomicPtr<DriverMessagesWithMutex> = AtomicPtr::new(null_mut());
static DRIVER_MESSAGES_CACHE: AtomicPtr<DriverMessagesCache> = AtomicPtr::new(null_mut());

/// DriverEntry is required to start the driver, and acts as the main entrypoint
/// for our driver.
#[export_name = "DriverEntry"] // WDF expects a symbol with the name DriverEntry
pub unsafe extern "system" fn driver_entry(
    driver: &mut DRIVER_OBJECT,
    registry_path: PCUNICODE_STRING,
) -> NTSTATUS {
    println!("[sanctum] [i] Starting Sanctum driver... Version: {}", VERSION_DRIVER);

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
    // Initialise the global DRIVER_MESSAGES variable
    //
    let messages = Box::new(DriverMessagesWithMutex::new());
    let messages_cache = Box::new(DriverMessagesCache::new());
    // take ownership of the pointer to the messages struct; the pointer shouldn't change as the 
    // struct contains a pointer to the vec, that is allowed to change.
    DRIVER_MESSAGES.store(Box::into_raw(messages), Ordering::SeqCst);
    DRIVER_MESSAGES_CACHE.store(Box::into_raw(messages_cache), Ordering::SeqCst);


    // Attempt to dereference the DRIVER_MESSAGES global; if the dereference is successful,
    // add the relevant data to the queue
    if !DRIVER_MESSAGES.load(Ordering::SeqCst).is_null() {
        let obj = unsafe { &mut *DRIVER_MESSAGES.load(Ordering::SeqCst) };
        obj.add_message_to_queue(format!("Starting Sanctum driver... Version: {}", VERSION_DRIVER));
    } else {
        println!("[sanctum] [-] Driver messages is null");
    }

    //
    // Configure the strings
    //
    let mut dos_name = DOS_DEVICE_NAME
        .to_unicode_string()
        .expect("[sanctum] [-] unable to encode string to unicode.");

    let mut nt_name = NT_DEVICE_NAME
        .to_unicode_string()
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
    // Create the symbolic link
    //
    let res = IoCreateSymbolicLink(&mut dos_name, &mut nt_name);
    if res != 0 {
        println!("[sanctum] [-] Failed to create driver symbolic link. Error: {res}");

        driver_exit(driver); // cleanup any resources before returning
        return STATUS_UNSUCCESSFUL;
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
    // Core callback functions for the EDR
    //

    // Intercepting process creation
    println!("[sanctum] [i] About to register.....");
    let res = PsSetCreateProcessNotifyRoutineEx(Some(core_callback_notify_ps), FALSE as u8);
    if res != STATUS_SUCCESS {
        println!("[sanctum] [-] Unable to create device via IoCreateDevice. Failed with code: {res}.");
        return res;
    }

    println!("[sanctum] [+] Done!");


    // Specifies the type of buffering that is used by the I/O manager for I/O requests that are sent to the device stack.
    (*device_object).Flags |= DO_BUFFERED_IO;

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
        .to_unicode_string()
        .expect("[sanctum] [-] unable to encode string to unicode.");
    let _ = unsafe { IoDeleteSymbolicLink(&mut device_name) };

    //
    // Unregister callback routines 
    //
    let res = unsafe { PsSetCreateProcessNotifyRoutineEx(Some(core_callback_notify_ps), TRUE as u8) };
    if res != STATUS_SUCCESS {
        println!("[sanctum] [-] Error removing PsSetCreateProcessNotifyRoutineEx from callback routines. Error: {res}");
    }

    // drop the driver messages
    let ptr = DRIVER_MESSAGES.swap(null_mut(), Ordering::SeqCst);
    if !ptr.is_null() {
        // allow rust to clean up the memory
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }

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
    let p_stack_location: *mut _IO_STACK_LOCATION = IoGetCurrentIrpStackLocation(pirp);

    if p_stack_location.is_null() {
        println!("[sanctum] [-] Unable to get stack location for IRP.");
        return STATUS_UNSUCCESSFUL;
    }

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
            if let Err(e) = ioctl_handler_ping(p_stack_location, pirp){
                println!("[sanctum] [-] Error: {e}");
                e
            } else {
                STATUS_SUCCESS
            }
        },
        SANC_IOCTL_PING_WITH_STRUCT => {
            if let Err(e) = ioctl_handler_ping_return_struct(p_stack_location, pirp){
                println!("[sanctum] [-] Error: {e}");
                e
            } else {
                STATUS_SUCCESS
            }
        },
        SANC_IOCTL_CHECK_COMPATIBILITY => {
            if let Err(e) = ioctl_check_driver_compatibility(p_stack_location, pirp){
                println!("[sanctum] [-] Error: {e}");
                e
            } else {
                STATUS_SUCCESS
            }
        }
        SANC_IOCTL_DRIVER_GET_MESSAGE_LEN => {
            if let Err(_) = ioctl_handler_get_kernel_msg_len(pirp){
                STATUS_UNSUCCESSFUL
            } else {
                STATUS_SUCCESS
            }
        }
        SANC_IOCTL_DRIVER_GET_MESSAGES => {
            // if let Err(e) = ioctl_handler_send_kernel_msgs_to_userland(pirp){
            //     STATUS_UNSUCCESSFUL
            // } else {
            //     STATUS_SUCCESS
            // }
            STATUS_SUCCESS
        }

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