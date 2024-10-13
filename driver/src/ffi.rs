// FFI for functions not yet implemented in the Rust Windows Driver project

use wdk_sys::{DRIVER_INITIALIZE, NTSTATUS, PIO_STACK_LOCATION, PIRP, PUNICODE_STRING};

#[link(name = "ntoskrnl")]
extern "system" {
    pub fn IoCreateDriver(driver_name: PUNICODE_STRING, driver_initialise: DRIVER_INITIALIZE) -> NTSTATUS;
}

pub unsafe fn IoGetCurrentIrpStackLocation(irp: PIRP) -> PIO_STACK_LOCATION {
    assert!((*irp).CurrentLocation <= (*irp).StackCount + 1); // todo maybe do error handling instead of an assert?
    (*irp)
        .Tail
        .Overlay
        .__bindgen_anon_2
        .__bindgen_anon_1
        .CurrentStackLocation
}
