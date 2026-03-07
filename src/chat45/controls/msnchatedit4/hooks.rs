use crate::patch::module_info::ModuleInfo;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{Arc, Mutex, OnceLock};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleA;
use windows::core::PCSTR;

use super::MSNChatEdit4;
use super::layout::MemoryLayout;

const OFFSET_PARENT_HWND: usize = 4;
const OFFSET_IS_RICHEDIT20: usize = 156;
const OFFSET_CHILD_HWND: usize = 160;
const OFFSET_CONTEXT_MENU: usize = 216;

const SUBCLASS_ID_EDIT4: usize = 1;
const CREATE_WINDOW_OK: i32 = 0;
const CREATE_WINDOW_FAIL: i32 = -1;

const ADDR_EDIT4_VTABLE: usize = 0x37203FD0;
const ADDR_EDIT4_CTOR: usize = 0x37226403;
const ADDR_EDIT4_CREATE_WINDOW: usize = 0x37225F94;
const ADDR_EDIT4_DTOR: usize = 0x37225931;
const ADDR_EDIT4_WND_PROC: usize = 0x3721FEDA;

static CONTROLS: OnceLock<Mutex<HashMap<usize, Arc<Mutex<MSNChatEdit4>>>>> = OnceLock::new();
static VTABLE_PTR: OnceLock<usize> = OnceLock::new();
static WINDOW_PROC: OnceLock<usize> = OnceLock::new();

#[allow(dead_code)]
unsafe extern "system" fn hook_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> i32 {
    unsafe {
        let Some(&orig_proc) = WINDOW_PROC.get() else {
            log::error!("MSNChatEdit4 hook_window_proc called before WINDOW_PROC was initialized");
            return 0;
        };

        let wnd_proc: unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> i32 =
            std::mem::transmute(orig_proc);

        // Filter out noisy messages like WM_NCHITTEST, WM_SETCURSOR, WM_MOUSEMOVE, etc.
        let is_noisy = matches!(
            msg,
            0x0200
                | 0x0084
                | 0x0020
                | 0x0113
                | 0x000F
                | 0x0085
                | 0x0014
                | 0x0211
                | 0x0007
                | 0x0008
                | 0x0100
                | 0x0101
                | 0x0102
        );

        if !is_noisy {
            log::trace!(
                "Edit4 WNDPROC MSG: {:#x}, w: {:?}, l: {:?}",
                msg,
                wparam,
                lparam
            );
        }

        wnd_proc(hwnd, msg, wparam, lparam)
    }
}

unsafe extern "system" fn rust_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id_subclass: usize,
    ref_data: usize,
) -> windows::Win32::Foundation::LRESULT {
    use super::MSNChatEdit4Layout;
    use windows::Win32::UI::Input::KeyboardAndMouse::VK_RETURN;
    use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass};
    use windows::Win32::UI::WindowsAndMessaging::GetDlgCtrlID;
    use windows::Win32::UI::WindowsAndMessaging::{GetWindowTextLengthW, GetWindowTextW};
    use windows::Win32::UI::WindowsAndMessaging::{WM_KEYDOWN, WM_NCDESTROY};
    use windows::core::PCWSTR;

    unsafe {
        if msg == WM_NCDESTROY {
            let _ = RemoveWindowSubclass(hwnd, Some(rust_subclass_proc), _id_subclass);
            return DefSubclassProc(hwnd, msg, wparam, lparam);
        }

        if msg == WM_KEYDOWN && wparam.0 == VK_RETURN.0 as usize {
            let layout_ptr = ref_data as *mut MSNChatEdit4Layout;
            if !layout_ptr.is_null() {
                let is_richedit20 = (*layout_ptr).is_richedit20_flag != 0;
                if is_richedit20 {
                    let len = GetWindowTextLengthW(hwnd);
                    if len > 0 {
                        let mut buf = vec![0u16; (len + 1) as usize];
                        GetWindowTextW(hwnd, &mut buf);

                        let sink = (*layout_ptr).event_sink;
                        if !sink.is_null() {
                            let vtable = *sink;
                            let fire_event: unsafe extern "system" fn(
                                *const *const usize,
                                i32,
                                PCWSTR,
                            ) = std::mem::transmute(*vtable);
                            let id = GetDlgCtrlID((*layout_ptr).hwnd_parent);
                            fire_event(sink, id, PCWSTR(buf.as_ptr()));
                        }
                    }
                }
            }
            return windows::Win32::Foundation::LRESULT(0); // consume return
        }

        DefSubclassProc(hwnd, msg, wparam, lparam)
    }
}

