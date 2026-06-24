use windows::{
    Win32::{
        Foundation::{E_FAIL, HMODULE, HWND, RECT},
        System::{
            Com::IClassFactory,
            LibraryLoader::{GetProcAddress, LoadLibraryW},
            Ole::{IOleClientSite, IOleInPlaceObject, IOleObject, OLEIVERB_SHOW},
        },
        UI::WindowsAndMessaging::GetClientRect,
    },
    core::{GUID, IUnknown, Interface, PCSTR, Result},
};

use super::site::{HostWrappers, create_host_wrappers};

type DllGetClassObjectFunc = unsafe extern "system" fn(
    *const GUID,
    *const GUID,
    *mut *mut std::ffi::c_void,
) -> windows::core::HRESULT;

pub struct OcxHost {
    _module: HMODULE,
    ole_object: IOleObject,
    inplace_object: Option<IOleInPlaceObject>,
    wrappers: Box<HostWrappers>,
    type_info: windows::Win32::System::Com::ITypeInfo,
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
    pub fn new(dll_path: &str, clsid: &GUID) -> Result<Self> {
        unsafe {
            let path_w = windows::core::HSTRING::from(dll_path);
            let module = LoadLibraryW(&path_w)?;

            if module.is_invalid() {
                return Err(windows::core::Error::from_hresult(windows::core::HRESULT(
                    E_FAIL.0,
                )));
            }

            let func_name = PCSTR::from_raw(c"DllGetClassObject".as_ptr() as *const u8);
            let get_class_object_ptr = GetProcAddress(module, func_name);

            let get_class_object: DllGetClassObjectFunc = match get_class_object_ptr {
                Some(p) => std::mem::transmute::<
                    unsafe extern "system" fn() -> isize,
                    DllGetClassObjectFunc,
                >(p),
                None => {
                    return Err(windows::core::Error::from_hresult(windows::core::HRESULT(
                        E_FAIL.0,
                    )));
                }
            };

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

            let dll_hstring = windows::core::HSTRING::from(dll_path);
            let type_lib = windows::Win32::System::Ole::LoadTypeLib(&dll_hstring)?;
            let type_info = type_lib.GetTypeInfo(0)?;

            Ok(Self {
                _module: module,
                ole_object,
                inplace_object: None,
                wrappers,
                type_info,
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
            if let Ok(cpc) = self.ole_object.cast::<windows::Win32::System::Com::IConnectionPointContainer>() {
                if let Ok(cp) = cpc.FindConnectionPoint(&super::site::events::IID_ICCHATFRAMEEVENTS) {
                    let events_raw = self.wrappers.events as *mut std::ffi::c_void;
                    super::site::events::add_ref(events_raw);
                    let events_unk = IUnknown::from_raw(events_raw);
                    let _cookie = cp.Advise(&events_unk);
                }
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
            let dispatch = self.dispatch()?;

            use windows::Win32::System::Com::{DISPATCH_PROPERTYPUT, DISPPARAMS, EXCEPINFO};
            use windows::Win32::System::Ole::DISPID_PROPERTYPUT;
            use windows::Win32::System::Variant::{VARIANT, VARIANT_0_0, VT_BSTR};
            use windows::core::BSTR;

            let mut dispid = 0;
            let name_hstring = windows::core::HSTRING::from(name);
            let name_pcwstr = windows::core::PCWSTR::from_raw(name_hstring.as_ptr());

            let hr_id = self.type_info.GetIDsOfNames(&name_pcwstr, 1, &mut dispid);

            if hr_id.is_err() {
                println!("GetIDsOfNames failed for {}: {:?}", name, hr_id);
                return hr_id;
            }

            let bstr_val = BSTR::from(value);
            let mut variant = VARIANT::default();

            let ptr = &mut variant as *mut VARIANT as *mut VARIANT_0_0;
            (*ptr).vt = VT_BSTR;
            (*ptr).Anonymous.bstrVal = std::mem::ManuallyDrop::new(bstr_val);

            let mut prop_put_dispid = DISPID_PROPERTYPUT;

            let mut dispparams = DISPPARAMS {
                rgvarg: &mut variant,
                rgdispidNamedArgs: &mut prop_put_dispid,
                cArgs: 1,
                cNamedArgs: 1,
            };

            let mut excepinfo = EXCEPINFO::default();
            let mut argerr = 0;

            let hr_invoke = self.type_info.Invoke(
                dispatch.as_raw(),
                dispid,
                DISPATCH_PROPERTYPUT,
                &mut dispparams,
                std::ptr::null_mut(),
                &mut excepinfo,
                &mut argerr,
            );

            if hr_invoke.is_err() {
                println!("Invoke failed for {}: {:?}", name, hr_invoke);
            }

            let _ = windows::Win32::System::Variant::VariantClear(&mut variant);

            hr_invoke
        }
    }
}
