use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW,
            DispatchMessageW, GetMessageW, MSG, PostQuitMessage, RegisterClassW, TranslateMessage,
            WM_DESTROY, WM_SIZE, WNDCLASSW, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
        },
    },
    core::{GUID, Result, w},
};

use crate::host::OcxHost;

pub struct OcxWindow {
    hwnd: HWND,
    host: Option<OcxHost>,
}

impl OcxWindow {
    pub fn new() -> Result<Self> {
        let instance = unsafe { GetModuleHandleW(None)? };

        // Define window class
        let class_name = w!("MsnChatOcxHostClass");
        let wc = WNDCLASSW {
            hCursor: unsafe {
                windows::Win32::UI::WindowsAndMessaging::LoadCursorW(
                    None,
                    windows::Win32::UI::WindowsAndMessaging::IDC_ARROW,
                )?
            },
            hInstance: instance.into(),
            lpszClassName: class_name,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(Self::wndproc),
            ..Default::default()
        };

        // Register class
        let atom = unsafe { RegisterClassW(&wc) };
        debug_assert!(atom != 0);

        // Create window
        let hwnd = unsafe {
            CreateWindowExW(
                windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE::default(),
                class_name,
                w!("MsnChat OCX Host"),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                800,
                600,
                None,
                None,
                Some(instance.into()),
                None,
            )?
        };

        Ok(Self { hwnd, host: None })
    }

    pub fn attach_ocx<F>(&mut self, dll_bytes: &[u8], clsid: &GUID, setup: F) -> Result<()>
    where
        F: FnOnce(&mut OcxHost),
    {
        let mut host = OcxHost::new(dll_bytes, clsid)?;
        setup(&mut host);
        host.attach(self.hwnd)?;
        self.host = Some(host);

        // Store self pointer in window user data for the wndproc
        unsafe {
            // we use the _ convention to cast safely for 32 bit pointers
            windows::Win32::UI::WindowsAndMessaging::SetWindowLongW(
                self.hwnd,
                windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
                (self as *const _ as isize) as i32,
            );
        }

        Ok(())
    }

    pub fn host(&self) -> Option<&OcxHost> {
        self.host.as_ref()
    }

    pub fn run_message_loop() -> Result<()> {
        unsafe {
            let mut message = MSG::default();
            while GetMessageW(&mut message, None, 0, 0).into() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        Ok(())
    }

    extern "system" fn wndproc(
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        unsafe {
            let user_data = windows::Win32::UI::WindowsAndMessaging::GetWindowLongW(
                window,
                windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
            );

            if user_data != 0 {
                let this = &mut *(user_data as *mut Self);
                match message {
                    WM_SIZE => {
                        let width = (lparam.0 & 0xFFFF) as i32;
                        let height = ((lparam.0 >> 16) & 0xFFFF) as i32;
                        if let Some(host) = &this.host {
                            let _ = host.resize(width, height);
                        }
                        return LRESULT(0);
                    }
                    WM_DESTROY => {
                        // Drop the host properly before quitting
                        this.host = None;
                        PostQuitMessage(0);
                        return LRESULT(0);
                    }
                    _ => {}
                }
            } else if message == WM_DESTROY {
                PostQuitMessage(0);
                return LRESULT(0);
            }

            DefWindowProcW(window, message, wparam, lparam)
        }
    }
}
