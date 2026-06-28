use minhook::MinHook;
use std::ffi::c_void;

pub mod channel;
pub mod directory;
pub mod font_style_patch;
pub mod loader_hook;
pub mod module_info;
pub mod network;
pub mod pe;
pub mod registry_hook;
pub mod sound_patch;
pub mod virtual_protect;

/// # Safety
///
/// This function is unsafe because it creates and enables an address hook on target.
pub unsafe fn hook(target: *mut c_void, detour: *mut c_void) -> Result<*mut c_void, String> {
    let original = match unsafe { MinHook::create_hook(target, detour) } {
        Ok(original) => original,
        Err(e) => return Err(format!("MH_CreateHook failed: {:?}", e)),
    };

    if let Err(e) = unsafe { MinHook::enable_hook(target) } {
        return Err(format!("MH_EnableHook failed: {:?}", e));
    }

    Ok(original)
}
