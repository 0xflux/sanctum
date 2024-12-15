use core::{ffi::c_void, mem, ptr::null_mut, slice, sync::atomic::Ordering};

use alloc::{format, string::String, vec::Vec};
use serde::{Deserialize, Serialize};
use shared_no_std::{constants::SanctumVersion, driver_ipc::{ProcessStarted, ProcessTerminated}, ioctl::{DriverMessages, SancIoctlPing}};
use wdk::println;
use wdk_sys::{ntddk::{ExAcquireFastMutex, ExReleaseFastMutex, KeGetCurrentIrql, RtlCopyMemoryNonTemporal}, APC_LEVEL, FAST_MUTEX, NTSTATUS, PIRP, STATUS_BUFFER_ALL_ZEROS, STATUS_INVALID_BUFFER_SIZE, STATUS_SUCCESS, STATUS_UNSUCCESSFUL, _IO_STACK_LOCATION};
use crate::{ffi::ExInitializeFastMutex, utils::{check_driver_version, DriverError, Log}, DRIVER_MESSAGES, DRIVER_MESSAGES_CACHE};

/// DriverMessagesWithMutex object which contains a spinlock to allow for mutable access to the queue.
/// This object should be used to safely manage access to the inner DriverMessages which contains 
/// the actual data. The DriverMessagesWithMutex contains metadata + the DriverMessages.
pub struct DriverMessagesWithMutex {
    lock: FAST_MUTEX,
    is_empty: bool,
    data: DriverMessages,
}

impl Default for DriverMessagesWithMutex {
    fn default() -> Self {
        let mut mutex = FAST_MUTEX::default();
        unsafe { ExInitializeFastMutex(&mut mutex) };
        let data = DriverMessages::default();

        DriverMessagesWithMutex { lock: mutex, is_empty: true, data }
    }
}

impl DriverMessagesWithMutex {
    pub fn new() -> Self {
        DriverMessagesWithMutex::default()
    }

    /// Adds a print msg to the queue.
    /// 
    /// This function will wait for an acquisition of the spin lock to continue and will block
    /// until that point.
    pub fn add_message_to_queue(&mut self, data: String)
     {

        let irql = unsafe { KeGetCurrentIrql() };
        if irql != 0 {
            println!("[sanctum] [-] IRQL is not PASSIVE_LEVEL: {}", irql);
            return;
        }

        unsafe { ExAcquireFastMutex(&mut self.lock) };

        let irql = unsafe { KeGetCurrentIrql() };
        if irql > APC_LEVEL as u8 {
            println!("[sanctum] [-] IRQL is not APIC_LEVEL: {}", irql);
            unsafe { ExReleaseFastMutex(&mut self.lock) }; 
            return;
        }

        self.is_empty = false;
        self.data.messages.push(data);

        unsafe { ExReleaseFastMutex(&mut self.lock) }; 
    }


    /// Adds serialised data to the message queue.
    /// 
    /// This function will wait for an acquisition of the spin lock to continue and will block
    /// until that point.
    pub fn add_process_creation_to_queue(&mut self, data: ProcessStarted)
     {

        let irql = unsafe { KeGetCurrentIrql() };
        if irql != 0 {
            println!("[sanctum] [-] IRQL is not PASSIVE_LEVEL: {}", irql);
            return;
        }

        unsafe { ExAcquireFastMutex(&mut self.lock) };

        let irql = unsafe { KeGetCurrentIrql() };
        if irql > APC_LEVEL as u8 {
            println!("[sanctum] [-] IRQL is not APIC_LEVEL: {}", irql);
            unsafe { ExReleaseFastMutex(&mut self.lock) }; 
            return;
        }

        self.is_empty = false;
        self.data.process_creations.push(data);
        
        unsafe { ExReleaseFastMutex(&mut self.lock) }; 
    }


