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

use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Memory::{
    PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS, VirtualProtect,
};
use windows::Win32::System::ProcessStatus::{GetModuleFileNameExW, K32EnumProcessModules};
use windows::Win32::System::Threading::OpenProcess;
use windows::Win32::{Foundation::HMODULE, System::Threading::PROCESS_ACCESS_RIGHTS};

use crate::control_socket;

const PROCESS_QUERY_INFORMATION: PROCESS_ACCESS_RIGHTS = PROCESS_ACCESS_RIGHTS(0x0400); // Standard access to query process info
const PROCESS_VM_READ: PROCESS_ACCESS_RIGHTS = PROCESS_ACCESS_RIGHTS(0x0010); // Required to read memory for module info

pub fn init_hacks() {
    // This function is for testing out what we can do to the MSN Chat Control whilst it's running.

    unsafe {
        let host_process_id = windows::Win32::System::Threading::GetCurrentProcessId();
        let activex_dll_name = OsStr::new("MsnChat45.ocx");

        let base = match get_module_base_address(host_process_id, activex_dll_name) {
            Some(base_address) => base_address,
            None => {
                println!(
                    "ActiveX control '{}' not found or unable to get its base address in process {}.",
                    activex_dll_name.to_string_lossy(),
                    host_process_id
                );
                return;
            }
        };

        // Patch User-Agent
        // OLD: "MSN-OCX"
        // NEW: "MSN-RS\0"
        patch_mem(0x372041C8 as *mut u8, b"MSN-RS\0");

        // CTCP version reply: NOP non-oper check (version reply to everyone)
        patch_mem(0x3722E83B as *mut u8, &[0x90, 0x90, 0x90, 0x90]);

        // Patch version string
        // OLD: "9.02.0310.2401"
        // NEW: "0.1.3\0" (or whatever is set by cargo)
        let cargo_version = env!("CARGO_PKG_VERSION");
        let mut version_bytes = [0u8; 14];
        let bytes = cargo_version.as_bytes();
        let len = bytes.len().min(13);
        version_bytes[..len].copy_from_slice(&bytes[..len]);
        version_bytes[len] = 0;
        patch_mem(0x37203AD4 as *mut u8, &version_bytes);

        // Patch UTF-16LE version label at 0x37203AE4
        // OLD: "MSN Chat Control, version #"
        // NEW: "(*) msnchat-rs (*) v\0"
        let cargo_name = env!("CARGO_PKG_NAME");
        let label = format!("(*) {} (*) v", cargo_name);
        let mut label_utf16: [u16; 28] = [0; 28];
        let label_encoded: Vec<u16> = label.encode_utf16().collect();
        let copy_len = label_encoded.len().min(27);
        label_utf16[..copy_len].copy_from_slice(&label_encoded[..copy_len]);
        for i in copy_len..28 {
            label_utf16[i] = 0;
        }
        let bytes: &[u8] =
            std::slice::from_raw_parts(label_utf16.as_ptr() as *const u8, label_utf16.len() * 2);
        patch_mem(0x37203AE4 as *mut u8, &bytes);

        patch_socket_fns();

        println!(
            "Base address of '{}' in process {} is: 0x{:X}",
            activex_dll_name.to_string_lossy(),
            host_process_id,
            base.0 as usize
        );
    }
}

unsafe fn patch_socket_fns() {
    unsafe {
        // Patch ADRESS_FAMILY on the socket ctor (for IPv6)
        // OLD: AF_INET
        // NEW: AF_INET6
        patch_mem(0x37232ec3 as *mut u8, &[0x17]);
    
        create_jmp(0x37232FDD, control_socket::recv_wrapper as usize);
        create_jmp(0x37233000, control_socket::send_wrapper as usize);
        create_jmp(0x37232F1D, control_socket::connect_wrapper as usize);
    };
}

unsafe fn create_jmp(addr: usize, f: usize) {
    let offset = f.wrapping_sub(addr + 5); // Offset for patch_bytes
    let patch_bytes = [
        0xE9, // JMP
        (offset & 0xFF) as u8,
        (offset >> 8) as u8,
        (offset >> 16) as u8,
        (offset >> 24) as u8,
    ];
    unsafe {
        patch_mem(addr as *mut u8, &patch_bytes);
    }
}

unsafe fn patch_mem(addr: *mut u8, bytes: &[u8]) {
    let mut old_protect = PAGE_PROTECTION_FLAGS(0);
    unsafe {
        let _ = VirtualProtect(
            addr as *mut _,
            bytes.len(),
            PAGE_EXECUTE_READWRITE,
            &mut old_protect,
        );
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), addr, bytes.len());
        let _ = VirtualProtect(
            addr as *mut _,
            bytes.len(),
            PAGE_EXECUTE_READWRITE,
            &mut old_protect,
        );
    }
}

fn get_module_base_address(process_id: u32, module_name: &OsStr) -> Option<HMODULE> {
    let process_handle = match unsafe {
        OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
            false,
            process_id,
        )
    } {
        Ok(val) => val,
        Err(err) => {
            eprintln!("Failed to open process {}. Error: {}", process_id, err);
            return None;
        }
    };

    let mut h_mods: [HMODULE; 1024] = [HMODULE(std::ptr::null_mut()); 1024];
    let mut cb_needed: u32 = 0;

    let result: bool = unsafe {
        K32EnumProcessModules(
            process_handle.clone(),
            h_mods.as_mut_ptr(),
            std::mem::size_of_val(&h_mods) as u32,
            &mut cb_needed,
        )
        .into()
    };

    if !result {
        eprintln!("Failed to enumerate process modules. Error: {}", unsafe {
            windows::Win32::Foundation::GetLastError().0
        });
        let _ = unsafe { CloseHandle(process_handle) };
        return None;
    }

    let num_modules = (cb_needed / std::mem::size_of::<HMODULE>() as u32) as usize;

    for i in 0..num_modules {
        let h_module = h_mods[i];
        let mut module_path_buffer = [0u16; 260]; // MAX_PATH wide chars

        let chars_copied = unsafe {
            GetModuleFileNameExW(
                Some(process_handle),
                Some(h_module),
                &mut module_path_buffer,
            )
        };

        if chars_copied > 0 {
            let path_os_string = OsString::from_wide(&module_path_buffer[..chars_copied as usize]);
            let path_buf = PathBuf::from(path_os_string);

            if let Some(filename) = path_buf.file_name() {
                if filename.eq_ignore_ascii_case(module_name) {
                    let _ = unsafe { CloseHandle(process_handle) };
                    return Some(h_module);
                }
            }
        }
    }

    let _ = unsafe { CloseHandle(process_handle) };
    None
}
