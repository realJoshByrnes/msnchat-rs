use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            AppendMenuW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateMenu, CreatePopupMenu,
            CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, MENU_ITEM_FLAGS, MSG,
            PostQuitMessage, RegisterClassW, TranslateMessage, WM_COMMAND, WM_DESTROY, WM_SIZE,
            WNDCLASSW, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
        },
    },
    core::{GUID, Result, w},
};

use crate::host::OcxHost;

pub struct OcxWindow {
    hwnd: HWND,
    host: Option<OcxHost>,
    children: Vec<OcxWindow>,
    pub is_main_window: bool,
    parent_hwnd: Option<HWND>,
}

impl OcxWindow {
    pub fn new(parent_hwnd: Option<HWND>) -> Result<Self> {
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
        unsafe { RegisterClassW(&wc) };

        // Only create the menu for the main window, not children
        let mut hmenu_opt = None;
        if parent_hwnd.is_none() {
            let hmenu = unsafe { CreateMenu()? };
            let h_tools_menu = unsafe { CreatePopupMenu()? };
            unsafe {
                let _ = AppendMenuW(
                    h_tools_menu,
                    MENU_ITEM_FLAGS(0), // MF_STRING
                    1001,
                    w!("Settings"),
                );
                let _ = AppendMenuW(
                    hmenu,
                    MENU_ITEM_FLAGS(16), // MF_POPUP
                    h_tools_menu.0 as usize,
                    w!("Tools"),
                );
            }
            hmenu_opt = Some(hmenu);
        }

        let style = if parent_hwnd.is_some() {
            windows::Win32::UI::WindowsAndMessaging::WS_OVERLAPPED
                | windows::Win32::UI::WindowsAndMessaging::WS_CAPTION
                | windows::Win32::UI::WindowsAndMessaging::WS_SYSMENU
                | WS_VISIBLE
        } else {
            WS_OVERLAPPEDWINDOW | WS_VISIBLE
        };

        // Create window
        let hwnd = unsafe {
            CreateWindowExW(
                windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE::default(),
                class_name,
                w!("MSN Chat (Rust)"),
                style,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                800,
                600,
                parent_hwnd,
                hmenu_opt,
                Some(instance.into()),
                None,
            )?
        };

        if let Some(parent) = parent_hwnd {
            unsafe {
                let _ = windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow(parent, false);
            }
        }

        Ok(Self {
            hwnd,
            host: None,
            children: Vec::new(),
            is_main_window: false,
            parent_hwnd,
        })
    }

    pub fn attach_ocx<F>(
        &mut self,
        module: std::sync::Arc<crate::patch::pe::ManualModule>,
        clsid: &GUID,
        setup: F,
    ) -> Result<()>
    where
        F: FnOnce(&mut OcxHost),
    {
        let mut host = OcxHost::new(module, clsid)?;

        if let Ok((w, h)) = host.get_size()
            && self.parent_hwnd.is_some()
        {
            unsafe {
                windows::Win32::UI::WindowsAndMessaging::SetWindowPos(
                    self.hwnd,
                    None,
                    0,
                    0,
                    w + 16, // Typical border padding
                    h + 38, // Typical titlebar padding
                    windows::Win32::UI::WindowsAndMessaging::SWP_NOMOVE
                        | windows::Win32::UI::WindowsAndMessaging::SWP_NOZORDER,
                )?;
            }
        }

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
                    WM_COMMAND => {
                        let wm_id = (wparam.0 & 0xFFFF) as u16;
                        if wm_id == 1001
                            && let Some(host) = &this.host
                        {
                            let module = host.module.clone();
                            if let Ok(settings_win) = OcxWindow::new(Some(window)) {
                                let clsid = GUID::from_values(
                                    0xFA980E7E,
                                    0x9E44,
                                    0x4D2F,
                                    [0xB3, 0xC2, 0x9A, 0x5B, 0xE4, 0x25, 0x25, 0xF8],
                                );
                                let mut settings_win = settings_win;
                                let _ = settings_win.attach_ocx(module, &clsid, |_| {});
                                this.children.push(settings_win);
                            }
                        }
                        return LRESULT(0);
                    }
                    WM_DESTROY => {
                        // Drop the host properly before quitting
                        this.host = None;
                        if this.is_main_window {
                            PostQuitMessage(0);
                        } else if let Some(parent) = this.parent_hwnd {
                            let _ = windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow(
                                parent, true,
                            );
                            let _ = windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(
                                parent,
                            );
                        }
                        return LRESULT(0);
                    }
                    _ => {}
                }
            } else if message == WM_DESTROY {
                // We don't have this, but if it gets here, it means we don't know who we are.
                return LRESULT(0);
            }

            DefWindowProcW(window, message, wparam, lparam)
        }
    }
}
