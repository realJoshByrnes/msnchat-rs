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
use windows::Win32::System::Com::{
    DISPATCH_FLAGS, DISPPARAMS, EXCEPINFO, IDispatch, IDispatch_Vtbl,
};
use windows::Win32::System::Ole::{IOleClientSite, IOleInPlaceSite, IOleInPlaceSiteEx};
use windows::Win32::System::Variant::VARIANT;
use windows::core::*;

use crate::com::shared::SharedSiteState;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MyDispatch {
    pub lp_vtbl: *const IDispatch_Vtbl,
    pub shared: *mut SharedSiteState,
}

unsafe extern "system" fn query_interface(
    this: *mut c_void,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    unsafe {
        println!("IDispatch::QueryInterface called for {:?}", riid);
        if ppv.is_null() {
            return E_POINTER;
        }
        let this = this as *mut MyDispatch;
        let shared = (*this).shared;
        let riid = &*riid;
        if riid == &IDispatch::IID || riid == &windows::core::IUnknown::IID {
            *ppv = this as *mut _ as *mut c_void;
            add_ref(this as *mut c_void);
            return S_OK;
        } else if riid == &IOleClientSite::IID {
            let client_site = (*shared).dispatch;
            *ppv = client_site as *mut c_void;
            crate::com::client_site::add_ref(client_site as *mut c_void);
            return S_OK;
        } else if riid == &IOleInPlaceSite::IID {
            let inplace_site = (*shared).inplace_site;
            *ppv = inplace_site as *mut c_void;
            crate::com::inplace_site::add_ref(inplace_site as *mut c_void);
            return S_OK;
        } else if riid == &IOleInPlaceSiteEx::IID {
            let inplace_site_ex = (*shared).inplace_site_ex;
            *ppv = inplace_site_ex as *mut c_void;
            crate::com::inplace_site_ex::add_ref(inplace_site_ex as *mut c_void);
            return S_OK;
        }
        *ppv = std::ptr::null_mut();
        E_NOINTERFACE
    }
}

pub unsafe extern "system" fn add_ref(this: *mut c_void) -> u32 {
    unsafe {
        let dispatch = &mut *(this as *mut MyDispatch);
        (*dispatch.shared).ref_count += 1;
        (*dispatch.shared).ref_count
    }
}

pub unsafe extern "system" fn release(this: *mut c_void) -> u32 {
    unsafe {
        let dispatch = &mut *(this as *mut MyDispatch);
        (*dispatch.shared).ref_count -= 1;
        let count = (*dispatch.shared).ref_count;
        if count == 0 {
            drop(Box::from_raw(this as *mut MyDispatch));
        }
        count
    }
}

unsafe extern "system" fn get_type_info_count(_this: *mut c_void, pctinfo: *mut u32) -> HRESULT {
    println!("IDispatch::GetTypeInfoCount called");
    if !pctinfo.is_null() {
        unsafe {
            *pctinfo = 0;
        }
    }
    S_OK
}
unsafe extern "system" fn get_type_info(
    _this: *mut c_void,
    _i_tinfo: u32,
    _lcid: u32,
    _pp_tinfo: *mut *mut c_void,
) -> HRESULT {
    println!("IDispatch::GetTypeInfo called for iTInfo: {}", _i_tinfo);
    E_NOTIMPL
}
unsafe extern "system" fn get_ids_of_names(
    _this: *mut c_void,
    _riid: *const GUID,
    _rgsz_names: *const PCWSTR,
    _c_names: u32,
    _lcid: u32,
    _rg_disp_id: *mut i32,
) -> HRESULT {
    println!("IDispatch::GetIDsOfNames called for names: {:?}", unsafe {
        std::slice::from_raw_parts(_rgsz_names, _c_names as usize)
    });
    E_NOTIMPL
}
unsafe extern "system" fn invoke(
    _this: *mut c_void,
    _disp_id_member: i32,
    _riid: *const GUID,
    _lcid: u32,
    _w_flags: DISPATCH_FLAGS,
    _p_disp_params: *const DISPPARAMS,
    _p_var_result: *mut VARIANT,
    _p_excep_info: *mut EXCEPINFO,
    _pu_arg_err: *mut u32,
) -> HRESULT {
    println!(
        // Show all parameters for debugging
        "IDispatch::Invoke called with dispIdMember: {}, riid: {:?}, lcid: {}, wFlags: {:?}",
        _disp_id_member,
        unsafe { *_riid },
        _lcid,
        _w_flags,
    );
    E_NOTIMPL
}

pub static IDISPATCH_VTBL: IDispatch_Vtbl = IDispatch_Vtbl {
    base__: IUnknown_Vtbl {
        QueryInterface: query_interface,
        AddRef: add_ref,
        Release: release,
    },
    GetTypeInfoCount: get_type_info_count,
    GetTypeInfo: get_type_info,
    GetIDsOfNames: get_ids_of_names,
    Invoke: invoke,
};
