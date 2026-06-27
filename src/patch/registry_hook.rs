//! Virtual Registry hook to intercept ADVAPI32 registry calls from the OCX
//! and redirect them to config.toml or in-memory virtual state.
use crate::config::MSNConfigManager;
use std::collections::HashMap;
use std::ffi::{CStr, CString, c_void};
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use uuid::Uuid;

// Function pointer types for original registry APIs
type RegOpenKeyExAType = unsafe extern "system" fn(usize, *const i8, u32, u32, *mut usize) -> i32;
type RegCreateKeyExAType = unsafe extern "system" fn(
    usize,
    *const i8,
    u32,
    *const i8,
    u32,
    u32,
    *mut c_void,
    *mut usize,
    *mut u32,
) -> i32;
type RegCloseKeyType = unsafe extern "system" fn(usize) -> i32;
type RegQueryValueExAType =
    unsafe extern "system" fn(usize, *const i8, *mut u32, *mut u32, *mut u8, *mut u32) -> i32;
type RegSetValueExAType =
    unsafe extern "system" fn(usize, *const i8, u32, u32, *const u8, u32) -> i32;
type RegDeleteKeyAType = unsafe extern "system" fn(usize, *const i8) -> i32;
type RegDeleteValueAType = unsafe extern "system" fn(usize, *const i8) -> i32;
type RegQueryInfoKeyAType = unsafe extern "system" fn(
    usize,
    *mut i8,
    *mut u32,
    *mut u32,
    *mut u32,
    *mut u32,
    *mut u32,
    *mut u32,
    *mut u32,
    *mut u32,
    *mut u32,
    *mut c_void,
) -> i32;
type RegEnumValueAType = unsafe extern "system" fn(
    usize,
    u32,
    *mut i8,
    *mut u32,
    *mut u32,
    *mut u32,
    *mut u8,
    *mut u32,
) -> i32;
type RegEnumKeyExAType = unsafe extern "system" fn(
    usize,
    u32,
    *mut i8,
    *mut u32,
    *mut u32,
    *mut i8,
    *mut u32,
    *mut c_void,
) -> i32;

// Original trampoline function pointers
static mut O_REG_OPEN_KEY_EX_A: Option<RegOpenKeyExAType> = None;
static mut O_REG_CREATE_KEY_EX_A: Option<RegCreateKeyExAType> = None;
static mut O_REG_CLOSE_KEY: Option<RegCloseKeyType> = None;
static mut O_REG_QUERY_VALUE_EX_A: Option<RegQueryValueExAType> = None;
static mut O_REG_SET_VALUE_EX_A: Option<RegSetValueExAType> = None;
static mut O_REG_DELETE_KEY_A: Option<RegDeleteKeyAType> = None;
static mut O_REG_DELETE_VALUE_A: Option<RegDeleteValueAType> = None;
static mut O_REG_QUERY_INFO_KEY_A: Option<RegQueryInfoKeyAType> = None;
static mut O_REG_ENUM_VALUE_A: Option<RegEnumValueAType> = None;
static mut O_REG_ENUM_KEY_EX_A: Option<RegEnumKeyExAType> = None;

// Predefined registry root handle constants
const HKEY_CLASSES_ROOT: usize = 0x80000000;
const HKEY_CURRENT_USER: usize = 0x80000001;
const HKEY_LOCAL_MACHINE: usize = 0x80000002;

// Virtual handle pool
static NEXT_VIRTUAL_HANDLE: AtomicUsize = AtomicUsize::new(0xDEADC000);

lazy_static::lazy_static! {
    // Maps virtual handle -> full key path (e.g. "HKCU\Software\Microsoft\MSNChat\4.0")
    static ref VIRTUAL_HANDLES: Mutex<HashMap<usize, String>> = Mutex::new(HashMap::new());
}

/// Helper to check if a path should be virtualized
fn is_virtual_path(path: &str) -> bool {
    let p = path.to_lowercase();
    p.contains("software\\microsoft\\msnchat")
        || p.contains("appevents")
        || p.contains("clsid\\{f58e1cef")
        || p.contains("clsid\\{fa980e7e")
        || p.contains("activex compatibility")
        || p.contains("currentversion")
}

