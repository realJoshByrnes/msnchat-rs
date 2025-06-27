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

use std::ops::Mul;
use std::os::raw::c_void;

// use windows::core::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoUninitialize, DISPATCH_METHOD, DISPATCH_PROPERTYGET, DISPATCH_PROPERTYPUT,
    DISPATCH_PROPERTYPUTREF, DISPPARAMS, EXCEPINFO, IDispatch,
};
use windows::Win32::System::LibraryLoader::{GetModuleHandleA, GetModuleHandleW};
use windows::Win32::System::Ole::{
    DISPID_UNKNOWN, IOleClientSite, IOleInPlaceFrame, IOleInPlaceObject, IOleInPlaceSite,
    IOleInPlaceSiteEx, IOleInPlaceUIWindow, IOleWindow, OLEINPLACEFRAMEINFO, OLEIVERB_SHOW,
};
use windows::Win32::System::Variant::{VARIANT, VT_BSTR, VariantInit};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::{self, Foundation::*, System};
use windows::core::*;

use windows::Win32::System::Ole::IOleClientSite_Vtbl;

use crate::com::dispatch::MyDispatch;
use crate::com::shared::SharedSiteState;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MyOleClientSite {
    pub lpVtbl: *const IOleClientSite_Vtbl,
    pub shared: *mut SharedSiteState,
}

unsafe extern "system" fn QueryInterface(
    this: *mut c_void,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    println!("client_site::QueryInterface called for {:?}", unsafe {
        *riid
    });
    if ppv.is_null() {
        return E_POINTER;
    }
    let this = this as *mut MyOleClientSite;
    let shared = unsafe { (*this).shared };
    let riid = &*riid;
    if riid == &IOleClientSite::IID || riid == &windows::core::IUnknown::IID {
        *ppv = this as *mut _ as *mut c_void;
        AddRef(this as *mut c_void);
        return S_OK;
    } else if riid == &IDispatch::IID {
        let dispatch = unsafe { (*shared).dispatch };
        *ppv = dispatch as *mut c_void;
        crate::com::dispatch::add_ref(dispatch as *mut c_void);
        return S_OK;
    } else if riid == &IOleInPlaceSite::IID {
        let inplace_site = unsafe { (*shared).inplace_site };
        *ppv = inplace_site as *mut c_void;
        crate::com::inplace_site::add_ref(inplace_site as *mut c_void);
        return S_OK;
    } else if riid == &IOleInPlaceSiteEx::IID {
        let inplace_site_ex = unsafe { (*shared).inplace_site_ex };
        *ppv = inplace_site_ex as *mut c_void;
        crate::com::inplace_site_ex::add_ref(inplace_site_ex as *mut c_void);
        return S_OK;
    }
    *ppv = std::ptr::null_mut();
    E_NOINTERFACE
}

pub unsafe extern "system" fn AddRef(this: *mut c_void) -> u32 {
    let site = &mut unsafe { *this.cast::<MyOleClientSite>() };
    (*site.shared).ref_count += 1;
    (*site.shared).ref_count
}

pub unsafe extern "system" fn Release(this: *mut c_void) -> u32 {
    let site = &mut unsafe { *this.cast::<MyOleClientSite>() };
    (*site.shared).ref_count -= 1;
    let count = (*site.shared).ref_count;
    if count == 0 {
        drop(Box::from_raw(this));
    }
    count
}

unsafe extern "system" fn SaveObject(_this: *mut c_void) -> HRESULT {
    println!("IOleClientSite::SaveObject called");
    S_OK
}
unsafe extern "system" fn GetMoniker(
    _this: *mut c_void,
    _dwAssign: u32,
    _dwWhichMoniker: u32,
    _ppmk: *mut *mut c_void,
) -> HRESULT {
    println!("IOleClientSite::GetMoniker called");
    E_NOTIMPL
}
unsafe extern "system" fn GetContainer(
    _this: *mut c_void,
    ppContainer: *mut *mut c_void,
) -> HRESULT {
    println!("IOleClientSite::GetContainer called");
    if !ppContainer.is_null() {
        *ppContainer = std::ptr::null_mut();
    }
    E_NOINTERFACE
}
unsafe extern "system" fn ShowObject(_this: *mut c_void) -> HRESULT {
    println!("IOleClientSite::ShowObject called");
    S_OK
}
unsafe extern "system" fn OnShowWindow(_this: *mut c_void, _fShow: BOOL) -> HRESULT {
    println!("IOleClientSite::OnShowWindow called");
    // No need to do anything here.
    S_OK
}
unsafe extern "system" fn RequestNewObjectLayout(_this: *mut c_void) -> HRESULT {
    println!("IOleClientSite::RequestNewObjectLayout called");
    E_NOTIMPL
}

// Define the vtable (must be static)
pub static IOLECLIENTSITE_VTBL: IOleClientSite_Vtbl = IOleClientSite_Vtbl {
    base__: windows::core::IUnknown_Vtbl {
        QueryInterface: QueryInterface,
        AddRef: AddRef,
        Release: Release,
    },
    SaveObject: SaveObject,
    GetMoniker: GetMoniker,
    GetContainer: GetContainer,
    ShowObject: ShowObject,
    OnShowWindow: OnShowWindow,
    RequestNewObjectLayout: RequestNewObjectLayout,
};
