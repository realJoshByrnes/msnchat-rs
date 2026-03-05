#![allow(unsafe_op_in_unsafe_fn)]

use crate::patch::module_info::ModuleInfo;
use serde_json::{Value, json};
use std::ffi::{CStr, c_char, c_void};
use std::fs;
use std::path::PathBuf;
use std::ptr;
use std::sync::{Mutex, OnceLock};
use windows::core::PCSTR;

// IMPORTANT NOTE ON HOOKING
//
// While we have built out a custom Rust `RegistryReaderVtbl` and overwrite the vtable
// pointer in the intercepted `ctor` function, we must currently STILL keep the MinHook
// detours for each individual query/set function.
//
// This is because the original MSNChat C++ codebase contains many direct `CALL` instructions
// to the absolute memory addresses of these functions (e.g., directly calling `RegistryReader_ReadDword`
// instead of doing an indirect virtual call via the VTable).
//
// Until we systematically track down and replace all of those rigid direct calls with virtual
// calls across the entire `.text` segment, removing the individual MinHooks would result in
// those direct calls slipping through to the original C++ implementation, bypassing our JSON backend.

static GLOBAL_SETTINGS: OnceLock<Mutex<Value>> = OnceLock::new();

fn get_settings() -> &'static Mutex<Value> {
    GLOBAL_SETTINGS.get_or_init(|| Mutex::new(load_settings()))
}

fn get_settings_path() -> PathBuf {
    let mut path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    path.set_file_name("msnchat_settings.json");
    path
}

fn load_settings() -> Value {
    if let Ok(data) = fs::read_to_string(get_settings_path())
        && let Ok(val) = serde_json::from_str(&data)
    {
        return val;
    }
    json!({})
}

fn save_settings(val: &Value) {
    if let Ok(data) = serde_json::to_string_pretty(val) {
        let _ = fs::write(get_settings_path(), data);
    }
}

// Trampoline Types
type RegistryReaderCtorType = unsafe extern "thiscall" fn(
    this: *mut c_void,
    h_key: *mut c_void,
    lp_sub_key: PCSTR,
    p_status: *mut u32,
    dw_disposition: u32,
    sam_desired: u32,
) -> *mut c_void;

type ScalarDeletingDestructorType =
    unsafe extern "thiscall" fn(this: *mut c_void, flags: u8) -> *mut c_void;

type DtorType = unsafe extern "thiscall" fn(this: *mut c_void) -> i32;

type QueryValueType = unsafe extern "thiscall" fn(
    this: *mut c_void,
    lp_value_name: PCSTR,
    lp_type: *mut u32,
    lp_data: *mut u8,
    lpcb_data: *mut u32,
) -> i32;

type SetValueType = unsafe extern "thiscall" fn(
    this: *mut c_void,
    lp_value_name: PCSTR,
    dw_type: u32,
    lp_data: *const u8,
    cb_data: u32,
) -> i32;

type WriteBooleanType =
    unsafe extern "thiscall" fn(this: *mut c_void, lp_value_name: PCSTR, dw_data: i32) -> i32;

type ReadBooleanType =
    unsafe extern "thiscall" fn(this: *mut c_void, lp_value_name: PCSTR, lp_data: *mut i32) -> i32;

type ReadStringType = unsafe extern "thiscall" fn(
    this: *mut c_void,
    lp_value_name: PCSTR,
    lp_data: *mut c_char,
    lpcb_data: *mut i32,
) -> i32;

type WriteStringType =
    unsafe extern "thiscall" fn(this: *mut c_void, lp_value_name: PCSTR, lp_data: PCSTR) -> i32;

type ReadDwordType =
    unsafe extern "thiscall" fn(this: *mut c_void, lp_value_name: PCSTR, lp_data: *mut u32) -> i32;

type WriteDwordType =
    unsafe extern "thiscall" fn(this: *mut c_void, lp_value_name: PCSTR, dw_data: u32) -> i32;