/// Resolves the full path of a key handle + optional subkey
fn resolve_path(hkey: usize, subkey: Option<&str>) -> Option<String> {
    let mut base = match hkey {
        HKEY_CLASSES_ROOT => Some("HKCR".to_string()),
        HKEY_CURRENT_USER => Some("HKCU".to_string()),
        HKEY_LOCAL_MACHINE => Some("HKLM".to_string()),
        other => {
            let guard = VIRTUAL_HANDLES.lock().unwrap();
            guard.get(&other).cloned()
        }
    }?;

    if let Some(sub) = subkey
        && !sub.is_empty()
    {
        base.push('\\');
        base.push_str(sub);
    }
    Some(base)
}

/// # Safety
///
/// Installs MinHook detours on ADVAPI32 registry APIs.
pub unsafe fn apply(_info: &super::module_info::ModuleInfo) -> Result<(), String> {
    use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
    use windows::core::{PCSTR, w};

    let advapi32 = unsafe { GetModuleHandleW(w!("advapi32.dll")) }
        .map_err(|e| format!("Failed to get advapi32.dll handle: {}", e))?;

    let apis = [
        (
            "RegOpenKeyExA",
            std::ptr::addr_of_mut!(O_REG_OPEN_KEY_EX_A) as *mut c_void,
            detour_reg_open_key_ex_a as *mut c_void,
        ),
        (
            "RegCreateKeyExA",
            std::ptr::addr_of_mut!(O_REG_CREATE_KEY_EX_A) as *mut c_void,
            detour_reg_create_key_ex_a as *mut c_void,
        ),
        (
            "RegCloseKey",
            std::ptr::addr_of_mut!(O_REG_CLOSE_KEY) as *mut c_void,
            detour_reg_close_key as *mut c_void,
        ),
        (
            "RegQueryValueExA",
            std::ptr::addr_of_mut!(O_REG_QUERY_VALUE_EX_A) as *mut c_void,
            detour_reg_query_value_ex_a as *mut c_void,
        ),
        (
            "RegSetValueExA",
            std::ptr::addr_of_mut!(O_REG_SET_VALUE_EX_A) as *mut c_void,
            detour_reg_set_value_ex_a as *mut c_void,
        ),
        (
            "RegDeleteKeyA",
            std::ptr::addr_of_mut!(O_REG_DELETE_KEY_A) as *mut c_void,
            detour_reg_delete_key_a as *mut c_void,
        ),
        (
            "RegDeleteValueA",
            std::ptr::addr_of_mut!(O_REG_DELETE_VALUE_A) as *mut c_void,
            detour_reg_delete_value_a as *mut c_void,
        ),
        (
            "RegQueryInfoKeyA",
            std::ptr::addr_of_mut!(O_REG_QUERY_INFO_KEY_A) as *mut c_void,
            detour_reg_query_info_key_a as *mut c_void,
        ),
        (
            "RegEnumValueA",
            std::ptr::addr_of_mut!(O_REG_ENUM_VALUE_A) as *mut c_void,
            detour_reg_enum_value_a as *mut c_void,
        ),
        (
            "RegEnumKeyExA",
            std::ptr::addr_of_mut!(O_REG_ENUM_KEY_EX_A) as *mut c_void,
            detour_reg_enum_key_ex_a as *mut c_void,
        ),
    ];

    for (name, trampoline, detour) in apis {
        let proc = unsafe {
            GetProcAddress(
                advapi32,
                PCSTR::from_raw(CString::new(name).unwrap().as_ptr() as *const u8),
            )
        };
        if let Some(p) = proc {
            let orig = unsafe { super::hook(p as *mut c_void, detour)? };
            unsafe {
                *(trampoline as *mut *mut c_void) = orig;
            }
        } else {
            return Err(format!("Failed to resolve registry API: {}", name));
        }
    }

    log::info!("Registry virtualization detours applied successfully!");
    Ok(())
}

// === API Detours ===

