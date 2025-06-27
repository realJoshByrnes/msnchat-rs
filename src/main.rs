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

use rand::Rng;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoUninitialize, IDispatch,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleA;
use windows::Win32::System::Ole::{IOleInPlaceObject, IOleInPlaceSiteEx, OLEIVERB_SHOW};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::*;

#[macro_use]
mod com;

use crate::com::helpers::set_string_property;
use crate::com::shared::create_host_wrappers;

// // Define your custom interface (as above)
// #[interface("12345678-ABCD-EF01-2345-6789ABCDEF00")]
// unsafe trait IMyInterface: IUnknown {
//     fn my_method(&self, value: u32) -> HRESULT;
//     fn get_name(&self, name: *mut BSTR) -> HRESULT;
// }

// // Your Rust struct that will serve as the COM object.
// // #[windows::implement] generates the necessary COM infrastructure for it.
// #[windows::implement(IMyInterface)] // List the interfaces this struct implements
// struct MyComObject {
//     // You can have any internal data here.
//     // windows-rs handles the ref counting and QueryInterface for you.
//     internal_counter: u32,
//     object_name: String,
// }

// // Implement the methods of your interfaces directly on your struct.
// // The #[windows::implement] macro links these methods to the vtable.
// impl IMyInterface_Impl for MyComObject {
//     unsafe fn my_method(&self, value: u32) -> HRESULT {
//         println!("MyComObject::MyMethod called with value: {}", value);
//         // You can access and modify internal state:
//         // self.internal_counter += value; // Note: self is &self, cannot modify directly
//         // For mutable access, use a RefCell or Mutex if `MyComObject` itself needs to be immutable.
//         // Or, define methods to take `&mut self` if allowed by the COM interface.
//         // For simplicity in this example, assume internal state is not mutated by `&self` methods.
//         S_OK
//     }

//     unsafe fn get_name(&self, name: *mut BSTR) -> HRESULT {
//         println!("MyComObject::GetName called.");
//         // Allocate a BSTR from your internal String and return it.
//         // The caller is responsible for freeing this BSTR.
//         let bstr = BSTR::from(&self.object_name);
//         unsafe {
//             *name = bstr;
//         }
//         S_OK
//     }
// }

// fn create_and_use_object() {
//     // ✅ CORRECT USAGE ✅
//     // Create your concrete struct, then use .into() to get a ComPtr to the interface.
//     let my_object: IMyInterface = MyComObject {
//         internal_counter: 0,
//         object_name: "Hello from Rust COM".to_string(),
//     }.into(); // `into()` generates the ComPtr<IMyInterface>

//     // Now you can call methods on `my_object` (which is a ComPtr<IMyInterface>)
//     unsafe { my_object.my_method(123).ok(); }
// }

// Define a unique window class name
const WINDOW_CLASS_NAME: &[u8] = b"MyActiveXHostWindow\0";

