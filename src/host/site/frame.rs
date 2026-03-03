use std::ffi::c_void;
use std::ptr;
use windows::Win32::{
    Foundation::{E_NOINTERFACE, E_NOTIMPL, E_POINTER, HWND, RECT, S_OK},
    System::Ole::{
        IOleClientSite, IOleInPlaceFrame, IOleInPlaceFrame_Vtbl, IOleInPlaceSite,
        IOleInPlaceUIWindow_Vtbl, IOleWindow_Vtbl, OLEMENUGROUPWIDTHS,
    },
    UI::WindowsAndMessaging::{HMENU, MSG},
};
use windows::core::{BOOL, GUID, HRESULT, Interface, PCWSTR};

use super::SharedSiteState;

#[repr(C)]
pub struct MyOleInPlaceFrame {
    pub lp_vtbl: *const IOleInPlaceFrame_Vtbl,
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
        let this = this as *mut MyOleInPlaceFrame;
        let shared = (*this).shared;
        let riid = &*riid;

        if riid == &IOleInPlaceFrame::IID
            || riid == &windows::Win32::System::Ole::IOleInPlaceUIWindow::IID
            || riid == &windows::Win32::System::Ole::IOleWindow::IID
            || riid == &windows::core::IUnknown::IID
        {
            *ppv = this as *mut _ as *mut c_void;
            add_ref(this as *mut c_void);
            return S_OK;
        } else if riid == &IOleClientSite::IID {
            let client_site = (*shared).client_site;
            *ppv = client_site as *mut c_void;
            super::client::add_ref(client_site as *mut c_void);
            return S_OK;
        } else if riid == &IOleInPlaceSite::IID {
            let inplace_site = (*shared).inplace_site;
            *ppv = inplace_site as *mut c_void;
            super::inplace::add_ref(inplace_site as *mut c_void);
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
        let site = &mut *(this as *mut MyOleInPlaceFrame);
        (*site.shared).ref_count += 1;
        (*site.shared).ref_count
    }
}

/// # Safety
/// This is a mock implementation of IUnknown::Release.
pub unsafe extern "system" fn release(this: *mut c_void) -> u32 {
    unsafe {
        let site = &mut *(this as *mut MyOleInPlaceFrame);
        (*site.shared).ref_count -= 1;
        let count = (*site.shared).ref_count;
        if count == 0 {
            drop(Box::from_raw(this as *mut MyOleInPlaceFrame));
        }
        count
    }
}

unsafe extern "system" fn get_window(this: *mut c_void, phwnd: *mut HWND) -> HRESULT {
    unsafe {
        if !phwnd.is_null() {
            let site = &mut *(this as *mut MyOleInPlaceFrame);
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

unsafe extern "system" fn get_border(_this: *mut c_void, _lprect_border: *mut RECT) -> HRESULT {
    E_NOTIMPL
}

unsafe extern "system" fn request_border_space(
    _this: *mut c_void,
    _pborderwidths: *const RECT,
) -> HRESULT {
    E_NOTIMPL
}

unsafe extern "system" fn set_border_space(
    _this: *mut c_void,
    _pborderwidths: *const RECT,
) -> HRESULT {
    E_NOTIMPL
}

unsafe extern "system" fn set_active_object(
    _this: *mut c_void,
    _p_active_object: *mut c_void,
    _psz_obj_name: PCWSTR,
) -> HRESULT {
    S_OK
}

unsafe extern "system" fn insert_menus(
    _this: *mut c_void,
    _hmenu_shared: HMENU,
    _lp_menu_widths: *mut OLEMENUGROUPWIDTHS,
) -> HRESULT {
    E_NOTIMPL
}

unsafe extern "system" fn set_menu(
    _this: *mut c_void,
    _hmenu_shared: HMENU,
    _holemenu: isize,
    _hwnd_active_object: HWND,
) -> HRESULT {
    E_NOTIMPL
}

unsafe extern "system" fn remove_menus(_this: *mut c_void, _hmenu_shared: HMENU) -> HRESULT {
    E_NOTIMPL
}

unsafe extern "system" fn set_status_text(_this: *mut c_void, _psz_status_text: PCWSTR) -> HRESULT {
    S_OK
}

unsafe extern "system" fn enable_modeless(_this: *mut c_void, _f_enable: BOOL) -> HRESULT {
    S_OK
}

unsafe extern "system" fn translate_accelerator(
    _this: *mut c_void,
    _lpmsg: *const MSG,
    _w_id: u16,
) -> HRESULT {
    E_NOTIMPL
}

pub static IOLEINPLACEFRAME_VTBL: IOleInPlaceFrame_Vtbl = IOleInPlaceFrame_Vtbl {
    base__: IOleInPlaceUIWindow_Vtbl {
        base__: IOleWindow_Vtbl {
            base__: windows::core::IUnknown_Vtbl {
                QueryInterface: query_interface,
                AddRef: add_ref,
                Release: release,
            },
            GetWindow: get_window,
            ContextSensitiveHelp: context_sensitive_help,
        },
        GetBorder: get_border,
        RequestBorderSpace: request_border_space,
        SetBorderSpace: set_border_space,
        SetActiveObject: set_active_object,
    },
    InsertMenus: insert_menus,
    SetMenu: set_menu,
    RemoveMenus: remove_menus,
    SetStatusText: set_status_text,
    EnableModeless: enable_modeless,
    TranslateAccelerator: translate_accelerator,
};
