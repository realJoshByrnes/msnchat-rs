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
    ffi::{CStr, CString, OsString, c_void},
    os::{
        raw::{c_char, c_int},
        windows::prelude::OsStringExt,
    },
    ptr::copy_nonoverlapping,
};
use windows::Win32::{
    Foundation::ERROR_SUCCESS,
    NetworkManagement::IpHelper::{
        NET_ADDRESS_INFO, NET_STRING_IPV4_ADDRESS, NET_STRING_IPV4_SERVICE,
        NET_STRING_IPV6_ADDRESS_NO_SCOPE, NET_STRING_IPV6_SERVICE_NO_SCOPE,
        NET_STRING_NAMED_ADDRESS, NET_STRING_NAMED_SERVICE, ParseNetworkString,
    },
};
use windows::Win32::{
    Foundation::WIN32_ERROR,
    NetworkManagement::IpHelper::{NET_ADDRESS_DNS_NAME, NET_ADDRESS_IPV4, NET_ADDRESS_IPV6},
    Networking::WinSock::{
        ADDRINFOA, AF_INET, AF_INET6, AI_V4MAPPED, FIONBIO, INET6_ADDRSTRLEN, IPPROTO_IPV6,
        IPPROTO_TCP, IPV6_V6ONLY, InetNtopW, SEND_RECV_FLAGS, SOCK_STREAM, SOCKADDR_IN6, SOCKET,
        SOCKET_ERROR, WINSOCK_SOCKET_TYPE, WSAEWOULDBLOCK, WSAGetLastError, connect, getaddrinfo,
        inet_ntop, ioctlsocket, recv, send, setsockopt, socket,
    },
};
use windows::core::{PCSTR, PCWSTR, PSTR, w};

use crate::patch::{msnchat45::reloc::PatchContext, utils::make_jmp_rel32};

const NET_STRING_IP_ADDRESS_NO_SCOPE: u32 =
    NET_STRING_IPV4_ADDRESS | NET_STRING_IPV6_ADDRESS_NO_SCOPE;
const NET_STRING_ANY_ADDRESS_NO_SCOPE: u32 =
    NET_STRING_NAMED_ADDRESS | NET_STRING_IP_ADDRESS_NO_SCOPE;
const NET_STRING_IP_SERVICE_NO_SCOPE: u32 =
    NET_STRING_IPV4_SERVICE | NET_STRING_IPV6_SERVICE_NO_SCOPE;
const NET_STRING_ANY_SERVICE_NO_SCOPE: u32 =
    NET_STRING_NAMED_SERVICE | NET_STRING_IP_SERVICE_NO_SCOPE;

pub fn init(ctx: &PatchContext) {
    make_jmp_rel32(ctx.adjust(0x37232EB9), socket_try_ctor as usize);
    make_jmp_rel32(ctx.adjust(0x37232FDD), recv_wrapper as usize);
    make_jmp_rel32(ctx.adjust(0x37233000), send_wrapper as usize);
    make_jmp_rel32(ctx.adjust(0x37232F1D), connect_wrapper as usize);
    make_jmp_rel32(ctx.adjust(0x3722C405), validate_server_address as usize);
}

// We're replacing the functions that can be found in the Chat Control OCX.
// We need to ensure we are returning what is expected from them.

