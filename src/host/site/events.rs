use std::ffi::c_void;
use windows::Win32::Foundation::{E_NOINTERFACE, E_POINTER, S_OK};
use windows::core::{GUID, HRESULT, Interface};

use super::SharedSiteState;

pub const IID_ICCHATFRAMEEVENTS: GUID = GUID::from_values(
    0x5EEB8014,
    0x53B2,
    0x448B,
    [0x9F, 0x3B, 0xC5, 0x53, 0x42, 0x48, 0x32, 0xE1],
);

#[repr(C)]
pub struct ICChatFrameEventsVtbl {
    pub QueryInterface:
        unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    pub AddRef: unsafe extern "system" fn(*mut c_void) -> u32,
    pub Release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub OnRedirect: unsafe extern "system" fn(
        *mut c_void,
        std::mem::ManuallyDrop<windows::core::BSTR>,
    ) -> HRESULT,
}

#[repr(C)]
pub struct MyChatFrameEvents {
    pub lp_vtbl: *const ICChatFrameEventsVtbl,
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

        let this = this as *mut MyChatFrameEvents;
        let riid = &*riid;

        if riid == &IID_ICCHATFRAMEEVENTS || riid == &windows::core::IUnknown::IID {
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
        let site = &mut *this.cast::<MyChatFrameEvents>();
        (*site.shared).ref_count += 1;
        (*site.shared).ref_count
    }
}

pub unsafe extern "system" fn release(this: *mut c_void) -> u32 {
    unsafe {
        let site = &mut *this.cast::<MyChatFrameEvents>();
        (*site.shared).ref_count -= 1;
        let count = (*site.shared).ref_count;
        if count == 0 {
            drop(Box::from_raw(this as *mut MyChatFrameEvents));
        }
        count
    }
}

unsafe extern "system" fn on_redirect(
    this: *mut c_void,
    str_url: std::mem::ManuallyDrop<windows::core::BSTR>,
) -> HRESULT {
    let _ = this;
    log::info!(
        "[_ICChatFrameEvents] OnRedirect triggered! Redirecting to: {}",
        *str_url
    );
    S_OK
}

pub static ICCHATFRAMEEVENTS_VTBL: ICChatFrameEventsVtbl = ICChatFrameEventsVtbl {
    QueryInterface: query_interface,
    AddRef: add_ref,
    Release: release,
    OnRedirect: on_redirect,
};