unsafe extern "system" fn detour_reg_open_key_ex_a(
    hkey: usize,
    lp_subkey: *const i8,
    _ul_options: u32,
    _sam_desired: u32,
    phk_result: *mut usize,
) -> i32 {
    unsafe {
        let subkey_str = if lp_subkey.is_null() {
            ""
        } else {
            CStr::from_ptr(lp_subkey).to_str().unwrap_or("")
        };
        let path = resolve_path(hkey, Some(subkey_str));

        if let Some(p) = &path
            && is_virtual_path(p)
        {
            let v_hkey = NEXT_VIRTUAL_HANDLE.fetch_add(1, Ordering::SeqCst);
            log::info!(
                "Virtualizing RegOpenKeyExA -> handle: {:#x} path: {}",
                v_hkey,
                p
            );
            VIRTUAL_HANDLES.lock().unwrap().insert(v_hkey, p.clone());
            unsafe { *phk_result = v_hkey };
            return 0; // Success
        }

        if let Some(orig) = unsafe { O_REG_OPEN_KEY_EX_A } {
            unsafe { orig(hkey, lp_subkey, _ul_options, _sam_desired, phk_result) }
        } else {
            2 // ERROR_FILE_NOT_FOUND
        }
    }
}

unsafe extern "system" fn detour_reg_create_key_ex_a(
    hkey: usize,
    lp_subkey: *const i8,
    reserved: u32,
    lp_class: *const i8,
    dw_options: u32,
    sam_desired: u32,
    lp_security_attributes: *mut c_void,
    phk_result: *mut usize,
    lp_dw_disposition: *mut u32,
) -> i32 {
    unsafe {
        let subkey_str = if lp_subkey.is_null() {
            ""
        } else {
            CStr::from_ptr(lp_subkey).to_str().unwrap_or("")
        };
        let path = resolve_path(hkey, Some(subkey_str));

        if let Some(p) = &path
            && is_virtual_path(p)
        {
            let v_hkey = NEXT_VIRTUAL_HANDLE.fetch_add(1, Ordering::SeqCst);
            log::info!(
                "Virtualizing RegCreateKeyExA -> handle: {:#x} path: {}",
                v_hkey,
                p
            );
            VIRTUAL_HANDLES.lock().unwrap().insert(v_hkey, p.clone());
            unsafe { *phk_result = v_hkey };
            if !lp_dw_disposition.is_null() {
                unsafe { *lp_dw_disposition = 1 }; // REG_CREATED_NEW_KEY
            }
            return 0; // Success
        }

        if let Some(orig) = unsafe { O_REG_CREATE_KEY_EX_A } {
            unsafe {
                orig(
                    hkey,
                    lp_subkey,
                    reserved,
                    lp_class,
                    dw_options,
                    sam_desired,
                    lp_security_attributes,
                    phk_result,
                    lp_dw_disposition,
                )
            }
        } else {
            2 // ERROR_FILE_NOT_FOUND
        }
    }
}

unsafe extern "system" fn detour_reg_close_key(hkey: usize) -> i32 {
    if hkey >= 0xDEADC000 {
        let mut guard = VIRTUAL_HANDLES.lock().unwrap();
        if guard.remove(&hkey).is_some() {
            log::debug!("Virtual key closed: {:#x}", hkey);
            return 0; // Success
        }
    }

    if let Some(orig) = unsafe { O_REG_CLOSE_KEY } {
        unsafe { orig(hkey) }
    } else {
        6 // ERROR_INVALID_HANDLE
    }
}

