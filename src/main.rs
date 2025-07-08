// msnchat-rs
// Copyright (C) 2025 Joshua Byrnes
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use msnchat_bindings::ChatFrame;
use rand::Rng;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::Com::{
    COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize, IDispatch,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleA;
use windows::Win32::System::Ole::{
    IOleClientSite, IOleInPlaceObject, IOleInPlaceSiteEx, IOleObject, OLEIVERB_SHOW,
};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::*;

#[macro_use]
mod com;
mod hacks;
mod patch;

mod control_socket;

use hacks::init_hacks;

use crate::com::shared::create_host_wrappers;
use crate::patch::msnchat45::startup::apply_patches;

// Define a unique window class name
const WINDOW_CLASS_NAME: &[u8] = b"MyActiveXHostWindow\0";

static mut IN_PLACE_OBJECT: Option<IOleInPlaceObject> = None;

fn main() -> Result<()> {
    // Register the Window Class
    let h_instance = unsafe { GetModuleHandleA(None)? };

    let wc = WNDCLASSEXA {
        cbSize: std::mem::size_of::<WNDCLASSEXA>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(wnd_proc), // Our window procedure callback
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: h_instance.into(),
        hIcon: HICON(std::ptr::null_mut()), // No icon
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW)? }, // Arrow cursor
        hbrBackground: unsafe { GetSysColorBrush(COLOR_WINDOW) }, // Default window background
        lpszMenuName: PCSTR(0 as *const u8), // No menu
        lpszClassName: PCSTR(WINDOW_CLASS_NAME.as_ptr()),
        hIconSm: HICON(std::ptr::null_mut()), // No small icon
    };

    unsafe {
        if RegisterClassExA(&wc) == 0 {
            return Err(Error::from_win32());
        }
    }

    // Create the Window
    let hwnd = unsafe {
        CreateWindowExA(
            WS_EX_OVERLAPPEDWINDOW,               // Extended window style
            PCSTR(WINDOW_CLASS_NAME.as_ptr()),    // Window class name
            PCSTR("msnchat-rs by JD\0".as_ptr()), // Window title
            WS_OVERLAPPEDWINDOW,                  // Window style
            100,                                  // X position
            100,                                  // Y position
            800,                                  // Width
            600,                                  // Height
            None,                                 // Parent window (none)
            None,                                 // Menu (none)
            Some(h_instance.into()),              // Instance handle
            None,                                 // Additional creation data
        )
    };

    let hwnd = match hwnd {
        Ok(h) => h,
        Err(_e) => return Err(Error::from_win32()),
    };

    // Initialize COM - We need this before using any COM objects
    unsafe {
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        if hr.is_err() {
            return Err(Error::from(hr));
        }
    }

    let chat_frame = ChatFrame::create()?;

    #[cfg(debug_assertions)]
    println!("Created ActiveX control.");

    // As soon as we load the Chat Control, we should be doing patches
    init_hacks();
    apply_patches();

    let _ = chat_frame.set_server(Some("dir.irc7.com:6667"));
    let _ = chat_frame.set_nick_name(Some("JD"));
    let _ = chat_frame.set_room_name(Some("The Lobby"));
    let _ = chat_frame.set_base_url(Some("http://chat.msn.com/"));
    let _ = chat_frame.set_res_dll(Some("https://web.archive.org/web/20120410044420if_/http://fdl.msn.com/public/chat/MsnChat40en-us.cab#Version=9,2,310,202"));
    let _ = chat_frame.set_message_of_the_day(Some("This is the MOTD"));
    let _ = chat_frame.set_audit_message(Some("Your IP is not <b>%1</b>."));
    let _ =
        chat_frame.set_whisper_content(Some("http://info.cern.ch/hypertext/WWW/TheProject.html"));

    // Random nickname generator to prevent clashes during testing.
    let random_number = rand::rng().random_range(1..=9999); // 1 to 100 inclusive
    let nickname = format!("User{}-rs\0", random_number);
    let _ = chat_frame.set_nick_name(Some(&nickname));

    let embedded_ole_object = chat_frame.cast::<IOleObject>()?;
    // NOTE: KEEP FOREVER!!!
    let wrappers = Box::new(create_host_wrappers(hwnd));
    unsafe {
        let ole_client_site = IOleClientSite::from_raw(wrappers.client_site as *mut _);
        embedded_ole_object.SetClientSite(&ole_client_site)?;

        // Pass wrappers.client_site to SetClientSite
        let ole_client_site = IOleClientSite::from_raw(wrappers.client_site as *mut _);

        embedded_ole_object.SetClientSite(&ole_client_site)?;

        let rect = RECT {
            left: 0,
            top: 0,
            right: 400,
            bottom: 400,
        };

        let mut in_place_object_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let hr = embedded_ole_object.query(&IOleInPlaceObject::IID, &mut in_place_object_ptr);
        if !hr.is_ok() || in_place_object_ptr.is_null() {
            return Err(Error::from(hr));
        }
        let in_place_object = IOleInPlaceObject::from_raw(in_place_object_ptr as *mut _);
        IN_PLACE_OBJECT = Some(in_place_object.clone());
        in_place_object.SetObjectRects(&rect, &rect)?; // Pass same rect for position and clip

        // Get IDispatch for the control
        let mut dispatch_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let hr = embedded_ole_object.query(&IDispatch::IID, &mut dispatch_ptr);
        if !hr.is_ok() || dispatch_ptr.is_null() {
            return Err(Error::from(hr));
        }

        embedded_ole_object.DoVerb(
            OLEIVERB_SHOW.0,
            std::ptr::null_mut(),
            &ole_client_site,
            0,    // LINDEX, reserved
            hwnd, // Parent window handle
            &rect,
        )?;

        // 4. Show and Update the Window
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = UpdateWindow(hwnd);
    }

    // Message Loop
    let mut msg = MSG::default();
    loop {
        // GetMessageA blocks until a message is available
        if unsafe { GetMessageA(&mut msg, None, 0, 0) }.as_bool() {
            unsafe {
                let _ = TranslateMessage(&msg); // Translates virtual-key messages into character messages
                DispatchMessageA(&msg); // Dispatches a message to a window procedure
            }
        } else {
            // GetMessageA returns 0 when WM_QUIT is received
            break;
        }
    }

    // Uninitialize COM (important for proper cleanup)
    unsafe {
        CoUninitialize();
    }

    Ok(())
}

// Window Procedure (Callback Function)
// This function handles messages sent to our window.
extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            // Basic drawing example
            let mut ps = PAINTSTRUCT::default();
            let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
            if !hdc.is_invalid() {
                let mut rect = unsafe {
                    let mut r = RECT::default();
                    let _ = GetClientRect(hwnd, &mut r);
                    r
                };

                let mut text = *b"Uhh... The MSN Chat Control should be here, not this text!\0";
                unsafe {
                    DrawTextA(
                        hdc,
                        &mut text,
                        &mut rect,
                        DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                    );
                }
                let _ = unsafe { EndPaint(hwnd, &ps) };
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            // Post a quit message when the window is closed
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        WM_SIZE => {
            // Notify the ActiveX control of the new size
            let mut rect = RECT::default();
            unsafe {
                if GetClientRect(hwnd, &mut rect).is_ok() {
                    if let Some(ref in_place_object) = IN_PLACE_OBJECT {
                        let _ = in_place_object.SetObjectRects(&rect, &rect);
                    }
                }
            }
            LRESULT(0)
        }
        _ => {
            // Default message processing
            unsafe { DefWindowProcA(hwnd, msg, wparam, lparam) }
        }
    }
}
