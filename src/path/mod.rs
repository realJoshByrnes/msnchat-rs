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

use crate::PSTR;
use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, HMODULE};
use windows::Win32::Storage::FileSystem::{CreateDirectoryA, GetTempPathA};

#[unsafe(no_mangle)]
pub extern "stdcall" fn get_resdll_storage(_: HMODULE, lp_filename: PSTR, n_size: u32) -> () {
    // Replacement for GetModuleFileNameA called at 0x3721481E
    // Original resulted (by default) in "C:\Windows\Downloaded Program Files\", which requires access priveliges.
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
                println!(
                    "ResDLL storage location: {}",
                    lp_filename.to_string().unwrap()
                );
            } else {
                eprintln!("Error: Couldn't create directory for Resource DLL.");
            }
        } else {
            eprintln!("Error: Not enough buffer space for Resource DLL directory.");
        }
    }
}
