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
use crate::path::get_resdll_storage;
use crate::url::check_buggy_tld_is_allowed;

const PROCESS_QUERY_INFORMATION: PROCESS_ACCESS_RIGHTS = PROCESS_ACCESS_RIGHTS(0x0400); // Standard access to query process info
const PROCESS_VM_READ: PROCESS_ACCESS_RIGHTS = PROCESS_ACCESS_RIGHTS(0x0010); // Required to read memory for module info

pub fn init_hacks() {
    // This function is for testing out what we can do to the MSN Chat Control whilst it's running.

    // unsafe {
    //     let host_process_id = windows::Win32::System::Threading::GetCurrentProcessId();
    //     let activex_dll_name = OsStr::new("MsnChat45.ocx");

    //     let base = match get_module_base_address(host_process_id, activex_dll_name) {
    //         Some(base_address) => base_address,
    //         None => {
    //             println!(
    //                 "ActiveX control '{}' not found or unable to get its base address in process {}.",
    //                 activex_dll_name.to_string_lossy(),
    //                 host_process_id
    //             );
    //             return;
    //         }
    //     };

    //     // Patch User-Agent
    //     // OLD: "MSN-OCX"
    //     // NEW: "MSN-RS\0"
    //     patch_mem(0x372041C8 as *mut u8, b"MSN-RS\0");

    //     // CTCP version reply: NOP non-oper check (version reply to everyone)
    //     patch_mem(0x3722E83B as *mut u8, &[0x90, 0x90, 0x90, 0x90]);

    //     // Patch version string
    //     // OLD: "9.02.0310.2401"
    //     // NEW: "0.1.3\0" (or whatever is set by cargo)
    //     let cargo_version = env!("CARGO_PKG_VERSION");
    //     let mut version_bytes = [0u8; 14];
    //     let bytes = cargo_version.as_bytes();
    //     let len = bytes.len().min(13);
    //     version_bytes[..len].copy_from_slice(&bytes[..len]);
    //     version_bytes[len] = 0;
    //     patch_mem(0x37203AD4 as *mut u8, &version_bytes);

    //     patch_socket_fns();

    //     println!(
    //         "Base address of '{}' in process {} is: 0x{:X}",
    //         activex_dll_name.to_string_lossy(),
    //         host_process_id,
    //         base.0 as usize
    //     );

    //     let addr = 0x3721481E;
    //     let offset = (get_resdll_storage as usize).wrapping_sub(addr + 5);
    //     let patch: [u8; 5] = [
    //         0xE8,
    //         (offset & 0xFF) as u8,
    //         ((offset >> 8) & 0xFF) as u8,
    //         ((offset >> 16) & 0xFF) as u8,
    //         ((offset >> 24) & 0xFF) as u8,
    //     ];
    //     patch_mem(addr as *mut u8, &patch);

    //     create_jmp(0x3724029b, check_buggy_tld_is_allowed as usize);
    // }
}

unsafe fn patch_socket_fns() {
    unsafe {
        create_jmp(0x37232EB9, control_socket::socket_try_ctor as usize);
        create_jmp(0x37232FDD, control_socket::recv_wrapper as usize);
        create_jmp(0x37233000, control_socket::send_wrapper as usize);
        create_jmp(0x37232F1D, control_socket::connect_wrapper as usize);
        create_jmp(0x3722C405, control_socket::validate_server_address as usize);
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
