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
    CoUninitialize, DISPATCH_FLAGS, DISPATCH_METHOD, DISPATCH_PROPERTYGET, DISPATCH_PROPERTYPUT,
    DISPATCH_PROPERTYPUTREF, DISPPARAMS, EXCEPINFO, IDispatch, IDispatch_Vtbl,
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

use crate::com::client_site::MyOleClientSite;
use crate::com::shared::SharedSiteState;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MyDispatch {
    pub lpVtbl: *const IDispatch_Vtbl,
    pub shared: *mut SharedSiteState,
}

unsafe extern "system" fn query_interface(
    this: *mut c_void,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    println!("IDispatch::QueryInterface called for {:?}", unsafe {
        *riid
    });
    if ppv.is_null() {
        return E_POINTER;
    }
    let this = this as *mut MyDispatch;
    let shared = unsafe { (*this).shared };
    let riid = &*riid;
    if riid == &IDispatch::IID || riid == &windows::core::IUnknown::IID {
        *ppv = this as *mut _ as *mut c_void;
        add_ref(this as *mut c_void);
        return S_OK;
    } else if riid == &IOleClientSite::IID {
        let client_site = unsafe { (*shared).dispatch };
        *ppv = client_site as *mut c_void;
        crate::com::client_site::AddRef(client_site as *mut c_void);
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

pub unsafe extern "system" fn add_ref(this: *mut c_void) -> u32 {
    let dispatch = &mut *(this as *mut MyDispatch);
    (*dispatch.shared).ref_count += 1;
    (*dispatch.shared).ref_count
}

pub unsafe extern "system" fn release(this: *mut c_void) -> u32 {
    let dispatch = &mut *(this as *mut MyDispatch);
    (*dispatch.shared).ref_count -= 1;
    let count = (*dispatch.shared).ref_count;
    if count == 0 {
        drop(Box::from_raw(this as *mut MyDispatch));
    }
    count
}

unsafe extern "system" fn GetTypeInfoCount(_this: *mut c_void, pctinfo: *mut u32) -> HRESULT {
    println!("IDispatch::GetTypeInfoCount called");
    if !pctinfo.is_null() {
        unsafe {
            *pctinfo = 0;
        }
    }
    S_OK
}
unsafe extern "system" fn GetTypeInfo(
    _this: *mut c_void,
    _iTInfo: u32,
    _lcid: u32,
    _ppTInfo: *mut *mut c_void,
) -> HRESULT {
    println!("IDispatch::GetTypeInfo called for iTInfo: {}", _iTInfo);
    E_NOTIMPL
}
unsafe extern "system" fn GetIDsOfNames(
    _this: *mut c_void,
    _riid: *const GUID,
    _rgszNames: *const PCWSTR,
    _cNames: u32,
    _lcid: u32,
    _rgDispId: *mut i32,
) -> HRESULT {
    println!("IDispatch::GetIDsOfNames called for names: {:?}", unsafe {
        std::slice::from_raw_parts(_rgszNames, _cNames as usize)
    });
    E_NOTIMPL
}
unsafe extern "system" fn Invoke(
    _this: *mut c_void,
    _dispIdMember: i32,
    _riid: *const GUID,
    _lcid: u32,
    _wFlags: DISPATCH_FLAGS,
    _pDispParams: *const DISPPARAMS,
    _pVarResult: *mut VARIANT,
    _pExcepInfo: *mut EXCEPINFO,
    _puArgErr: *mut u32,
) -> HRESULT {
    println!(
        // Show all parameters for debugging
        "IDispatch::Invoke called with dispIdMember: {}, riid: {:?}, lcid: {}, wFlags: {:?}",
        _dispIdMember,
        unsafe { *_riid },
        _lcid,
        _wFlags,
    );
    E_NOTIMPL
}

pub static IDISPATCH_VTBL: IDispatch_Vtbl = IDispatch_Vtbl {
    base__: IUnknown_Vtbl {
        QueryInterface: query_interface,
        AddRef: add_ref,
        Release: release,
    },
    GetTypeInfoCount: GetTypeInfoCount,
    GetTypeInfo: GetTypeInfo,
    GetIDsOfNames: GetIDsOfNames,
    Invoke: Invoke,
};
