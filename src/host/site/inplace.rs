use std::ffi::c_void;
use std::ptr;
use windows::Win32::{
    Foundation::{E_NOINTERFACE, E_NOTIMPL, E_POINTER, HWND, RECT, S_OK, SIZE},
    System::Ole::{
        IOleClientSite, IOleInPlaceFrame, IOleInPlaceSite, IOleInPlaceSite_Vtbl, IOleWindow_Vtbl,
        OLEINPLACEFRAMEINFO,
    },
};
use windows::core::{BOOL, GUID, HRESULT, Interface};

use super::SharedSiteState;

#[repr(C)]
pub struct MyOleInPlaceSite {
    pub lp_vtbl: *const IOleInPlaceSite_Vtbl,
    pub shared: *mut SharedSiteState,
}

/// # Safety
/// This is a mock implementation of IUnknown::QueryInterface.
pub unsafe extern "system" fn query_interface(
    this: *mut c_void,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    unsafe {
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
        } else if riid == &IOleClientSite::IID {
            let client_site = (*shared).client_site;
            *ppv = client_site as *mut c_void;
            super::client::add_ref(client_site as *mut c_void);
            return S_OK;
        } else if riid == &IOleInPlaceFrame::IID {
            let frame = (*shared).frame;
            *ppv = frame as *mut c_void;
            super::frame::add_ref(frame as *mut c_void);
            return S_OK;
        }
        *ppv = ptr::null_mut();
        E_NOINTERFACE
    }
}

/// # Safety
/// This is a mock implementation of IUnknown::AddRef.
pub unsafe extern "system" fn add_ref(this: *mut c_void) -> u32 {
    unsafe {
        let site = &mut *(this as *mut MyOleInPlaceSite);
        (*site.shared).ref_count += 1;
        (*site.shared).ref_count
    }
}

/// # Safety
/// This is a mock implementation of IUnknown::Release.
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

unsafe extern "system" fn get_window(this: *mut c_void, phwnd: *mut HWND) -> HRESULT {
    unsafe {
        if !phwnd.is_null() {
            let site = &mut *(this as *mut MyOleInPlaceSite);
            *phwnd = (*site.shared).hwnd;
        }
        S_OK
    }
}
unsafe extern "system" fn context_sensitive_help(
    _this: *mut c_void,
    _f_enter_mode: BOOL,
) -> HRESULT {
    E_NOTIMPL
}
unsafe extern "system" fn can_in_place_activate(_this: *mut c_void) -> HRESULT {
    S_OK
}
unsafe extern "system" fn on_in_place_activate(_this: *mut c_void) -> HRESULT {
    S_OK
}
unsafe extern "system" fn on_ui_activate(_this: *mut c_void) -> HRESULT {
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
    unsafe {
        let site = &mut *(this as *mut MyOleInPlaceSite);

        if !pp_frame.is_null() {
            let frame = (*site.shared).frame;
            *pp_frame = frame as *mut c_void;
            super::frame::add_ref(frame as *mut c_void);
        }
        if !pp_doc.is_null() {
            // Document window is not used here
            *pp_doc = ptr::null_mut();
        }
        if !lprc_pos_rect.is_null() {
            _ = windows::Win32::UI::WindowsAndMessaging::GetClientRect(
                (*site.shared).hwnd,
                lprc_pos_rect,
            );
        }
        if !lprc_clip_rect.is_null() {
            _ = windows::Win32::UI::WindowsAndMessaging::GetClientRect(
                (*site.shared).hwnd,
                lprc_clip_rect,
            );
        }
        if !lp_frame_info.is_null() {
            (*lp_frame_info).cb = std::mem::size_of::<OLEINPLACEFRAMEINFO>() as u32;
            (*lp_frame_info).fMDIApp = BOOL(0);
            (*lp_frame_info).hwndFrame = (*site.shared).hwnd;
            (*lp_frame_info).haccel = windows::Win32::UI::WindowsAndMessaging::HACCEL::default();
            (*lp_frame_info).cAccelEntries = 0;
        }
        S_OK
    }
}
unsafe extern "system" fn scroll(_this: *mut c_void, _scroll_extent: SIZE) -> HRESULT {
    E_NOTIMPL
}
unsafe extern "system" fn on_ui_deactivate(_this: *mut c_void, _f_undoable: BOOL) -> HRESULT {
    S_OK
}
unsafe extern "system" fn on_in_place_deactivate(_this: *mut c_void) -> HRESULT {
    S_OK
}
unsafe extern "system" fn discard_undo_state(_this: *mut c_void) -> HRESULT {
    S_OK
}
unsafe extern "system" fn deactivate_and_notify(_this: *mut c_void) -> HRESULT {
    S_OK
}
unsafe extern "system" fn on_pos_rect_change(
    _this: *mut c_void,
    _lprc_pos_rect: *const RECT,
) -> HRESULT {
    S_OK
}

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
