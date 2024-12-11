use std::{sync::Arc, thread::sleep, time::Duration};

use crate::engine::UmEngine;

pub struct Core {
    driver_poll_rate: u64,
}


impl Core {
    /// Starts the core of the usermode engine; kicking off the frequent polling of the 
    pub async fn start_core(engine: Arc<UmEngine>) -> ! {

        // create a local self contained instance of Core, as we don't need to instantiate 
        // the core outside of this entry function
        let core = Core {
            driver_poll_rate: 60,
        };

        //
        // Enter the polling & decision making loop, this here is the core / engine of the usermode engine.
        //
        loop {
            // contact the driver and get any messages from the kernel 
            let driver_response = engine.driver_manager.lock().unwrap().ioctl_get_driver_messages();
                if driver_response.is_some() {
                    // println!("x: {:?}", driver_response);
                    // todo

                    // cache messages 
                    // add process creations to a hashmap

                    // todo long term: thread creation & handle re quests metadata to the abv hashmap
                }

                sleep(Duration::from_millis(core.driver_poll_rate));
        }
    }

}