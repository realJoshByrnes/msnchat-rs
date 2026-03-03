use std::ffi::c_void;
use windows::core::BOOL;
use windows::{
    Win32::{
        Foundation::{E_NOINTERFACE, E_NOTIMPL, E_POINTER, S_OK},
        System::Ole::{IOleClientSite, IOleClientSite_Vtbl, IOleInPlaceFrame, IOleInPlaceSite},
    },
    core::{GUID, HRESULT, IUnknown_Vtbl, Interface},
};

use super::SharedSiteState;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MyOleClientSite {
    pub lp_vtbl: *const IOleClientSite_Vtbl,
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

        let this = this as *mut MyOleClientSite;
        let shared = (*this).shared;
        let riid = &*riid;

        if riid == &IOleClientSite::IID || riid == &windows::core::IUnknown::IID {
            *ppv = this as *mut _ as *mut c_void;
            add_ref(this as *mut c_void);
            S_OK
        } else if riid == &IOleInPlaceSite::IID {
            let inplace_site = (*shared).inplace_site;
            *ppv = inplace_site as *mut c_void;
            super::inplace::add_ref(inplace_site as *mut c_void);
            S_OK
        } else if riid == &IOleInPlaceFrame::IID {
            let frame = (*shared).frame;
            *ppv = frame as *mut c_void;
            super::frame::add_ref(frame as *mut c_void);
            S_OK
        } else {
            *ppv = std::ptr::null_mut();
            E_NOINTERFACE
        }
    }
}

/// # Safety
/// This is a mock implementation of IUnknown::AddRef and assumes `this` is valid.
pub unsafe extern "system" fn add_ref(this: *mut c_void) -> u32 {
    unsafe {
        let site = &mut *this.cast::<MyOleClientSite>();
        (*site.shared).ref_count += 1;
        (*site.shared).ref_count
    }
}

/// # Safety
/// This is a mock implementation of IUnknown::Release and assumes `this` is valid.
pub unsafe extern "system" fn release(this: *mut c_void) -> u32 {
    unsafe {
        let site = &mut *this.cast::<MyOleClientSite>();
        (*site.shared).ref_count -= 1;
        let count = (*site.shared).ref_count;
        if count == 0 {
            drop(Box::from_raw(this as *mut MyOleClientSite));
        }
        count
    }
}

unsafe extern "system" fn save_object(_this: *mut c_void) -> HRESULT {
    S_OK
}
unsafe extern "system" fn get_moniker(
    _this: *mut c_void,
    _dw_assign: u32,
    _dw_which_moniker: u32,
    _ppmk: *mut *mut c_void,
) -> HRESULT {
    E_NOTIMPL
}
unsafe extern "system" fn get_container(
    _this: *mut c_void,
    pp_container: *mut *mut c_void,
) -> HRESULT {
    unsafe {
        if !pp_container.is_null() {
            *pp_container = std::ptr::null_mut();
        }
        E_NOINTERFACE
    }
}
unsafe extern "system" fn show_object(_this: *mut c_void) -> HRESULT {
    S_OK
}
unsafe extern "system" fn on_show_window(_this: *mut c_void, _f_show: BOOL) -> HRESULT {
    S_OK
}
unsafe extern "system" fn request_new_object_layout(_this: *mut c_void) -> HRESULT {
    E_NOTIMPL
}

pub static IOLECLIENTSITE_VTBL: IOleClientSite_Vtbl = IOleClientSite_Vtbl {
    base__: IUnknown_Vtbl {
        QueryInterface: query_interface,
        AddRef: add_ref,
        Release: release,
    },
    SaveObject: save_object,
    GetMoniker: get_moniker,
    GetContainer: get_container,
    ShowObject: show_object,
    OnShowWindow: on_show_window,
    RequestNewObjectLayout: request_new_object_layout,
};