fn main() -> Result<()> {
    // 1. Initialize COM (required for ActiveX controls)
    // CoInitializeEx(pvReserved: PCVOID, dwCoInit: COINIT)
    // COINIT_APARTMENTTHREADED is typically used for UI-related COM objects like ActiveX controls.
    unsafe {
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        if !hr.is_ok() {
            return Err(Error::from(hr));
        }
    }

    // 2. Register the Window Class
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
        hbrBackground: HBRUSH(0 as *mut _), // Default window background
        lpszMenuName: PCSTR(0 as *const u8), // No menu
        lpszClassName: PCSTR(WINDOW_CLASS_NAME.as_ptr()),
        hIconSm: HICON(std::ptr::null_mut()), // No small icon
    };

    unsafe {
        if RegisterClassExA(&wc) == 0 {
            return Err(Error::from_win32());
        }
    }

    // 3. Create the Window
    let hwnd = unsafe {
        CreateWindowExA(
            WS_EX_OVERLAPPEDWINDOW,                        // Extended window style
            PCSTR(WINDOW_CLASS_NAME.as_ptr()),             // Window class name
            PCSTR("Hello, world! (from Rust)\0".as_ptr()), // Window title
            WS_OVERLAPPEDWINDOW,                           // Window style
            100,                                           // X position
            100,                                           // Y position
            800,                                           // Width
            600,                                           // Height
            None,                                          // Parent window (none)
            None,                                          // Menu (none)
            Some(h_instance.into()),                       // Instance handle
            None,                                          // Additional creation data
        )
    };

    let hwnd = match hwnd {
        Ok(h) => h,
        Err(_e) => return Err(Error::from_win32()),
    };

    unsafe {
        let ole_object_result = create_activex_control(hwnd, h_instance);
        let embedded_ole_object = match ole_object_result {
            Ok(obj) => {
                println!("ActiveX control instantiated successfully!");
                obj
            }
            Err(e) => {
                eprintln!("Failed to instantiate ActiveX control: {:?}", e);
                // Handle error, maybe return from main or display error message
                return Err(e);
            }
        };

        // Use the correct function to create your host site with the proper vtables
        // Create the wrappers and shared state
        //let wrappers = create_host_wrappers();
        // NOTE: KEEP FOREVER!!!
        let wrappers = Box::new(create_host_wrappers(hwnd));

        // Pass wrappers.client_site to SetClientSite (as IOleClientSite)
        let ole_client_site =
            windows::Win32::System::Ole::IOleClientSite::from_raw(wrappers.client_site as *mut _);

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
        in_place_object.SetObjectRects(&rect, &rect)?; // Pass same rect for position and clip

        // Get IDispatch for the control
        let mut dispatch_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let hr = embedded_ole_object.query(&IDispatch::IID, &mut dispatch_ptr);
        if !hr.is_ok() || dispatch_ptr.is_null() {
            return Err(Error::from(hr));
        }
        let dispatch = IDispatch::from_raw(dispatch_ptr as *mut _);

        // Set a property on the control (e.g., URL)
        let server = "localhost:6667\0";
        let hr = set_string_property(&dispatch, "Server", server);
        if hr.is_err() {
            eprintln!("Failed to set property on ActiveX control: {:?}", hr);
        } else {
            println!("Successfully set property on ActiveX control.");
        }

        let mut rng = rand::rng();
        let random_number: u32 = rng.random_range(1..=9999); // 1 to 100 inclusive
        let nickname = format!("User{}-rs\0", random_number);
        let hr = set_string_property(&dispatch, "NickName", &nickname);
        if hr.is_err() {
            eprintln!("Failed to set property on ActiveX control: {:?}", hr);
        } else {
            println!("Successfully set property on ActiveX control.");
        }

        let room = "The Lobby\0";
        let hr = set_string_property(&dispatch, "RoomName", room);
        if hr.is_err() {
            eprintln!("Failed to set property on ActiveX control: {:?}", hr);
        } else {
            println!("Successfully set property on ActiveX control.");
        }

        // We need this set, otherise the control will crash when it tries to read it while loading a Whisper Window
        let base = "\0";
        let hr = set_string_property(&dispatch, "BaseUrl", base);
        if hr.is_err() {
            eprintln!("Failed to set property on ActiveX control: {:?}", hr);
        } else {
            println!("Successfully set property on ActiveX control.");
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

    // 5. Message Loop
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

    // 6. Uninitialize COM (important for proper cleanup)
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
                let mut text = *b"Uhh... There should be MSN Chat here, not this text!\0";
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
        _ => {
            // Default message processing
            unsafe { DefWindowProcA(hwnd, msg, wparam, lparam) }
        }
    }
}

fn create_activex_control(
    _parent_hwnd: HWND,
    _h_instance: HMODULE,
) -> Result<windows::Win32::System::Ole::IOleObject> {
    let clsid_web_browser = &windows::core::GUID::from_u128(
        // 0x8856F961_340A_11D0_A96B_00C04FD705A2u128,
        0xF58E1CEF_A068_4c15_BA5E_587CAF3EE8C6,
    );

    // Create an instance of the control
    use windows::Win32::System::Ole::IOleObject;

    unsafe {
        // Create an instance of the WebBrowser control and get IOleObject directly
        let ole_object: IOleObject =
            CoCreateInstance(clsid_web_browser, None, CLSCTX_INPROC_SERVER)?;

        print!("Created ActiveX control: {:?}\n", ole_object);

        // At this point, you have an IOleObject pointer, but the control is not yet "sited" in your window.
        // This is where the real complexity begins.

        Ok(ole_object)
    }
}