#[repr(C)]
#[allow(non_snake_case)]
pub struct RegistryReaderVtbl {
    pub ScalarDeletingDestructor: ScalarDeletingDestructorType,
    pub QueryValue: QueryValueType,
    pub SetValue: SetValueType,
    pub ReadBoolean: ReadBooleanType,
    pub WriteBoolean: WriteBooleanType,
    pub ReadString: ReadStringType,
    pub WriteString: WriteStringType,
    pub ReadDword: ReadDwordType,
    pub WriteDword: WriteDwordType,
}

#[repr(C)]
pub struct RegistryReader {
    pub vtbl: *const RegistryReaderVtbl,
    pub hkey: *mut c_void,
}

static mut TRAMPOLINE_CTOR: Option<RegistryReaderCtorType> = None;
static mut TRAMPOLINE_SCALAR_DELETING_DESTRUCTOR: Option<ScalarDeletingDestructorType> = None;
static mut TRAMPOLINE_DTOR: Option<DtorType> = None;
static mut TRAMPOLINE_QUERY: Option<QueryValueType> = None;
static mut TRAMPOLINE_SET: Option<SetValueType> = None;
static mut TRAMPOLINE_READ_BOOLEAN: Option<ReadBooleanType> = None;
static mut TRAMPOLINE_WRITE_BOOLEAN: Option<WriteBooleanType> = None;
static mut TRAMPOLINE_READ_STRING: Option<ReadStringType> = None;
static mut TRAMPOLINE_WRITE_STRING: Option<WriteStringType> = None;
static mut TRAMPOLINE_READ_DWORD: Option<ReadDwordType> = None;
static mut TRAMPOLINE_WRITE_DWORD: Option<WriteDwordType> = None;

static mut RUST_REGISTRY_READER_VTABLE: RegistryReaderVtbl = RegistryReaderVtbl {
    ScalarDeletingDestructor: hook_scalar_deleting_destructor,
    QueryValue: hook_query_value,
    SetValue: hook_set_value,
    ReadBoolean: hook_read_boolean,
    WriteBoolean: hook_write_boolean,
    ReadString: hook_read_string,
    WriteString: hook_write_string,
    ReadDword: hook_read_dword,
    WriteDword: hook_write_dword,
};

// Hooks
unsafe extern "thiscall" fn hook_registry_reader_ctor(
    this: *mut c_void,
    h_key: *mut c_void,
    lp_sub_key: PCSTR,
    p_status: *mut u32,
    dw_disposition: u32,
    sam_desired: u32,
) -> *mut c_void {
    let trampoline = TRAMPOLINE_CTOR.unwrap();
    let ret = trampoline(
        this,
        h_key,
        lp_sub_key,
        p_status,
        dw_disposition,
        sam_desired,
    );

    if !ret.is_null() {
        let reader = ret as *mut RegistryReader;
        (*reader).vtbl = &raw const RUST_REGISTRY_READER_VTABLE;
    }

    if !p_status.is_null() {
        *p_status = 0; // ERROR_SUCCESS
    }

    ret
}

unsafe extern "thiscall" fn hook_query_value(
    _this: *mut c_void,
    lp_value_name: PCSTR,
    lp_type: *mut u32,
    lp_data: *mut u8,
    lpcb_data: *mut u32,
) -> i32 {
    if lp_value_name.is_null() {
        return 2; // ERROR_FILE_NOT_FOUND
    }

    let key_name = match CStr::from_ptr(lp_value_name.0 as *const i8).to_str() {
        Ok(s) => s,
        Err(_) => return 2,
    };

    let settings = get_settings().lock().unwrap();

    if let Some(val) = settings.get(key_name) {
        if val.is_number() {
            let num = val.as_u64().unwrap_or(0) as u32;
            if !lp_type.is_null() {
                *lp_type = 4; // REG_DWORD
            }
            if !lpcb_data.is_null() {
                let size = *lpcb_data;
                if size >= 4 && !lp_data.is_null() {
                    let bytes = num.to_le_bytes();
                    ptr::copy_nonoverlapping(bytes.as_ptr(), lp_data, 4);
                }
                *lpcb_data = 4;
            }
            return 0; // ERROR_SUCCESS
        } else if val.is_string() {
            let s = val.as_str().unwrap();
            if !lp_type.is_null() {
                *lp_type = 1; // REG_SZ
            }
            if !lpcb_data.is_null() {
                let required_size = s.len() as u32 + 1;
                let available_size = *lpcb_data;
                *lpcb_data = required_size;

                if !lp_data.is_null() && available_size >= required_size {
                    ptr::copy_nonoverlapping(s.as_ptr(), lp_data, s.len());
                    *lp_data.add(s.len()) = 0; // Null terminator
                }
            }
            return 0; // ERROR_SUCCESS
        } else if val.is_array() {
            let arr = val.as_array().unwrap();
            if !lp_type.is_null() {
                *lp_type = 3; // REG_BINARY
            }
            if !lpcb_data.is_null() {
                let required_size = arr.len() as u32;
                let available_size = *lpcb_data;
                *lpcb_data = required_size;

                if !lp_data.is_null() && available_size >= required_size {
                    for (i, v) in arr.iter().enumerate() {
                        *lp_data.add(i) = v.as_u64().unwrap_or(0) as u8;
                    }
                }
            }
            return 0; // ERROR_SUCCESS
        }
    }

    2 // ERROR_FILE_NOT_FOUND
}

