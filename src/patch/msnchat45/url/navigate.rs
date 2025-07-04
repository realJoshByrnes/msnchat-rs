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

use windows::Win32::UI::{
    Shell::ShellExecuteW,
    WindowsAndMessaging::{SW_SHOWNORMAL},
};

use crate::{
    PCWSTR,
    patch::{msnchat45::reloc::PatchContext, utils::make_jmp_rel32},
    w,
};

pub fn init(ctx: &PatchContext) {
    make_jmp_rel32(ctx.adjust(0x3721783C), handle_navigate as usize);
}

// This is a stopgap, see https://github.com/realJoshByrnes/msnchat-rs/issues/10
extern "thiscall" fn handle_navigate(
    _this: *mut u32,
    new_window: bool,
    str: PCWSTR,
    str2: PCWSTR,
) -> i8 {
    unsafe {
        if str.as_ptr() == std::ptr::null() {
            #[cfg(debug_assertions)]
            println!("Navigate fn called, No URL given.");
            return 0;
        }
        let mut url = str.to_string().unwrap();

        if str2.as_ptr() != std::ptr::null() {
            url = format!("{}{}", url, str2.to_string().unwrap());
        }

        // Special fix for /credits command (archived at archive.org)
        if url == "http://communities.msn.com/MSNChatTeam/" {
            url = "https://web.archive.org/web/20050603210240if_/http://groups.msn.com/msnchatteam".to_string();
        }

        // Convert to PCWSTR
        let wide_url: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
        let pcwstr_url = PCWSTR(wide_url.as_ptr());

        if !new_window {
            #[cfg(debug_assertions)]
            println!("Unable to \"move\" to new location: {}", url);
        } else {
            #[cfg(debug_assertions)]
            println!("Opening URL: {}", url);
            ShellExecuteW(None, w!("open"), pcwstr_url, None, None, SW_SHOWNORMAL);
        }
    }
    1
}