unsafe extern "system" fn detour_reg_query_value_ex_a(
    hkey: usize,
    lp_value_name: *const i8,
    lp_reserved: *mut u32,
    lp_type: *mut u32,
    lp_data: *mut u8,
    lpcb_data: *mut u32,
) -> i32 {
    unsafe {
        let val_name = if lp_value_name.is_null() {
            ""
        } else {
            CStr::from_ptr(lp_value_name).to_str().unwrap_or("")
        };
        let path = resolve_path(hkey, None);

        if let Some(p) = &path
            && is_virtual_path(p)
        {
            log::info!(
                "Virtualizing RegQueryValueExA -> path: {} value: {}",
                p,
                val_name
            );
            let p_lower = p.to_lowercase();

            // 1. Session tracking HKCU\Software\Microsoft\MSNChat\4.0
            if p_lower.ends_with("msnchat\\4.0") {
                let manager = MSNConfigManager::new(Path::new("config.toml"));
                if let Ok(config) = manager.load() {
                    if val_name.eq_ignore_ascii_case("UserData1") {
                        // Return the token (REG_SZ)
                        let token = if config.session.token.is_empty() {
                            manager.update_user_session().unwrap_or_default()
                        } else {
                            config.session.token.clone()
                        };
                        let bytes = token.as_bytes();
                        let len = bytes.len();
                        if !lp_type.is_null() {
                            unsafe { *lp_type = 1 };
                        } // REG_SZ
                        if !lpcb_data.is_null() {
                            let max_len = unsafe { *lpcb_data } as usize;
                            unsafe { *lpcb_data = (len + 1) as u32 };
                            if !lp_data.is_null() && max_len > len {
                                unsafe {
                                    std::ptr::copy_nonoverlapping(bytes.as_ptr(), lp_data, len);
                                    *lp_data.add(len) = 0;
                                }
                            }
                        }
                        return 0; // Success
                    } else if val_name.eq_ignore_ascii_case("UserData2") {
                        // Return last rotated timestamp (REG_DWORD)
                        if !lp_type.is_null() {
                            unsafe { *lp_type = 4 };
                        } // REG_DWORD
                        if !lpcb_data.is_null() {
                            unsafe { *lpcb_data = 4 };
                            if !lp_data.is_null() {
                                unsafe { *(lp_data as *mut u32) = config.session.last_rotated };
                            }
                        }
                        return 0;
                    } else if let Some((value_type, data_bytes)) = config.settings.get_value(val_name) {
                        if !lp_type.is_null() {
                            unsafe { *lp_type = value_type };
                        }
                        if !lpcb_data.is_null() {
                            let max_len = unsafe { *lpcb_data } as usize;
                            unsafe { *lpcb_data = data_bytes.len() as u32 };
                            if !lp_data.is_null() && max_len >= data_bytes.len() {
                                unsafe {
                                    std::ptr::copy_nonoverlapping(data_bytes.as_ptr(), lp_data, data_bytes.len());
                                }
                            }
                        }
                        return 0; // Success
                    } else {
                        return 2; // ERROR_FILE_NOT_FOUND
                    }
                }
            }

            // 2. Resource DLL folder HKCU\Software\Microsoft\MSNChat\4.0\ResDLLInstalled
            if p_lower.contains("resdllinstalled") {
                // If it is querying any DLL file name path, return it as installed
                if !lp_type.is_null() {
                    unsafe { *lp_type = 4 };
                } // REG_DWORD
                if !lpcb_data.is_null() {
                    unsafe { *lpcb_data = 4 };
                    if !lp_data.is_null() {
                        unsafe { *(lp_data as *mut u32) = 0 };
                    }
                }
                return 0;
            }

            // 3. Sound Scheme Schemes HKCU\AppEvents\Schemes\Apps\ChatOCX\<Event>\.Current
            if p_lower.contains("appevents\\schemes\\apps\\chatocx") {
                // Extract event name (e.g. "msnchat_Whisper") from the path
                let parts: Vec<&str> = p.split('\\').collect();
                let event_id = parts.iter().rev().nth(1).unwrap_or(&"");

                // Mapped wav or default if missing
                let default_wav = match *event_id {
                    "msnchat_Whisper" | "msnchat_HostWhisper" | "msnchat_TagWhisper" => {
                        "ChatWhsp.wav"
                    }
                    "msnchat_Arrival" => "ChatJoin.wav",
                    "msnchat_TagMessage" => "ChatTag.wav",
                    "msnchat_Kick" => "ChatKick.wav",
                    "msnchat_Invitation" => "ChatInvt.wav",
                    _ => "",
                };
                let wav = default_wav;

                let bytes = wav.as_bytes();
                let len = bytes.len();
                if !lp_type.is_null() {
                    unsafe { *lp_type = 1 };
                } // REG_SZ
                if !lpcb_data.is_null() {
                    let max_len = unsafe { *lpcb_data } as usize;
                    unsafe { *lpcb_data = (len + 1) as u32 };
                    if !lp_data.is_null() && max_len > len {
                        unsafe {
                            std::ptr::copy_nonoverlapping(bytes.as_ptr(), lp_data, len);
                            *lp_data.add(len) = 0;
                        }
                    }
                }
                return 0;
            }

            // 4. Default Schemes HKCU\AppEvents\Schemes
            if p_lower.ends_with("appevents\\schemes") && val_name.is_empty() {
                let bytes = b".Current";
                if !lp_type.is_null() {
                    unsafe { *lp_type = 1 };
                }
                if !lpcb_data.is_null() {
                    let max_len = unsafe { *lpcb_data } as usize;
                    unsafe { *lpcb_data = 9 };
                    if !lp_data.is_null() && max_len >= 9 {
                        unsafe {
                            std::ptr::copy_nonoverlapping(bytes.as_ptr(), lp_data, 8);
                            *lp_data.add(8) = 0;
                        }
                    }
                }
                return 0;
            }

            // 5. System MediaPath HKLM\Software\Microsoft\Windows\CurrentVersion
            if p_lower.contains("windows\\currentversion")
                && val_name.eq_ignore_ascii_case("MediaPath")
            {
                let bytes = b"C:\\Windows\\Media";
                if !lp_type.is_null() {
                    unsafe { *lp_type = 1 };
                }
                if !lpcb_data.is_null() {
                    let max_len = unsafe { *lpcb_data } as usize;
                    unsafe { *lpcb_data = 17 };
                    if !lp_data.is_null() && max_len >= 17 {
                        unsafe {
                            std::ptr::copy_nonoverlapping(bytes.as_ptr(), lp_data, 16);
                            *lp_data.add(16) = 0;
                        }
                    }
                }
                return 0;
            }

            // 6. COM integrity key verification under CLSID
            if p_lower.contains("clsid\\{f58e1cef") || p_lower.contains("clsid\\{fa980e7e") {
                let manager = MSNConfigManager::new(Path::new("config.toml"));
                let config = manager.load().unwrap_or_default();

                if val_name.eq_ignore_ascii_case("{E113C6A6-D44A-4639-A40E-3B6DE32A1A40}") {
                    let guid_str = if config.licensing.guid.is_empty() {
                        let new_guid = Uuid::new_v4().simple().to_string();
                        let mut updated = config.clone();
                        updated.licensing.guid = new_guid.clone();
                        let _ = manager.save(&updated);
                        new_guid
                    } else {
                        config.licensing.guid.clone()
                    };

                    if let Ok(bytes) = hex::decode(&guid_str) {
                        if !lp_type.is_null() {
                            unsafe { *lp_type = 3 };
                        } // REG_BINARY
                        if !lpcb_data.is_null() {
                            unsafe { *lpcb_data = 16 };
                            if !lp_data.is_null() {
                                unsafe {
                                    std::ptr::copy_nonoverlapping(bytes.as_ptr(), lp_data, 16)
                                };
                            }
                        }
                        return 0;
                    }
                } else if val_name.eq_ignore_ascii_case("{5954F421-4768-46bc-B331-3DC37B1E7048}") {
                    let hash_str = if config.licensing.hash.is_empty() {
                        let new_hash = Uuid::new_v4().simple().to_string(); // Simple random 16 bytes for mock
                        let mut updated = config.clone();
                        updated.licensing.hash = new_hash.clone();
                        let _ = manager.save(&updated);
                        new_hash
                    } else {
                        config.licensing.hash.clone()
                    };

                    if let Ok(bytes) = hex::decode(&hash_str) {
                        if !lp_type.is_null() {
                            unsafe { *lp_type = 3 };
                        } // REG_BINARY
                        if !lpcb_data.is_null() {
                            unsafe { *lpcb_data = 16 };
                            if !lp_data.is_null() {
                                unsafe {
                                    std::ptr::copy_nonoverlapping(bytes.as_ptr(), lp_data, 16)
                                };
                            }
                        }
                        return 0;
                    }
                }
            }

            // 7. Internet Explorer killbit
            if p_lower.contains("activex compatibility")
                && val_name.eq_ignore_ascii_case("Compatibility Flags")
            {
                if !lp_type.is_null() {
                    unsafe { *lp_type = 4 };
                } // REG_DWORD
                if !lpcb_data.is_null() {
                    unsafe { *lpcb_data = 4 };
                    if !lp_data.is_null() {
                        unsafe { *(lp_data as *mut u32) = 0 }; // Return 0 (Allowed / No killbit)
                    }
                }
                return 0;
            }
        }

        if let Some(orig) = unsafe { O_REG_QUERY_VALUE_EX_A } {
            unsafe {
                orig(
                    hkey,
                    lp_value_name,
                    lp_reserved,
                    lp_type,
                    lp_data,
                    lpcb_data,
                )
            }
        } else {
            2 // ERROR_FILE_NOT_FOUND
        }
    }
}