unsafe extern "thiscall" fn hook_set_value(
    _this: *mut c_void,
    lp_value_name: PCSTR,
    dw_type: u32,
    lp_data: *const u8,
    cb_data: u32,
) -> i32 {
    if lp_value_name.is_null() || lp_data.is_null() {
        return 87; // ERROR_INVALID_PARAMETER
    }

    let key_name = match CStr::from_ptr(lp_value_name.0 as *const i8).to_str() {
        Ok(s) => s,
        Err(_) => return 87,
    };

    let mut settings = get_settings().lock().unwrap();

    if dw_type == 4 && cb_data == 4 {
        let mut bytes = [0u8; 4];
        ptr::copy_nonoverlapping(lp_data, bytes.as_mut_ptr(), 4);
        let num = u32::from_le_bytes(bytes);
        settings[key_name] = json!(num);
    } else if dw_type == 1 && cb_data > 0 {
        let mut chars = Vec::new();
        for i in 0..cb_data {
            let c = *lp_data.add(i as usize);
            if c == 0 {
                break;
            }
            chars.push(c);
        }
        if let Ok(s) = String::from_utf8(chars) {
            settings[key_name] = json!(s);
        }
    } else if dw_type == 3 {
        // REG_BINARY
        let mut arr = Vec::new();
        for i in 0..cb_data {
            arr.push(json!(*lp_data.add(i as usize)));
        }
        settings[key_name] = json!(arr);
    }

    save_settings(&settings);

    0 // ERROR_SUCCESS
}

unsafe extern "thiscall" fn hook_read_string(
    _this: *mut c_void,
    lp_value_name: PCSTR,
    lp_data: *mut c_char,
    lpcb_data: *mut i32,
) -> i32 {
    let mut dw_type: u32 = 0;
    let mut cb_data: u32 = if !lpcb_data.is_null() {
        *lpcb_data as u32
    } else {
        0
    };

    let res = hook_query_value(
        _this,
        lp_value_name,
        &mut dw_type,
        lp_data as *mut u8,
        &mut cb_data,
    );

    if !lpcb_data.is_null() {
        *lpcb_data = cb_data as i32;
    }

    res
}

unsafe extern "thiscall" fn hook_write_string(
    _this: *mut c_void,
    lp_value_name: PCSTR,
    lp_data: PCSTR,
) -> i32 {
    let mut len = 0;
    if !lp_data.is_null() {
        unsafe {
            let mut ptr = lp_data.0;
            while *ptr != 0 {
                len += 1;
                ptr = ptr.add(1);
            }
            len += 1; // Include null terminator
        }
    }

    hook_set_value(
        _this,
        lp_value_name,
        1, // REG_SZ
        lp_data.0,
        len,
    )
}

unsafe extern "thiscall" fn hook_read_dword(
    _this: *mut c_void,
    lp_value_name: PCSTR,
    lp_data: *mut u32,
) -> i32 {
    let mut dw_type: u32 = 0;
    let mut cb_data: u32 = 4;

    hook_query_value(
        _this,
        lp_value_name,
        &mut dw_type,
        lp_data as *mut u8,
        &mut cb_data,
    )
}

