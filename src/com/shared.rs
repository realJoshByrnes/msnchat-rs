// msnchat-rs
// Copyright (C) 2025 Joshua Byrnes
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use windows::Win32::Foundation::HWND;

use crate::com::{
    client_site::MyOleClientSite, control_site::MyOleControlSite, dispatch::MyDispatch, inplace_site::MyOleInPlaceSite, inplace_site_ex::MyOleInPlaceSiteEx
};

#[repr(C)]
pub struct SharedSiteState {
    pub ref_count: u32,
    pub hwnd: HWND,
    pub client_site: *mut MyOleClientSite,
    pub dispatch: *mut MyDispatch,
    pub inplace_site: *mut MyOleInPlaceSite,
    pub inplace_site_ex: *mut MyOleInPlaceSiteEx,
    pub control_site: *mut MyOleControlSite,
}

pub struct HostWrappers {
    pub client_site: *mut MyOleClientSite,
    pub dispatch: *mut MyDispatch,
    pub inplace_site: *mut MyOleInPlaceSite,
    pub inplace_site_ex: *mut MyOleInPlaceSiteEx,
    pub control_site: *mut MyOleControlSite,
    pub shared: *mut SharedSiteState,
}

pub fn create_host_wrappers(hwnd: HWND) -> HostWrappers {
    let shared = Box::into_raw(Box::new(SharedSiteState {
        ref_count: 1,
        hwnd,
        client_site: std::ptr::null_mut(),
        dispatch: std::ptr::null_mut(),
        inplace_site: std::ptr::null_mut(),
        inplace_site_ex: std::ptr::null_mut(),
        control_site: std::ptr::null_mut(),
    }));
    let client_site = Box::into_raw(Box::new(MyOleClientSite {
        lpVtbl: &crate::com::client_site::IOLECLIENTSITE_VTBL,
        shared,
    }));
    let dispatch = Box::into_raw(Box::new(MyDispatch {
        lpVtbl: &crate::com::dispatch::IDISPATCH_VTBL,
        shared,
    }));
    let inplace_site = Box::into_raw(Box::new(MyOleInPlaceSite {
        lpVtbl: &crate::com::inplace_site::IOLEINPLACESITE_VTBL,
        shared,
    }));
    let inplace_site_ex = Box::into_raw(Box::new(MyOleInPlaceSiteEx {
        lpVtbl: &crate::com::inplace_site_ex::IOLEINPLACESITEEX_VTBL,
        shared,
    }));
    let control_site = Box::into_raw(Box::new(MyOleControlSite {
        lpVtbl: &crate::com::control_site::IOLECONTROLSITE_VTBL,
        shared,
    }));
    unsafe {
        (*shared).client_site = client_site;
        (*shared).dispatch = dispatch;
        (*shared).inplace_site = inplace_site;
        (*shared).inplace_site_ex = inplace_site_ex;
        (*shared).control_site = control_site;
    }
    HostWrappers {
        client_site,
        dispatch,
        inplace_site,
        inplace_site_ex,
        control_site,
        shared,
    }
}