unsafe extern "system" fn detour_reg_set_value_ex_a(
    hkey: usize,
    lp_value_name: *const i8,
    _reserved: u32,
    _dw_type: u32,
    lp_data: *const u8,
    cb_data: u32,
) -> i32 {
    unsafe {
        let val_name = if lp_value_name.is_null() {
            ""
        } else {
            CStr::from_ptr(lp_value_name).to_str().unwrap_or("")
        };
        let path = resolve_path(hkey, None);

        if let Some(p) = &path
            && is_virtual_path(p)
        {
            log::info!(
                "Virtualizing RegSetValueExA -> path: {} value: {}",
                p,
                val_name
            );
            let p_lower = p.to_lowercase();
            let manager = MSNConfigManager::new(Path::new("config.toml"));
            let mut config = manager.load().unwrap_or_default();

            // 1. Session state HKCU\Software\Microsoft\MSNChat\4.0
            if p_lower.ends_with("msnchat\\4.0") {
                if val_name.eq_ignore_ascii_case("UserData1") {
                    let bytes = unsafe { std::slice::from_raw_parts(lp_data, cb_data as usize) };
                    let token = String::from_utf8_lossy(bytes)
                        .trim_end_matches('\0')
                        .to_string();
                    config.session.token = token;
                    let _ = manager.save(&config);
                    return 0;
                } else if val_name.eq_ignore_ascii_case("UserData2") {
                    let ts = unsafe { *(lp_data as *const u32) };
                    config.session.last_rotated = ts;
                    let _ = manager.save(&config);
                    return 0;
                } else {
                    let bytes = unsafe { std::slice::from_raw_parts(lp_data, cb_data as usize) };
                    if config.settings.set_value(val_name, _dw_type, bytes) {
                        let _ = manager.save(&config);
                    }
                    return 0;
                }
            }

            // 2. DLL Registration HKCU\Software\Microsoft\MSNChat\4.0\ResDLLInstalled
            if p_lower.contains("resdllinstalled") {
                // DLL file path is the value name
                let _ = manager.register_res_dll(Path::new(val_name));
                return 0;
            }

            // 3. COM registration integrity keys
            if p_lower.contains("clsid\\{f58e1cef") || p_lower.contains("clsid\\{fa980e7e") {
                if val_name.eq_ignore_ascii_case("{E113C6A6-D44A-4639-A40E-3B6DE32A1A40}") {
                    let bytes = unsafe { std::slice::from_raw_parts(lp_data, 16) };
                    config.licensing.guid = hex::encode(bytes);
                    let _ = manager.save(&config);
                    return 0;
                } else if val_name.eq_ignore_ascii_case("{5954F421-4768-46bc-B331-3DC37B1E7048}") {
                    let bytes = unsafe { std::slice::from_raw_parts(lp_data, 16) };
                    config.licensing.hash = hex::encode(bytes);
                    let _ = manager.save(&config);
                    return 0;
                }
            }

            // 4. Sound Scheme settings HKCU\AppEvents\Schemes\Apps\ChatOCX\<Event>\.Current
            if p_lower.contains("appevents\\schemes\\apps\\chatocx") {
                let parts: Vec<&str> = p.split('\\').collect();
                let event_id = parts.iter().rev().nth(1).unwrap_or(&"");
                let bytes = unsafe { std::slice::from_raw_parts(lp_data, cb_data as usize) };
                let wav = String::from_utf8_lossy(bytes)
                    .trim_end_matches('\0')
                    .to_string();
                log::info!(
                    "Sound Scheme write bypassed for event: {}, wav: {}",
                    event_id,
                    wav
                );
                return 0;
            }

            return 0; // Mock success for other virtual key writes
        }

        if let Some(orig) = unsafe { O_REG_SET_VALUE_EX_A } {
            unsafe { orig(hkey, lp_value_name, _reserved, _dw_type, lp_data, cb_data) }
        } else {
            6 // ERROR_INVALID_HANDLE
        }
    }
}

