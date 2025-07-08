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

use crate::com::shared::SharedSiteState;
use std::ffi::c_void;
use std::ptr;
use windows::Win32::Foundation::E_NOINTERFACE;
use windows::Win32::System::Com::IServiceProvider_Vtbl;
use windows::core::{GUID, HRESULT};

#[repr(C)]
pub struct MyServiceProvider {
    pub lp_vtbl: *const IServiceProvider_Vtbl,
    pub shared: *mut SharedSiteState,
}

// --- IUnknown methods ---
pub unsafe extern "system" fn query_interface(
    _this: *mut c_void,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    unsafe {
        println!(
            "*** IServiceProvider::QueryInterface called for {:?}",
            *riid
        );
        *ppv = ptr::null_mut();
        E_NOINTERFACE
    }
}

pub unsafe extern "system" fn add_ref(this: *mut c_void) -> u32 {
    unsafe {
        let site = &mut *(this as *mut MyServiceProvider);
        (*site.shared).ref_count += 1;
        (*site.shared).ref_count
    }
}

pub unsafe extern "system" fn release(this: *mut c_void) -> u32 {
    unsafe {
        let site = &mut *(this as *mut MyServiceProvider);
        (*site.shared).ref_count -= 1;
        let count = (*site.shared).ref_count;
        if count == 0 {
            drop(Box::from_raw(this as *mut MyServiceProvider));
        }
        count
    }
}

// --- IServiceProvider methods (stubs) ---
unsafe extern "system" fn query_service(
    _this: *mut c_void,
    guid_service: *const GUID,
    riid: *const GUID,
    ppv_object: *mut *mut c_void,
) -> HRESULT {
    println!("*** IServiceProvider::QueryService called");
    unsafe {
        if !guid_service.is_null() {
            println!(
                "  guidService: {{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
                (*guid_service).data1,
                (*guid_service).data2,
                (*guid_service).data3,
                (*guid_service).data4[0],
                (*guid_service).data4[1],
                (*guid_service).data4[2],
                (*guid_service).data4[3],
                (*guid_service).data4[4],
                (*guid_service).data4[5],
                (*guid_service).data4[6],
                (*guid_service).data4[7]
            );
        } else {
            println!("  guidService: NULL");
        }

        if !riid.is_null() {
            println!(
                "  riid:        {{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
                (*riid).data1,
                (*riid).data2,
                (*riid).data3,
                (*riid).data4[0],
                (*riid).data4[1],
                (*riid).data4[2],
                (*riid).data4[3],
                (*riid).data4[4],
                (*riid).data4[5],
                (*riid).data4[6],
                (*riid).data4[7]
            );
        } else {
            println!("  riid: NULL");
        }

        if !ppv_object.is_null() {
            *ppv_object = std::ptr::null_mut();
        }
    }
    E_NOINTERFACE
}

pub static ISERVICEPROVIDER_VTBL: IServiceProvider_Vtbl = IServiceProvider_Vtbl {
    base__: windows::core::IUnknown_Vtbl {
        QueryInterface: query_interface,
        AddRef: add_ref,
        Release: release,
    },
    QueryService: query_service,
};
