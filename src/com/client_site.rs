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

use std::os::raw::c_void;

// use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::IDispatch;
use windows::Win32::System::Ole::{
    IOleClientSite, IOleControlSite, IOleInPlaceSite, IOleInPlaceSiteEx,
};
use windows::core::*;

use windows::Win32::System::Ole::IOleClientSite_Vtbl;

use crate::com::shared::SharedSiteState;

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

        match riid {
            &IOleClientSite::IID | &windows::core::IUnknown::IID => {
                println!(
                    "client_site::QueryInterface: {:?} (IOleClientSite / IUknown)",
                    riid
                );
                *ppv = this as *mut _ as *mut c_void;
                add_ref(this as *mut c_void);
                S_OK
            }
            &IOleControlSite::IID => {
                println!("client_site::QueryInterface: {:?} (IOleControlSite)", riid);
                let control_site = (*shared).control_site;
                *ppv = control_site as *mut c_void;
                crate::com::control_site::add_ref(control_site as *mut c_void);
                S_OK
            }
            &IDispatch::IID => {
                println!("client_site::QueryInterface: {:?} (IDispatch)", riid);
                let dispatch = (*shared).dispatch;
                *ppv = dispatch as *mut c_void;
                crate::com::dispatch::add_ref(dispatch as *mut c_void);
                S_OK
            }
            &IOleInPlaceSiteEx::IID => {
                println!(
                    "client_site::QueryInterface: {:?} (IOleInPlaceSiteEx)",
                    riid
                );
                let inplace_site_ex = (*shared).inplace_site_ex;
                *ppv = inplace_site_ex as *mut c_void;
                crate::com::inplace_site_ex::add_ref(inplace_site_ex as *mut c_void);
                S_OK
            }
            &IOleInPlaceSite::IID => {
                println!("client_site::QueryInterface: {:?} (IOleInPlaceSite)", riid);
                let inplace_site = (*shared).inplace_site;
                *ppv = inplace_site as *mut c_void;
                crate::com::inplace_site::add_ref(inplace_site as *mut c_void);
                S_OK
            }
            // // TODO: This is not working yet.
            // &IServiceProvider::IID => {
            //     println!("client_site::QueryInterface: {:?} (IServiceProvider)", riid);
            //     let service_provider = (*shared).service_provider;
            //     *ppv = service_provider as *mut c_void;
            //     crate::com::service_provider::add_ref(service_provider as *mut c_void);
            //     S_OK
            // }
            _ => {
                println!("client_site::QueryInterface: {:?}", riid);
                *ppv = std::ptr::null_mut();
                E_NOINTERFACE
            }
        }
    }
}

pub unsafe extern "system" fn add_ref(this: *mut c_void) -> u32 {
    unsafe {
        let site = &mut *this.cast::<MyOleClientSite>();
        (*site.shared).ref_count += 1;
        (*site.shared).ref_count
    }
}

pub unsafe extern "system" fn release(this: *mut c_void) -> u32 {
    unsafe {
        let site = &mut *this.cast::<MyOleClientSite>();
        (*site.shared).ref_count -= 1;
        let count = (*site.shared).ref_count;
        if count == 0 {
            drop(Box::from_raw(this));
        }
        count
    }
}

unsafe extern "system" fn save_object(_this: *mut c_void) -> HRESULT {
    println!("IOleClientSite::SaveObject called");
    S_OK
}
unsafe extern "system" fn get_moniker(
    _this: *mut c_void,
    _dw_assign: u32,
    _dw_which_moniker: u32,
    _ppmk: *mut *mut c_void,
) -> HRESULT {
    println!("IOleClientSite::GetMoniker called");
    E_NOTIMPL
}
unsafe extern "system" fn get_container(
    _this: *mut c_void,
    pp_container: *mut *mut c_void,
) -> HRESULT {
    unsafe {
        println!("IOleClientSite::GetContainer called");
        if !pp_container.is_null() {
            *pp_container = std::ptr::null_mut();
        }
        E_NOINTERFACE
    }
}
unsafe extern "system" fn show_object(_this: *mut c_void) -> HRESULT {
    println!("IOleClientSite::ShowObject called");
    S_OK
}
unsafe extern "system" fn on_show_window(_this: *mut c_void, _f_show: BOOL) -> HRESULT {
    println!("IOleClientSite::OnShowWindow called");
    // No need to do anything here.
    S_OK
}
unsafe extern "system" fn request_new_object_layout(_this: *mut c_void) -> HRESULT {
    println!("IOleClientSite::RequestNewObjectLayout called");
    E_NOTIMPL
}

// Define the vtable (must be static)
pub static IOLECLIENTSITE_VTBL: IOleClientSite_Vtbl = IOleClientSite_Vtbl {
    base__: windows::core::IUnknown_Vtbl {
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
