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

use windows::{
    Win32::{
        Foundation::{ERROR_ALREADY_EXISTS, HMODULE},
        Storage::FileSystem::{CreateDirectoryA, GetTempPathA},
    },
    core::PSTR,
};

use crate::patch::{
    msnchat45::{reloc::PatchContext, shared::is_allowed_domain},
    utils::make_call_rel32,
};

pub fn init(ctx: &PatchContext) {
    make_call_rel32(ctx.adjust(0x372147F3), is_allowed_domain as usize);
    make_call_rel32(ctx.adjust(0x3721481E), get_cache_folder as usize);
}

/// Replacement for GetModuleFileNameA called at 0x3721481E
///
/// The original resulted (by default) in "C:\Windows\Downloaded Program Files\MSNChat45.ocx", which requires priveliges.
///
/// This function writes the path for the ResDLL storage location to `lp_filename` and returns.
///
/// Note: The caller must ensure that `lp_filename` points to a buffer of at least `n_size` bytes in size.
///
/// # Safety
///
/// This function is safe to call from any thread context.

pub extern "stdcall" fn get_cache_folder(_: HMODULE, lp_filename: PSTR, n_size: u32) -> () {
    unsafe {
        let slice = std::slice::from_raw_parts_mut(lp_filename.as_ptr(), n_size as usize);

        let user_temp_dir_len = GetTempPathA(Some(slice));
        let cache_dir = b"\\msnchat-rs.cache\\\0";

        if user_temp_dir_len as usize + cache_dir.len() < n_size as usize {
            // Check we have room to write
            std::ptr::copy_nonoverlapping(
                cache_dir.as_ptr(),
                slice.as_mut_ptr().add(user_temp_dir_len as usize - 1),
                cache_dir.len(),
            );

            let result = match CreateDirectoryA(lp_filename, None) {
                Ok(_) => true,
                Err(e) => e.code() == ERROR_ALREADY_EXISTS.into(),
            };
            if result {
                #[cfg(debug_assertions)]
                println!(
                    "ResDLL storage location: {}",
                    lp_filename.to_string().unwrap()
                );
            } else {
                #[cfg(debug_assertions)]
                eprintln!("Error: Couldn't create directory for Resource DLL.");
            }
        } else {
            #[cfg(debug_assertions)]
            eprintln!("Error: Not enough buffer space for Resource DLL directory.");
        }
    }
}
