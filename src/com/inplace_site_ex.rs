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

use crate::SIZE;
use crate::com::shared::SharedSiteState;
use std::ffi::c_void;
use std::ptr;
use windows::Win32::Foundation::{E_NOINTERFACE, E_NOTIMPL, E_POINTER, HWND, RECT, S_OK};
use windows::Win32::System::Ole::{
    IOleInPlaceFrame, IOleInPlaceSiteEx, IOleInPlaceSiteEx_Vtbl, IOleInPlaceSite_Vtbl, IOleInPlaceUIWindow, IOleWindow_Vtbl,
    OLEINPLACEFRAMEINFO,
};
use windows::core::{BOOL, GUID, HRESULT};
use windows::Win32::UI::WindowsAndMessaging::GetClientRect;
use windows_core::Interface;

#[repr(C)]
pub struct MyOleInPlaceSiteEx {
    pub lpVtbl: *const IOleInPlaceSiteEx_Vtbl,
    pub shared: *mut SharedSiteState,
}

// --- IUnknown methods ---
pub unsafe extern "system" fn query_interface(
    this: *mut c_void,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    println!("MyOleInPlaceSiteEx::QueryInterface called for {:?}", unsafe {
        *riid
    });
    *ppv = ptr::null_mut();
    E_NOINTERFACE
}

pub unsafe extern "system" fn add_ref(this: *mut c_void) -> u32 {
    let site = &mut *(this as *mut MyOleInPlaceSiteEx);
    (*site.shared).ref_count += 1;
    (*site.shared).ref_count
}

pub unsafe extern "system" fn release(this: *mut c_void) -> u32 {
    let site = &mut *(this as *mut MyOleInPlaceSiteEx);
    (*site.shared).ref_count -= 1;
    let count = (*site.shared).ref_count;
    if count == 0 {
        drop(Box::from_raw(this as *mut MyOleInPlaceSiteEx));
    }
    count
}

// --- IOleInPlaceSite methods (stubs) ---
unsafe extern "system" fn get_window(this: *mut c_void, phwnd: *mut HWND) -> HRESULT {
    println!("IOleInPlaceSiteEx::GetWindow called");

    if phwnd.is_null() {
        return E_POINTER;
    }
    let site = &*(this as *mut MyOleInPlaceSiteEx);
    let hwnd = unsafe { (*site.shared).hwnd };
    *phwnd = hwnd;
    println!("- Returning hwnd: {:?}", hwnd);
    S_OK
}
unsafe extern "system" fn context_sensitive_help(
    _this: *mut c_void,
    _f_enter_mode: BOOL,
) -> HRESULT {
    println!("IOleInPlaceSiteEx::ContextSensitiveHelp called");
    E_NOTIMPL
}
unsafe extern "system" fn can_in_place_activate(_this: *mut c_void) -> HRESULT {
    println!("IOleInPlaceSiteEx::CanInPlaceActivate called");
    S_OK
}
unsafe extern "system" fn on_in_place_activate(_this: *mut c_void) -> HRESULT {
    println!("IOleInPlaceSiteEx::OnInPlaceActivate called");
    S_OK
}
unsafe extern "system" fn on_ui_activate(_this: *mut c_void) -> HRESULT {
    println!("IOleInPlaceSiteEx::OnUIActivate called");
    S_OK
}
unsafe extern "system" fn get_window_context(
    this: *mut c_void,
    pp_frame: *mut *mut c_void,
    pp_doc: *mut *mut c_void,
    lprc_pos_rect: *mut RECT,
    lprc_clip_rect: *mut RECT,
    lp_frame_info: *mut OLEINPLACEFRAMEINFO,
) -> HRESULT {
    println!("IOleInPlaceSiteEx::GetWindowContext called");
    if !pp_frame.is_null() {
        *pp_frame = ptr::null_mut();
    }
    if !pp_doc.is_null() {
        *pp_doc = ptr::null_mut();
    }

    // This sets the position and size of the in-place window.
    let mut rect = RECT::default();
    let hwnd = unsafe { (*(*this.cast::<MyOleInPlaceSiteEx>()).shared).hwnd };
    let _ = unsafe { GetClientRect(hwnd, &mut rect) };

    if !lprc_pos_rect.is_null() {
        *lprc_pos_rect = rect;
    }
    if !lprc_clip_rect.is_null() {
        *lprc_clip_rect = rect;
    }
    if !lp_frame_info.is_null() {
        *lp_frame_info = OLEINPLACEFRAMEINFO::default();
    }
    S_OK
}
unsafe extern "system" fn scroll(_this: *mut c_void, _scroll_extent: SIZE) -> HRESULT {
    println!("IOleInPlaceSiteEx::Scroll called");
    E_NOTIMPL
}
unsafe extern "system" fn on_ui_deactivate(_this: *mut c_void, _f_undoable: BOOL) -> HRESULT {
    println!("IOleInPlaceSiteEx::OnUIDeactivate called");
    S_OK
}
unsafe extern "system" fn on_in_place_deactivate(_this: *mut c_void) -> HRESULT {
    println!("IOleInPlaceSiteEx::OnInPlaceDeactivate called");
    S_OK
}
unsafe extern "system" fn discard_undo_state(_this: *mut c_void) -> HRESULT {
    println!("IOleInPlaceSiteEx::DiscardUndoState called");
    S_OK
}
unsafe extern "system" fn deactivate_and_notify(_this: *mut c_void) -> HRESULT {
    println!("IOleInPlaceSiteEx::DeactivateAndUndo called");
    S_OK
}
unsafe extern "system" fn on_pos_rect_change(
    _this: *mut c_void,
    _lprc_pos_rect: *const RECT,
) -> HRESULT {
    println!("IOleInPlaceSiteEx::OnPosRectChange called");
    S_OK
}

unsafe extern "system" fn on_in_place_activate_ex(
    _this: *mut c_void,
    _f_no_redraw: *mut BOOL,
    _lprc_pos_rect: u32,
) -> HRESULT {
    println!("IOleInPlaceSiteEx::OnInPlaceActivateEx called");
    E_NOTIMPL
}

unsafe extern "system" fn on_in_place_deactivate_ex(
    _this: *mut c_void,
    _f_no_redraw: BOOL,
) -> HRESULT {
    println!("IOleInPlaceSiteEx::OnInPlaceDeactivateEx called");
    E_NOTIMPL
}

unsafe extern "system" fn request_ui_activate(_this: *mut c_void) -> HRESULT {
    println!("IOleInPlaceSiteEx::RequestUIActivate called");
    E_NOTIMPL
}

// --- Vtable ---
pub static IOLEINPLACESITEEX_VTBL: IOleInPlaceSiteEx_Vtbl = IOleInPlaceSiteEx_Vtbl {
    base__: IOleInPlaceSite_Vtbl {
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
    },
    OnInPlaceActivateEx: on_in_place_activate_ex, // Placeholder for future implementation
    OnInPlaceDeactivateEx: on_in_place_deactivate_ex, // Placeholder for future implementation
    RequestUIActivate: request_ui_activate, // Placeholder for future implementation
};