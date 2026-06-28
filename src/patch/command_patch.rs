use super::module_info::ModuleInfo;
use std::ffi::c_void;

static mut TRAMPOLINE: Option<FnProcessCommand> = None;

type FnProcessCommand =
    unsafe extern "thiscall" fn(this: *mut c_void, lp_wide_char_str: *const u16, a3: *mut u8) -> i8;

type FnSend = unsafe extern "thiscall" fn(
    this: *mut c_void,
    command_id: *mut c_void,
    lp_critical_section: *mut c_void,
    lp_string: *const u8,
    a5: *const u8,
    a6: *const u8,
    a7: *const u8,
    a8: *const u8,
    a9: i32,
    a10: i32,
    a11: i32,
) -> i8;

type FnAppendText = unsafe extern "thiscall" fn(
    this: *mut c_void,
    lp_string: *const u16,
    a3: i32,
    a4: i32,
    a5: i32,
    a6: i32,
) -> i32;

static mut FN_SEND: Option<FnSend> = None;
static mut FN_APPEND_TEXT: Option<FnAppendText> = None;

/// # Safety
///
/// This function is unsafe because it modifies executable code in memory and installs hooks.
pub unsafe fn apply(info: &ModuleInfo) -> Result<(), String> {
    let target = info.resolve(0x37218743);
    let trampoline = unsafe { super::hook(target, detour_process_command as *mut c_void) }?;

    unsafe {
        TRAMPOLINE = Some(std::mem::transmute::<*mut c_void, FnProcessCommand>(
            trampoline,
        ));
        FN_SEND = Some(std::mem::transmute::<*mut c_void, FnSend>(
            info.resolve(0x37230eb3),
        ));
        FN_APPEND_TEXT = Some(std::mem::transmute::<*mut c_void, FnAppendText>(
            info.resolve(0x372246f4),
        ));
    }
    Ok(())
}

#[unsafe(no_mangle)]
unsafe extern "thiscall" fn detour_process_command(
    this: *mut c_void,
    lp_wide_char_str: *const u16,
    a3: *mut u8,
) -> i8 {
    // 1. Convert command string to Rust String
    let mut len = 0;
    while unsafe { *lp_wide_char_str.add(len) } != 0 {
        len += 1;
    }
    let wide_slice = unsafe { std::slice::from_raw_parts(lp_wide_char_str, len) };
    let full_cmd = String::from_utf16_lossy(wide_slice);

    // 2. Check for custom commands
    if full_cmd.starts_with("/nick ") || full_cmd == "/nick" {
        let nick = full_cmd[5..].trim();
        if nick.is_empty() {
            unsafe {
                append_system_message(this, "Usage: /nick <new_nickname>");
            }
        } else {
            // Send NICK command
            if let Some(send_fn) = unsafe { FN_SEND } {
                let socket_writer = unsafe { (this as *mut u8).add(7480) as *mut c_void };
                let nick_c = std::ffi::CString::new(nick).unwrap();
                unsafe {
                    send_fn(
                        socket_writer,
                        0x1C as *mut c_void, // NICK command ID (28)
                        std::ptr::null_mut(),
                        nick_c.as_ptr() as *const u8,
                        std::ptr::null(),
                        std::ptr::null(),
                        std::ptr::null(),
                        std::ptr::null(),
                        0,
                        0,
                        0,
                    );
                }
            }
        }
        return 0; // Handled, clears the editbox
    } else if full_cmd == "/help" {
        unsafe {
            append_system_message(
                this,
                "Available commands: /nick, /topic, /me, /away, /clear, /credits, /version, /quit, /part, /help",
            );
        }
        return 0; // Handled, clears the editbox
    }

    // 3. Fallback to original command processor
    if let Some(trampoline) = unsafe { TRAMPOLINE } {
        unsafe { trampoline(this, lp_wide_char_str, a3) }
    } else {
        0
    }
}

unsafe fn append_system_message(this: *mut c_void, text: &str) {
    if let Some(append_fn) = unsafe { FN_APPEND_TEXT } {
        let chat_output = unsafe { (this as *mut u8).add(18400) as *mut c_void };

        // Convert text to wide string
        let mut wide_text: Vec<u16> = text.encode_utf16().collect();
        wide_text.push(0); // null terminator

        unsafe {
            append_fn(
                chat_output,
                wide_text.as_ptr(),
                0, // Indent (a3)
                7, // Color index (a4)
                0, // Bold/Italic/Underline bitmask (a5)
                0, // Unused by sub_372246F4 (a6)
            );
        }
    }
}
