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

pub fn init_hacks() {
    //     // Patch User-Agent
    //     // OLD: "MSN-OCX"
    //     // NEW: "MSN-RS\0"
    //     patch_mem(0x372041C8 as *mut u8, b"MSN-RS\0");

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
}

// unsafe fn patch_socket_fns() {
//     unsafe {
//         create_jmp(0x37232EB9, control_socket::socket_try_ctor as usize);
//         create_jmp(0x37232FDD, control_socket::recv_wrapper as usize);
//         create_jmp(0x37233000, control_socket::send_wrapper as usize);
//         create_jmp(0x37232F1D, control_socket::connect_wrapper as usize);
//         create_jmp(0x3722C405, control_socket::validate_server_address as usize);
//     };
// }
