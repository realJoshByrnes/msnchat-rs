use std::ffi::c_void;
use windows::core::{GUID, HRESULT, Interface};
use windows::Win32::Foundation::{E_NOINTERFACE, E_POINTER, S_OK};

use super::SharedSiteState;

pub const IID_IOLENAVIGATE: GUID = GUID::from_values(
    0x3E11EE5C,
    0x9666,
    0x11D0,
    [0x95, 0x18, 0x00, 0xC0, 0x4F, 0xD9, 0x15, 0x2D],
);

#[repr(C)]
pub struct IOleNavigateVtbl {
    pub QueryInterface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    pub AddRef: unsafe extern "system" fn(*mut c_void) -> u32,
    pub Release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub OnNavigate: unsafe extern "system" fn(
        *mut c_void,
        *const std::ffi::c_char,
        u32,
        *const std::ffi::c_char,
        u32,
        u32,
        u32,
        u32,
        u32,
    ) -> HRESULT,
}

#[repr(C)]
pub struct MyOleNavigate {
    pub lp_vtbl: *const IOleNavigateVtbl,
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

        let this = this as *mut MyOleNavigate;
        let riid = &*riid;

        if riid == &IID_IOLENAVIGATE || riid == &windows::core::IUnknown::IID {
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
        let site = &mut *this.cast::<MyOleNavigate>();
        (*site.shared).ref_count += 1;
        (*site.shared).ref_count
    }
}

pub unsafe extern "system" fn release(this: *mut c_void) -> u32 {
    unsafe {
        let site = &mut *this.cast::<MyOleNavigate>();
        (*site.shared).ref_count -= 1;
        let count = (*site.shared).ref_count;
        if count == 0 {
            drop(Box::from_raw(this as *mut MyOleNavigate));
        }
        count
    }
}

unsafe extern "system" fn on_navigate(
    this: *mut c_void,
    url: *const std::ffi::c_char,
    flags: u32,
    target_frame: *const std::ffi::c_char,
    arg5: u32,
    arg6: u32,
    arg7: u32,
    arg8: u32,
    arg9: u32,
) -> HRESULT {
    let _ = this;
    let url_str = if !url.is_null() {
        unsafe { std::ffi::CStr::from_ptr(url).to_string_lossy().into_owned() }
    } else {
        "None".to_string()
    };
    
    let target_frame_str = if !target_frame.is_null() {
        unsafe { std::ffi::CStr::from_ptr(target_frame).to_string_lossy().into_owned() }
    } else {
        "None".to_string()
    };

    log::info!(
        "[IOleNavigate::OnNavigate] URL: {}, Flags: {}, Target Frame: {}, args: [{}, {}, {}, {}, {}]",
        url_str, flags, target_frame_str, arg5, arg6, arg7, arg8, arg9
    );
    S_OK
}

pub static IOLENAVIGATE_VTBL: IOleNavigateVtbl = IOleNavigateVtbl {
    QueryInterface: query_interface,
    AddRef: add_ref,
    Release: release,
    OnNavigate: on_navigate,
};
