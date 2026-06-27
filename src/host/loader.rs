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

    pub fn resize(&self, width: i32, height: i32) -> Result<()> {
        if let Some(inplace) = &self.inplace_object {
            let rect = RECT {
                left: 0,
                top: 0,
                right: width,
                bottom: height,
            };
            unsafe {
                inplace.SetObjectRects(&rect, &rect)?;
            }
        }
        Ok(())
    }

    pub fn dispatch(&self) -> Result<windows::Win32::System::Com::IDispatch> {
        self.ole_object.cast()
    }

    pub fn put_property(&self, name: &str, value: &str) -> Result<()> {
        unsafe {
            use windows::Win32::System::Com::{DISPATCH_PROPERTYPUT, DISPPARAMS, EXCEPINFO};
            use windows::Win32::System::Ole::DISPID_PROPERTYPUT;
            use windows::Win32::System::Variant::{VARIANT, VARIANT_0_0, VT_BSTR};
            use windows::core::BSTR;

            let dispatch = self.dispatch()?;

            let dispid = match name {
                "BackColor" => -501,
                "ForeColor" => -513,
                "RoomName" => 2,
                "HexRoomName" => 3,
                "NickName" => 4,
                "Server" => 5,
                "BackHighlightColor" => 6,
                "ButtonFrameColor" => 7,
                "TopBackHighlightColor" => 8,
                "ChatMode" => 9,
                "URLBack" => 10,
                "Category" => 11,
                "Topic" => 12,
                "WelcomeMsg" => 13,
                "BaseURL" => 15,
                "InputBorderColor" => 16,
                "CreateRoom" => 17,
                "ChatHome" => 19,
                "Locale" => 20,
                "ResDLL" => 21,
                "ButtonTextColor" => 22,
                "ButtonBackColor" => 23,
                "PassportTicket" => 24,
                "PassportProfile" => 25,
                "Feature" => 26,
                "MessageOfTheDay" => 27,
                "ChannelLanguage" => 28,
                "InvitationCode" => 29,
                "NicknameToInvite" => 30,
                "MSNREGCookie" => 31,
                "CreationModes" => 32,
                "MSNProfile" => 33,
                "Market" => 34,
                "WhisperContent" => 35,
                "UserRole" => 36,
                "AuditMessage" => 37,
                "SubscriberInfo" => 38,
                "UpsellURL" => 39,
                _ => {
                    log::error!("Unknown property name: {}", name);
                    return Err(windows::core::Error::from_hresult(windows::core::HRESULT(
                        E_FAIL.0,
                    )));
                }
            };

            let bstr_val = BSTR::from(value);
            let mut variant = VARIANT::default();

            let ptr = &mut variant as *mut VARIANT as *mut VARIANT_0_0;
            (*ptr).vt = VT_BSTR;
            (*ptr).Anonymous.bstrVal = std::mem::ManuallyDrop::new(bstr_val);

            let mut prop_put_dispid = DISPID_PROPERTYPUT;

            let dispparams = DISPPARAMS {
                rgvarg: &mut variant,
                rgdispidNamedArgs: &mut prop_put_dispid,
                cArgs: 1,
                cNamedArgs: 1,
            };

            let mut excepinfo = EXCEPINFO::default();
            let mut argerr = 0;

            let hr_invoke = dispatch.Invoke(
                dispid,
                &GUID::default(),
                0, // LCID
                DISPATCH_PROPERTYPUT,
                &dispparams,
                None,
                Some(&mut excepinfo),
                Some(&mut argerr),
            );

            if hr_invoke.is_err() {
                println!("Invoke failed for {}: {:?}", name, hr_invoke);
            }

            let _ = windows::Win32::System::Variant::VariantClear(&mut variant);

            hr_invoke
        }
    }
}
