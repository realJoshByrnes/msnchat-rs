use windows::{
    Win32::{
        Foundation::{E_FAIL, HWND, RECT},
        System::{
            Com::IClassFactory,
            Ole::{IOleClientSite, IOleInPlaceObject, IOleObject, OLEIVERB_SHOW},
        },
        UI::WindowsAndMessaging::GetClientRect,
    },
    core::{GUID, IUnknown, Interface, Result},
};

use super::site::{HostWrappers, create_host_wrappers};

type DllGetClassObjectFunc = unsafe extern "system" fn(
    *const GUID,
    *const GUID,
    *mut *mut std::ffi::c_void,
) -> windows::core::HRESULT;

pub struct OcxHost {
    pub module: std::sync::Arc<crate::patch::pe::ManualModule>,
    ole_object: IOleObject,
    inplace_object: Option<IOleInPlaceObject>,
    wrappers: Box<HostWrappers>,
}

impl Drop for OcxHost {
    fn drop(&mut self) {
        unsafe {
            if let Some(inplace) = &self.inplace_object {
                let _ = inplace.InPlaceDeactivate();
            }
            let _ = self
                .ole_object
                .Close(windows::Win32::System::Ole::OLECLOSE_NOSAVE);
        }
    }
}

impl OcxHost {
    pub fn new(
        module: std::sync::Arc<crate::patch::pe::ManualModule>,
        clsid: &GUID,
    ) -> Result<Self> {
        unsafe {
            let get_class_object_ptr = module.get_export("DllGetClassObject").map_err(|e| {
                log::error!("DllGetClassObject not found: {}", e);
                windows::core::Error::from_hresult(windows::core::HRESULT(E_FAIL.0))
            })?;

            let get_class_object: DllGetClassObjectFunc = std::mem::transmute(get_class_object_ptr);

            let mut factory_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
            let hr = get_class_object(clsid as *const GUID, &IClassFactory::IID, &mut factory_ptr);

            if hr.is_err() || factory_ptr.is_null() {
                return Err(windows::core::Error::from_hresult(hr));
            }

            let factory = IClassFactory::from_raw(factory_ptr);

            let unk: IUnknown = factory.CreateInstance(None)?;

            let ole_object: IOleObject = unk.cast()?;

            if let Ok(size) = ole_object.GetExtent(windows::Win32::System::Com::DVASPECT(1)) {
                log::info!(
                    "ActiveX Control CLSID {:?} extent: cx={}, cy={}",
                    clsid,
                    size.cx,
                    size.cy
                );
            }

            let dummy_hwnd = HWND(std::ptr::null_mut());
            let wrappers = Box::new(create_host_wrappers(dummy_hwnd));

            Ok(Self {
                module,
                ole_object,
                inplace_object: None,
                wrappers,
            })
        }
    }