    /// Adds a terminated process to the queue.
    /// 
    /// This function will wait for an acquisition of the spin lock to continue and will block
    /// until that point.
    pub fn add_process_termination_to_queue(&mut self, data: ProcessTerminated)
     {

        let irql = unsafe { KeGetCurrentIrql() };
        if irql != 0 {
            println!("[sanctum] [-] IRQL is not PASSIVE_LEVEL: {}", irql);
            return;
        }

        unsafe { ExAcquireFastMutex(&mut self.lock) };

        let irql = unsafe { KeGetCurrentIrql() };
        if irql > APC_LEVEL as u8 {
            println!("[sanctum] [-] IRQL is not APIC_LEVEL: {}", irql);
            unsafe { ExReleaseFastMutex(&mut self.lock) }; 
            return;
        }

        self.is_empty = false;
        self.data.process_terminations.push(data);
        
        unsafe { ExReleaseFastMutex(&mut self.lock) }; 
    }


    /// Extract all data out of the queue if there is data.
    /// 
    /// # Returns
    /// 
    /// The function will return None if the queue was empty.
    fn extract_all(&mut self) -> Option<DriverMessages> {

        let irql = unsafe { KeGetCurrentIrql() };
        if irql != 0 {
            println!("[sanctum] [-] IRQL is not PASSIVE_LEVEL: {}", irql);
            return None;
        }

        unsafe { ExAcquireFastMutex(&mut self.lock) };

        let irql = unsafe { KeGetCurrentIrql() };
        if irql > APC_LEVEL as u8 {
            println!("[sanctum] [-] IRQL is not APIC_LEVEL: {}", irql);
            unsafe { ExReleaseFastMutex(&mut self.lock) }; 
            return None;
        }

        if self.is_empty {
            unsafe { ExReleaseFastMutex(&mut self.lock) }; 
            return None;
        }
        
        //
        // Using mem::take now seems safe against kernel panics; we were having some issues
        // previous with this, leading to IRQL_NOT_LESS_OR_EQUAL bsod. That was likely a programming
        // error as opposed to a safety error with mem::take. If further bsod's occur around mem::take,
        // try swapping to mem::swap; however, the core functionality of both should be the same.
        //
        let extracted_data = mem::take(&mut self.data);

        self.is_empty = true; // reset flag

        unsafe { ExReleaseFastMutex(&mut self.lock) }; 

        Some(extracted_data)
    }


    fn add_existing_queue(&mut self, q: &mut DriverMessages) -> usize {

        self.is_empty = false;
        self.data.messages.append(&mut q.messages);
        self.data.process_creations.append(&mut q.process_creations);
        self.data.process_terminations.append(&mut q.process_terminations);

        let tmp = serde_json::to_vec(&DriverMessages{
            messages: self.data.messages.clone(),
            process_creations: self.data.process_creations.clone(),
            process_terminations: self.data.process_terminations.clone(),
        });

        let len = match tmp {
            Ok(v) => v.len(),
            Err(e) => {
                println!("[sanctum] [-] Error serializing temp object for len. {e}.");
                return 0;
            },
        };

        len
    }
}

struct IoctlBuffer {
    len: u32,
    buf: *mut c_void,
    p_stack_location: *mut _IO_STACK_LOCATION,
    pirp: PIRP,
}


impl IoctlBuffer {

    /// Creates a new instance of the IOCTL buffer type
    fn new(
        p_stack_location: *mut _IO_STACK_LOCATION,
        pirp: PIRP
    ) -> Self {
        IoctlBuffer {
            len: 0,
            buf: null_mut(),
            p_stack_location,
            pirp
        }
    }

    /// Converts the input buffer from the IO Manager into a valid utf8 string.
    fn get_buf_to_str(
        &mut self,
    ) -> Result<&str, NTSTATUS> {

        // first initialise the fields with buf and len
        self.receive()?;

        // construct the message from the pointer (ascii &[u8])
        let input_buffer = unsafe {core::slice::from_raw_parts(self.buf as *const u8, self.len as usize)};
        if input_buffer.is_empty() { 
            println!("[sanctum] [-] Error reading string passed to PING IOCTL");
            return Err(STATUS_UNSUCCESSFUL);
        }

        let input_buffer = core::str::from_utf8(input_buffer).unwrap();

        // this does not result in a dangling reference as we are referring to memory owned by Self, we are returning 
        // a slice of that memory.
        Ok(input_buffer)
    }

