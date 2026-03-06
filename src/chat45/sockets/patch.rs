use crate::chat45::sockets::connection::Connection;
use crate::patch::module_info::ModuleInfo;
use std::collections::HashMap;
use std::ffi::c_void;
use std::os::raw::c_char;
use std::sync::{Arc, Mutex, OnceLock};

// Global map
static CONNECTIONS: OnceLock<Mutex<HashMap<usize, Arc<Mutex<Connection>>>>> = OnceLock::new();

fn get_connections() -> &'static Mutex<HashMap<usize, Arc<Mutex<Connection>>>> {
    CONNECTIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn get_conn(this: *mut c_void) -> Option<Arc<Mutex<Connection>>> {
    let map = get_connections().lock().unwrap();
    map.get(&(this as usize)).cloned()
}

// Hooks
unsafe extern "thiscall" fn hook_create(this: *mut c_void) -> bool {
    log::trace!(">>> ENTER hook_create for {:?}", this);
    let conn = {
        let mut map = get_connections().lock().unwrap();
        map.entry(this as usize)
            .or_insert_with(Connection::new)
            .clone()
    };
    let mut conn_lock = conn.lock().unwrap();

    // Create the TCP stream in Rust
    if conn_lock.create() {
        unsafe {
            let sock_ptr = (this as usize + 12) as *mut u32; // offset +12
            *sock_ptr = conn_lock.stream.0 as u32; // Set MSNChat's socket id
        }
        log::trace!(">>> Rust socket created, handle: {}", conn_lock.stream.0);
        true
    } else {
        false
    }
}

unsafe extern "thiscall" fn hook_close(this: *mut c_void) -> bool {
    log::trace!(">>> ENTER hook_close for {:?}", this);

    // Clear the SOCKET handle in the C++ wrapper immediately (just like native Close)
    let sock_ptr = (this as usize + 12) as *mut i32;
    unsafe {
        *sock_ptr = -1;
    }

    let conn = if let Some(c) = get_conn(this) {
        c
    } else {
        return false;
    };

    let mut conn_lock = conn.lock().unwrap();
    conn_lock.close();

    // Remove it from the map
    let mut map = get_connections().lock().unwrap();
    map.remove(&(this as usize));

    log::trace!(">>> Socket closed and removed for {:?}", this);
    true
}

unsafe extern "thiscall" fn hook_shutdown(this: *mut c_void) -> i32 {
    log::trace!(">>> ENTER hook_shutdown for {:?}", this);
    let conn = if let Some(c) = get_conn(this) {
        c
    } else {
        return -1;
    };
    let mut conn_lock = conn.lock().unwrap();
    if conn_lock.shutdown() { 0 } else { -1 }
}

unsafe extern "thiscall" fn hook_connect(
    this: *mut c_void,
    cp: *const c_char,
    hostshort: u32,
) -> bool {
    log::trace!(">>> ENTER hook_connect");

    let conn = if let Some(c) = get_conn(this) {
        c
    } else {
        return false;
    };
    let mut conn_lock = conn.lock().unwrap();

    // Connect the Rust TCP stream
    if conn_lock.connect_raw(cp, hostshort as u16) {
        log::trace!(">>> connect_raw success");
        true
    } else {
        log::trace!(">>> connect_raw failed");
        false
    }
}

unsafe extern "thiscall" fn hook_recv(this: *mut c_void, buf: *mut u8, len: i32) -> i32 {
    log::trace!(
        ">>> ENTER hook_recv for {:?}, buf: {:?}, len: {}",
        this,
        buf,
        len
    );
    if buf.is_null() || len <= 0 {
        log::trace!("<<< EXIT hook_recv for {:?} (bad params)", this);
        return -1;
    }
    let conn = if let Some(c) = get_conn(this) {
        c
    } else {
        return -1;
    };
    let mut conn_lock = conn.lock().unwrap();
    let slice = unsafe { std::slice::from_raw_parts_mut(buf, len as usize) };
    let res = conn_lock.recv(slice);
    if res > 0 {
        let string =
            String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(buf, res as usize) });
        log::trace!(
            "<<< EXIT hook_recv for {:?}, returning {}\nCONTENT: {:?}",
            this,
            res,
            string
        );
    } else {
        log::trace!("<<< EXIT hook_recv for {:?}, returning {}", this, res);
    }
    res
}

unsafe extern "thiscall" fn hook_send(this: *mut c_void, buf: *const u8, len: i32) -> bool {
    let string = String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(buf, len as usize) });
    log::trace!(
        ">>> ENTER hook_send for {:?}, len: {}\nCONTENT: {:?}",
        this,
        len,
        string
    );
    if buf.is_null() || len <= 0 {
        log::trace!("<<< EXIT hook_send for {:?} (bad params)", this);
        return false;
    }
    let conn = if let Some(c) = get_conn(this) {
        c
    } else {
        return false;
    };
    let mut conn_lock = conn.lock().unwrap();
    let slice = unsafe { std::slice::from_raw_parts(buf, len as usize) };
    let res = conn_lock.send(slice);
    log::trace!("<<< EXIT hook_send for {:?}, returning {}", this, res >= 0);
    res >= 0
}

