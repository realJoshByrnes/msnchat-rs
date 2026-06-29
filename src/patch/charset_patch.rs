//! Charset hook and patch implementations for `MsnChat45.ocx`.
//!
//! Replaces manual UTF-8 encoding and decoding routines with standard UTF-8 / CESU-8 hybrid
//! logic, ensuring proper transmission and rendering of emojis and Unicode.

use super::module_info::ModuleInfo;
use std::ffi::c_void;

type FnOperatorNew = unsafe extern "cdecl" fn(size: usize) -> *mut c_void;
static mut OPERATOR_NEW: Option<FnOperatorNew> = None;

/// # Safety
///
/// This function is unsafe because it modifies executable code in memory and installs hooks.
pub unsafe fn apply(info: &ModuleInfo) -> Result<(), String> {
    // Resolve operator new address (0x372365c7)
    unsafe {
        OPERATOR_NEW = Some(std::mem::transmute::<*mut c_void, FnOperatorNew>(
            info.resolve(0x372365c7),
        ));
    }

    // 1. Hook sub_3723E659 (Manual UTF-8 encoder used for outgoing text/styles)
    let target = info.resolve(0x3723e659);
    let _ = unsafe { super::hook(target, detour_sub_3723e659 as *mut c_void)? };

    // 2. Hook sub_3723E7A4 (Manual UTF-8 decoder used for incoming text)
    let target = info.resolve(0x3723e7a4);
    let _ = unsafe { super::hook(target, detour_sub_3723e7a4 as *mut c_void)? };

    log::info!("Manual UTF-8 / CESU-8 encoder/decoder patches successfully applied");
    Ok(())
}

#[unsafe(no_mangle)]
unsafe extern "cdecl" fn detour_sub_3723e659(
    lp_string: *const u16,
    a2: i32,
    a3: *mut *mut u8,
    a4: *mut i32,
    a5: i8,
) -> i8 {
    if lp_string.is_null() || a3.is_null() {
        return 0;
    }

    // 1. Determine input string length
    let len = if a2 > 0 {
        a2 as usize
    } else {
        let mut l = 0;
        while unsafe { *lp_string.add(l) } != 0 {
            l += 1;
        }
        l
    };

    let wide_slice = unsafe { std::slice::from_raw_parts(lp_string, len) };
    let mut utf8_bytes = String::from_utf16_lossy(wide_slice).into_bytes();

    // 2. Perform escaping if a5 is non-zero
    if a5 != 0 {
        let mut escaped = Vec::with_capacity(utf8_bytes.len());
        for &b in &utf8_bytes {
            match b {
                0 => {
                    escaped.push(b'\\');
                    escaped.push(b'0');
                }
                0x0A => {
                    escaped.push(b'\\');
                    escaped.push(b'n');
                }
                0x0D => {
                    escaped.push(b'\\');
                    escaped.push(b'r');
                }
                0x09 => {
                    escaped.push(b'\\');
                    escaped.push(b't');
                }
                0x20 => {
                    escaped.push(b'\\');
                    escaped.push(b'b');
                }
                0x2C => {
                    escaped.push(b'\\');
                    escaped.push(b'c');
                }
                0x5C => {
                    escaped.push(b'\\');
                    escaped.push(b'\\');
                }
                _ => {
                    escaped.push(b);
                }
            }
        }
        utf8_bytes = escaped;
    }

    // 3. Allocate using operator new
    if let Some(op_new) = unsafe { OPERATOR_NEW } {
        let alloc_size = utf8_bytes.len() + 1;
        let ptr = unsafe { op_new(alloc_size) as *mut u8 };
        if ptr.is_null() {
            return 0;
        }

        // Copy bytes and null-terminate
        unsafe {
            std::ptr::copy_nonoverlapping(utf8_bytes.as_ptr(), ptr, utf8_bytes.len());
            *ptr.add(utf8_bytes.len()) = 0;
        }

        unsafe {
            *a3 = ptr;
            if !a4.is_null() {
                *a4 = utf8_bytes.len() as i32;
            }
        }
        1
    } else {
        0
    }
}

