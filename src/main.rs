#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use windows::Win32::System::Ole::OleInitialize;
use windows::core::{GUID, Result};

pub mod chat45;
pub mod host;
pub mod patch;
pub mod window;

use window::OcxWindow;

fn main() -> Result<()> {
    if cfg!(debug_assertions) {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace")).init();
    } else {
        env_logger::init();
    }

    // Initialize OLE / COM
    unsafe {
        OleInitialize(None)?;
    }

    // Create the main window
    let mut main_window = OcxWindow::new(None)?;
    main_window.is_main_window = true;

    // Attach the MSN Chat OCX
    let clsid = GUID::from_values(
        0xF58E1CEF,
        0xA068,
        0x4c15,
        [0xBA, 0x5E, 0x58, 0x7C, 0xAF, 0x3E, 0xE8, 0xC6],
    );
    #[repr(C, align(64))]
    struct AlignedBytes<const N: usize>(pub [u8; N]);

    static OCX_BYTES: AlignedBytes<
        { include_bytes!("../assets/vendor/microsoft/MsnChat45.ocx").len() },
    > = AlignedBytes(*include_bytes!("../assets/vendor/microsoft/MsnChat45.ocx"));

    let dll_bytes = &OCX_BYTES.0;

    let manual_module =
        std::sync::Arc::new(unsafe { patch::pe::ManualModule::load(dll_bytes) }.unwrap());

    log::info!("Attaching OCX to main window...");
    match main_window.attach_ocx(manual_module, &clsid, |host| {
        log::info!("Setting BaseURL");
        let _ = host.put_property("BaseURL", "http://chat.msn.com/");
        let random_digits: u16 = rand::random::<u16>() % 10000;
        let nickname = format!("🦀{:04}", random_digits);
        log::info!("Setting NickName");
        let _ = host.put_property("NickName", &nickname);
        log::info!("Setting RoomName");
        let _ = host.put_property("RoomName", "The Lobby");
        log::info!("Setting Server");
        let _ = host.put_property("Server", "dir.irc7.com");
    }) {
        Ok(_) => {
            log::info!("OCX attached successfully. Running message loop.");
            // Run the standard message pump
            OcxWindow::run_message_loop()?;
        }
        Err(e) => {
            log::error!("Failed to attach OCX: {}", e);
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
