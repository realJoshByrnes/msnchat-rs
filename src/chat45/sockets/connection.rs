use std::sync::{Arc, Mutex};
use windows::Win32::Foundation::GetLastError;
use windows::Win32::Networking::WinSock::{
    AF_INET, FIONBIO, INVALID_SOCKET, IPPROTO_TCP, SD_BOTH, SOCK_STREAM, SOCKADDR, SOCKADDR_IN,
    SOCKET, closesocket, connect as winsock_connect, ioctlsocket, recv as winsock_recv,
    send as winsock_send, shutdown as winsock_shutdown, socket,
};

/// Represents the state of a Socket connection (replaces `off_37204B00`)
pub struct Connection {
    pub stream: SOCKET,
    pub send_buffer: Vec<u8>,
}

impl Connection {
    /// Constructs a new, disconnected Connection (replaces `off_37204B40` Factory)
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            stream: INVALID_SOCKET,
            send_buffer: Vec::new(),
        }))
    }

    /// Creates the OS socket handle. Call this before connect.
    pub fn create(&mut self) -> bool {
        // SAFETY: Calling standard Winsock APIs with correct parameters
        unsafe {
            let s = match socket(AF_INET.0 as i32, SOCK_STREAM, IPPROTO_TCP.0) {
                Ok(sock) => sock,
                Err(_) => return false,
            };
            // Set non-blocking mode like the original
            let mut argp: u32 = 1;
            if ioctlsocket(s, FIONBIO, &mut argp) == -1 {
                closesocket(s);
                return false;
            }
            self.stream = s;
            true
        }
    }

    /// Appends data to the send buffer, flushing if it exceeds 1024 bytes.
    pub fn buffered_send(&mut self, buf: &[u8]) -> bool {
        if self.send_buffer.len() + buf.len() > 1024 && !self.flush_buffer() {
            return false;
        }
        self.send_buffer.extend_from_slice(buf);
        true
    }

    /// Flushes the send buffer to the underlying socket.
    pub fn flush_buffer(&mut self) -> bool {
        if self.send_buffer.is_empty() {
            return true;
        }

        let data_to_send = self.send_buffer.clone();
        if self.send(&data_to_send) > 0 {
            self.send_buffer.clear();
            true
        } else {
            false
        }
    }

    /// Connects to the given original host string and port
    pub fn connect_raw(&mut self, cp: *const i8, hostshort: u16) -> bool {
        if self.stream == INVALID_SOCKET {
            return false;
        }
        // SAFETY: Converting pointers to Winsock address structures and interacting with FFI.
        // We ensure raw pointers like `cp` point to valid null-terminated strings where expected.
        unsafe {
            let mut name: SOCKADDR_IN = core::mem::zeroed();
            name.sin_family = AF_INET;
            // The original uses htons(hostshort), hostshort is already in host byte order here?
            // Wait, htons converts host to network byte order. `to_be` does the same.
            name.sin_port = hostshort.to_be();

            let addr_num = windows::Win32::Networking::WinSock::inet_addr(windows::core::PCSTR(
                cp as *const u8,
            ));
            if addr_num == windows::Win32::Networking::WinSock::INADDR_NONE {
                let host = windows::Win32::Networking::WinSock::gethostbyname(
                    windows::core::PCSTR(cp as *const u8),
                );
                if host.is_null() {
                    return false;
                }
                let host_ent = &*host;
                let addr_list = host_ent.h_addr_list as *const *const u8;
                if !addr_list.is_null() && !(*addr_list).is_null() {
                    std::ptr::copy_nonoverlapping(
                        *addr_list,
                        &mut name.sin_addr as *mut _ as *mut u8,
                        host_ent.h_length as usize,
                    );
                } else {
                    return false;
                }
            } else {
                name.sin_addr.S_un.S_addr = addr_num;
            }

            let res = winsock_connect(
                self.stream,
                &name as *const _ as *const SOCKADDR,
                core::mem::size_of::<SOCKADDR_IN>() as i32,
            );
            if res != -1 {
                true
            } else {
                let err = GetLastError().0;
                err == 10035 // WSAEWOULDBLOCK
            }
        }
    }

    /// Sends a buffer over the socket
    pub fn send(&mut self, buf: &[u8]) -> i32 {
        if self.stream == INVALID_SOCKET {
            return -1;
        }
        // SAFETY: We guarantee our buffer slice holds valid memory for its length.
        unsafe {
            let res = winsock_send(
                self.stream,
                buf,
                windows::Win32::Networking::WinSock::SEND_RECV_FLAGS(0),
            );
            if res == -1 && GetLastError().0 == 10035 {
                0
            } else {
                res
            }
        }
    }

    /// Receives data from the socket into the provided buffer
    pub fn recv(&mut self, buf: &mut [u8]) -> i32 {
        if self.stream == INVALID_SOCKET {
            return -1;
        }
        // SAFETY: Our caller guarantees buf holds a mutable slice of adequate capacities.
        unsafe {
            let res = winsock_recv(
                self.stream,
                buf,
                windows::Win32::Networking::WinSock::SEND_RECV_FLAGS(0),
            );
            if res < 0 {
                // Original MSNChat wrapper ALWAYS returns 0 on all WS errors
                // Returning <0 causes caller to corrupt heap with negative index
                0
            } else {
                res
            }
        }
    }

    /// Shuts down the connection gracefully (replaces index 10)
    pub fn shutdown(&mut self) -> bool {
        if self.stream != INVALID_SOCKET {
            // SAFETY: Shutting down a valid Winsock handle is sound.
            unsafe {
                let _ = winsock_shutdown(self.stream, SD_BOTH);
            }
        }
        true
    }

    /// Closes the socket (replaces index 8)
    pub fn close(&mut self) {
        if self.stream != INVALID_SOCKET {
            // SAFETY: Disposing the socket handle.
            unsafe {
                closesocket(self.stream);
            }
            self.stream = INVALID_SOCKET;
        }
    }
}
