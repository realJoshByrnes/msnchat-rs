use windows::Win32::System::Ole::OleInitialize;
use windows::core::{GUID, Result};

pub mod audio;
pub mod auth;
pub mod host;
pub mod network;
pub mod patch;

use host::window::OcxWindow;

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    if let Err(e) = unsafe { patch::loader_hook::init_dll_hooks() } {
        log::error!("Failed to init hooks: {}", e);
    }
    // Initialize OLE / COM
    unsafe {
        OleInitialize(None)?;
    }

    // Create the main window
    let mut main_window = OcxWindow::new()?;

    // Attach the MSN Chat OCX
    let clsid = GUID::from_values(
        0xF58E1CEF,
        0xA068,
        0x4c15,
        [0xBA, 0x5E, 0x58, 0x7C, 0xAF, 0x3E, 0xE8, 0xC6],
    );
    let dll_path = r".\assets\MsnChat45.ocx";

    // Attempt to load and embed the control
    match main_window.attach_ocx(dll_path, &clsid, |host| {
        let _ = host.put_property("BaseURL", "http://chat.msn.com/");
        let _ = host.put_property("Market", "en-au");

        let random_id = (uuid::Uuid::new_v4().as_u128() % 10000) as u32;
        let nickname = format!("JD{:04}", random_id);
        let _ = host.put_property("NickName", &nickname);
        let _ = host.put_property("RoomName", "The Lobby");
        let _ = host.put_property("Server", "dir.irc7.com");
    }) {
        Ok(_) => {
            // Run the standard message pump
            OcxWindow::run_message_loop()?;
        }
        Err(e) => {
            // Display an error message if loading fails
            unsafe {
                windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
                    None,
                    &windows::core::HSTRING::from(format!("Failed to load OCX: {}", e)),
                    windows::core::w!("Error"),
                    windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
                );
            }
        }
    }

    Ok(())
}