unsafe extern "system" fn detour_reg_delete_key_a(hkey: usize, lp_subkey: *const i8) -> i32 {
    unsafe {
        let subkey_str = if lp_subkey.is_null() {
            ""
        } else {
            CStr::from_ptr(lp_subkey).to_str().unwrap_or("")
        };
        let path = resolve_path(hkey, Some(subkey_str));

        if let Some(p) = &path
            && is_virtual_path(p)
        {
            log::info!("Virtualizing RegDeleteKeyA -> path: {}", p);
            let p_lower = p.to_lowercase();
            if p_lower.ends_with("msnchat\\4.0") {
                let manager = MSNConfigManager::new(Path::new("config.toml"));
                let _ = manager.clean_and_unregister();
            }
            return 0;
        }

        if let Some(orig) = unsafe { O_REG_DELETE_KEY_A } {
            unsafe { orig(hkey, lp_subkey) }
        } else {
            2 // ERROR_FILE_NOT_FOUND
        }
    }
}

unsafe extern "system" fn detour_reg_delete_value_a(hkey: usize, lp_value_name: *const i8) -> i32 {
    unsafe {
        let val_name = if lp_value_name.is_null() {
            ""
        } else {
            CStr::from_ptr(lp_value_name).to_str().unwrap_or("")
        };
        let path = resolve_path(hkey, None);

        if let Some(p) = &path
            && is_virtual_path(p)
        {
            log::info!(
                "Virtualizing RegDeleteValueA -> path: {} value: {}",
                p,
                val_name
            );
            let p_lower = p.to_lowercase();
            if p_lower.ends_with("msnchat\\4.0") {
                let manager = MSNConfigManager::new(Path::new("config.toml"));
                if let Ok(mut config) = manager.load() {
                    if val_name.eq_ignore_ascii_case("UserData1") {
                        config.session.token.clear();
                    } else if val_name.eq_ignore_ascii_case("UserData2") {
                        config.session.last_rotated = 0;
                    }
                    let _ = manager.save(&config);
                }
            }
            return 0;
        }

        if let Some(orig) = unsafe { O_REG_DELETE_VALUE_A } {
            unsafe { orig(hkey, lp_value_name) }
        } else {
            2 // ERROR_FILE_NOT_FOUND
        }
    }
}

