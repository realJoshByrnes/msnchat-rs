#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]
#![allow(unused_unsafe)]

use windows::Win32::System::Ole::OleInitialize;
use windows::core::{GUID, Result};

pub mod audio;
pub mod config;
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

    #[repr(C, align(64))]
    struct AlignedBytes<const N: usize>(pub [u8; N]);

    static OCX_BYTES: AlignedBytes<{ include_bytes!("../assets/MsnChat45.ocx").len() }> =
        AlignedBytes(*include_bytes!("../assets/MsnChat45.ocx"));

    let dll_bytes = &OCX_BYTES.0;

    // Write OCX to temp directory to load and register TypeLib
    let temp_ocx_path = std::env::temp_dir().join("MsnChat45.ocx");
    if !temp_ocx_path.exists() {
        std::fs::write(&temp_ocx_path, dll_bytes).map_err(|e| {
            windows::core::Error::new(
                windows::core::HRESULT(0x80004005u32 as i32),
                format!("Failed to write OCX: {}", e),
            )
        })?;
    }

    let path_hstring = windows::core::HSTRING::from(temp_ocx_path.to_string_lossy().as_ref());
    let pcwstr = windows::core::PCWSTR::from_raw(path_hstring.as_ptr());
    unsafe {
        let typelib = windows::Win32::System::Ole::LoadTypeLib(pcwstr)?;
        windows::Win32::System::Ole::RegisterTypeLibForUser(&typelib, pcwstr, None)?;
    }

    let manual_module =
        std::sync::Arc::new(unsafe { patch::pe::ManualModule::load(dll_bytes) }.unwrap());

    // Attempt to load and embed the control
    match main_window.attach_ocx(manual_module, &clsid, |host| {
        let _ = host.put_property("BaseURL", "http://chat.msn.com/");
        let _ = host.put_property("Market", "en-au");

        let random_id = (uuid::Uuid::new_v4().as_u128() % 10000) as u32;
        let nickname = format!("JD{:04}", random_id);
        let _ = host.put_property("AuditMessage", "Note: MSN has detected that you are connected to this chat session from the IP address <b>%1</b>.");
        let _ = host.put_property("ChatMode", "0");
        let _ = host.put_property("InvitationCode", "5355");
        let _ = host.put_property("MessageOfTheDay", "Welcome to MSN Chat. Important: MSN does not control or endorse the content, messages or information found in chat. MSN specifically disclaims any liability with regard to these areas. To review the guidelines for use of MSN Chat, go to http://chat.msn.com/conduct.asp.");
        let _ = host.put_property("NickName", &nickname);
        let _ = host.put_property("RoomName", "The Lobby");
        let _ = host.put_property("Server", "dir.irc7.com");
        let _ = host.put_property("WhisperContent", "http://test.example.com/whisper");
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