    /// Receives raw data from the IO Manager and checks the validity of the data. If the data was valid, it will set the member 
    /// fields for the length, buffer, and raw pointers to the required structs. 
    /// 
    /// If you want to get a string out of an ioctl buffer, it would be better to call get_buf_to_str.
    /// 
    /// # Returns
    /// 
    /// Success: a IoctlBuffer which will hold the length and a pointer to the buffer
    /// 
    /// Error: NTSTATUS
    fn receive(
        &mut self,
    ) -> Result<(), NTSTATUS> {
    
        // length of in buffer
        let input_len: u32 = unsafe {(*self.p_stack_location).Parameters.DeviceIoControl.InputBufferLength};
        // if input_len == 0 { 
        //     println!("[sanctum] [-] IOCTL PING input length invalid.");
        //     return Err(STATUS_BUFFER_TOO_SMALL) 
        // };
    
        // For METHOD_BUFFERED, the driver should use the buffer pointed to by Irp->AssociatedIrp.SystemBuffer as the output buffer.
        let input_buffer: *mut c_void = unsafe {(*self.pirp).AssociatedIrp.SystemBuffer};
        if input_buffer.is_null() { 
            println!("[sanctum] [-] Input buffer is null.");
            return Err(STATUS_BUFFER_ALL_ZEROS) 
        };
    
        // validate the pointer
        if input_buffer.is_null() {
            println!("[sanctum] [-] IOCTL input buffer was null.");
            return Err(STATUS_UNSUCCESSFUL);
        }
    
        self.len = input_len;
        self.buf = input_buffer;
    
        Ok(())
    }


    /// Sends a str slice &[u8] back to the userland application taking in a &str and making 
    /// the necessary conversions.
    /// 
    /// # Returns
    /// 
    /// Success: ()
    /// 
    /// Error: NTSTATUS
    fn send_str(
        &self,
        input_str: &str,
    ) -> Result<(), NTSTATUS> {

        // handled the request successfully
        unsafe {(*self.pirp).IoStatus.__bindgen_anon_1.Status = STATUS_SUCCESS};

        // response back to userland
        let response = input_str.as_bytes();
        let response_len = response.len();
        unsafe {(*self.pirp).IoStatus.Information = response_len as u64};

        println!("[sanctum] [i] Sending back to userland {:?}", core::str::from_utf8(response).unwrap());

        // Copy the data now into the buffer to send back to usermode.
        // The driver should not write directly to the buffer pointed to by Irp->UserBuffer.
        unsafe {
            if !(*self.pirp).AssociatedIrp.SystemBuffer.is_null() {
                RtlCopyMemoryNonTemporal((*self.pirp).AssociatedIrp.SystemBuffer as *mut c_void, response as *const _ as *mut c_void, response_len as u64);
            } else {
                println!("[sanctum] [-] Error handling IOCTL PING, SystemBuffer was null.");
                return Err(STATUS_UNSUCCESSFUL);
            }
        }

        Ok(())
    }
}

/// Simple IOCTL test ping from usermode
pub fn ioctl_handler_ping(
    p_stack_location: *mut _IO_STACK_LOCATION,
    pirp: PIRP,
) -> Result<(), NTSTATUS> {

    let mut ioctl_buffer = IoctlBuffer::new(p_stack_location, pirp);
    // ioctl_buffer.receive()?;

    let input_buffer = ioctl_buffer.get_buf_to_str()?;
    println!("[sanctum] [+] Input buffer: {:?}", input_buffer);

    // send a str response back to userland
    ioctl_buffer.send_str("Msg received!")?;

    Ok(())
}

