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
use windows::Win32::Foundation::{E_NOINTERFACE, E_NOTIMPL, POINTL, S_OK};
use windows::Win32::System::Ole::{IOleControlSite_Vtbl, KEYMODIFIERS, POINTF};
use windows::Win32::UI::WindowsAndMessaging;
use windows::core::{BOOL, GUID, HRESULT};

#[repr(C)]
pub struct MyOleControlSite {
    pub lp_vtbl: *const IOleControlSite_Vtbl,
    pub shared: *mut SharedSiteState,
}

// --- IUnknown methods ---
pub unsafe extern "system" fn query_interface(
    _this: *mut c_void,
    riid: *const GUID,
    _ppv: *mut *mut c_void,
) -> HRESULT {
    println!("control_site::QueryInterface called for {:?}", unsafe {
        *riid
    });
    E_NOINTERFACE
}

pub unsafe extern "system" fn add_ref(this: *mut c_void) -> u32 {
    unsafe {
        let site = &mut *(this as *mut MyOleControlSite);
        (*site.shared).ref_count += 1;
        (*site.shared).ref_count
    }
}

pub unsafe extern "system" fn release(this: *mut c_void) -> u32 {
    unsafe {
        let site = &mut *(this as *mut MyOleControlSite);
        (*site.shared).ref_count -= 1;
        let count = (*site.shared).ref_count;
        if count == 0 {
            drop(Box::from_raw(this as *mut MyOleControlSite));
        }
        count
    }
}

// --- IOleControlSite methods (stubs) ---
unsafe extern "system" fn on_control_info_changed(_this: *mut c_void) -> HRESULT {
    println!("IOleControlSite::OnControlInfoChanged called");
    S_OK
}

unsafe extern "system" fn lock_in_place_active(_this: *mut c_void, _f_lock: BOOL) -> HRESULT {
    println!("IOleControlSite::LockInPlaceActive called");
    S_OK
}

unsafe extern "system" fn get_extended_control(
    _this: *mut c_void,
    _pp_disp: *mut *mut c_void,
) -> HRESULT {
    println!("IOleControlSite::GetExtendedControl called");
    E_NOTIMPL
}

unsafe extern "system" fn transform_coords(
    _this: *mut c_void,
    _p_point: *mut POINTL,
    _p_pixel: *mut POINTF,
    _dw_flags: u32,
) -> HRESULT {
    println!("IOleControlSite::TransformCoords called");
    S_OK
}

unsafe extern "system" fn translate_accelerator(
    _this: *mut c_void,
    _p_msg: *const WindowsAndMessaging::MSG,
    _grf_modifiers: KEYMODIFIERS,
) -> HRESULT {
    println!("IOleControlSite::TranslateAccelerator called");
    S_OK
}

unsafe extern "system" fn on_focus(_this: *mut c_void, _f_focus: BOOL) -> HRESULT {
    println!("IOleControlSite::OnFocus called");
    S_OK
}

unsafe extern "system" fn show_property_frame(_this: *mut c_void) -> HRESULT {
    println!("IOleControlSite::ShowPropertyFrame called");
    S_OK
}

pub static IOLECONTROLSITE_VTBL: IOleControlSite_Vtbl = IOleControlSite_Vtbl {
    base__: windows::core::IUnknown_Vtbl {
        QueryInterface: query_interface,
        AddRef: add_ref,
        Release: release,
    },
    OnControlInfoChanged: on_control_info_changed,
    LockInPlaceActive: lock_in_place_active,
    GetExtendedControl: get_extended_control,
    TransformCoords: transform_coords,
    TranslateAccelerator: translate_accelerator,
    OnFocus: on_focus,
    ShowPropertyFrame: show_property_frame,
};
