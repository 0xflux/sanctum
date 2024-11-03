use windows::{core::PWSTR, Win32::System::WindowsProgramming::GetUserNameW};

pub fn get_logged_in_username() -> Result<String, String>{
    // get the username of the current logged in user to resolve locations
    let mut buffer: [u16; 256] = [0; 256];
    let mut size = buffer.len() as u32;

    let result = unsafe {GetUserNameW(PWSTR(buffer.as_mut_ptr()), &mut size) };

    if let Err(e) = result {
        return Err(format!("Error getting UserName: {e}"));
    }

    Ok(String::from_utf16_lossy(&buffer[..size as usize -1]))
}