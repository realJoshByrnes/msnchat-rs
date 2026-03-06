use crate::patch::module_info::ModuleInfo;
use log::{info, trace};
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{Arc, Mutex, OnceLock};
use windows::Win32::Networking::WinSock::{FD_SET, SOCKET, TIMEVAL, select};

/// The structure of the IConnectionCallback that MSNChat passes to Add Socket
#[repr(C)]
pub struct IConnectionCallbackVTable {
    pub on_write: unsafe extern "thiscall" fn(this: *mut c_void, context: i32),
    pub on_exception: unsafe extern "thiscall" fn(this: *mut c_void, context: i32),
    pub on_close: unsafe extern "thiscall" fn(this: *mut c_void, context: i32),
    pub on_read: unsafe extern "thiscall" fn(this: *mut c_void, context: i32) -> u8,
}

#[repr(C)]
pub struct IConnectionCallback {
    pub vtable: *const IConnectionCallbackVTable,
}

#[derive(Clone)]
pub struct RegisteredSocket {
    pub raw_socket: SOCKET,
    pub callback: *mut IConnectionCallback,
    pub context: i32,
    pub wants_write: bool,
    pub flagged_for_exception: bool,
    pub flagged_for_closure: bool,
}

unsafe impl Send for RegisteredSocket {}
unsafe impl Sync for RegisteredSocket {}

// Global thread-safe map of active managers to their tracked sockets
static MANAGERS: OnceLock<Mutex<HashMap<usize, Arc<Mutex<ManagerData>>>>> = OnceLock::new();

fn get_managers() -> &'static Mutex<HashMap<usize, Arc<Mutex<ManagerData>>>> {
    MANAGERS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub struct ManagerData {
    pub sockets: Vec<RegisteredSocket>,
    pub running: bool,
}

impl Default for ManagerData {
    fn default() -> Self {
        Self::new()
    }
}

impl ManagerData {
    pub fn new() -> Self {
        Self {
            sockets: Vec::new(),
            running: false,
        }
    }
}

// -----------------------------------------------------------------------
// VTable Implementations
// -----------------------------------------------------------------------

/// # Safety
/// This function is called from C++ side via COM vtable.
pub unsafe extern "thiscall" fn manager_dtor(this: *mut c_void, _flags: i32) -> *mut c_void {
    trace!("Manager DTOR called");
    get_managers().lock().unwrap().remove(&(this as usize));
    // We let the original application free the C++ block using its own scalar deleting destructor if needed.
    // However, since we patched the vtable directly, we might leak the C++ memory if the original destructor isn't called.
    // For now, returning `this` fulfills the standard `__thiscall` destructor signature.
    this
}

/// # Safety
/// This function is called from C++ side via COM vtable.
pub unsafe extern "thiscall" fn manager_add_ref(_this: *mut c_void) -> i32 {
    1
}

/// # Safety
/// This function is called from C++ side via COM vtable.
pub unsafe extern "thiscall" fn manager_release(_this: *mut c_void) -> i32 {
    1
}