fn get_controls() -> &'static Mutex<HashMap<usize, Arc<Mutex<MSNChatEdit4>>>> {
    CONTROLS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Trampoline Constructor Hook
/// Replaces the original C++ MSNChatEdit4 constructor.
unsafe extern "thiscall" fn hook_ctor(this: *mut c_void) -> *mut c_void {
    log::trace!(">>> ENTER MSNChatEdit4 ctor: {:?}", this);

    unsafe {
        let result_this = this;

        // Initialize VTable so other C++ virtual calls before/after our hook don't crash.
        if let Some(&vtable) = VTABLE_PTR.get() {
            *(result_this as *mut usize) = vtable;
        }

        // Apply our custom byte-for-byte exact memory initialization
        MemoryLayout::apply(this);

        let h_module = GetModuleHandleA(PCSTR::null()).unwrap_or_default();
        let h_instance = HINSTANCE(h_module.0);

        // Create our Rust representation
        let ctrl = MSNChatEdit4::new(h_instance);

        // Mirror the original constructor's +156 flag semantics:
        // non-zero means RichEdit20 path is active.
        *((this as usize + OFFSET_IS_RICHEDIT20) as *mut u32) = u32::from(ctrl.is_richedit20);

        match get_controls().lock() {
            Ok(mut map) => {
                map.insert(result_this as usize, Arc::new(Mutex::new(ctrl)));
            }
            Err(err) => {
                log::error!("MSNChatEdit4 ctor map lock poisoned: {}", err);
            }
        }

        result_this
    }
}

/// Window Creation Hook
/// Fired when the OLE container actually tells the control to instantiate its child HWND.
unsafe extern "thiscall" fn hook_create_window(
    this: *mut c_void,
    _a2: i32,
    _a3: i32,
    _a4: i32,
    _a5: i32,
) -> i32 {
    log::trace!(
        ">>> ENTER MSNChatEdit4 create_window: {:?}, {}, {}, {}, {}",
        this,
        _a2,
        _a3,
        _a4,
        _a5
    );

    let ctrl_arc = match get_controls().lock() {
        Ok(map) => map.get(&(this as usize)).cloned(),
        Err(err) => {
            log::error!("MSNChatEdit4 create_window map lock poisoned: {}", err);
            return CREATE_WINDOW_FAIL;
        }
    };

    if let Some(ctrl_arc) = ctrl_arc {
        let mut ctrl = match ctrl_arc.lock() {
            Ok(ctrl) => ctrl,
            Err(err) => {
                log::error!("MSNChatEdit4 create_window control lock poisoned: {}", err);
                return CREATE_WINDOW_FAIL;
            }
        };

        unsafe {
            let parent_hwnd_ptr = (this as usize + OFFSET_PARENT_HWND) as *const HWND;
            let parent_hwnd = *parent_hwnd_ptr;
            let id_val = this as isize;

            let h_module = GetModuleHandleA(PCSTR::null()).unwrap_or_default();
            let h_instance = HINSTANCE(h_module.0);

            if ctrl.create_window(parent_hwnd, id_val, h_instance) {
                // Completely bypass CContainedWindow::SubclassWindow.
                // We wire the HWND into our struct offset, then add our native SetWindowSubclass logic.
                let hwnd_ptr = (this as usize + OFFSET_CHILD_HWND) as *mut HWND;
                *hwnd_ptr = ctrl.hwnd;
                let _ = windows::Win32::UI::Shell::SetWindowSubclass(
                    ctrl.hwnd,
                    Some(rust_subclass_proc),
                    SUBCLASS_ID_EDIT4,
                    this as usize,
                );

                // Bind the extracted Context Menu HMENU to offset 216 so WM_CONTEXTMENU maps catch it
                let menu_ptr = (this as usize + OFFSET_CONTEXT_MENU)
                    as *mut windows::Win32::UI::WindowsAndMessaging::HMENU;
                *menu_ptr = ctrl.context_menu;

                // Call the original formatting routines to apply default colors and fonts
                ctrl.format_layout(this);
                ctrl.format_font(this);

                return CREATE_WINDOW_OK;
            }
        }
    }

    CREATE_WINDOW_FAIL
}

