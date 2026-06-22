use std::ffi::c_void;
use windows::core::{GUID, HRESULT, Interface};
use windows::Win32::Foundation::{E_NOINTERFACE, E_NOTIMPL, E_POINTER, S_OK};

use super::SharedSiteState;

pub const IID_IWEBBROWSER2: GUID = GUID::from_values(
    0x0002DF05,
    0x0000,
    0x0000,
    [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
);

#[repr(C)]
pub struct IWebBrowser2Vtbl {
    pub QueryInterface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    pub AddRef: unsafe extern "system" fn(*mut c_void) -> u32,
    pub Release: unsafe extern "system" fn(*mut c_void) -> u32,
    
    // IDispatch methods (Index 3-6)
    pub GetTypeInfoCount: unsafe extern "system" fn(*mut c_void, *mut u32) -> HRESULT,
    pub GetTypeInfo: unsafe extern "system" fn(*mut c_void, u32, u32, *mut *mut c_void) -> HRESULT,
    pub GetIDsOfNames: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut u8, u32, u32, *mut i32) -> HRESULT,
    pub Invoke: unsafe extern "system" fn(*mut c_void, i32, *const GUID, u32, u16, *mut c_void, *mut c_void, *mut c_void, *mut u32) -> HRESULT,
    
    // IWebBrowser methods (Index 7-10)
    pub GoBack: unsafe extern "system" fn(*mut c_void) -> HRESULT,
    pub GoForward: unsafe extern "system" fn(*mut c_void) -> HRESULT,
    pub GoHome: unsafe extern "system" fn(*mut c_void) -> HRESULT,
    pub GoSearch: unsafe extern "system" fn(*mut c_void) -> HRESULT,
    
    // Navigate (Index 11 / offset 0x2C)
    pub Navigate: unsafe extern "system" fn(
        *mut c_void,
        *const u16, // BSTR URL representation
        *mut c_void, // VARIANT* Flags
        *mut c_void, // VARIANT* TargetFrameName
        *mut c_void, // VARIANT* PostData
        *mut c_void, // VARIANT* Headers
    ) -> HRESULT,
}

#[repr(C)]
pub struct MyWebBrowser {
    pub lp_vtbl: *const IWebBrowser2Vtbl,
    pub shared: *mut SharedSiteState,
}

unsafe extern "system" fn query_interface(
    this: *mut c_void,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    unsafe {
        if ppv.is_null() {
            return E_POINTER;
        }

        let this = this as *mut MyWebBrowser;
        let riid = &*riid;

        if riid == &IID_IWEBBROWSER2 || riid == &windows::core::IUnknown::IID {
            *ppv = this as *mut c_void;
            add_ref(this as *mut c_void);
            S_OK
        } else {
            *ppv = std::ptr::null_mut();
            E_NOINTERFACE
        }
    }
}

pub unsafe extern "system" fn add_ref(this: *mut c_void) -> u32 {
    unsafe {
        let site = &mut *this.cast::<MyWebBrowser>();
        (*site.shared).ref_count += 1;
        (*site.shared).ref_count
    }
}

pub unsafe extern "system" fn release(this: *mut c_void) -> u32 {
    unsafe {
        let site = &mut *this.cast::<MyWebBrowser>();
        (*site.shared).ref_count -= 1;
        let count = (*site.shared).ref_count;
        if count == 0 {
            drop(Box::from_raw(this as *mut MyWebBrowser));
        }
        count
    }
}

#[repr(C)]
pub struct VARIANT {
    pub vt: u16,
    pub w_reserved1: u16,
    pub w_reserved2: u16,
    pub w_reserved3: u16,
    pub data: [u8; 8],
}

unsafe fn variant_to_string(var_ptr: *mut c_void) -> String {
    if var_ptr.is_null() {
        return "Null".to_string();
    }
    let var = unsafe { &*(var_ptr as *const VARIANT) };
    match var.vt {
        0 => "Empty".to_string(),
        3 => {
            let val = i32::from_ne_bytes(var.data[0..4].try_into().unwrap());
            format!("I4({})", val)
        }
        8 => {
            let bstr_ptr = usize::from_ne_bytes(var.data[0..4].try_into().unwrap()) as *const u16;
            if bstr_ptr.is_null() {
                "BSTR(null)".to_string()
            } else {
                unsafe {
                    let len = *((bstr_ptr as *const u32).offset(-1)) / 2;
                    let slice = std::slice::from_raw_parts(bstr_ptr, len as usize);
                    let s = String::from_utf16_lossy(slice);
                    format!("BSTR(\"{}\")", s)
                }
            }
        }
        _ => format!("Type({})", var.vt),
    }
}

unsafe extern "system" fn dummy_method(_this: *mut c_void) -> HRESULT {
    E_NOTIMPL
}

unsafe extern "system" fn dummy_get_type_info_count(_this: *mut c_void, _count: *mut u32) -> HRESULT {
    E_NOTIMPL
}

unsafe extern "system" fn dummy_get_type_info(_this: *mut c_void, _i: u32, _lcid: u32, _pp: *mut *mut c_void) -> HRESULT {
    E_NOTIMPL
}

unsafe extern "system" fn dummy_get_ids_of_names(_this: *mut c_void, _riid: *const GUID, _names: *mut *mut u8, _cnames: u32, _lcid: u32, _dispid: *mut i32) -> HRESULT {
    E_NOTIMPL
}

unsafe extern "system" fn dummy_invoke(_this: *mut c_void, _dispid: i32, _riid: *const GUID, _lcid: u32, _flags: u16, _params: *mut c_void, _result: *mut c_void, _excep: *mut c_void, _argerr: *mut u32) -> HRESULT {
    E_NOTIMPL
}

unsafe extern "system" fn navigate(
    this: *mut c_void,
    url: *const u16,
    flags: *mut c_void,
    target_frame: *mut c_void,
    post_data: *mut c_void,
    headers: *mut c_void,
) -> HRESULT {
    let _ = this;
    
    let url_str = if url.is_null() {
        "None".to_string()
    } else {
        unsafe {
            let len = *((url as *const u32).offset(-1)) / 2;
            let slice = std::slice::from_raw_parts(url, len as usize);
            String::from_utf16_lossy(slice)
        }
    };

    let flags_str = unsafe { variant_to_string(flags) };
    let target_frame_str = unsafe { variant_to_string(target_frame) };
    let post_data_str = unsafe { variant_to_string(post_data) };
    let headers_str = unsafe { variant_to_string(headers) };

    log::info!(
        "[IWebBrowser2::Navigate] URL: {}, Flags: {}, TargetFrame: {}, PostData: {}, Headers: {}",
        url_str, flags_str, target_frame_str, post_data_str, headers_str
    );
    
    S_OK
}

pub static IWEBBROWSER2_VTBL: IWebBrowser2Vtbl = IWebBrowser2Vtbl {
    QueryInterface: query_interface,
    AddRef: add_ref,
    Release: release,
    GetTypeInfoCount: dummy_get_type_info_count,
    GetTypeInfo: dummy_get_type_info,
    GetIDsOfNames: dummy_get_ids_of_names,
    Invoke: dummy_invoke,
    GoBack: dummy_method,
    GoForward: dummy_method,
    GoHome: dummy_method,
    GoSearch: dummy_method,
    Navigate: navigate,
};
