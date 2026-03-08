#![allow(unsafe_op_in_unsafe_fn)]

use super::parse_emoticon;
use crate::patch::module_info::ModuleInfo;
use std::ffi::c_void;

type EmoticonParserType =
    unsafe extern "cdecl" fn(is_wide: i32, input: *const c_void, len: u32, out_id: *mut u32) -> u32;

static mut TRAMPOLINE_EMOTICON_PARSER: Option<EmoticonParserType> = None;

unsafe extern "cdecl" fn hook_emoticon_parser(
    is_wide: i32,
    input: *const c_void,
    len: u32,
    out_id: *mut u32,
) -> u32 {
    if !out_id.is_null() {
        *out_id = 0;
    }

    if len < 2 || input.is_null() {
        return 0;
    }

    // Convert input to Rust String (max 5 chars needed to check)
    let max_len = std::cmp::min(len as usize, 5);
    let mut chars_buf = String::with_capacity(max_len);

    if is_wide != 0 {
        let ptr = input as *const u16;
        for i in 0..max_len {
            let c = *ptr.add(i) as u32;
            chars_buf.push(std::char::from_u32(c).unwrap_or('?'));
        }
    } else {
        let ptr = input as *const u8;
        for i in 0..max_len {
            chars_buf.push(*ptr.add(i) as char);
        }
    }

    if let Some(match_res) = parse_emoticon(&chars_buf) {
        if !out_id.is_null() {
            *out_id = match_res.id;
        }
        return match_res.length as u32;
    }

    0
}

/// # Safety
/// Relies on accurate ModuleInfo mapping for the PE image.
pub unsafe fn apply(info: &ModuleInfo) -> Result<(), String> {
    let target = info.resolve(0x372119ee);
    let hook = minhook::MinHook::create_hook(target, hook_emoticon_parser as *mut c_void)
        .map_err(|e| format!("MinHook create error for emoticon parser: {:?}", e))?;

    minhook::MinHook::queue_enable_hook(target)
        .map_err(|e| format!("Queue emoticon parser: {:?}", e))?;

    TRAMPOLINE_EMOTICON_PARSER = Some(std::mem::transmute::<*mut c_void, EmoticonParserType>(hook));

    Ok(())
}