/// Destructor Hook
/// Cleans up the Rust state when the C++ object is destroyed.
unsafe extern "thiscall" fn hook_dtor(this: *mut c_void) {
    log::trace!(">>> ENTER MSNChatEdit4 dtor: {:?}", this);

    match get_controls().lock() {
        Ok(mut map) => {
            map.remove(&(this as usize));
        }
        Err(err) => {
            log::error!("MSNChatEdit4 dtor map lock poisoned: {}", err);
        }
    }
}

/// Applies all MinHook detours for this object lifecycle.
///
/// # Safety
/// Relies on accurately resolving offsets inside the `msnchat45.ocx` module.
pub unsafe fn apply(info: &ModuleInfo) -> Result<(), String> {
    log::info!("Patching MSNChatEdit4 Lifecycle methods...");

    VTABLE_PTR
        .set(info.resolve(ADDR_EDIT4_VTABLE) as usize)
        .map_err(|_| "Failed to set VTABLE_PTR")?;

    let ctor_target = info.resolve(ADDR_EDIT4_CTOR);
    let create_window_target = info.resolve(ADDR_EDIT4_CREATE_WINDOW);
    let dtor_target = info.resolve(ADDR_EDIT4_DTOR);
    let wnd_proc_target = info.resolve(ADDR_EDIT4_WND_PROC);

    unsafe {
        minhook::MinHook::create_hook(ctor_target, hook_ctor as *mut c_void)
            .map_err(|e| format!("MinHook create error for MSNChatEdit4 ctor: {:?}", e))?;

        minhook::MinHook::create_hook(create_window_target, hook_create_window as *mut c_void)
            .map_err(|e| {
                format!(
                    "MinHook create error for MSNChatEdit4 create_window: {:?}",
                    e
                )
            })?;
        minhook::MinHook::create_hook(dtor_target, hook_dtor as *mut c_void)
            .map_err(|e| format!("MinHook create error for MSNChatEdit4 dtor: {:?}", e))?;

        let orig_wnd_proc =
            minhook::MinHook::create_hook(wnd_proc_target, hook_window_proc as *mut c_void)
                .map_err(|e| format!("MinHook create error for MSNChatEdit4 wnd_proc: {:?}", e))?;
        WINDOW_PROC
            .set(orig_wnd_proc as usize)
            .map_err(|_| "Failed to set WINDOW_PROC")?;

        minhook::MinHook::queue_enable_hook(ctor_target)
            .map_err(|e| format!("Queue ctor: {:?}", e))?;
        minhook::MinHook::queue_enable_hook(create_window_target)
            .map_err(|e| format!("Queue create_window: {:?}", e))?;
        minhook::MinHook::queue_enable_hook(dtor_target)
            .map_err(|e| format!("Queue dtor: {:?}", e))?;
        minhook::MinHook::queue_enable_hook(wnd_proc_target)
            .map_err(|e| format!("Queue wnd_proc: {:?}", e))?;
    }

    log::info!("MSNChatEdit4 lifecycle patches queued successfully.");
    Ok(())
}
