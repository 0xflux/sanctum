// FFI for functions not yet implemented in the Rust Windows Driver project

use wdk_sys::{ntddk::KeInitializeEvent, FALSE, FAST_MUTEX, FM_LOCK_BIT, PIO_STACK_LOCATION, PIRP, _EVENT_TYPE::SynchronizationEvent};

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