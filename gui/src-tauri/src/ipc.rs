use std::fmt::Debug;

use serde::de::DeserializeOwned;
use serde_json::to_vec;
use shared_std::ipc::{CommandRequest, PIPE_NAME};
use tokio::{io::{self, AsyncReadExt, AsyncWriteExt}, net::windows::named_pipe::{ClientOptions, NamedPipeClient}};

pub struct IpcClient {
    client: NamedPipeClient,
}

impl IpcClient {
    /// Creates a new instance of the IPC client for the GUI
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // configure IPC client
        let ipc_client = ClientOptions::new()
            .open(PIPE_NAME)?;

        let c = IpcClient {
            client: ipc_client,
        };

        Ok(c)
    }

    /// Main mechanism for sending IPC requests to the usermode engine for the EDR. This function
    /// requires a turbofish generic which will be whatever the function on the other side of the IPC
    /// (aka the usermode EDR engine) returns.
    /// 
    /// # Returns
    /// 
    /// This function will return:
    /// 
    /// - Ok T: where T is the return type of the function run by the usermode engine.
    /// - Err: where the error relates to the reading / writing of the IPC, and NOT the function run
    /// by the IPC server. 
    pub async fn send_ipc<T>(&mut self, command: &str) -> io::Result<T> 
    where 
        T: DeserializeOwned + Debug
    {

        let message = CommandRequest {
            command: command.to_string(),
        };

        let message_data = to_vec(&message)?;
        self.client.write_all(&message_data).await?;

        // read the response
        let mut buffer = vec![0u8; 1024];
        let bytes_read = self.client.read(&mut buffer).await?;
        let received_data = &buffer[..bytes_read];

        // Deserialize the received JSON data into a Message struct
        let response_message: T = serde_json::from_slice(received_data)?;
        println!("Received: {:?}", response_message);


        Ok(response_message)

    }

}