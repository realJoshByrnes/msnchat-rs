use std::ffi::c_void;
use windows::core::{GUID, HRESULT, Interface};
use windows::Win32::Foundation::{E_NOINTERFACE, E_POINTER, S_OK};

use super::SharedSiteState;

pub const IID_ISERVICEPROVIDER: GUID = GUID::from_values(
    0x6D5140C1,
    0x7436,
    0x11CE,
    [0x80, 0x34, 0x00, 0xAA, 0x00, 0x60, 0x09, 0xFA],
);

#[repr(C)]
pub struct IServiceProviderVtbl {
    pub QueryInterface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    pub AddRef: unsafe extern "system" fn(*mut c_void) -> u32,
    pub Release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub QueryService: unsafe extern "system" fn(
        *mut c_void,
        *const GUID,
        *const GUID,
        *mut *mut c_void,
    ) -> HRESULT,
}

#[repr(C)]
pub struct MyServiceProvider {
    pub lp_vtbl: *const IServiceProviderVtbl,
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

        let this = this as *mut MyServiceProvider;
        let riid = &*riid;

        if riid == &IID_ISERVICEPROVIDER || riid == &windows::core::IUnknown::IID {
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
        let site = &mut *this.cast::<MyServiceProvider>();
        (*site.shared).ref_count += 1;
        (*site.shared).ref_count
    }
}

pub unsafe extern "system" fn release(this: *mut c_void) -> u32 {
    unsafe {
        let site = &mut *this.cast::<MyServiceProvider>();
        (*site.shared).ref_count -= 1;
        let count = (*site.shared).ref_count;
        if count == 0 {
            drop(Box::from_raw(this as *mut MyServiceProvider));
        }
        count
    }
}

unsafe extern "system" fn query_service(
    this: *mut c_void,
    guid_service: *const GUID,
    riid: *const GUID,
    ppv_object: *mut *mut c_void,
) -> HRESULT {
    unsafe {
        if ppv_object.is_null() {
            return E_POINTER;
        }

        let this = this as *mut MyServiceProvider;
        let guid_service = &*guid_service;
        let riid = &*riid;
        let shared = (*this).shared;

        // SID_SWebBrowserApp is 0002DF05-0000-0000-C000-000000000046
        let sid_web_browser_app = GUID::from_values(
            0x0002DF05,
            0x0000,
            0x0000,
            [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
        );

        if guid_service == &sid_web_browser_app {
            let browser = (*shared).browser;
            // Delegate query interface to browser wrapper
            return ((*(*browser).lp_vtbl).QueryInterface)(browser as *mut c_void, riid, ppv_object);
        }

        *ppv_object = std::ptr::null_mut();
        E_NOINTERFACE
    }
}

pub static ISERVICEPROVIDER_VTBL: IServiceProviderVtbl = IServiceProviderVtbl {
    QueryInterface: query_interface,
    AddRef: add_ref,
    Release: release,
    QueryService: query_service,
};
