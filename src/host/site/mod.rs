use windows::Win32::Foundation::HWND;

use client::MyOleClientSite;
use frame::MyOleInPlaceFrame;
use inplace::MyOleInPlaceSite;

pub mod browser;
pub mod client;
pub mod events;
pub mod frame;
pub mod inplace;
pub mod navigate;
pub mod provider;

#[repr(C)]
pub struct SharedSiteState {
    pub ref_count: u32,
    pub hwnd: HWND,
    pub client_site: *mut MyOleClientSite,
    pub inplace_site: *mut MyOleInPlaceSite,
    pub frame: *mut MyOleInPlaceFrame,
    pub events: *mut events::MyChatFrameEvents,
    pub navigate: *mut navigate::MyOleNavigate,
    pub browser: *mut browser::MyWebBrowser,
    pub provider: *mut provider::MyServiceProvider,
    pub rect: windows::Win32::Foundation::RECT,
}

pub struct HostWrappers {
    pub client_site: *mut MyOleClientSite,
    pub _inplace_site: *mut MyOleInPlaceSite,
    pub _frame: *mut MyOleInPlaceFrame,
    pub events: *mut events::MyChatFrameEvents,
    pub navigate: *mut navigate::MyOleNavigate,
    pub _browser: *mut browser::MyWebBrowser,
    pub _provider: *mut provider::MyServiceProvider,
    pub _shared: *mut SharedSiteState,
}

pub fn create_host_wrappers(hwnd: HWND) -> HostWrappers {
    let shared = Box::into_raw(Box::new(SharedSiteState {
        ref_count: 1,
        hwnd,
        client_site: std::ptr::null_mut(),
        inplace_site: std::ptr::null_mut(),
        frame: std::ptr::null_mut(),
        events: std::ptr::null_mut(),
        navigate: std::ptr::null_mut(),
        browser: std::ptr::null_mut(),
        provider: std::ptr::null_mut(),
        rect: windows::Win32::Foundation::RECT::default(),
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

    let events = Box::into_raw(Box::new(events::MyChatFrameEvents {
        lp_vtbl: &events::ICCHATFRAMEEVENTS_VTBL,
        shared,
    }));

    let navigate = Box::into_raw(Box::new(navigate::MyOleNavigate {
        lp_vtbl: &navigate::IOLENAVIGATE_VTBL,
        shared,
    }));

    let browser = Box::into_raw(Box::new(browser::MyWebBrowser {
        lp_vtbl: &browser::IWEBBROWSER2_VTBL,
        shared,
    }));

    let provider = Box::into_raw(Box::new(provider::MyServiceProvider {
        lp_vtbl: &provider::ISERVICEPROVIDER_VTBL,
        shared,
    }));

    unsafe {
        (*shared).client_site = client_site;
        (*shared).inplace_site = inplace_site;
        (*shared).frame = frame;
        (*shared).events = events;
        (*shared).navigate = navigate;
        (*shared).browser = browser;
        (*shared).provider = provider;
    }

    HostWrappers {
        client_site,
        _inplace_site: inplace_site,
        _frame: frame,
        events,
        navigate,
        _browser: browser,
        _provider: provider,
        _shared: shared,
    }
}