/// Get the response size of the message we need to send back to the usermode application.
/// This function will also shift the kernel message queue into a temp (global) object which will
/// retain the size, resetting the live queue.
pub fn ioctl_handler_get_kernel_msg_len(
    pirp: PIRP,
) -> Result<(), DriverError> {

    unsafe { 
        if (*pirp).AssociatedIrp.SystemBuffer.is_null() {
            println!("[sanctum] [-] SystemBuffer is a null pointer.");
            return Err(DriverError::NullPtr);
        }
    }

    let len_of_response = if !DRIVER_MESSAGES.load(Ordering::SeqCst).is_null() {
        let driver_messages = unsafe { &mut *DRIVER_MESSAGES.load(Ordering::SeqCst) };
        
        let local_drained_driver_messages = driver_messages.extract_all();
        if local_drained_driver_messages.is_none() {
            return Err(DriverError::NoDataToSend);
        }
        
        //
        // At this point, the transferred data form the queue has data in. Now try obtain a valid reference to
        // the driver message cache global
        //

        if !DRIVER_MESSAGES_CACHE.load(Ordering::SeqCst).is_null() {
            let driver_message_cache = unsafe { &mut *DRIVER_MESSAGES_CACHE.load(Ordering::SeqCst) };
            
            // add the drained data from the live driver messages to the cache, and return the size of the data
            let size_of_serialised_cache: usize = driver_message_cache.add_existing_queue(&mut local_drained_driver_messages.unwrap());

            size_of_serialised_cache
        } else {
            println!("[sanctum] [-] Driver messages is null");
            return Err(DriverError::DriverMessagePtrNull);
        }
    } else {
        println!("[sanctum] [-] Invalid pointer");
        return Err(DriverError::DriverMessagePtrNull);
    };


    if len_of_response == 0 {
        return Err(DriverError::NoDataToSend);
    }

    unsafe {(*pirp).IoStatus.Information = mem::size_of::<usize>() as u64};

    // copy the memory into the buffer
    unsafe {
        RtlCopyMemoryNonTemporal(
            (*pirp).AssociatedIrp.SystemBuffer, 
            &len_of_response as *const _ as *const _, 
            mem::size_of::<usize>() as u64
        )
    };

    Ok(())
}

/// Send any kernel messages in the DriverMessages struct back to userland.
pub fn ioctl_handler_send_kernel_msgs_to_userland(
    pirp: PIRP,
) -> Result<(), DriverError> {

    unsafe { 
        if (*pirp).AssociatedIrp.SystemBuffer.is_null() {
            println!("[sanctum] [-] SystemBuffer is a null pointer.");
            return Err(DriverError::NullPtr);
        }
    }

    // Attempt to dereference the DRIVER_MESSAGES global; if the dereference is successful,
    // make a call to extract_all to get all data from the message queue.
    let data = if !DRIVER_MESSAGES_CACHE.load(Ordering::SeqCst).is_null() {
        let obj = unsafe { &mut *DRIVER_MESSAGES_CACHE.load(Ordering::SeqCst) };
        obj.extract_all()
    } else {
        println!("[sanctum] [-] Invalid pointer");
        return Err(DriverError::DriverMessagePtrNull);
    };

    if data.is_none() {
        return Err(DriverError::NoDataToSend);
    }

    let encoded_data = match serde_json::to_vec(&data.unwrap()) {
        Ok(v) => v,
        Err(_) => {
            println!("[sanctum] [-] Error serializing data to string in ioctl_handler_send_kernel_msgs_to_userland");
            return Err(DriverError::CouldNotSerialize);
        },
    };

    let size_of_struct = encoded_data.len() as u64;
    unsafe {(*pirp).IoStatus.Information = size_of_struct};

    // copy the memory into the buffer
    unsafe {
        RtlCopyMemoryNonTemporal(
            (*pirp).AssociatedIrp.SystemBuffer, 
            encoded_data.as_ptr() as *const _, 
            size_of_struct
        )
    };

    Ok(())
}


