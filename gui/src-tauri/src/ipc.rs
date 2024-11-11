use std::fmt::Debug;

use serde::{de::DeserializeOwned, Serialize};
use serde_json::{to_value, to_vec};
use shared_std::ipc::{CommandRequest, PIPE_NAME};
use tokio::{io::{self, AsyncReadExt, AsyncWriteExt}, net::windows::named_pipe::{ClientOptions, NamedPipeClient}};

pub struct IpcClient {
    client: NamedPipeClient,
}

impl IpcClient {
    /// Main mechanism for sending IPC requests to the usermode engine for the EDR. This function
    /// requires a turbofish generic which will be whatever the function on the other side of the IPC
    /// (aka the usermode EDR engine) returns.
    /// 
    /// This contains the command in question as a String, and 'args' which is a generic JSON serialised "Value"
    /// from Serde which allows the struct to contain any number of arguments, serialised to / from a struct that 
    /// is appropriate for the calling / receiving functions.
    /// 
    /// # Sending function
    /// 
    /// The first parameter in the turbofish is the return type.
    /// 
    /// The sending function must encode data like so:
    /// 
    /// ## No data to send:
    /// 
    /// ```
    /// // where IPC is of type IpcClient as implemented in the GUI.
    /// IpcClient::send_ipc::<(), Option<Value>>("scanner_cancel_scan", None).await
    /// ```
    /// 
    /// ## Data of type A to send:
    /// 
    /// ```
    /// let path = to_value(vec![PathBuf::from(file_path)]).unwrap();
    /// IpcClient::send_ipc::<FileScannerState, _>("scanner_start_folder_scan", Some(path)).await
    /// ```
    /// 
    /// # Returns
    /// 
    /// This function will return:
    /// 
    /// - Ok T: where T is the return type of the function run by the usermode engine.
    /// - Err: where the error relates to the reading / writing of the IPC, and NOT the function run
    /// by the IPC server. 
    pub async fn send_ipc<T, A>(command: &str, args: Option<A>) -> io::Result<T> 
    where 
        T: DeserializeOwned + Debug,
        A: Serialize
    {

        let mut client = ClientOptions::new()
            .open(PIPE_NAME)?;

        // where there are args, serialise, otherwise, set to none
        let args = match args {
            Some(a) => Some(to_value(a).unwrap()),
            None => None,
        };

        let message = CommandRequest {
            command: command.to_string(),
            args,
        };

        let message_data = to_vec(&message)?;
        client.write_all(&message_data).await?;

        // read the response
        let mut buffer = vec![0u8; 1024];
        let bytes_read = client.read(&mut buffer).await?;
        let received_data = &buffer[..bytes_read];

        // Deserialize the received JSON data into a Message struct
        let response_message: T = serde_json::from_slice(received_data)?;

        Ok(response_message)

    }

}