#[unsafe(no_mangle)]
pub extern "thiscall" fn connect_wrapper(this: *mut c_void, cp: PCSTR, u_short: u16) -> bool {
    unsafe {
        println!(
            "[control_socket:connect_wrapper] Requested address: {}",
            cp.to_string().unwrap()
        );
        let sock_ptr = (this as *const usize).add(3); // this + 3
        let socket = SOCKET(*sock_ptr as usize);

        let mut hints: ADDRINFOA = std::mem::zeroed();
        hints.ai_family = AF_INET6.0 as i32;
        hints.ai_flags = AI_V4MAPPED as i32;
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

        if family == AF_INET6.0.into() {
            let ipv6 = *(addr as *const SOCKADDR_IN6);

            let mut ip_str_buf = [0u8; INET6_ADDRSTRLEN as usize];
            let ip_str_pcstr = inet_ntop(family, &ipv6.sin6_addr as *const _ as _, &mut ip_str_buf);
            println!(
                "[control_socket:connect_wrapper] Resolved address: {}",
                ip_str_pcstr.to_string().unwrap()
            );

            let connect_result = connect(socket, addr, addrlen as i32);
            if connect_result != SOCKET_ERROR || WSAGetLastError() == WSAEWOULDBLOCK {
                return true;
            }
            return false;
        }

        // Prevent connecting to IPv4 (should be IPv6 mapped), file socket etc.
        println!(
            "[control_socket:connect_wrapper] Prevented connection to unknown address (family {})",
            family
        );
        false
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "thiscall" fn socket_try_ctor(this: *mut c_void) -> bool {
    unsafe {
        let s = socket(
            AF_INET6.0.into(),
            WINSOCK_SOCKET_TYPE(SOCK_STREAM.0),
            IPPROTO_TCP.0,
        );
        let s = match s {
            Ok(s) => {
                // We re-wrote this fn in rust just so we could (try) and disable IPV6_V6ONLY.
                setsockopt(
                    s,
                    IPPROTO_IPV6.0,
                    IPV6_V6ONLY,
                    Some(&(false as i32).to_ne_bytes()),
                );
                *(this as *mut usize).add(3) = s.0 as usize; // Store in this[2]
                s
            }
            Err(_) => return false,
        };

        let mut argp: u32 = 1;
        if ioctlsocket(s, FIONBIO, &mut argp) == -1 {
            // Non-blocking IO
            let vtable = *(this as *const *const usize);
            let destructor: extern "thiscall" fn(*mut u32) = std::mem::transmute(*vtable.add(8));
            destructor(this as *mut u32);
            return false;
        }

        true
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

// We are adding IPv6 support here, but as the channel server address can only be 15 characters + null terminator,
// the ability to use an IPv4 address is limited. Short addresses like "[::1]" shouldn't be an issue.
// The same limitation already applies to hostnames.
// Directory server allows 128 bytes (127+null) for the name.
#[unsafe(no_mangle)]
pub extern "stdcall" fn validate_server_address(
    addr_and_port_in: PCSTR, // Hostname/IP and port seperated by space (613 reply) or colon (server param)
    addr_out: PSTR,
    max_length: i32, // This is just 16 for channel server, so 15 characters + null terminator. Be warned!
    port_out: *mut i32,
) -> u8 {
    let input = match addr_and_port_in.as_ptr() as usize {
        0 => "dir.irc7.com\0".to_string(), // We now provide a default!
        _ => pcstr_to_string(addr_and_port_in),
    };
    let input = PCWSTR(string_to_wstr_vec(input.replace(" ", ":")).as_ptr());

    let allowed_net_string_types =
        NET_STRING_ANY_ADDRESS_NO_SCOPE | NET_STRING_ANY_SERVICE_NO_SCOPE;

    let mut address_info = std::mem::MaybeUninit::<NET_ADDRESS_INFO>::uninit();
    let mut port_u16: u16 = 0;
    let result = WIN32_ERROR(unsafe {
        ParseNetworkString(
            input,
            allowed_net_string_types,
            Some(address_info.as_mut_ptr()),
            Some(&mut port_u16 as *mut u16),
            None,
        )
    });
    if result == ERROR_SUCCESS {
        unsafe {
            let address = address_info.assume_init();
            let mut ipv4_buffer = [0u16; 16];
            let mut ipv6_buffer = [0u16; 46];
            let address_wstr = match address.Format {
                NET_ADDRESS_DNS_NAME => PCWSTR(address.Anonymous.NamedAddress.Address.as_ptr()),
                NET_ADDRESS_IPV4 => {
                    let sockaddr_ptr =
                        &address.Anonymous.Ipv4Address.sin_addr as *const _ as *const c_void;
                    InetNtopW(AF_INET.0 as i32, sockaddr_ptr, ipv4_buffer.as_mut_slice());
                    PCWSTR(ipv4_buffer.as_ptr())
                }
                NET_ADDRESS_IPV6 => {
                    let sockaddr_ptr =
                        &address.Anonymous.Ipv6Address.sin6_addr as *const _ as *const c_void;
                    InetNtopW(AF_INET6.0 as i32, sockaddr_ptr, ipv6_buffer.as_mut_slice());
                    PCWSTR(ipv6_buffer.as_ptr())
                }
                _ => w!(""), // We should never reach this.
            };
            let address_str_vec = pcwstr_to_lpstr_vec(address_wstr);
            let address_str = PCSTR(address_str_vec.as_ptr());
            copy_nonoverlapping(
                address_str.as_ptr(),
                addr_out.as_ptr(),
                address_str_vec.len().min(max_length as usize),
            );
            if !port_out.is_null() {
                if port_u16 == 0 {
                    *port_out = 6667;
                } else {
                    *port_out = port_u16 as i32;
                }
            }
            return 1;
        }
    }
    0
}

// These functions don't convert from UTF8 <-> ANSI. They're just a quick hack.
// There's an issue occasionally:
// [control_socket:connect_wrapper] Requested address: C:\Windows\S
// I'm not sure if it's the string functions, or something else.

fn pcstr_to_string(s: PCSTR) -> String {
    unsafe { CStr::from_ptr(s.0 as *const c_char) }
        .to_string_lossy()
        .into_owned()
}

fn pcwstr_to_lpstr_vec(s: PCWSTR) -> Vec<u8> {
    // unwrap() will cause panic only if this contains a null byte (it can't if it's a PCWSTR)
    let mut bytes = CString::new(
        OsString::from_wide(unsafe { std::slice::from_raw_parts(s.as_ptr(), s.len()) })
            .to_string_lossy()
            .into_owned(),
    )
    .unwrap()
    .into_bytes();
    bytes.push(0);
    bytes
}

fn string_to_wstr_vec(s: String) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}