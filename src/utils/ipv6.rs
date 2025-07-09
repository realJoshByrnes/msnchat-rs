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

use std::sync::OnceLock;
use windows::Win32::Networking::WinSock::{
    AF_INET6, IPPROTO_TCP, SOCK_STREAM, WSA_FLAG_OVERLAPPED, WSACleanup, WSADATA, WSASocketW,
    WSAStartup, closesocket,
};

static IPV6_ENABLED: OnceLock<bool> = OnceLock::new();

fn detect_ipv6_support() -> bool {
    unsafe {
        let mut wsa_data = WSADATA::default();
        if WSAStartup(0x202, &mut wsa_data) != 0 {
            return false;
        }

        let sock = WSASocketW(
            AF_INET6.0.into(),
            SOCK_STREAM.0,
            IPPROTO_TCP.0,
            None,
            0,
            WSA_FLAG_OVERLAPPED,
        );

        let supported = match sock {
            Ok(sock) => {
                closesocket(sock);
                true
            }
            Err(e) => {
                eprintln!("IPv6 socket creation failed: {:?}", e);
                false
            }
        };

        WSACleanup();
        supported
    }
}

pub fn is_ipv6_enabled() -> bool {
    *IPV6_ENABLED.get_or_init(detect_ipv6_support)
}
