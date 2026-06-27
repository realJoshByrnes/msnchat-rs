use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            AppendMenuW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateMenu, CreatePopupMenu,
            CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, MF_POPUP, MF_STRING,
            MSG, PostQuitMessage, RegisterClassW, SetMenu, TranslateMessage, WM_DESTROY, WM_SIZE,
            WNDCLASSW, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
        },
    },
    core::{GUID, Result, w},
};

use crate::host::OcxHost;

pub struct OcxWindow {
    hwnd: HWND,
    host: Option<OcxHost>,
    parent: Option<HWND>,
    module: Option<std::sync::Arc<crate::patch::pe::ManualModule>>,
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
        let _atom = unsafe { RegisterClassW(&wc) };

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

        // Create menu
        unsafe {
            let hmenu = CreateMenu()?;
            let hsubmenu = CreatePopupMenu()?;

            AppendMenuW(hsubmenu, MF_STRING, 1001, w!("&Options"))?;
            AppendMenuW(hsubmenu, MF_STRING, 1002, w!("E&xit"))?;

            AppendMenuW(hmenu, MF_POPUP, hsubmenu.0 as usize, w!("&File"))?;
            SetMenu(hwnd, Some(hmenu))?;
        }

        Ok(Self {
            hwnd,
            host: None,
            parent: None,
            module: None,
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
        self.module = Some(module.clone());
        let mut host = OcxHost::new(module, clsid)?;
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

    pub fn hwnd(&self) -> HWND {
        self.hwnd
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

    pub fn show_settings_modal(
        parent: HWND,
        module: std::sync::Arc<crate::patch::pe::ManualModule>,
    ) -> Result<()> {
        unsafe {
            let clsid_settings = GUID::from_values(
                0xFA980E7E,
                0x9E44,
                0x4D2F,
                [0xB3, 0xC2, 0x9A, 0x5B, 0xE4, 0x25, 0x25, 0xF8],
            );

            // 1. Create host first to get preferred extent size
            let mut host = OcxHost::new(module.clone(), &clsid_settings)?;
            let _ = host.put_property("BackColor", "16777215"); // White background

            // 2. Query preferred client size from control's extent
            let mut client_width = 400;
            let mut client_height = 350;
            if let Ok(size) = host.get_extent() {
                let hdc = windows::Win32::Graphics::Gdi::GetDC(None);
                let dpi_x = windows::Win32::Graphics::Gdi::GetDeviceCaps(
                    Some(hdc),
                    windows::Win32::Graphics::Gdi::LOGPIXELSX,
                );
                let dpi_y = windows::Win32::Graphics::Gdi::GetDeviceCaps(
                    Some(hdc),
                    windows::Win32::Graphics::Gdi::LOGPIXELSY,
                );
                let _ = windows::Win32::Graphics::Gdi::ReleaseDC(None, hdc);
                client_width = (size.cx * dpi_x) / 2540;
                client_height = (size.cy * dpi_y) / 2540;
            }

            // 3. Adjust window size to fit the client area perfectly
            let mut rect = RECT {
                left: 0,
                top: 0,
                right: client_width,
                bottom: client_height,
            };
            let _ = windows::Win32::UI::WindowsAndMessaging::AdjustWindowRectEx(
                &mut rect,
                windows::Win32::UI::WindowsAndMessaging::WS_POPUPWINDOW
                    | windows::Win32::UI::WindowsAndMessaging::WS_CAPTION,
                false,
                windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE::default(),
            );
            let win_width = rect.right - rect.left;
            let win_height = rect.bottom - rect.top;

            // 4. Disable parent window to make this modal
            let _ = windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow(parent, false);

            let instance = GetModuleHandleW(None)?;
            let class_name = w!("MsnChatOcxHostClass"); // Use same class

            // Create settings window
            let hwnd = CreateWindowExW(
                windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE::default(),
                class_name,
                w!("Chat Settings"),
                windows::Win32::UI::WindowsAndMessaging::WS_POPUPWINDOW
                    | windows::Win32::UI::WindowsAndMessaging::WS_CAPTION
                    | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                win_width,
                win_height,
                Some(parent),
                None,
                Some(instance.into()),
                None,
            )?;

            // Center settings window relative to parent
            let mut parent_rect = RECT::default();
            let mut child_rect = RECT::default();
            let _ =
                windows::Win32::UI::WindowsAndMessaging::GetWindowRect(parent, &mut parent_rect);
            let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut child_rect);
            let parent_width = parent_rect.right - parent_rect.left;
            let parent_height = parent_rect.bottom - parent_rect.top;
            let child_width = child_rect.right - child_rect.left;
            let child_height = child_rect.bottom - child_rect.top;
            let x = parent_rect.left + (parent_width - child_width) / 2;
            let y = parent_rect.top + (parent_height - child_height) / 2;
            let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowPos(
                hwnd,
                Some(HWND(std::ptr::null_mut())),
                x,
                y,
                0,
                0,
                windows::Win32::UI::WindowsAndMessaging::SWP_NOSIZE
                    | windows::Win32::UI::WindowsAndMessaging::SWP_NOZORDER,
            );

            // 5. Attach the pre-created host to the window
            host.attach(hwnd)?;

            let settings_win = Box::new(OcxWindow {
                hwnd,
                host: Some(host),
                parent: Some(parent),
                module: Some(module),
            });

            // Leak the box pointer into GWLP_USERDATA
            windows::Win32::UI::WindowsAndMessaging::SetWindowLongW(
                hwnd,
                windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
                (Box::into_raw(settings_win) as isize) as i32,
            );
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
                    windows::Win32::UI::WindowsAndMessaging::WM_COMMAND => {
                        let id = (wparam.0 & 0xFFFF) as u32;
                        match id {
                            1001 => {
                                if let Some(module) = &this.module
                                    && let Err(e) =
                                        Self::show_settings_modal(this.hwnd, module.clone())
                                {
                                    log::error!("Failed to show settings dialog: {}", e);
                                }
                            }
                            1002 => {
                                let _ = windows::Win32::UI::WindowsAndMessaging::DestroyWindow(
                                    this.hwnd,
                                );
                            }
                            _ => {}
                        }
                        return LRESULT(0);
                    }
                    WM_DESTROY => {
                        let parent = this.parent;
                        this.host = None;

                        if let Some(parent) = parent {
                            // Clear USERDATA first to prevent use-after-free
                            windows::Win32::UI::WindowsAndMessaging::SetWindowLongW(
                                window,
                                windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
                                0,
                            );
                            let _boxed = Box::from_raw(this as *mut Self);
                            let _ = windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow(
                                parent, true,
                            );
                            let _ = windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(
                                parent,
                            );
                            let _ =
                                windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(Some(parent));
                        } else {
                            PostQuitMessage(0);
                        }
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
