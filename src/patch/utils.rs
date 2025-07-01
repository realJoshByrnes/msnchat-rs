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

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use windows::Win32::{
    Foundation::CloseHandle,
    System::{
        Diagnostics::Debug::FlushInstructionCache,
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, MODULEENTRY32W, Module32FirstW, Module32NextW,
            TH32CS_SNAPMODULE, TH32CS_SNAPMODULE32,
        },
        Memory::{PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS, VirtualProtect},
        Threading::{GetCurrentProcess, GetCurrentProcessId},
    },
};

/// # Safety
/// This function is unsafe because it directly modifies memory at the given address,
/// which can lead to undefined behavior if not used correctly. It is the caller's
/// responsibility to ensure that the address is valid and that the patching operation
/// does not violate any memory safety guarantees.
///
/// # Parameters
/// - `address`: The starting memory address where the patch should be applied.
/// - `patch`: A slice of bytes to be copied to the specified address.
///
/// # Returns
/// Returns `true` if the patching operation was successful, otherwise `false`.
///
/// # Errors
/// The function returns `false` in the following cases:
/// - If the `patch` is empty or `address` is zero.
/// - If changing the memory protection to writable or restoring it fails.
/// - If flushing the instruction cache fails.

pub unsafe fn patch_bytes(address: usize, patch: &[u8]) -> bool {
    if patch.is_empty() || address == 0 {
        #[cfg(debug_assertions)]
        eprintln!("patch_bytes: invalid input (empty patch or null address)");
        return false;
    }

    let dst = address as *mut u8;
    let len = patch.len();
    let mut old_protect = PAGE_PROTECTION_FLAGS(0);

    unsafe {
        // Step 1: Make memory writable
        if let Err(e) = VirtualProtect(dst as _, len, PAGE_EXECUTE_READWRITE, &mut old_protect) {
            #[cfg(debug_assertions)]
            eprintln!("VirtualProtect (enable write) failed: {:?}", e);
            return false;
        }

        // Step 2: Patch the bytes
        std::ptr::copy_nonoverlapping(patch.as_ptr(), dst, len);

        // Step 3: Restore protection
        if let Err(e) = VirtualProtect(dst as _, len, old_protect, &mut old_protect) {
            #[cfg(debug_assertions)]
            eprintln!("VirtualProtect (restore protect) failed: {:?}", e);
            return false;
        }

        // Step 4: Flush instruction cache
        if let Err(e) = FlushInstructionCache(GetCurrentProcess(), Some(dst as _), len) {
            #[cfg(debug_assertions)]
            eprintln!("FlushInstructionCache failed: {:?}", e);
            return false;
        }
    }
    true
}

/// Finds the base address of a module (DLL) in the current process by name.
///
/// # Parameters
/// - `target_name`: The name of the module to search for.
///
/// # Returns
/// Returns the base address of the module if found, otherwise `None`.

pub fn find_module_base(target_name: &str) -> Option<usize> {
    unsafe {
        let snapshot_result = CreateToolhelp32Snapshot(
            TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32,
            GetCurrentProcessId(),
        );
        let snapshot = match snapshot_result {
            Ok(handle) => handle,
            Err(e) => {
                #[cfg(debug_assertions)]
                eprintln!("CreateToolhelp32Snapshot failed: {:?}", e);
                return None;
            }
        };

        let mut entry = MODULEENTRY32W {
            dwSize: std::mem::size_of::<MODULEENTRY32W>() as u32,
            ..Default::default()
        };

        if Module32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let name = OsString::from_wide(&entry.szModule)
                    .to_string_lossy()
                    .trim_end_matches('\0')
                    .to_lowercase();

                if name == target_name.to_lowercase() {
                    let _ = CloseHandle(snapshot);
                    return Some(entry.modBaseAddr as usize);
                }

                if Module32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }

        let _ = CloseHandle(snapshot);
        None
    }
}
