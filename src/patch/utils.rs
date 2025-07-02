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
        // Make memory writable
        if let Err(e) = VirtualProtect(dst as _, len, PAGE_EXECUTE_READWRITE, &mut old_protect) {
            #[cfg(debug_assertions)]
            eprintln!("VirtualProtect (enable write) failed: {:?}", e);
            return false;
        }

        // Patch the bytes
        std::ptr::copy_nonoverlapping(patch.as_ptr(), dst, len);

        // Restore protection
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

        #[cfg(debug_assertions)]
        println!("Patched 0x{:08X}", address);
    }
    true
}

/// Encodes a UTF-8 string as a null-terminated UTF-16 string.
///
/// This function takes a UTF-8 string as input and returns a vector of
/// `u16` values, which is the UTF-16 representation of the input string.
/// The vector is null-terminated, meaning that the last element of the
/// vector is always 0.

pub fn encode_utf16z(str: &str) -> Vec<u16> {
    return str.encode_utf16().chain(std::iter::once(0)).collect();
}

/// Emits a 5-byte JMP rel32 from `src` to `dst`.
/// This assumes the distance between src and dst is within ±2 GB.

pub fn make_jmp_rel32(addr: usize, dst: usize) -> () {
    println!("Patching 0x{:08X} with JMP rel32 to 0x{:08X}", addr, dst);
    let offset = (dst as isize).wrapping_sub((addr + 5) as isize);
    let rel = (offset as i32).to_le_bytes();
    unsafe { patch_bytes(addr, &[0xE9, rel[0], rel[1], rel[2], rel[3]]) };
}

/// Emits a 5-byte CALL rel32 from `src` to `dst`.
/// This assumes the distance between src and dst is within ±2 GB.

pub fn make_call_rel32(addr: usize, dst: usize) -> () {
    println!("Patching 0x{:08X} with CALL rel32 to 0x{:08X}", addr, dst);
    let offset = (dst as isize).wrapping_sub((addr + 5) as isize);
    let rel = (offset as i32).to_le_bytes();
    unsafe { patch_bytes(addr, &[0xE8, rel[0], rel[1], rel[2], rel[3]]) };
}

/// Performs an unconditional jump to the specified address without returning.
///
/// This is used to resume execution at a specific address after detouring into
/// a patch or hook. Unlike a regular Rust return, this transfers control directly
/// without preserving the current function's frame or stack.
///
/// # Safety
///
/// - The target address must be valid executable memory.
/// - The calling context must expect a direct transfer of control (i.e., this does not return).
/// - Use only in scenarios like inline hooks or trampolines where the control flow is nonstandard.
///
/// Equivalent to a `jmp [addr]` instruction with `noreturn` semantics.

#[inline(always)]
pub unsafe fn jmp_resume(addr: usize) -> ! {
    unsafe {
        std::arch::asm!(
            "jmp [{0}]",
            in(reg) addr,
            options(noreturn)
        );
    }
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