unsafe extern "thiscall" fn hook_buffered_send(this: *mut c_void, buf: *const u8, len: u32) -> i32 {
    let conn = if let Some(c) = get_conn(this) {
        c
    } else {
        return 0; // Failure return for buffered send (char 0 = false)
    };
    let mut conn_lock = conn.lock().unwrap();

    let slice = unsafe { std::slice::from_raw_parts(buf, len as usize) };
    if conn_lock.buffered_send(slice) {
        log::trace!("<<< EXIT hook_buffered_send, success");
        1
    } else {
        log::trace!("<<< EXIT hook_buffered_send, wouldblock");
        0 // Original wrapper returned 0 on failure (which might be the cause of looping if it returned anything else)
    }
}

unsafe extern "thiscall" fn hook_flush_buffer(this: *mut c_void) -> i32 {
    log::trace!(">>> ENTER hook_flush_buffer");
    let conn = if let Some(c) = get_conn(this) {
        c
    } else {
        return 1; // Already closed, treat flush as success
    };
    let mut conn_lock = conn.lock().unwrap();
    let string = String::from_utf8_lossy(&conn_lock.send_buffer);
    log::trace!(
        ">>> hook_flush_buffer: pending bytes to flush: {}\nCONTENT: {:?}",
        conn_lock.send_buffer.len(),
        string
    );
    let res = if conn_lock.flush_buffer() { 1 } else { 0 };
    log::trace!("<<< EXIT hook_flush_buffer, {}", res);
    res
}

unsafe extern "thiscall" fn hook_get_internal_id(this: *mut c_void) -> i32 {
    let id = unsafe { *((this as usize + 12) as *const u32) as i32 };
    log::trace!(">>> get_internal_id: {}", id);
    id
}

/// # Safety
/// This function is called from C++ side via COM vtable.
pub unsafe extern "thiscall" fn socket_dtor(this: *mut c_void, _flags: i32) -> *mut c_void {
    log::trace!("Socket DTOR called");
    unsafe {
        hook_close(this);
    }
    this
}

/// # Safety
/// This function is called from C++ side via COM vtable.
pub unsafe extern "thiscall" fn socket_add_ref(_this: *mut c_void) -> i32 {
    1
}

/// # Safety
/// This function is called from C++ side via COM vtable.
pub unsafe extern "thiscall" fn socket_release(_this: *mut c_void) -> i32 {
    1
}

/// # Safety
/// This function is called from C++ side via COM vtable.
pub unsafe extern "thiscall" fn socket_unk(_this: *mut c_void) {}

/// # Safety
/// This function is called from C++ side via COM vtable.
pub unsafe extern "thiscall" fn socket_unk_4(_this: *mut c_void, _char: i32) {}

/// # Safety
/// This function relies on correctly resolving RVAs from the memory-mapped
/// original executable module, and then replacing the legacy `off_37204B00` VTable
/// entirely with our native Rust equivalents.
pub unsafe fn apply(info: &ModuleInfo) -> Result<(), String> {
    unsafe {
        crate::chat45::sockets::manager::apply(info)?;
    }

    log::info!("Patching Socket Wrapper VTable directly...");

    let vtable_addr = info.resolve(0x37204B00);

    let mut old_protect = windows::Win32::System::Memory::PAGE_PROTECTION_FLAGS(0);
    let size = 16 * 4;

    unsafe {
        windows::Win32::System::Memory::VirtualProtect(
            vtable_addr as *const c_void,
            size,
            windows::Win32::System::Memory::PAGE_EXECUTE_READWRITE,
            &mut old_protect,
        )
        .map_err(|e| format!("VirtualProtect failed: {:?}", e))?;
    }

    let vtable = vtable_addr as *mut usize;

    unsafe {
        vtable.add(0).write(socket_dtor as usize);
        vtable.add(1).write(socket_add_ref as usize);
        vtable.add(2).write(socket_release as usize);
        vtable.add(3).write(socket_release as usize);
        vtable.add(4).write(socket_unk_4 as usize);
        vtable.add(5).write(socket_unk as usize);
        vtable.add(6).write(socket_add_ref as usize);
        vtable.add(7).write(hook_create as usize);
        vtable.add(8).write(hook_close as usize);
        vtable.add(9).write(hook_connect as usize);
        vtable.add(10).write(hook_shutdown as usize); // Shutdown
        vtable.add(11).write(hook_recv as usize);
        vtable.add(12).write(hook_send as usize);
        vtable.add(13).write(hook_buffered_send as usize);
        vtable.add(14).write(hook_flush_buffer as usize);
        vtable.add(15).write(hook_get_internal_id as usize);
    }

    unsafe {
        windows::Win32::System::Memory::VirtualProtect(
            vtable_addr as *const c_void,
            size,
            old_protect,
            &mut old_protect,
        )
        .map_err(|e| format!("VirtualProtect restore failed: {:?}", e))?;
    }

    log::info!("Socket Wrapper VTable patched successfully.");
    Ok(())
}