unsafe extern "thiscall" fn hook_write_dword(
    _this: *mut c_void,
    lp_value_name: PCSTR,
    dw_data: u32,
) -> i32 {
    let bytes = dw_data.to_le_bytes();
    hook_set_value(
        _this,
        lp_value_name,
        4, // REG_DWORD
        bytes.as_ptr(),
        4,
    )
}

unsafe extern "thiscall" fn hook_read_boolean(
    _this: *mut c_void,
    lp_value_name: PCSTR,
    lp_data: *mut i32,
) -> i32 {
    let mut dw_type: u32 = 0;
    let mut cb_data: u32 = 4;

    hook_query_value(
        _this,
        lp_value_name,
        &mut dw_type,
        lp_data as *mut u8,
        &mut cb_data,
    )
}

unsafe extern "thiscall" fn hook_write_boolean(
    _this: *mut c_void,
    lp_value_name: PCSTR,
    dw_data: i32,
) -> i32 {
    let bytes = (dw_data as u32).to_le_bytes();
    hook_set_value(
        _this,
        lp_value_name,
        4, // REG_DWORD
        bytes.as_ptr(),
        4,
    )
}

unsafe extern "thiscall" fn hook_dtor(_this: *mut c_void) -> i32 {
    let trampoline = TRAMPOLINE_DTOR.unwrap();
    trampoline(_this)
}

unsafe extern "thiscall" fn hook_scalar_deleting_destructor(
    _this: *mut c_void,
    flags: u8,
) -> *mut c_void {
    let trampoline = TRAMPOLINE_SCALAR_DELETING_DESTRUCTOR.unwrap();
    trampoline(_this, flags)
}

