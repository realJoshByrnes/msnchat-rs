use msnchat_bindings::ChatSettings;
use std::{
    os::raw::c_void,
    sync::{Arc, LazyLock, Mutex, Weak},
};
use windows::{
    Win32::{
        Foundation::*,
        Graphics::Gdi::{COLOR_WINDOW, GetSysColorBrush},
        System::{
            LibraryLoader::GetModuleHandleW,
            Ole::{
                IOleClientSite, IOleInPlaceActiveObject, IOleInPlaceObject, IOleObject,
                OLEIVERB_SHOW,
            },
        },
        UI::WindowsAndMessaging::*,
    },
    core::*,
};

use crate::com::shared::create_host_wrappers;
static mut IN_PLACE_OBJECT: Option<IOleInPlaceObject> = None;

static SETTINGS_DIALOG: LazyLock<Mutex<Weak<DialogWindow>>> =
    LazyLock::new(|| Mutex::new(Weak::new()));

pub unsafe fn register_window_class() -> Result<()> {
    let hinstance = unsafe { GetModuleHandleW(None)? };

    let wc = WNDCLASSW {
        hInstance: hinstance.into(),
        lpszClassName: w!("MyDialogWindowClass"),
        lpfnWndProc: Some(dialog_proc),
        style: CS_HREDRAW | CS_VREDRAW,
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW).unwrap() },
        hbrBackground: unsafe { GetSysColorBrush(COLOR_WINDOW) },
        ..Default::default()
    };

    if unsafe { RegisterClassW(&wc) } == 0 {
        return Err(Error::from_thread());
    }

    Ok(())
}

pub struct DialogWindow {
    hwnd: HWND,
}

unsafe impl Send for DialogWindow {}
unsafe impl Sync for DialogWindow {}

impl DialogWindow {
    pub fn new(title: &str) -> Result<Arc<Self>> {
        unsafe {
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("MyDialogWindowClass"),
                &HSTRING::from(title),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                450,
                700,
                None,
                None,
                Some(GetModuleHandleW(None)?.into()),
                None,
            );

            if hwnd.is_err() {
                return Err(Error::from_thread());
            }

            let hwnd = hwnd.unwrap();

            let window = Arc::new(Self { hwnd });

            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Arc::into_raw(window.clone()) as i32);
            let prev_proc = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, dialog_proc as i32);
            let _ = SetPropW(
                hwnd,
                w!("prev_proc"),
                Some(HANDLE(prev_proc as *mut c_void)),
            );

            let chat_settings = ChatSettings::create()?;
            let _ = chat_settings.set_redirect_url(Some("http://chat.msn.com/"));
            let embedded_ole_object = chat_settings.cast::<IOleObject>()?;
            // NOTE: KEEP FOREVER!!!
            let wrappers = Box::new(create_host_wrappers(hwnd));
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
            let hr =
                embedded_ole_object.query(&IOleInPlaceActiveObject::IID, &mut in_place_object_ptr);
            if !hr.is_ok() || in_place_object_ptr.is_null() {
                return Err(Error::from(hr));
            }
            let in_place_object = IOleInPlaceObject::from_raw(in_place_object_ptr as *mut _);
            IN_PLACE_OBJECT = Some(in_place_object);

            embedded_ole_object.DoVerb(
                OLEIVERB_SHOW.0,
                std::ptr::null_mut(),
                &ole_client_site,
                0,    // LINDEX, reserved
                hwnd, // Parent window handle
                &rect,
            )?;

            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = crate::UpdateWindow(hwnd);
            Ok(window)
        }
    }

    pub fn bring_to_front(&self) {
        unsafe {
            let _ = SetForegroundWindow(self.hwnd);
        }
    }

    pub fn show(&self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_SHOW);
        }
    }
}

unsafe extern "system" fn dialog_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CLOSE => {
            // SAFELY retrieve the Arc<DialogWindow> from GWLP_USERDATA
            let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const DialogWindow;

            if !ptr.is_null() {
                // Reconstruct the Arc to decrement the ref count
                let _arc = unsafe { Arc::from_raw(ptr) };

                // Clear the singleton if it matches
                if let Ok(mut w) = SETTINGS_DIALOG.lock() {
                    if let Some(existing) = w.upgrade() {
                        if Arc::ptr_eq(&existing, &_arc) {
                            *w = Weak::new();
                        }
                    }
                }
            }

            let _ = unsafe { DestroyWindow(hwnd) };
            LRESULT(0)
        }
        WM_DESTROY => LRESULT(0),
        WM_SIZE => LRESULT(0),
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

pub fn show_settings_dialog() -> Result<()> {
    let mut weak = SETTINGS_DIALOG.lock().unwrap();

    if let Some(existing) = weak.upgrade() {
        existing.bring_to_front();
    } else {
        let dialog = DialogWindow::new("Settings")?;
        *weak = Arc::downgrade(&dialog);
        dialog.show();
    }

    Ok(())
}