#[unsafe(no_mangle)]
unsafe extern "cdecl" fn detour_sub_3723e7a4(
    lp_string: *const u8,
    a2: i32,
    a3: *mut *mut u16,
    a4: *mut i32,
    a5: i8,
) -> i8 {
    if lp_string.is_null() || a3.is_null() {
        return 0;
    }

    // 1. Determine input string length
    let len = if a2 > 0 {
        a2 as usize
    } else {
        let mut l = 0;
        while unsafe { *lp_string.add(l) } != 0 {
            l += 1;
        }
        l
    };

    let mut input_bytes = unsafe { std::slice::from_raw_parts(lp_string, len) }.to_vec();

    // 2. Perform unescaping if a5 is non-zero
    if a5 != 0 {
        let mut unescaped = Vec::with_capacity(input_bytes.len());
        let mut i = 0;
        while i < input_bytes.len() {
            if input_bytes[i] == b'\\' && i + 1 < input_bytes.len() {
                let next = input_bytes[i + 1];
                match next {
                    b'0' => unescaped.push(0),
                    b'n' => unescaped.push(0x0A),
                    b'r' => unescaped.push(0x0D),
                    b't' => unescaped.push(0x09),
                    b'b' => unescaped.push(0x20), // space
                    b'c' => unescaped.push(0x2C), // comma
                    b'\\' => unescaped.push(b'\\'),
                    _ => unescaped.push(next),
                }
                i += 2;
            } else {
                unescaped.push(input_bytes[i]);
                i += 1;
            }
        }
        input_bytes = unescaped;
    }

    // 3. Convert clean bytes (which could be standard UTF-8 or legacy CESU-8) to UTF-16
    let utf16_chars = decode_utf8_cesu8(&input_bytes);

    // 4. Allocate using operator new (size in bytes)
    if let Some(op_new) = unsafe { OPERATOR_NEW } {
        let alloc_size = (utf16_chars.len() + 1) * 2;
        let ptr = unsafe { op_new(alloc_size) as *mut u16 };
        if ptr.is_null() {
            return 0;
        }

        // Copy chars and null-terminate
        unsafe {
            std::ptr::copy_nonoverlapping(utf16_chars.as_ptr(), ptr, utf16_chars.len());
            *ptr.add(utf16_chars.len()) = 0;
        }

        unsafe {
            *a3 = ptr;
            if !a4.is_null() {
                *a4 = utf16_chars.len() as i32;
            }
        }
        1
    } else {
        0
    }
}

/// Decodes a byte sequence containing standard UTF-8 and/or CESU-8 encoded surrogate pairs
/// into a standard UTF-16 code unit vector.
fn decode_utf8_cesu8(bytes: &[u8]) -> Vec<u16> {
    let mut utf16 = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b1 = bytes[i];
        if b1 <= 0x7F {
            utf16.push(b1 as u16);
            i += 1;
        } else if (b1 & 0xE0) == 0xC0 {
            // 2-byte UTF-8
            if i + 1 < bytes.len() {
                let b2 = bytes[i + 1];
                let val = (((b1 & 0x1F) as u16) << 6) | (b2 & 0x3F) as u16;
                utf16.push(val);
                i += 2;
            } else {
                utf16.push(b1 as u16);
                i += 1;
            }
        } else if (b1 & 0xF0) == 0xE0 {
            // 3-byte UTF-8 / CESU-8
            if i + 2 < bytes.len() {
                let b2 = bytes[i + 1];
                let b3 = bytes[i + 2];
                let val = (((b1 & 0x0F) as u16) << 12) | (((b2 & 0x3F) as u16) << 6) | (b3 & 0x3F) as u16;
                utf16.push(val);
                i += 3;
            } else {
                utf16.push(b1 as u16);
                i += 1;
            }
        } else if (b1 & 0xF8) == 0xF0 {
            // 4-byte UTF-8 (standard emojis / surrogate-inducing BMP characters)
            if i + 3 < bytes.len() {
                let b2 = bytes[i + 1];
                let b3 = bytes[i + 2];
                let b4 = bytes[i + 3];
                let cp = (((b1 & 0x07) as u32) << 18)
                    | (((b2 & 0x3F) as u32) << 12)
                    | (((b3 & 0x3F) as u32) << 6)
                    | (b4 & 0x3F) as u32;
                if cp >= 0x10000 && cp <= 0x10FFFF {
                    // Split into high and low surrogates
                    let adjusted = cp - 0x10000;
                    let high = ((adjusted >> 10) as u16) + 0xD800;
                    let low = ((adjusted & 0x3FF) as u16) + 0xDC00;
                    utf16.push(high);
                    utf16.push(low);
                } else {
                    utf16.push(0xFFFD); // replacement char
                }
                i += 4;
            } else {
                utf16.push(b1 as u16);
                i += 1;
            }
        } else {
            // Fallback for invalid sequences
            utf16.push(b1 as u16);
            i += 1;
        }
    }
    utf16
}
