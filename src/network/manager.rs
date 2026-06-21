use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::unbounded_channel;

use crate::network::socket::RustSocket;

static TOKIO_RT: OnceLock<Runtime> = OnceLock::new();
static SOCKET_REGISTRY: OnceLock<Mutex<HashMap<u32, Arc<Mutex<RustSocket>>>>> = OnceLock::new();
static NEXT_SOCKET_ID: AtomicU32 = AtomicU32::new(1000);

pub fn get_rt() -> &'static Runtime {
    TOKIO_RT.get_or_init(|| {
        log::info!("Initializing Tokio runtime for network module...");
        Runtime::new().unwrap()
    })
}

pub fn get_registry() -> &'static Mutex<HashMap<u32, Arc<Mutex<RustSocket>>>> {
    SOCKET_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

// C++ Callback function typings
type FnOnWrite = unsafe extern "thiscall" fn(this: *mut c_void, context: *mut c_void);
type FnOnRead = unsafe extern "thiscall" fn(this: *mut c_void, context: *mut c_void);

/// Triggers the C++ `OnWrite` virtual callback (offset 0).
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers and invokes external
/// C++ `thiscall` functions. The `callback_ptr` must point to a valid object with a vtable.
pub unsafe fn trigger_on_write(callback_ptr: *mut c_void, context_ptr: *mut c_void) {
    if callback_ptr.is_null() {
        return;
    }
    unsafe {
        let vtable = *(callback_ptr as *mut *mut *mut c_void);
        let func_ptr = *vtable.offset(0);
        let func: FnOnWrite = std::mem::transmute(func_ptr);
        func(callback_ptr, context_ptr);
    }
}

/// Triggers the C++ `OnRead` virtual callback (offset 2 / 8 bytes).
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers and invokes external
/// C++ `thiscall` functions. The `callback_ptr` must point to a valid object with a vtable.
pub unsafe fn trigger_on_read(callback_ptr: *mut c_void, context_ptr: *mut c_void) {
    if callback_ptr.is_null() {
        return;
    }
    unsafe {
        let vtable = *(callback_ptr as *mut *mut *mut c_void);
        let func_ptr = *vtable.offset(2); // Offset 2 is OnRead (offset 8 bytes)
        let func: FnOnRead = std::mem::transmute(func_ptr);
        func(callback_ptr, context_ptr);
    }
}

type FnOnReadReady = unsafe extern "thiscall" fn(this: *mut c_void, context: *mut c_void) -> u8;

/// Triggers the C++ `OnReadReady` virtual callback (offset 3 / 12 bytes).
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers and invokes external
/// C++ `thiscall` functions. The `callback_ptr` must point to a valid object with a vtable.
pub unsafe fn trigger_on_read_ready(callback_ptr: *mut c_void, context_ptr: *mut c_void) -> bool {
    if callback_ptr.is_null() {
        return false;
    }
    unsafe {
        let vtable = *(callback_ptr as *mut *mut *mut c_void);
        let func_ptr = *vtable.offset(3); // Offset 3 is OnReadReady (offset 12 bytes)
        let func: FnOnReadReady = std::mem::transmute(func_ptr);
        func(callback_ptr, context_ptr) != 0
    }
}

/// Creates a new socket and returns its generated unique ID/descriptor.
pub fn create_socket() -> u32 {
    let id = NEXT_SOCKET_ID.fetch_add(1, Ordering::SeqCst);
    log::info!("network::create_socket: Assigned ID {}", id);

    let socket_ref = Arc::new(Mutex::new(RustSocket::new(id)));
    if let Ok(mut reg) = get_registry().lock() {
        reg.insert(id, socket_ref);
    }
    id
}

/// Closes the socket, terminating reader/writer tasks.
pub fn close_socket(id: u32) {
    log::info!("network::close_socket called for ID: {}", id);

    if let Ok(mut reg) = get_registry().lock() {
        if let Some(socket_arc) = reg.remove(&id) {
            if let Ok(mut socket) = socket_arc.lock() {
                socket.closed = true;
                socket.tx = None;
            }
        }
    }
}

