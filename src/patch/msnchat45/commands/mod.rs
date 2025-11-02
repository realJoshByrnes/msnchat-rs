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

use std::{ffi::OsString, os::windows::ffi::OsStringExt};
use windows::{
    Win32::UI::WindowsAndMessaging::{MESSAGEBOX_STYLE, MessageBoxExW},
    core::{BSTR, PCWSTR},
};

pub mod version;

use crate::{
    chat_options,
    patch::{msnchat45::reloc::PatchContext, utils::make_call_rel32},
    w,
};

pub fn init(ctx: PatchContext) {
    make_call_rel32(ctx.adjust(0x3721C0E8), command_handler as usize);
}

/// Hook for the MSN Chat Control's command handler.
///
/// This function is called for every command entered into the chat control's
/// edit box. It intercepts the following commands:
///
/// - `/jd`: Displays a message box with a message from JD.
/// - `/ravi`: Fakes a kick event with a humorous message.
///
/// All other commands are passed through to the original handler.
///
/// # Panics
///
/// If the `PatchContext` singleton has not been initialized with a valid
/// base address, this function will panic when attempting to access the
/// singleton.
extern "thiscall" fn command_handler(
    pbstr: *mut u16,
    lp_wide_char_str: PCWSTR,
    a3: *mut u8,
) -> bool {
    unsafe {
        let ctx = PatchContext::get().unwrap();

        let slice = std::slice::from_raw_parts(lp_wide_char_str.as_ptr(), lp_wide_char_str.len());
        let os_string = OsString::from_wide(slice);
        let string = os_string.to_string_lossy().into_owned();

        // // HWND for the main richedit can be found at:
        // let hwnd = *(pbstr.add(9268) as *const HWND);

        #[cfg(debug_assertions)]
        println!("Command entered: {}", string);
        match string.to_ascii_lowercase().as_str() {
            "/jd" => {
                MessageBoxExW(
                    None,
                    w!("Made with ❤️ by JD"),
                    w!("About msnchat-rs"),
                    MESSAGEBOX_STYLE(0),
                    0,
                );
                return false; // Clear editbox
            }
            "/ravi" => {
                // int __thiscall IChatHistoryCtl_Add(void *this, WCHAR *strText, int nIndent, int nColor, int nFormat, int nLinesBefore)
                let chat_history_add: extern "thiscall" fn(_, PCWSTR, u32, u32, u32, u32) -> u32 =
                    std::mem::transmute(ctx.adjust(0x372246f4));
                chat_history_add(
                    pbstr.wrapping_add(9200),
                    w!(
                        "Host xgodzhand kicked Sysop_Daneel out of the chat room: Violate this, bitch!"
                    ),
                    0,
                    10,
                    1,
                    0,
                );
                return false;
            }
            "/options" => {
                let _ = chat_options::show_settings_dialog();
                return true; // This is ugly because it doesn't clear the text, but false also causes us to lose focus.
            }
            "/test" => {
                let maybe_richedit_writeline: extern "thiscall" fn(
                    _,
                    PCWSTR,
                    i32,
                    u32,
                    u8,
                    u8,
                    *const u16,
                    u32,
                    i32,
                ) -> i32 = std::mem::transmute(ctx.adjust(0x3722402B));

                // Allocate a BSTR for the font name "Marlett" and call the original
                // sequence used in the binary: this+4600, &word_37243E2C ("8"), 2,0,9,0,(int)v6,2,0
                let v6 = BSTR::from("Marlett");
                // Use the same 'this' pointer as the original snippet (this + 4600)
                let _res = maybe_richedit_writeline(
                    pbstr.wrapping_add(9200),
                    w!("8"),
                    2, // Indent level
                    0,
                    9, // (9 = gray)
                    0,
                    v6.as_ptr(), // Font name (BSTR ptr)
                    2,           // (2 = symbol)
                    0,
                );

                let _res = maybe_richedit_writeline(
                    pbstr.wrapping_add(9200),
                    w!("JD was here in 2025."),
                    0,
                    1,
                    9,
                    0,
                    std::ptr::null(), // No custom font
                    0,
                    0,
                );

                let font = BSTR::from("Courier New");
                let ascii_art = w!(
                    "<color #BA55D3>;,,,             `       '             ,,,;</color>\r\n\
<color #8A2BE2>`YES8888bo.       :     :       .od8888YES'</color>\r\n\
<color #1E90FF>  888IO8DO88b.     :   :     .d8888I8DO88</color>\r\n\
<color #00BFFF>  8LOVEY'  `Y8b.   `   '   .d8Y'  `YLOVE8</color>\r\n\
<color #32CD32> jTHEE!  .db.  Yb. '   ' .dY  .db.  8THEE!</color>\r\n\
<color #00FF00>   `888  Y88Y    `b ( ) d'    Y88Y  888'</color>\r\n\
<color #FFFF00>    8MYb  '\"        ,',        \"'  dMY8</color>\r\n\
<color #FFD700>   j8prECIOUSgf\"'   ':'   `\"?g8prECIOUSk</color>\r\n\
<color #FF69B4>     'Y'   .8'     d' 'b     '8.   'Y'</color>\r\n\
<color #FF4500>      !   .8' db  d'; ;`b  db '8.   !</color>\r\n\
<color #FF6347>         d88  `'  8 ; ; 8  `'  88b</color>\r\n\
<color #FF7F50>        d88Ib   .g8 ',' 8g.   dI88b</color>\r\n\
<color #DC143C>       :888LOVE88Y'     'Y88LOVE888: </color>\r\n\
<color #C71585>       '! THEE888'       `888THEE !'</color>\r\n\
<color #DB7093>          '8Y  `Y         Y'  Y8'</color>\r\n\
<color #DDA0DD>           Y                   Y</color>\r\n\
<color #EE82EE>           !                   !</color>\r\n"
                );

                let _res = maybe_richedit_writeline(
                    pbstr.wrapping_add(9200),
                    ascii_art,
                    0,             // Indent level
                    0x07,          // Newline + preserve control chars + enable styling
                    9,             // Base color (fallback)
                    0,             // Style flags
                    font.as_ptr(), // Fixed-width font
                    0,             // Charset
                    0,             // Apply default formatting
                );

                return false;
            }
            _ => {
                // Unhandled command; Re-use original handler
                let original_fn: extern "thiscall" fn(_, _, _) -> _ =
                    std::mem::transmute(ctx.adjust(0x37218743));
                original_fn(pbstr, lp_wide_char_str, a3)
            }
        }
    }
}