unsafe extern "system" fn detour_reg_query_info_key_a(
    hkey: usize,
    lp_class: *mut i8,
    lpcch_class: *mut u32,
    lp_reserved: *mut u32,
    lpc_subkeys: *mut u32,
    lpcb_max_subkey_len: *mut u32,
    lpcb_max_class_len: *mut u32,
    lpc_values: *mut u32,
    lpcb_max_value_name_len: *mut u32,
    lpcb_max_value_len: *mut u32,
    lpcb_security_descriptor: *mut u32,
    lpft_last_write_time: *mut c_void,
) -> i32 {
    let path = resolve_path(hkey, None);

    if let Some(p) = &path
        && is_virtual_path(p)
    {
        log::info!("Virtualizing RegQueryInfoKeyA -> path: {}", p);
        if !lpc_subkeys.is_null() {
            unsafe { *lpc_subkeys = 0 };
        }
        if !lpc_values.is_null() {
            unsafe { *lpc_values = 0 };
        }
        return 0;
    }

    if let Some(orig) = unsafe { O_REG_QUERY_INFO_KEY_A } {
        unsafe {
            orig(
                hkey,
                lp_class,
                lpcch_class,
                lp_reserved,
                lpc_subkeys,
                lpcb_max_subkey_len,
                lpcb_max_class_len,
                lpc_values,
                lpcb_max_value_name_len,
                lpcb_max_value_len,
                lpcb_security_descriptor,
                lpft_last_write_time,
            )
        }
    } else {
        6 // ERROR_INVALID_HANDLE
    }
}