    pub fn attach(&mut self, hwnd: HWND) -> Result<()> {
        unsafe {
            (*self.wrappers._shared).hwnd = hwnd;

            let client_site_raw = self.wrappers.client_site as *mut std::ffi::c_void;
            super::site::client::add_ref(client_site_raw);
            let client_site = IOleClientSite::from_raw(client_site_raw);

            self.ole_object.SetClientSite(&client_site)?;

            // Query connection point container and advise our event sink
            if let Ok(cpc) = self
                .ole_object
                .cast::<windows::Win32::System::Com::IConnectionPointContainer>()
                && let Ok(cp) = cpc.FindConnectionPoint(&super::site::events::IID_ICCHATFRAMEEVENTS)
            {
                let events_raw = self.wrappers.events as *mut std::ffi::c_void;
                super::site::events::add_ref(events_raw);
                let events_unk = IUnknown::from_raw(events_raw);
                let _cookie = cp.Advise(&events_unk);
            }

            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut rect);

            let mut inplace_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
            let hr = self
                .ole_object
                .query(&IOleInPlaceObject::IID, &mut inplace_ptr);

            if hr.is_ok() && !inplace_ptr.is_null() {
                let inplace_object = IOleInPlaceObject::from_raw(inplace_ptr);
                inplace_object.SetObjectRects(&rect, &rect)?;
                self.inplace_object = Some(inplace_object.clone());
            }

            self.ole_object.DoVerb(
                OLEIVERB_SHOW.0,
                std::ptr::null_mut(),
                &client_site,
                0,
                hwnd,
                &rect,
            )?;
        }
        Ok(())
    }

    pub fn resize(&self, rect: &RECT) -> Result<()> {
        if let Some(inplace) = &self.inplace_object {
            unsafe {
                inplace.SetObjectRects(rect, rect)?;
            }
        }
        Ok(())
    }

    pub fn get_control_hwnd(&self) -> Result<HWND> {
        if let Some(inplace) = &self.inplace_object {
            unsafe { inplace.GetWindow() }
        } else {
            Err(windows::core::Error::from_hresult(windows::core::HRESULT(
                E_FAIL.0,
            )))
        }
    }

    pub fn get_extent(&self) -> Result<windows::Win32::Foundation::SIZE> {
        unsafe {
            self.ole_object
                .GetExtent(windows::Win32::System::Com::DVASPECT(1))
        }
    }

    pub fn dispatch(&self) -> Result<windows::Win32::System::Com::IDispatch> {
        self.ole_object.cast()
    }

    pub fn put_property(&self, name: &str, value: &str) -> Result<()> {
        let dispatch = self.dispatch()?;
        let this_ptr = windows::core::Interface::as_raw(&dispatch);

        // Detect if this is IChatSettings or IChatFrame
        let settings_guid = windows::core::GUID::from_values(
            0xD5EF4299,
            0x12F1,
            0x474D,
            [0x98, 0xC5, 0x3C, 0x65, 0x8F, 0xD2, 0xE3, 0x43],
        );
        let mut settings_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let is_settings = unsafe { dispatch.query(&settings_guid, &mut settings_ptr).is_ok() };
        if is_settings && !settings_ptr.is_null() {
            unsafe {
                let _ = windows::core::IUnknown::from_raw(settings_ptr);
            }
        }

        unsafe {
            let vtable = *(this_ptr as *const *const *const std::ffi::c_void);

            let call_put_bstr = |index: usize, val: &str| -> Result<()> {
                let func_ptr = *vtable.add(index);
                let func: unsafe extern "system" fn(
                    *mut std::ffi::c_void,
                    windows::core::BSTR,
                ) -> windows::core::HRESULT = std::mem::transmute(func_ptr);
                let bstr = windows::core::BSTR::from(val);
                let hr = func(this_ptr, bstr);
                hr.ok()
            };

            let call_put_i32 = |index: usize, val: i32| -> Result<()> {
                let func_ptr = *vtable.add(index);
                let func: unsafe extern "system" fn(
                    *mut std::ffi::c_void,
                    i32,
                ) -> windows::core::HRESULT = std::mem::transmute(func_ptr);
                let hr = func(this_ptr, val);
                hr.ok()
            };

            let call_put_u32 = |index: usize, val: u32| -> Result<()> {
                let func_ptr = *vtable.add(index);
                let func: unsafe extern "system" fn(
                    *mut std::ffi::c_void,
                    u32,
                ) -> windows::core::HRESULT = std::mem::transmute(func_ptr);
                let hr = func(this_ptr, val);
                hr.ok()
            };

            if is_settings {
                match name {
                    "BackColor" => call_put_u32(7, value.parse().unwrap_or(0)),
                    "ForeColor" => call_put_u32(9, value.parse().unwrap_or(0)),
                    "RedirectURL" => call_put_bstr(11, value),
                    "ResDLL" => call_put_bstr(13, value),
                    _ => {
                        log::error!("Unknown IChatSettings property name: {}", name);
                        Err(windows::core::Error::from_hresult(windows::core::HRESULT(
                            windows::Win32::Foundation::E_FAIL.0,
                        )))
                    }
                }
            } else {
                match name {
                    "BackColor" => call_put_u32(7, value.parse().unwrap_or(0)),
                    "RoomName" => call_put_bstr(10, value),
                    "HexRoomName" => call_put_bstr(12, value),
                    "NickName" => call_put_bstr(14, value),
                    "Server" => call_put_bstr(16, value),
                    "BackHighlightColor" => call_put_u32(18, value.parse().unwrap_or(0)),
                    "ButtonFrameColor" => call_put_u32(20, value.parse().unwrap_or(0)),
                    "TopBackHighlightColor" => call_put_u32(22, value.parse().unwrap_or(0)),
                    "ChatMode" => call_put_i32(24, value.parse().unwrap_or(0)),
                    "URLBack" => call_put_bstr(26, value),
                    "Category" => call_put_bstr(28, value),
                    "Topic" => call_put_bstr(30, value),
                    "WelcomeMsg" => call_put_bstr(32, value),
                    "BaseURL" => call_put_bstr(34, value),
                    "InputBorderColor" => call_put_u32(36, value.parse().unwrap_or(0)),
                    "CreateRoom" => call_put_bstr(38, value),
                    "ChatHome" => call_put_bstr(40, value),
                    "Locale" => call_put_bstr(42, value),
                    "ResDLL" => call_put_bstr(44, value),
                    "ButtonTextColor" => call_put_u32(46, value.parse().unwrap_or(0)),
                    "ButtonBackColor" => call_put_u32(48, value.parse().unwrap_or(0)),
                    "PassportTicket" => call_put_bstr(50, value),
                    "PassportProfile" => call_put_bstr(52, value),
                    "Feature" => call_put_u32(54, value.parse().unwrap_or(0)),
                    "MessageOfTheDay" => call_put_bstr(56, value),
                    "ChannelLanguage" => call_put_bstr(58, value),
                    "InvitationCode" => call_put_bstr(60, value),
                    "NicknameToInvite" => call_put_bstr(62, value),
                    "MSNREGCookie" => call_put_bstr(64, value),
                    "CreationModes" => call_put_bstr(66, value),
                    "MSNProfile" => call_put_bstr(68, value),
                    "Market" => call_put_bstr(70, value),
                    "WhisperContent" => call_put_bstr(72, value),
                    "UserRole" => call_put_bstr(74, value),
                    "AuditMessage" => call_put_bstr(76, value),
                    "SubscriberInfo" => call_put_bstr(78, value),
                    "UpsellURL" => call_put_bstr(80, value),
                    _ => {
                        log::error!("Unknown IChatFrame property name: {}", name);
                        Err(windows::core::Error::from_hresult(windows::core::HRESULT(
                            windows::Win32::Foundation::E_FAIL.0,
                        )))
                    }
                }
            }
        }
    }
}
