use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassInfoExA, GetClassInfoExW, RegisterClassExA, RegisterClassExW, UnregisterClassA,
    UnregisterClassW, WNDCLASSEXA, WNDCLASSEXW,
};
use windows::core::{PCSTR, PCWSTR};

/// Checks if a PCSTR string ends with 'W'
///
/// # Safety
/// Requires `name` to be a valid pointer to a null-terminated ANSI string.
pub unsafe fn is_wide_class(name: PCSTR) -> bool {
    let bytes = unsafe { std::ffi::CStr::from_ptr(name.0 as *const i8).to_bytes() };
    bytes.last() == Some(&b'W')
}

/// Helper function to convert PCSTR to a null-terminated UTF-16 Vec
///
/// # Safety
/// Requires `text` to be a valid pointer to a null-terminated string.
pub unsafe fn pcstr_to_utf16(text: PCSTR) -> Vec<u16> {
    let str_ref = unsafe {
        std::ffi::CStr::from_ptr(text.0 as *const i8)
            .to_str()
            .unwrap_or("")
    };
    let mut utf16: Vec<u16> = str_ref.encode_utf16().collect();
    utf16.push(0);
    utf16
}

/// Superclasses an existing window class, handling both ANSI and Unicode targets.
///
/// # Safety
/// Relies on successful internal Win32 API calls (`GetClassInfoExA`, `GetClassInfoExW`, etc.)
pub unsafe fn superclass_window(
    h_instance: HINSTANCE,
    base_class: PCSTR,
    target_class: PCSTR,
    register: bool,
    extra_wnd_bytes: Option<&mut i32>,
) -> bool {
    let is_wide = unsafe { is_wide_class(base_class) };

    if is_wide {
        let base_w = unsafe { pcstr_to_utf16(base_class) };
        let target_w = unsafe { pcstr_to_utf16(target_class) };

        let pcw_base = PCWSTR(base_w.as_ptr());
        let pcw_target = PCWSTR(target_w.as_ptr());

        let mut wcx = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            ..Default::default()
        };

        if !register {
            if unsafe { GetClassInfoExW(Some(h_instance), pcw_target, &mut wcx) }.is_ok() {
                return unsafe { UnregisterClassW(pcw_target, Some(h_instance)) }.is_ok();
            }
            return true;
        }

        if unsafe { GetClassInfoExW(Some(h_instance), pcw_target, &mut wcx) }.is_ok() {
            if let Some(extra) = extra_wnd_bytes {
                *extra = wcx.cbWndExtra;
            }
            return true;
        }

        if unsafe { GetClassInfoExW(None, pcw_base, &mut wcx) }.is_err() {
            log::error!("Failed to get class info for base class {:?}", base_class);
            return false;
        }

        wcx.lpszClassName = pcw_target;
        wcx.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
        wcx.hInstance = h_instance;

        if let Some(extra) = extra_wnd_bytes {
            wcx.cbWndExtra += *extra;
            *extra = wcx.cbWndExtra;
        }

        let result = unsafe { RegisterClassExW(&wcx) };
        if result == 0 {
            log::error!("Failed to register class {:?}", target_class);
            false
        } else {
            log::trace!("Successfully registered class {:?}", target_class);
            true
        }
    } else {
        let mut wcx = WNDCLASSEXA {
            cbSize: std::mem::size_of::<WNDCLASSEXA>() as u32,
            ..Default::default()
        };

        unsafe {
            if !register {
                if GetClassInfoExA(Some(h_instance), target_class, &mut wcx).is_ok() {
                    return UnregisterClassA(target_class, Some(h_instance)).is_ok();
                }
                return true;
            }

            if GetClassInfoExA(Some(h_instance), target_class, &mut wcx).is_ok() {
                if let Some(extra) = extra_wnd_bytes {
                    *extra = wcx.cbWndExtra;
                }
                return true;
            }

            if GetClassInfoExA(None, base_class, &mut wcx).is_err() {
                log::error!("Failed to get class info for base class {:?}", base_class);
                return false;
            }

            wcx.lpszClassName = target_class;
            wcx.cbSize = std::mem::size_of::<WNDCLASSEXA>() as u32;
            wcx.hInstance = h_instance;

            if let Some(extra) = extra_wnd_bytes {
                wcx.cbWndExtra += *extra;
                *extra = wcx.cbWndExtra;
            }

            let result = RegisterClassExA(&wcx);
            if result == 0 {
                log::error!("Failed to register class {:?}", target_class);
                false
            } else {
                log::trace!("Successfully registered class {:?}", target_class);
                true
            }
        }
    }
}