/// # Safety
/// This function is called from C++ side via COM vtable.
pub unsafe extern "thiscall" fn manager_start_thread(this: *mut c_void) {
    info!("Manager Start Thread Called! Initiating pure Rust multiplexer.");

    let mut map = get_managers().lock().unwrap();
    let data = map
        .entry(this as usize)
        .or_insert_with(|| Arc::new(Mutex::new(ManagerData::new())))
        .clone();

    data.lock().unwrap().running = true;

    std::thread::spawn(move || {
        info!("Rust Multiplexer Background Thread Started");
        let timeout = TIMEVAL {
            tv_sec: 1,
            tv_usec: 0,
        };

        loop {
            let mut sockets = {
                let lock = data.lock().unwrap();
                if !lock.running {
                    break;
                }
                lock.sockets.clone()
            };

            if sockets.is_empty() {
                std::thread::sleep(std::time::Duration::from_millis(100));
                continue;
            }

            let mut read_fds = FD_SET {
                fd_count: 0,
                fd_array: [SOCKET(0); 64],
            };
            let mut write_fds = FD_SET {
                fd_count: 0,
                fd_array: [SOCKET(0); 64],
            };
            let mut except_fds = FD_SET {
                fd_count: 0,
                fd_array: [SOCKET(0); 64],
            };

            for s in &sockets {
                if s.flagged_for_exception {
                    // Do not poll a cancelled connect socket
                    continue;
                }
                if read_fds.fd_count < 64 {
                    read_fds.fd_array[read_fds.fd_count as usize] = s.raw_socket;
                    read_fds.fd_count += 1;
                }
                if s.wants_write {
                    if write_fds.fd_count < 64 {
                        write_fds.fd_array[write_fds.fd_count as usize] = s.raw_socket;
                        write_fds.fd_count += 1;
                    }
                    if except_fds.fd_count < 64 {
                        except_fds.fd_array[except_fds.fd_count as usize] = s.raw_socket;
                        except_fds.fd_count += 1;
                    }
                }
            }

            let res = unsafe {
                select(
                    0,
                    Some(&mut read_fds),
                    Some(&mut write_fds),
                    Some(&mut except_fds),
                    Some(&timeout),
                )
            };
            if res > 0 {
                log::trace!(
                    "Select returned {}. read={}, write={}, except={}",
                    res,
                    read_fds.fd_count,
                    write_fds.fd_count,
                    except_fds.fd_count
                );
                // Something is readable or errored
                // Ensure we don't hold `data_lock` during C++ callbacks!
                // C++ callbacks might call Add Socket or Flag Socket and cause deadlock.
                for s in &mut sockets {
                    let mut is_handled = false;

                    // Handle canceled connecting sockets (manager_flag_socket)
                    if s.wants_write && s.flagged_for_exception {
                        unsafe {
                            let cb = &*s.callback;
                            let vtable = &*cb.vtable;
                            (vtable.on_exception)(s.callback as *mut c_void, s.context);
                        }
                        s.flagged_for_closure = true;
                        continue;
                    }

                    // Check Reads
                    for i in 0..read_fds.fd_count {
                        if read_fds.fd_array[i as usize] == s.raw_socket {
                            is_handled = true;
                            unsafe {
                                let cb = &*s.callback;
                                let vtable = &*cb.vtable;

                                // Call OnRead
                                let keep_alive =
                                    (vtable.on_read)(s.callback as *mut c_void, s.context);
                                if keep_alive == 0 {
                                    // Call OnClose and flag for removal
                                    (vtable.on_close)(s.callback as *mut c_void, s.context);
                                    s.flagged_for_closure = true;
                                }
                            }
                            break;
                        }
                    }

                    if s.wants_write {
                        for i in 0..write_fds.fd_count {
                            if write_fds.fd_array[i as usize] == s.raw_socket {
                                is_handled = true;
                                s.wants_write = false; // Clear the flag
                                log::trace!("Firing on_write for socket {}", s.raw_socket.0);
                                unsafe {
                                    let cb = &*s.callback;
                                    let vtable = &*cb.vtable;
                                    (vtable.on_write)(s.callback as *mut c_void, s.context);
                                }
                                break;
                            }
                        }
                    }

                    // Check Exceptions
                    for i in 0..except_fds.fd_count {
                        if except_fds.fd_array[i as usize] == s.raw_socket {
                            if !is_handled {
                                unsafe {
                                    let cb = &*s.callback;
                                    let vtable = &*cb.vtable;
                                    (vtable.on_exception)(s.callback as *mut c_void, s.context);
                                }
                                s.flagged_for_closure = true;
                            }
                            break;
                        }
                    }
                }
                let mut data_lock = data.lock().unwrap();
                // Sync state changes back to the main un-cloned list
                // (In case manager_add_socket added new elements while we awaited select)
                for modified in &sockets {
                    for real_s in &mut data_lock.sockets {
                        if real_s.raw_socket == modified.raw_socket {
                            // Do not overwrite wants_write if another thread explicitly set it to true while we were in select
                            if !modified.wants_write {
                                real_s.wants_write = false;
                            }
                            if modified.flagged_for_closure {
                                real_s.flagged_for_closure = true;
                            }
                            break;
                        }
                    }
                }

                // Remove closed sockets
                data_lock.sockets.retain(|s| !s.flagged_for_closure);
            }
        }
        info!("Rust Multiplexer Background Thread Ended");
    });
}

/// # Safety
/// This function is called from C++ side via COM vtable.
pub unsafe extern "thiscall" fn manager_stop_thread(this: *mut c_void) {
    info!("Manager Stop Thread Called");
    if let Some(data) = get_managers().lock().unwrap().get(&(this as usize)) {
        data.lock().unwrap().running = false;
    }
}

/// # Safety
/// This function is called from C++ side via COM vtable.
pub unsafe extern "thiscall" fn manager_add_socket(
    this: *mut c_void,
    socket: SOCKET,
    callback: *mut IConnectionCallback,
    context: i32,
) -> u8 {
    info!("Manager Add Socket Called!");
    let mut map = get_managers().lock().unwrap();
    if let Some(data) = map.get_mut(&(this as usize)) {
        let mut d = data.lock().unwrap();
        d.sockets.push(RegisteredSocket {
            raw_socket: socket,
            callback,
            context,
            wants_write: true,
            flagged_for_exception: false,
            flagged_for_closure: false,
        });
        return 1;
    }
    0
}

/// # Safety
/// This function is called from C++ side via COM vtable.
pub unsafe extern "thiscall" fn manager_flag_socket(this: *mut c_void, handle: SOCKET) -> u8 {
    info!("Manager Flag Socket Called for handle: {}", handle.0);
    let mut map = get_managers().lock().unwrap();
    if let Some(data) = map.get_mut(&(this as usize)) {
        let mut d = data.lock().unwrap();
        for s in &mut d.sockets {
            if s.raw_socket == handle && s.wants_write {
                s.flagged_for_exception = true;
                return 1;
            }
        }
    }
    0
}

/// # Safety
/// This function is called from C++ side via COM vtable.
pub unsafe extern "thiscall" fn manager_unk(_this: *mut c_void) {}

/// # Safety
/// This function modifies memory protections and writes raw pointers.
pub unsafe fn apply(info: &ModuleInfo) -> Result<(), String> {
    log::info!("Patching Connection Manager VTable...");

    let vtable_addr = info.resolve(0x37204AD4);
    // We need to unprotect the memory to write to the .rdata section
    let mut old_protect = windows::Win32::System::Memory::PAGE_PROTECTION_FLAGS(0);
    let size = 12 * 4;

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
        vtable.add(0).write(manager_dtor as usize);
        vtable.add(1).write(manager_add_ref as usize);
        vtable.add(2).write(manager_release as usize);
        vtable.add(3).write(manager_release as usize);
        vtable.add(4).write(manager_unk as usize);
        vtable.add(5).write(manager_unk as usize);
        vtable.add(6).write(manager_add_ref as usize);
        vtable.add(7).write(manager_start_thread as usize);
        vtable.add(8).write(manager_stop_thread as usize);
        vtable.add(9).write(manager_add_socket as usize);
        vtable.add(10).write(manager_flag_socket as usize);
        vtable.add(11).write(manager_unk as usize);
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

    info!("Connection Manager VTable patched successfully.");
    Ok(())
}
