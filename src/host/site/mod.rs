use windows::Win32::Foundation::HWND;

use client::MyOleClientSite;
use frame::MyOleInPlaceFrame;
use inplace::MyOleInPlaceSite;

pub mod client;
pub mod frame;
pub mod inplace;

#[repr(C)]
pub struct SharedSiteState {
    pub ref_count: u32,
    pub hwnd: HWND,
    pub client_site: *mut MyOleClientSite,
    pub inplace_site: *mut MyOleInPlaceSite,
    pub frame: *mut MyOleInPlaceFrame,
}

pub struct HostWrappers {
    pub client_site: *mut MyOleClientSite,
    pub _inplace_site: *mut MyOleInPlaceSite,
    pub _frame: *mut MyOleInPlaceFrame,
    pub _shared: *mut SharedSiteState,
}

pub fn create_host_wrappers(hwnd: HWND) -> HostWrappers {
    let shared = Box::into_raw(Box::new(SharedSiteState {
        ref_count: 1,
        hwnd,
        client_site: std::ptr::null_mut(),
        inplace_site: std::ptr::null_mut(),
        frame: std::ptr::null_mut(),
    }));

    let client_site = Box::into_raw(Box::new(MyOleClientSite {
        lp_vtbl: &client::IOLECLIENTSITE_VTBL,
        shared,
    }));

    let inplace_site = Box::into_raw(Box::new(MyOleInPlaceSite {
        lp_vtbl: &inplace::IOLEINPLACESITE_VTBL,
        shared,
    }));

    let frame = Box::into_raw(Box::new(MyOleInPlaceFrame {
        lp_vtbl: &frame::IOLEINPLACEFRAME_VTBL,
        shared,
    }));

    unsafe {
        (*shared).client_site = client_site;
        (*shared).inplace_site = inplace_site;
        (*shared).frame = frame;
    }

    HostWrappers {
        client_site,
        _inplace_site: inplace_site,
        _frame: frame,
        _shared: shared,
    }
}
