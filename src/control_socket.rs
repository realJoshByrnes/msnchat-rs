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

use std::{os::raw::{c_char, c_int}, ffi::c_void};
use windows::Win32::Networking::WinSock::{gethostbyname, inet_addr, recv, send, connect, WSAGetLastError, AF_INET, htons, HOSTENT, SEND_RECV_FLAGS, SOCKADDR, SOCKET, WSAEWOULDBLOCK};
use crate::PSTR;
// We're replacing the functions that can be found in the Chat Control OCX.
// We need to ensure we are returning what is expected from them.

#[unsafe(no_mangle)]
pub extern "thiscall" fn connect_wrapper(this: *mut c_void, cp: PSTR, u_short: u16) -> bool {
    // TODO: Add IPv6 support (in progress, see issue #4 at https://github.com/realJoshByrnes/msnchat-rs/issues/4)
    unsafe {
        let sock_ptr = (this as *const usize).add(3); // this + 3
        let socket = SOCKET(*sock_ptr as usize);

        let mut sockaddr_in: SOCKADDR = std::mem::zeroed(); // Note: This is using sockaddr_in (IPv4), I've just kept it as sockaddr for simplicity.

        let ip_as_int = inet_addr(cp); // NOTE: v4 only
        if ip_as_int == u32::MAX {
            let resolved = gethostbyname(cp); // NOTE: Deprecated (IPv4 only). Use getaddrinfo (check out AF_UNSPEC) instead.
            if resolved.is_null() {
                return false;
            }

            let h_addr_list = (*(resolved as *const HOSTENT)).h_addr_list;
            let h_addr = *h_addr_list;
            std::ptr::copy_nonoverlapping(
                h_addr,
                sockaddr_in.sa_data.as_mut_ptr().add(2) as *mut _,
                4, // NOTE: This is only big enough for IPv4
            );
        } else {
            let ip_bytes = ip_as_int.to_ne_bytes();
            std::ptr::copy_nonoverlapping(
                ip_bytes.as_ptr(),
                sockaddr_in.sa_data.as_mut_ptr().add(2) as *mut _,
                4, // NOTE: This is only big enough for IPv4
            );
        }

        sockaddr_in.sa_family = AF_INET; //  NOTE: IPv4. IPv6 is AF_INET6

        let port = htons(u_short);
        sockaddr_in.sa_data[0] = (port & 0xFF) as c_char;
        sockaddr_in.sa_data[1] = (port >> 8) as c_char;

        let result = connect(socket, &sockaddr_in, size_of::<SOCKADDR>() as c_int);

        if result != -1 || WSAGetLastError() == WSAEWOULDBLOCK {
            println!("Connecting to ...{:X?} ({:?})", sockaddr_in.sa_data, sockaddr_in.sa_family.0);
            return true;
        }

        false
    }
}


#[unsafe(no_mangle)]
pub extern "thiscall" fn recv_wrapper(this: *mut c_void, buf: *mut c_char, len: c_int) -> c_int {
    unsafe {
        let sock_ptr = (this as *const usize).add(3); // this + 0x0C
        let socket = SOCKET(*sock_ptr as usize);

        let slice = std::slice::from_raw_parts_mut(buf as *mut u8, len as usize);
        let result = recv(socket, slice, SEND_RECV_FLAGS::default());

        if result > 0 {
            let printable = String::from_utf8_lossy(&slice[..result as usize]);
            println!("[hooked_recv_proxy] SOCKET=0x{:X}, len={}, text=\"{}\"", socket.0 as usize, result, printable);
        } else {
            println!(
                "[hooked_recv_proxy] SOCKET=0x{:X}, len={}, result={}, error={}",
                socket.0 as usize, len, result, WSAGetLastError().0
            );
        }
        if result == -1 {
            WSAGetLastError();
            return 0;
        }

        result
    }
}

#[unsafe(no_mangle)]
pub extern "thiscall" fn send_wrapper(this: *mut c_void, buf: *const c_char, len: c_int) -> bool {
    unsafe {
        let sock_ptr = (this as *const usize).add(3); // this + 0x0C
        let socket = SOCKET(*sock_ptr);

        let slice = std::slice::from_raw_parts(buf as *mut u8, len as usize);

        if slice.len() > 0 {
            let printable = String::from_utf8_lossy(&slice[..slice.len() as usize]);
            println!("[hook_send_proxy] SOCKET=0x{:X}, len={}, text=\"{}\"", socket.0 as usize, slice.len(), printable);
        }

        return send(socket, slice, SEND_RECV_FLAGS::default()) != -1 || WSAGetLastError() == WSAEWOULDBLOCK;
    }
}