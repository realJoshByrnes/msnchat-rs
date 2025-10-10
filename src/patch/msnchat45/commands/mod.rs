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
    core::PCWSTR,
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
            _ => {
                // Unhandled command; Re-use original handler
                let original_fn: extern "thiscall" fn(_, _, _) -> _ =
                    std::mem::transmute(ctx.adjust(0x37218743));
                original_fn(pbstr, lp_wide_char_str, a3)
            }
        }
    }
}
