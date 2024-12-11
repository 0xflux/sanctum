// FFI for functions not yet implemented in the Rust Windows Driver project

use core::{ffi::c_void, ptr::null_mut};

use wdk_sys::{ntddk::KeInitializeEvent, FALSE, FAST_MUTEX, FM_LOCK_BIT, HANDLE, HANDLE_PTR, OBJECT_ATTRIBUTES, PIO_STACK_LOCATION, PIRP, POBJECT_ATTRIBUTES, PSECURITY_DESCRIPTOR, PUNICODE_STRING, ULONG, _EVENT_TYPE::SynchronizationEvent};

// #[link(name = "ntoskrnl")]
// extern "system" {
//     pub fn ExInitializeFastMutex(mutex: PFAST_MUTEX);
// }

pub unsafe fn IoGetCurrentIrpStackLocation(irp: PIRP) -> PIO_STACK_LOCATION {
    assert!((*irp).CurrentLocation <= (*irp).StackCount + 1); // todo maybe do error handling instead of an assert?
    (*irp)
        .Tail
        .Overlay
        .__bindgen_anon_2
        .__bindgen_anon_1
        .CurrentStackLocation
}

#[allow(non_snake_case)]
pub unsafe fn ExInitializeFastMutex(kmutex: *mut FAST_MUTEX) {
    core::ptr::write_volatile(&mut (*kmutex).Count, FM_LOCK_BIT as i32);

    (*kmutex).Owner = core::ptr::null_mut();
    (*kmutex).Contention = 0;
    KeInitializeEvent(&mut (*kmutex).Event, SynchronizationEvent, FALSE as _)
}

/// The InitializeObjectAttributes macro initializes the opaque OBJECT_ATTRIBUTES structure, 
/// which specifies the properties of an object handle to routines that open handles.
/// 
/// # Returns
/// This function will return an Err if the POBJECT_ATTRIBUTES is null. Otherwise, it will return
/// Ok(())
#[allow(non_snake_case)]
pub unsafe fn InitializeObjectAttributes(
    p: POBJECT_ATTRIBUTES,
    n: PUNICODE_STRING,
    a: ULONG,
    r: HANDLE,
    s: PSECURITY_DESCRIPTOR,
) -> Result<(), ()>{
    // check the validity of the OBJECT_ATTRIBUTES pointer
    if p.is_null() {
        return Err(());
    }

    (*p).Length = size_of::<OBJECT_ATTRIBUTES>() as u32;
    (*p).RootDirectory = r;
    (*p).Attributes = a;
    (*p).ObjectName = n;
    (*p).SecurityDescriptor = s;
    (*p).SecurityQualityOfService = null_mut();

    Ok(())
}