unsafe extern "system" fn detour_reg_enum_value_a(
    hkey: usize,
    dw_index: u32,
    lp_value_name: *mut i8,
    lpcch_value_name: *mut u32,
    lp_reserved: *mut u32,
    lp_type: *mut u32,
    lp_data: *mut u8,
    lpcb_data: *mut u32,
) -> i32 {
    let path = resolve_path(hkey, None);

    if let Some(p) = &path
        && is_virtual_path(p)
    {
        log::info!(
            "Virtualizing RegEnumValueA -> path: {} index: {}",
            p,
            dw_index
        );
        let p_lower = p.to_lowercase();
        if p_lower.contains("resdllinstalled") {
            let manager = MSNConfigManager::new(Path::new("config.toml"));
            let config = manager.load().unwrap_or_default();
            let dlls = &config.paths.resource_dlls;
            if (dw_index as usize) < dlls.len() {
                let path_str = dlls[dw_index as usize].to_string_lossy();
                let bytes = path_str.as_bytes();
                let len = bytes.len();

                let max_name_len = unsafe { *lpcch_value_name } as usize;
                if max_name_len > len {
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            bytes.as_ptr(),
                            lp_value_name as *mut u8,
                            len,
                        );
                        *lp_value_name.add(len) = 0;
                        *lpcch_value_name = len as u32;
                    }
                }
                if !lp_type.is_null() {
                    unsafe { *lp_type = 4 };
                } // REG_DWORD
                if !lpcb_data.is_null() {
                    unsafe { *lpcb_data = 4 };
                    if !lp_data.is_null() {
                        unsafe { *(lp_data as *mut u32) = 0 };
                    }
                }
                return 0;
            }
        }
        return 259; // ERROR_NO_MORE_ITEMS
    }

    if let Some(orig) = unsafe { O_REG_ENUM_VALUE_A } {
        unsafe {
            orig(
                hkey,
                dw_index,
                lp_value_name,
                lpcch_value_name,
                lp_reserved,
                lp_type,
                lp_data,
                lpcb_data,
            )
        }
    } else {
        6 // ERROR_INVALID_HANDLE
    }
}

unsafe extern "system" fn detour_reg_enum_key_ex_a(
    hkey: usize,
    dw_index: u32,
    lp_name: *mut i8,
    lpcch_name: *mut u32,
    lp_reserved: *mut u32,
    lp_class: *mut i8,
    lpcch_class: *mut u32,
    lpft_last_write_time: *mut c_void,
) -> i32 {
    let path = resolve_path(hkey, None);

    if let Some(p) = &path
        && is_virtual_path(p)
    {
        log::info!(
            "Virtualizing RegEnumKeyExA -> path: {} index: {}",
            p,
            dw_index
        );
        return 259; // ERROR_NO_MORE_ITEMS
    }

    if let Some(orig) = unsafe { O_REG_ENUM_KEY_EX_A } {
        unsafe {
            orig(
                hkey,
                dw_index,
                lp_name,
                lpcch_name,
                lp_reserved,
                lp_class,
                lpcch_class,
                lpft_last_write_time,
            )
        }
    } else {
        6 // ERROR_INVALID_HANDLE
    }
}
