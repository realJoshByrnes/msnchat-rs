#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]
#![allow(unused_unsafe)]

use windows::core::Result;

pub mod audio;
pub mod config;
pub mod host;
pub mod network;
pub mod patch;

fn main() -> Result<()> {

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    if let Err(e) = unsafe { patch::loader_hook::init_dll_hooks() } {
        log::error!("Failed to init hooks: {}", e);
    }

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

    // Run the WinUI 3 application
    if let Err(e) = host::window::run_winui_app(manual_module) {
        log::error!("WinUI 3 application failed to run: {}", e);
        if e.code().0 == 0x80040154u32 as i32 || e.message().contains("0x80040154") {
            show_missing_runtime_dialog();
            std::process::exit(0);
        }
        return Err(e);
    }

    Ok(())
}

fn show_missing_runtime_dialog() {
    use windows::core::w;
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_YESNO, MB_ICONERROR, IDYES};
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};

    unsafe {
        // Initialize COM on this thread so ShellExecuteW can resolve protocol handlers
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let result = MessageBoxW(
            None,
            w!("This application requires the Windows App SDK runtime to run.\n\nError: Class not registered (0x80040154).\n\nWould you like to download and install the Windows App SDK runtime now?"),
            w!("Missing Windows App SDK Runtime"),
            MB_YESNO | MB_ICONERROR,
        );

        if result == IDYES {
            let _ = ShellExecuteW(
                None,
                w!("open"),
                w!("https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/downloads"),
                None,
                None,
                SW_SHOWNORMAL,
            );
        }
    }
}
