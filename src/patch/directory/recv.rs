use super::super::module_info::ModuleInfo;
use std::ffi::c_void;

type OnLineReceivedFn =
    unsafe extern "thiscall" fn(this: *mut c_void, line: *const std::ffi::c_char, len: u32) -> i8;

static mut TRAMPOLINE: Option<OnLineReceivedFn> = None;

/// # Safety
///
/// This function is unsafe because it modifies global state (`TRAMPOLINE`) and performs inline hooking of target addresses.
/// The caller must ensure that `info` contains a valid module reference and that the hook resides in a safe, write-enabled memory page.
pub unsafe fn apply(info: &ModuleInfo) -> Result<(), String> {
    let target = info.resolve(0x372327dc);
    let trampoline =
        unsafe { crate::patch::hook(target, hook_on_line_received_ds as *mut c_void) }?;

    unsafe {
        TRAMPOLINE = Some(std::mem::transmute::<*mut c_void, OnLineReceivedFn>(
            trampoline,
        ));
    }

    Ok(())
}

unsafe extern "thiscall" fn hook_on_line_received_ds(
    this: *mut c_void,
    line: *const std::ffi::c_char,
    len: u32,
) -> i8 {
    if !line.is_null() {
        unsafe {
            let cstr = std::ffi::CStr::from_ptr(line);
            if let Ok(text) = cstr.to_str() {
                log::info!("{}", text);
            }
        }
    }

    unsafe {
        if let Some(orig) = TRAMPOLINE {
            orig(this, line, len)
        } else {
            1
        }
    }
}