/// # Safety
/// This function relies on an accurate ModuleInfo representing the mapped PE image.
pub unsafe fn apply(info: &ModuleInfo) -> Result<(), String> {
    let ctor_target = info.resolve(0x3721186e);
    let scalar_deleting_destructor_target = info.resolve(0x372119d2);
    let dtor_target = info.resolve(0x372118c1);

    // The following are part of the RegistryReader vtable, and have direct calls (so we need to hook them)
    let query_target = info.resolve(0x372118d6);
    let set_target = info.resolve(0x372118f4);
    let read_boolean_target = info.resolve(0x37211912);
    let write_boolean_target = info.resolve(0x37211917);
    let read_string_target = info.resolve(0x3721191c);
    let write_string_target = info.resolve(0x37211942);
    let read_dword_target = info.resolve(0x37211967);
    let write_dword_target = info.resolve(0x372119a2);

    let ctor_hook =
        minhook::MinHook::create_hook(ctor_target, hook_registry_reader_ctor as *mut c_void)
            .map_err(|e| format!("MinHook create error for RegistryReader_ctor: {:?}", e))?;

    let scalar_deleting_destructor_hook = minhook::MinHook::create_hook(
        scalar_deleting_destructor_target,
        hook_scalar_deleting_destructor as *mut c_void,
    )
    .map_err(|e| format!("MinHook create error for ScalarDeletingDestructor: {:?}", e))?;

    let dtor_hook = minhook::MinHook::create_hook(dtor_target, hook_dtor as *mut c_void)
        .map_err(|e| format!("MinHook create error for Dtor: {:?}", e))?;

    let query_hook = minhook::MinHook::create_hook(query_target, hook_query_value as *mut c_void)
        .map_err(|e| format!("MinHook create error for QueryValue: {:?}", e))?;

    let set_hook = minhook::MinHook::create_hook(set_target, hook_set_value as *mut c_void)
        .map_err(|e| format!("MinHook create error for SetValue: {:?}", e))?;

    let read_boolean_hook =
        minhook::MinHook::create_hook(read_boolean_target, hook_read_boolean as *mut c_void)
            .map_err(|e| format!("MinHook create error for ReadBoolean: {:?}", e))?;

    let write_boolean_hook =
        minhook::MinHook::create_hook(write_boolean_target, hook_write_boolean as *mut c_void)
            .map_err(|e| format!("MinHook create error for WriteBoolean: {:?}", e))?;

    let read_string_hook =
        minhook::MinHook::create_hook(read_string_target, hook_read_string as *mut c_void)
            .map_err(|e| format!("MinHook create error for ReadString: {:?}", e))?;

    let write_string_hook =
        minhook::MinHook::create_hook(write_string_target, hook_write_string as *mut c_void)
            .map_err(|e| format!("MinHook create error for WriteString: {:?}", e))?;

    let read_dword_hook =
        minhook::MinHook::create_hook(read_dword_target, hook_read_dword as *mut c_void)
            .map_err(|e| format!("MinHook create error for ReadDword: {:?}", e))?;

    let write_dword_hook =
        minhook::MinHook::create_hook(write_dword_target, hook_write_dword as *mut c_void)
            .map_err(|e| format!("MinHook create error for WriteDword: {:?}", e))?;

    minhook::MinHook::queue_enable_hook(ctor_target).map_err(|e| format!("Queue ctor {:?}", e))?;
    minhook::MinHook::queue_enable_hook(scalar_deleting_destructor_target)
        .map_err(|e| format!("Queue scalar dtor {:?}", e))?;
    minhook::MinHook::queue_enable_hook(dtor_target).map_err(|e| format!("Queue dtor {:?}", e))?;
    minhook::MinHook::queue_enable_hook(query_target)
        .map_err(|e| format!("Queue query {:?}", e))?;
    minhook::MinHook::queue_enable_hook(set_target).map_err(|e| format!("Queue set {:?}", e))?;
    minhook::MinHook::queue_enable_hook(read_boolean_target)
        .map_err(|e| format!("Queue read boolean {:?}", e))?;
    minhook::MinHook::queue_enable_hook(write_boolean_target)
        .map_err(|e| format!("Queue write boolean {:?}", e))?;
    minhook::MinHook::queue_enable_hook(read_string_target)
        .map_err(|e| format!("Queue read string {:?}", e))?;
    minhook::MinHook::queue_enable_hook(write_string_target)
        .map_err(|e| format!("Queue write string {:?}", e))?;
    minhook::MinHook::queue_enable_hook(read_dword_target)
        .map_err(|e| format!("Queue read dword {:?}", e))?;
    minhook::MinHook::queue_enable_hook(write_dword_target)
        .map_err(|e| format!("Queue write dword {:?}", e))?;

    TRAMPOLINE_CTOR = Some(std::mem::transmute::<*mut c_void, RegistryReaderCtorType>(
        ctor_hook,
    ));
    TRAMPOLINE_SCALAR_DELETING_DESTRUCTOR = Some(std::mem::transmute::<
        *mut c_void,
        ScalarDeletingDestructorType,
    >(scalar_deleting_destructor_hook));
    TRAMPOLINE_DTOR = Some(std::mem::transmute::<*mut c_void, DtorType>(dtor_hook));
    TRAMPOLINE_QUERY = Some(std::mem::transmute::<*mut c_void, QueryValueType>(
        query_hook,
    ));
    TRAMPOLINE_SET = Some(std::mem::transmute::<*mut c_void, SetValueType>(set_hook));
    TRAMPOLINE_READ_BOOLEAN = Some(std::mem::transmute::<*mut c_void, ReadBooleanType>(
        read_boolean_hook,
    ));
    TRAMPOLINE_WRITE_BOOLEAN = Some(std::mem::transmute::<*mut c_void, WriteBooleanType>(
        write_boolean_hook,
    ));
    TRAMPOLINE_READ_STRING = Some(std::mem::transmute::<*mut c_void, ReadStringType>(
        read_string_hook,
    ));
    TRAMPOLINE_WRITE_STRING = Some(std::mem::transmute::<*mut c_void, WriteStringType>(
        write_string_hook,
    ));
    TRAMPOLINE_READ_DWORD = Some(std::mem::transmute::<*mut c_void, ReadDwordType>(
        read_dword_hook,
    ));
    TRAMPOLINE_WRITE_DWORD = Some(std::mem::transmute::<*mut c_void, WriteDwordType>(
        write_dword_hook,
    ));

    Ok(())
}
