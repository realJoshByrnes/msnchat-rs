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

use windows::core::{GUID, HRESULT, PCWSTR, BSTR, Interface}; // From the 'windows' crate
use windows::Win32::Foundation::S_OK; // S_OK is defined here
use windows::Win32::System::Com::{
    IDispatch, DISPPARAMS,
    DISPATCH_PROPERTYPUT, // For string properties
};
use windows::Win32::System::Ole::{DISPID_PROPERTYPUT, DISPID_UNKNOWN};
// use windows::Win32::System::Com::IID_NULL;
const IID_NULL: GUID = GUID::from_u128(0x00000000_0000_0000_0000_000000000000);
use windows::Win32::System::Variant::{VariantClear, VariantInit, VT_BSTR, VARIANT};
use std::ptr; // For ptr::null_mut()
use std::ffi::c_void;


// --- Helper function to set a string property via IDispatch ---
/// Sets a string property on a COM object via its IDispatch interface.
///
/// # Arguments
/// * `dispatch` - A ComPtr to the IDispatch interface of the target object.
/// * `property_name` - The string name of the property to set (e.g., "Text", "Caption").
/// * `new_value` - The new string value to assign to the property.
///
/// # Returns
/// A `windows::core::Result` indicating success or failure.
pub unsafe fn set_string_property(
    dispatch: &IDispatch,
    property_name: &str,
    new_value: &str,
) -> windows::core::Result<()> {
    // 1. Get the DISPID of the property from its string name
    let mut dispid: i32 = DISPID_UNKNOWN;
    // BSTR for GetIDsOfNames needs to be created from the Rust string
    let mut prop_name_bstr = BSTR::from(property_name);
    let mut prop_name_pcwstr = PCWSTR(prop_name_bstr.as_ptr());

    let hr = unsafe {
        dispatch.GetIDsOfNames(
            &IID_NULL, // Reserved, must be IID_NULL
            &mut prop_name_pcwstr as *mut PCWSTR, // Array of names (just one here)
            1,         // Count of names
            0,         // Locale ID (0 for default user locale)
            &mut dispid,
        )
    };

    // BSTR is no longer needed after the call
    drop(prop_name_bstr);

    if dispid == DISPID_UNKNOWN {
        eprintln!("Failed to get DISPID for property '{}': {:?}", property_name, hr);
        return Err(windows::core::Error::empty());
    }

    println!("Got DISPID {} for property '{}'.", dispid, property_name);

    // 2. Prepare the new value in a VARIANT structure
    let mut args_variant = unsafe { VariantInit() };
    (*args_variant.Anonymous.Anonymous).vt = VT_BSTR; // Set the VARIANT type to BSTR
    // Create a new BSTR for the value and store it in the VARIANT.
    // The field expects a ManuallyDrop<BSTR>.
    (*args_variant.Anonymous.Anonymous).Anonymous.bstrVal = std::mem::ManuallyDrop::new(BSTR::from(new_value));

    // 3. Prepare the DISPPARAMS structure
    let mut dispparams: DISPPARAMS = std::mem::zeroed();
    dispparams.cArgs = 1; // We have one argument: the new property value
    dispparams.rgvarg = &mut args_variant; // Pointer to our array of VARIANTs (just one here)
    
    // When setting a property, we must use a named argument indicating DISPID_PROPERTYPUT
    let mut named_arg_dispid = DISPID_PROPERTYPUT; // Standard constant for property "put"
    dispparams.cNamedArgs = 1;
    dispparams.rgdispidNamedArgs = &mut named_arg_dispid; // Pointer to our named argument DISPID

    // 4. Call IDispatch::Invoke to set the property
    let hr = unsafe {
        dispatch.Invoke(
            dispid,                 // The DISPID of the property to set
            &IID_NULL,              // Reserved, must be IID_NULL for most Invoke calls
            0,                      // Locale ID
            DISPATCH_PROPERTYPUT,   // Flags: We are putting (setting) a property's value
            &dispparams,            // Our prepared DISPPARAMS structure
            None,                   // pvarResult: Not expecting a return value for a property set
            None,                   // pExcepInfo: No exception info needed
            None,                   // puArgErr: No argument error info needed
        )
    };

    // 5. Clean up the VARIANT
    // VariantClear will automatically free the BSTR allocated inside `args_variant`
    VariantClear(&mut args_variant);

    // 6. Check the HRESULT
    if hr.is_err() {
        println!("Successfully set property '{}' to '{}'.", property_name, new_value);
        Ok(())
    } else {
        eprintln!("Failed to set property '{}': {:?}", property_name, hr);
        Err(windows::core::Error::empty())
    }
}