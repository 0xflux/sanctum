use serde_json::to_vec;
use shared_std::ipc::{CommandRequest, CommandResponse, PIPE_NAME};
use tokio::{io::{self, AsyncReadExt, AsyncWriteExt}, net::windows::named_pipe::{ClientOptions, NamedPipeClient}};

pub struct IpcClient {
    client: NamedPipeClient,
}

impl IpcClient {

    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // configure IPC client
        let ipc_client = ClientOptions::new()
            .open(PIPE_NAME)?;

        let c = IpcClient {
            client: ipc_client,
        };

        Ok(c)
    }

    //
    // Communication protocol functions
    //

    /// Main mechanism for sending IPC requests to the usermode engine for the EDR.
    pub async fn send_ipc(&mut self) -> io::Result<()> {

        let message = CommandRequest {
            command: "install_driver".to_string(),
        };

        let message_data = to_vec(&message)?;
        self.client.write_all(&message_data).await?;

        // read the response
        let mut buffer = vec![0u8; 1024];
        let bytes_read = self.client.read(&mut buffer).await?;
        let received_data = &buffer[..bytes_read];

        // Deserialize the received JSON data into a Message struct
        let response_message: CommandResponse = serde_json::from_slice(received_data)?;
        println!("Received: {:?}", response_message);


        Ok(())

    }


    //
    // IPC entrypoints for calling GUI functions, grouped by module. These should be 
    // prepended with the module name to help with cataloguing.
    //

    async fn test(&mut self) {
        let res = self.send_ipc().await;
        match res {
            Ok(_) => println!("No error"),
            Err(e) => eprintln!("[-] Error from IPC: {e}"),
        }
    }

}