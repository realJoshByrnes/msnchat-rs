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

use std::{
    ffi::c_void,
    os::raw::{c_char, c_int},
};
use windows::Win32::Networking::WinSock::{
    ADDRESS_FAMILY, ADDRINFOA, AF_INET, AF_INET6, AF_UNSPEC, INET6_ADDRSTRLEN, IPPROTO_TCP,
    SEND_RECV_FLAGS, SOCK_STREAM, SOCKADDR_IN, SOCKADDR_IN6, SOCKET, SOCKET_ERROR, WSAEWOULDBLOCK,
    WSAGetLastError, connect, getaddrinfo, inet_ntop, recv, send,
};
use windows_core::{PCSTR, s};
// We're replacing the functions that can be found in the Chat Control OCX.
// We need to ensure we are returning what is expected from them.

#[unsafe(no_mangle)]
pub extern "thiscall" fn connect_wrapper(this: *mut c_void, cp: PCSTR, u_short: u16) -> bool {
    unsafe {
        println!("[control_socket:connect_wrapper] Requested address: {}", cp.to_string().unwrap());
        let sock_ptr = (this as *const usize).add(3); // this + 3
        let socket = SOCKET(*sock_ptr as usize);

        let mut hints: ADDRINFOA = std::mem::zeroed();
        hints.ai_family = AF_UNSPEC.0 as i32;
        hints.ai_socktype = SOCK_STREAM.0;
        hints.ai_protocol = IPPROTO_TCP.0;

        let port_str = std::ffi::CString::new(format!("{}", u_short)).unwrap();
        let port_cstr = PCSTR(port_str.as_ptr() as *const u8);

        let mut result: *mut ADDRINFOA = std::ptr::null_mut();

        if getaddrinfo(cp, port_cstr, Some(&hints), &mut result) != 0 || result.is_null() {
            println!("[control_socket:connect_wrapper] Unable to resolve address");
            return false;
        }

        let addrinfoa = *result;
        let addr = addrinfoa.ai_addr;
        let addrlen = addrinfoa.ai_addrlen;
        let family = addrinfoa.ai_family;

        let mut ip_buf = [0u8; INET6_ADDRSTRLEN as usize];
        let ip_str_ptr = match ADDRESS_FAMILY(family as u16) {
            AF_INET => {
                let ipv4 = *(addr as *const SOCKADDR_IN);
                inet_ntop(family, &ipv4.sin_addr as *const _ as _, &mut ip_buf)
            }
            AF_INET6 => {
                let ipv6 = *(addr as *const SOCKADDR_IN6);
                inet_ntop(family, &ipv6.sin6_addr as *const _ as _, &mut ip_buf)
            }
            _ => s!(""),
        };

        if !ip_str_ptr.is_null() {
            println!("[control_socket:connect_wrapper] Resolved address: {}", ip_str_ptr.to_string().unwrap());
            let connect_result = connect(socket, addr, addrlen as i32);

            if connect_result != SOCKET_ERROR || WSAGetLastError() == WSAEWOULDBLOCK {
                return true;
            }
            return false;
        }

        // Prevent connecting to a file socket etc.
        println!("[control_socket:connect_wrapper] Prevented connection to unknown address (family {})", family);
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
            println!(
                "[hooked_recv_proxy] SOCKET=0x{:X}, len={}, text=\"{}\"",
                socket.0 as usize, result, printable
            );
        } else {
            println!(
                "[hooked_recv_proxy] SOCKET=0x{:X}, len={}, result={}, error={}",
                socket.0 as usize,
                len,
                result,
                WSAGetLastError().0
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
            println!(
                "[hook_send_proxy] SOCKET=0x{:X}, len={}, text=\"{}\"",
                socket.0 as usize,
                slice.len(),
                printable
            );
        }

        return send(socket, slice, SEND_RECV_FLAGS::default()) != -1
            || WSAGetLastError() == WSAEWOULDBLOCK;
    }
}
