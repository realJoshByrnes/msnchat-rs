//! Network patch and detour hook implementations for `MsnChat45.ocx`.
//!
//! Maps C++ Virtual Methods to the clean `crate::network` Rust implementation.

use std::ffi::c_void;
use windows::core::BOOL;

// Function pointer typings
type FnSocketCreate = unsafe extern "thiscall" fn(this: *mut c_void) -> BOOL;
type FnSocketClose = unsafe extern "thiscall" fn(this: *mut c_void) -> BOOL;
type FnSocketConnect =
    unsafe extern "thiscall" fn(this: *mut c_void, cp: *const std::ffi::c_char, port: u16) -> bool;
type FnSocketShutdown = unsafe extern "thiscall" fn(this: *mut c_void) -> bool;
type FnSocketReceive =
    unsafe extern "thiscall" fn(this: *mut c_void, buf: *mut u8, len: i32) -> i32;
type FnSocketSend =
    unsafe extern "thiscall" fn(this: *mut c_void, buf: *const u8, len: i32) -> bool;
type FnSocketManagerRegister = unsafe extern "thiscall" fn(
    this: *mut c_void,
    socket_id: u32,
    callback_ptr: *mut c_void,
    context_ptr: *mut c_void,
) -> bool;

// Trampolines
static mut TRAMPOLINE_CREATE: Option<FnSocketCreate> = None;
static mut TRAMPOLINE_CLOSE: Option<FnSocketClose> = None;
static mut TRAMPOLINE_CONNECT: Option<FnSocketConnect> = None;
static mut TRAMPOLINE_SHUTDOWN: Option<FnSocketShutdown> = None;
static mut TRAMPOLINE_RECEIVE: Option<FnSocketReceive> = None;
static mut TRAMPOLINE_SEND: Option<FnSocketSend> = None;
static mut TRAMPOLINE_REGISTER: Option<FnSocketManagerRegister> = None;

/// # Safety
///
/// This function is unsafe because it installs detours on active virtual table addresses.
pub unsafe fn apply(info: &super::module_info::ModuleInfo) -> Result<(), String> {
    let create_target = info.resolve(0x37232eb9);
    let close_target = info.resolve(0x37232f00);
    let connect_target = info.resolve(0x37232f1d);
    let shutdown_target = info.resolve(0x37232fc2);
    let receive_target = info.resolve(0x37232fdd);
    let send_target = info.resolve(0x37233000);
    let register_target = info.resolve(0x372329d0);

    unsafe {
        TRAMPOLINE_CREATE = Some(std::mem::transmute::<*mut c_void, FnSocketCreate>(
            super::hook(create_target, detour_socket_create as *mut c_void)?,
        ));
        TRAMPOLINE_CLOSE = Some(std::mem::transmute::<*mut c_void, FnSocketClose>(
            super::hook(close_target, detour_socket_close as *mut c_void)?,
        ));
        TRAMPOLINE_CONNECT = Some(std::mem::transmute::<*mut c_void, FnSocketConnect>(
            super::hook(connect_target, detour_socket_connect as *mut c_void)?,
        ));
        TRAMPOLINE_SHUTDOWN = Some(std::mem::transmute::<*mut c_void, FnSocketShutdown>(
            super::hook(shutdown_target, detour_socket_shutdown as *mut c_void)?,
        ));
        TRAMPOLINE_RECEIVE = Some(std::mem::transmute::<*mut c_void, FnSocketReceive>(
            super::hook(receive_target, detour_socket_receive as *mut c_void)?,
        ));
        TRAMPOLINE_SEND = Some(std::mem::transmute::<*mut c_void, FnSocketSend>(
            super::hook(send_target, detour_socket_send as *mut c_void)?,
        ));
        TRAMPOLINE_REGISTER = Some(std::mem::transmute::<*mut c_void, FnSocketManagerRegister>(
            super::hook(
                register_target,
                detour_socket_manager_register as *mut c_void,
            )?,
        ));
    }

    log::info!("Network detours applied successfully.");
    Ok(())
}

/// # Safety
///
/// Called as detour for Socket::Create.
unsafe extern "thiscall" fn detour_socket_create(this: *mut c_void) -> BOOL {
    let id = crate::network::create_socket();

    // Set socket descriptor in this object (offset 12 / index 3 of DWORD)
    unsafe {
        let fd_ptr = (this as *mut u8).offset(12) as *mut u32;
        *fd_ptr = id;
    }

    BOOL::from(true)
}

/// # Safety
///
/// Called as detour for Socket::Close.
unsafe extern "thiscall" fn detour_socket_close(this: *mut c_void) -> BOOL {
    let fd_ptr = unsafe { (this as *mut u8).offset(12) as *mut u32 };
    let id = unsafe { *fd_ptr };

    crate::network::close_socket(id);

    unsafe {
        *fd_ptr = u32::MAX; // -1
    }

    BOOL::from(true)
}

/// # Safety
///
/// Called as detour for Socket::Connect.
unsafe extern "thiscall" fn detour_socket_connect(
    this: *mut c_void,
    cp: *const std::ffi::c_char,
    port: u16,
) -> bool {
    let fd_ptr = unsafe { (this as *mut u8).offset(12) as *mut u32 };
    let id = unsafe { *fd_ptr };

    let host = unsafe {
        if cp.is_null() {
            return false;
        }
        std::ffi::CStr::from_ptr(cp).to_string_lossy().into_owned()
    };

    crate::network::connect_socket(id, host, port)
}

/// # Safety
///
/// Called as detour for Socket::Shutdown.
unsafe extern "thiscall" fn detour_socket_shutdown(this: *mut c_void) -> bool {
    let fd_ptr = unsafe { (this as *mut u8).offset(12) as *mut u32 };
    let id = unsafe { *fd_ptr };

    crate::network::shutdown_socket(id);
    true
}

/// # Safety
///
/// Called as detour for Socket::Receive.
unsafe extern "thiscall" fn detour_socket_receive(
    this: *mut c_void,
    buf: *mut u8,
    len: i32,
) -> i32 {
    let fd_ptr = unsafe { (this as *mut u8).offset(12) as *mut u32 };
    let id = unsafe { *fd_ptr };

    if len <= 0 || buf.is_null() {
        return 0;
    }

    let dest_slice = unsafe { std::slice::from_raw_parts_mut(buf, len as usize) };
    crate::network::receive_socket(id, dest_slice)
}

/// # Safety
///
/// Called as detour for Socket::Send.
unsafe extern "thiscall" fn detour_socket_send(
    this: *mut c_void,
    buf: *const u8,
    len: i32,
) -> bool {
    let fd_ptr = unsafe { (this as *mut u8).offset(12) as *mut u32 };
    let id = unsafe { *fd_ptr };

    if len <= 0 || buf.is_null() {
        return true;
    }

    let src_slice = unsafe { std::slice::from_raw_parts(buf, len as usize) };
    crate::network::send_socket(id, src_slice)
}

/// # Safety
///
/// Called as detour for SocketManager::Register.
unsafe extern "thiscall" fn detour_socket_manager_register(
    _this: *mut c_void,
    socket_id: u32,
    callback_ptr: *mut c_void,
    context_ptr: *mut c_void,
) -> bool {
    crate::network::register_socket(socket_id, callback_ptr, context_ptr)
}
