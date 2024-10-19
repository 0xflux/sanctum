// FFI for functions not yet implemented in the Rust Windows Driver project

use wdk_sys::{PIO_STACK_LOCATION, PIRP};

#[link(name = "ntoskrnl")]
extern "system" {
    // pub fn RtlCopyMemory(dest: *mut u64, source: *mut u64, length: usize);
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