/// Initiates an asynchronous connection to the remote host.
pub fn connect_socket(id: u32, host: String, port: u16) -> bool {
    log::info!(
        "network::connect_socket called for ID: {} -> {}:{}",
        id,
        host,
        port
    );

    let socket_arc = if let Ok(reg) = get_registry().lock() {
        match reg.get(&id) {
            Some(arc) => arc.clone(),
            None => return false,
        }
    } else {
        return false;
    };

    let rt = get_rt();
    let socket_arc_clone = socket_arc.clone();

    rt.spawn(async move {
        log::info!("Tokio task attempting connection to {}:{}...", host, port);
        match TcpStream::connect((host.as_str(), port)).await {
            Ok(stream) => {
                log::info!(
                    "Tokio connection to {}:{} established successfully!",
                    host,
                    port
                );
                let (mut read_half, mut write_half) = stream.into_split();
                let (tx, mut rx) = unbounded_channel::<Vec<u8>>();

                // Update socket status
                let mut callback_to_trigger = None;
                let mut context_to_trigger = None;
                {
                    if let Ok(mut socket) = socket_arc_clone.lock() {
                        socket.tx = Some(tx);
                        socket.connected = true;
                        if !socket.callback_ptr.is_null() {
                            callback_to_trigger = Some(socket.callback_ptr);
                            context_to_trigger = Some(socket.context_ptr);
                        }
                    }
                }

                // If callback is already registered, trigger OnWrite immediately
                if let (Some(cb), Some(ctx)) = (callback_to_trigger, context_to_trigger) {
                    log::info!("Triggering OnWrite for socket {}", id);
                    unsafe { trigger_on_write(cb, ctx) };
                }

                // Spawn Writer task
                let socket_arc_writer = socket_arc_clone.clone();
                tokio::spawn(async move {
                    while let Some(data) = rx.recv().await {
                        let is_closed = socket_arc_writer.lock().map(|s| s.closed).unwrap_or(false);
                        if is_closed {
                            break;
                        }
                        if let Err(e) = write_half.write_all(&data).await {
                            log::error!("Writer task write_all error: {:?}", e);
                            break;
                        }
                    }
                });

                // Spawn Reader task
                let socket_arc_reader = socket_arc_clone.clone();
                tokio::spawn(async move {
                    let mut buf = [0u8; 4096];
                    loop {
                        let is_closed = socket_arc_reader.lock().map(|s| s.closed).unwrap_or(false);
                        if is_closed {
                            break;
                        }

                        match read_half.read(&mut buf).await {
                            Ok(0) => {
                                log::info!("Socket {} closed by remote.", id);
                                let mut callback = None;
                                let mut context = None;
                                {
                                    if let Ok(socket) = socket_arc_reader.lock() {
                                        if !socket.callback_ptr.is_null() {
                                            callback = Some(socket.callback_ptr);
                                            context = Some(socket.context_ptr);
                                        }
                                    }
                                }
                                if let (Some(cb), Some(ctx)) = (callback, context) {
                                    unsafe { trigger_on_read(cb, ctx) };
                                }
                                break;
                            }
                            Ok(n) => {
                                let mut callback = None;
                                let mut context = None;
                                {
                                    if let Ok(mut socket) = socket_arc_reader.lock() {
                                        socket.rx_buffer.extend_from_slice(&buf[..n]);
                                        if !socket.callback_ptr.is_null() {
                                            callback = Some(socket.callback_ptr);
                                            context = Some(socket.context_ptr);
                                        }
                                    }
                                }
                                if let (Some(cb), Some(ctx)) = (callback, context) {
                                    unsafe { trigger_on_read_ready(cb, ctx) };
                                }
                            }
                            Err(e) => {
                                log::error!("Reader task read error: {:?}", e);
                                break;
                            }
                        }
                    }
                });
            }
            Err(e) => {
                log::error!("Tokio connection failed to {}:{}: {:?}", host, port, e);
                // Trigger error callback
                let mut callback = None;
                let mut context = None;
                if let Ok(socket) = socket_arc_clone.lock() {
                    if !socket.callback_ptr.is_null() {
                        callback = Some(socket.callback_ptr);
                        context = Some(socket.context_ptr);
                    }
                }
                if let (Some(cb), Some(ctx)) = (callback, context) {
                    unsafe {
                        let vtable = *(cb as *mut *mut *mut c_void);
                        let error_func: FnOnWrite = std::mem::transmute(*vtable.offset(1)); // Offset 1 is OnError
                        error_func(cb, ctx);
                    }
                }
            }
        }
    });

    true
}

/// Shuts down writing half of the connection.
pub fn shutdown_socket(id: u32) {
    log::info!("network::shutdown_socket called for ID: {}", id);

    if let Ok(reg) = get_registry().lock() {
        if let Some(socket_arc) = reg.get(&id) {
            if let Ok(mut socket) = socket_arc.lock() {
                socket.tx = None;
            }
        }
    }
}

/// Copies buffered incoming bytes to target buffer. Returns number of bytes read.
pub fn receive_socket(id: u32, buf: &mut [u8]) -> i32 {
    if let Ok(reg) = get_registry().lock() {
        if let Some(socket_arc) = reg.get(&id) {
            if let Ok(mut socket) = socket_arc.lock() {
                let to_copy = std::cmp::min(buf.len(), socket.rx_buffer.len());
                if to_copy > 0 {
                    buf[..to_copy].copy_from_slice(&socket.rx_buffer[..to_copy]);
                    socket.rx_buffer.drain(0..to_copy);
                    return to_copy as i32;
                }
            }
        }
    }
    0
}

/// Sends data asynchronously via the writer task.
pub fn send_socket(id: u32, data: &[u8]) -> bool {
    let mut sent = false;
    if let Ok(reg) = get_registry().lock() {
        if let Some(socket_arc) = reg.get(&id) {
            if let Ok(socket) = socket_arc.lock() {
                if let Some(ref tx) = socket.tx {
                    let _ = tx.send(data.to_vec());
                    sent = true;
                }
            }
        }
    }
    sent
}

/// Associates callback delegates to the socket for async event dispatch.
pub fn register_socket(
    socket_id: u32,
    callback_ptr: *mut c_void,
    context_ptr: *mut c_void,
) -> bool {
    log::info!(
        "network::register_socket. ID: {}, Callback: {:?}, Context: {:?}",
        socket_id,
        callback_ptr,
        context_ptr
    );

    let socket_arc = if let Ok(reg) = get_registry().lock() {
        match reg.get(&socket_id) {
            Some(arc) => arc.clone(),
            None => {
                log::warn!(
                    "Socket ID {} not found in registry during registration",
                    socket_id
                );
                return false;
            }
        }
    } else {
        return false;
    };

    let mut trigger_write = false;
    {
        if let Ok(mut socket) = socket_arc.lock() {
            socket.callback_ptr = callback_ptr;
            socket.context_ptr = context_ptr;
            if socket.connected {
                trigger_write = true;
            }
        }
    }

    if trigger_write {
        log::info!(
            "Socket {} already connected. Triggering OnWrite immediately.",
            socket_id
        );
        unsafe { trigger_on_write(callback_ptr, context_ptr) };
    }

    true
}