pub fn ioctl_handler_ping_return_struct(
    p_stack_location: *mut _IO_STACK_LOCATION,
    pirp: PIRP,
) -> Result<(), NTSTATUS> {

    let mut ioctl_buffer = IoctlBuffer::new(p_stack_location, pirp);
    ioctl_buffer.receive()?; // receive the data

    let input_data = ioctl_buffer.buf as *mut c_void as *mut SancIoctlPing;
    if input_data.is_null() {
        println!("[sanctum] [-] Input struct data in IOCTL PING with struct was null.");
        return Err(STATUS_INVALID_BUFFER_SIZE);
    }

    let input_data = unsafe { &(*input_data) };

    // construct the input str from the array
    let input_str = unsafe { core::slice::from_raw_parts(input_data.version.as_ptr() as *const u8, input_data.str_len) };
    let input_str = match core::str::from_utf8(input_str) {
        Ok(v) => v,
        Err(e) => {
            println!("[sanctum] [-] Error converting input slice to string. {e}");
            return Err(STATUS_UNSUCCESSFUL);
        },
    };

    println!("[sanctum] [+] Input bool: {}, input str: {:#?}", input_data.received, input_str);

    // setup output 
    let msg = b"Msg received from the Kernel!";
    let mut out_buf = SancIoctlPing::new(); 

    if msg.len() > out_buf.capacity {
        println!("[sanctum] [-] Message too large to send back to usermode.");
        return Err(STATUS_UNSUCCESSFUL);
    }

    out_buf.received = true;
    out_buf.version[..msg.len()].copy_from_slice(msg);
    out_buf.str_len = msg.len();

    unsafe { 
        if (*pirp).AssociatedIrp.SystemBuffer.is_null() {
            println!("[sanctum] [-] SystemBuffer is a null pointer.");
            return Err(STATUS_UNSUCCESSFUL);
        }
    }
    let size_of_struct = core::mem::size_of_val(&out_buf) as u64;
    unsafe {(*pirp).IoStatus.Information = size_of_struct};

    unsafe {
        RtlCopyMemoryNonTemporal((*pirp).AssociatedIrp.SystemBuffer, &out_buf as *const _ as *const c_void, size_of_struct)
    };

    Ok(())
}


/// Checks the compatibility of the driver version with client version. For all intents and purposes this can be 
/// considered the real 'ping' with the current pings being POC for passing data between UM and KM.
pub fn ioctl_check_driver_compatibility(
    p_stack_location: *mut _IO_STACK_LOCATION,
    pirp: PIRP,
) -> Result<(), NTSTATUS> {

    let mut ioctl_buffer = IoctlBuffer::new(p_stack_location, pirp);
    ioctl_buffer.receive()?; // receive the data

    let input_data = ioctl_buffer.buf as *const _ as *const SanctumVersion;
    if input_data.is_null() {
        println!("[sanctum] [-] Error receiving input data for checking driver compatibility.");
        return Err(STATUS_UNSUCCESSFUL);
    }

    // validated the pointer, data should be safe to dereference
    let input_data: &SanctumVersion = unsafe {&*input_data};

    // check whether we are compatible
    let response = check_driver_version(input_data);
    println!("[sanctum] [i] Client version: {}.{}.{}, is compatible with driver version: {}.", input_data.major, input_data.minor, input_data.patch, response);
    let log = Log::new();
    log.log_to_userland(format!("[i] Client version: {}.{}.{}, is compatible with driver version: {}.", input_data.major, input_data.minor, input_data.patch, response));

    // prepare the data
    let res_size = core::mem::size_of_val(&response) as u64;
    unsafe { (*pirp).IoStatus.Information = res_size };

    unsafe {
        RtlCopyMemoryNonTemporal((*pirp).AssociatedIrp.SystemBuffer, &response as *const bool as *const c_void, res_size);
    }

    Ok(())
}