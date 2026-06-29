use crate::patch::module_info::ModuleInfo;
use std::ffi::c_void;

type OnLineReceivedFn =
    unsafe extern "thiscall" fn(this: *mut c_void, line: *const std::ffi::c_char, len: u32) -> i8;

static mut TRAMPOLINE: Option<OnLineReceivedFn> = None;

/// # Safety
///
/// This function is unsafe because it modifies global state (`TRAMPOLINE`) and performs inline hooking of target addresses.
/// The caller must ensure that `info` contains a valid module reference and that the hook resides in a safe, write-enabled memory page.
pub unsafe fn apply(info: &ModuleInfo) -> Result<(), String> {
    let target = info.resolve(0x37231e07);
    let trampoline =
        unsafe { crate::patch::hook(target, hook_on_line_received_cs as *mut c_void) }?;

    unsafe {
        TRAMPOLINE = Some(std::mem::transmute::<*mut c_void, OnLineReceivedFn>(
            trampoline,
        ));
    }

    Ok(())
}

unsafe extern "thiscall" fn hook_on_line_received_cs(
    this: *mut c_void,
    line: *const std::ffi::c_char,
    len: u32,
) -> i8 {
    let mut trimmed_line = Vec::new();
    let mut final_line = line;
    let mut final_len = len;

    // TODO: Temporary workaround for IRC7 issue #199: trim lines over 510 bytes to prevent OCX logout.
    if !line.is_null() && len > 512 {
        unsafe {
            let slice = std::slice::from_raw_parts(line as *const u8, len as usize);
            let ends_with_crlf = slice.ends_with(b"\r\n");
            
            let payload_limit = 510;
            let trimmed_payload = if ends_with_crlf {
                &slice[..std::cmp::min(slice.len() - 2, payload_limit)]
            } else {
                &slice[..std::cmp::min(slice.len(), payload_limit)]
            };

            trimmed_line.extend_from_slice(trimmed_payload);
            if ends_with_crlf {
                trimmed_line.extend_from_slice(b"\r\n");
            }
            trimmed_line.push(0);

            let text_lossy = String::from_utf8_lossy(trimmed_payload);
            log::warn!("⚠️🚨 Received line over 510 bytes (original length: {}). Trimming to 510 bytes! Preview: {} 🚨⚠️", len, text_lossy);

            final_line = trimmed_line.as_ptr() as *const std::ffi::c_char;
            final_len = (trimmed_line.len() - 1) as u32;
        }
    } else if !line.is_null() {
        unsafe {
            let cstr = std::ffi::CStr::from_ptr(line);
            if let Ok(text) = cstr.to_str() {
                log::info!("{}", text);
            }
        }
    }

    unsafe {
        if let Some(orig) = TRAMPOLINE {
            orig(this, final_line, final_len)
        } else {
            1
        }
    }
}
