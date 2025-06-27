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

use crate::IOleInPlaceSiteEx;
use crate::SIZE;
use crate::com::shared::SharedSiteState;
use std::ffi::c_void;
use std::ptr;
use windows::Win32::Foundation::{E_NOINTERFACE, E_NOTIMPL, E_POINTER, HWND, RECT, S_OK};
use windows::Win32::System::Ole::{
    IOleInPlaceSite, IOleInPlaceSite_Vtbl, IOleWindow_Vtbl, OLEINPLACEFRAMEINFO,
};
use windows::core::{BOOL, GUID, HRESULT};
use windows_core::Interface;
#[repr(C)]
pub struct MyOleInPlaceSite {
    pub lp_vtbl: *const IOleInPlaceSite_Vtbl,
    pub shared: *mut SharedSiteState,
}

// --- IUnknown methods ---
pub unsafe extern "system" fn query_interface(
    this: *mut c_void,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    unsafe {
        println!("IOleInPlaceSite::QueryInterface called for {:?}", *riid);
        if ppv.is_null() {
            return E_POINTER;
        }
        let this = this as *mut MyOleInPlaceSite;
        let shared = (*this).shared;
        let riid = &*riid;
        if riid == &IOleInPlaceSite::IID || riid == &windows::core::IUnknown::IID {
            *ppv = this as *mut _ as *mut c_void;
            add_ref(this as *mut c_void);
            return S_OK;
        } else if riid == &windows::Win32::System::Com::IDispatch::IID {
            let dispatch = (*shared).dispatch;
            *ppv = dispatch as *mut c_void;
            crate::com::dispatch::add_ref(dispatch as *mut c_void);
            return S_OK;
        } else if riid == &windows::Win32::System::Ole::IOleClientSite::IID {
            let client_site = (*shared).client_site;
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
        *ppv = ptr::null_mut();
        E_NOINTERFACE
    }
}

pub unsafe extern "system" fn add_ref(this: *mut c_void) -> u32 {
    unsafe {
        let site = &mut *(this as *mut MyOleInPlaceSite);
        (*site.shared).ref_count += 1;
        (*site.shared).ref_count
    }
}

pub unsafe extern "system" fn release(this: *mut c_void) -> u32 {
    unsafe {
        let site = &mut *(this as *mut MyOleInPlaceSite);
        (*site.shared).ref_count -= 1;
        let count = (*site.shared).ref_count;
        if count == 0 {
            drop(Box::from_raw(this as *mut MyOleInPlaceSite));
        }
        count
    }
}

// --- IOleInPlaceSite methods (stubs) ---
unsafe extern "system" fn get_window(_this: *mut c_void, phwnd: *mut HWND) -> HRESULT {
    unsafe {
        println!("IOleInPlaceSite::GetWindow called");
        if !phwnd.is_null() {
            *phwnd = HWND(std::ptr::null_mut());
        }
        S_OK
    }
}
unsafe extern "system" fn context_sensitive_help(
    _this: *mut c_void,
    _f_enter_mode: BOOL,
) -> HRESULT {
    println!("IOleInPlaceSite::ContextSensitiveHelp called");
    E_NOTIMPL
}
unsafe extern "system" fn can_in_place_activate(_this: *mut c_void) -> HRESULT {
    println!("IOleInPlaceSite::CanInPlaceActivate called");
    S_OK
}
unsafe extern "system" fn on_in_place_activate(_this: *mut c_void) -> HRESULT {
    println!("IOleInPlaceSite::OnInPlaceActivate called");
    S_OK
}
unsafe extern "system" fn on_ui_activate(_this: *mut c_void) -> HRESULT {
    println!("IOleInPlaceSite::OnUIActivate called");
    S_OK
}
unsafe extern "system" fn get_window_context(
    _this: *mut c_void,
    pp_frame: *mut *mut c_void,
    pp_doc: *mut *mut c_void,
    lprc_pos_rect: *mut RECT,
    lprc_clip_rect: *mut RECT,
    lp_frame_info: *mut OLEINPLACEFRAMEINFO,
) -> HRESULT {
    unsafe {
        println!("IOleInPlaceSite::GetWindowContext called");
        if !pp_frame.is_null() {
            *pp_frame = ptr::null_mut();
        }
        if !pp_doc.is_null() {
            *pp_doc = ptr::null_mut();
        }
        if !lprc_pos_rect.is_null() {
            *lprc_pos_rect = RECT::default();
        }
        if !lprc_clip_rect.is_null() {
            *lprc_clip_rect = RECT::default();
        }
        if !lp_frame_info.is_null() {
            *lp_frame_info = OLEINPLACEFRAMEINFO::default();
        }
        S_OK
    }
}
unsafe extern "system" fn scroll(_this: *mut c_void, _scroll_extent: SIZE) -> HRESULT {
    println!("IOleInPlaceSite::Scroll called");
    E_NOTIMPL
}
unsafe extern "system" fn on_ui_deactivate(_this: *mut c_void, _f_undoable: BOOL) -> HRESULT {
    println!("IOleInPlaceSite::OnUIDeactivate called");
    S_OK
}
unsafe extern "system" fn on_in_place_deactivate(_this: *mut c_void) -> HRESULT {
    println!("IOleInPlaceSite::OnInPlaceDeactivate called");
    S_OK
}
unsafe extern "system" fn discard_undo_state(_this: *mut c_void) -> HRESULT {
    println!("IOleInPlaceSite::DiscardUndoState called");
    S_OK
}
unsafe extern "system" fn deactivate_and_notify(_this: *mut c_void) -> HRESULT {
    println!("IOleInPlaceSite::DeactivateAndUndo called");
    S_OK
}
unsafe extern "system" fn on_pos_rect_change(
    _this: *mut c_void,
    _lprc_pos_rect: *const RECT,
) -> HRESULT {
    println!("IOleInPlaceSite::OnPosRectChange called");
    S_OK
}

// --- Vtable ---
pub static IOLEINPLACESITE_VTBL: IOleInPlaceSite_Vtbl = IOleInPlaceSite_Vtbl {
    base__: IOleWindow_Vtbl {
        base__: windows::core::IUnknown_Vtbl {
            QueryInterface: query_interface,
            AddRef: add_ref,
            Release: release,
        },
        GetWindow: get_window,
        ContextSensitiveHelp: context_sensitive_help,
    },
    CanInPlaceActivate: can_in_place_activate,
    OnInPlaceActivate: on_in_place_activate,
    OnUIActivate: on_ui_activate,
    GetWindowContext: get_window_context,
    Scroll: scroll,
    OnUIDeactivate: on_ui_deactivate,
    OnInPlaceDeactivate: on_in_place_deactivate,
    DiscardUndoState: discard_undo_state,
    DeactivateAndUndo: deactivate_and_notify,
    OnPosRectChange: on_pos_rect_change,
};
