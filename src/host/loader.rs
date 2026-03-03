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
    pub module: crate::patch::pe::ManualModule,
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
    pub fn new(dll_bytes: &[u8], clsid: &GUID) -> Result<Self> {
        unsafe {
            let module = crate::patch::pe::ManualModule::load(dll_bytes).map_err(|e| {
                log::error!("Manual load failed: {}", e);
                windows::core::Error::from_hresult(windows::core::HRESULT(E_FAIL.0))
            })?;

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

            let mut current_exe = [0u16; 1024];
            let _len =
                windows::Win32::System::LibraryLoader::GetModuleFileNameW(None, &mut current_exe);
            let exe_path = windows::core::PCWSTR::from_raw(current_exe.as_ptr());

            let typelib = windows::Win32::System::Ole::LoadTypeLib(exe_path).map_err(|e| {
                log::error!("Failed to load embedded TYPELIB: {}", e);
                windows::core::Error::from_hresult(windows::core::HRESULT(E_FAIL.0))
            })?;

            let type_info = typelib.GetTypeInfo(0).map_err(|e| {
                log::error!("Failed to get TypeInfo at index 0: {}", e);
                windows::core::Error::from_hresult(windows::core::HRESULT(E_FAIL.0))
            })?;

            Ok(Self {
                module,
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
            let client_site = IOleClientSite::from_raw(client_site_raw);

            self.ole_object.SetClientSite(&client_site)?;

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

            let mut dispid = 0;
            let name_hstring = windows::core::HSTRING::from(name);
            let pnames = [windows::core::PCWSTR::from_raw(name_hstring.as_ptr())];

            let hr_id = self
                .type_info
                .GetIDsOfNames(pnames.as_ptr(), 1, &mut dispid);

            if hr_id.is_err() {
                log::error!("GetIDsOfNames failed for {}: {:?}", name, hr_id);
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
                log::error!("Invoke failed for {}: {:?}", name, hr_invoke);
            }

            let _ = windows::Win32::System::Variant::VariantClear(&mut variant);

            hr_invoke
        }
    }
}
