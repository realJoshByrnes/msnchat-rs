use crate::patch::module_info::ModuleInfo;
use std::ffi::c_void;
use windows::Win32::System::Memory::{
    PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS, VirtualProtect,
};
use windows::core::BOOL;

type AtlThunkInit = unsafe extern "thiscall" fn(this: *mut c_void, a2: u32, a3: u32) -> BOOL;
static mut TRAMPOLINE: Option<AtlThunkInit> = None;

/// # Safety
/// This function relies on `ModuleInfo` which contains raw pointers to the PE memory.
pub unsafe fn apply(info: &ModuleInfo) -> Result<(), String> {
    let target = info.resolve(0x3720e0a5);

    let hook_addr = unsafe { minhook::MinHook::create_hook(target, atl_thunk_hook as *mut c_void) }
        .map_err(|e| format!("MinHook create hook error for atl thunk: {:?}", e))?;

    unsafe { minhook::MinHook::queue_enable_hook(target) }
        .map_err(|e| format!("MinHook queue enable error for atl thunk: {:?}", e))?;

    unsafe {
        TRAMPOLINE = Some(std::mem::transmute::<
            *mut c_void,
            unsafe extern "thiscall" fn(*mut c_void, u32, u32) -> BOOL,
        >(hook_addr))
    };
    Ok(())
}

#[unsafe(no_mangle)]
unsafe extern "thiscall" fn atl_thunk_hook(this: *mut c_void, a2: u32, a3: u32) -> BOOL {
    unsafe {
        if !this.is_null() {
            let mut old_protect = PAGE_PROTECTION_FLAGS::default();
            let _ = VirtualProtect(
                this,
                13, // ATL thunk size
                PAGE_EXECUTE_READWRITE,
                &mut old_protect,
            );
        }

        if let Some(trampoline) = TRAMPOLINE {
            trampoline(this, a2, a3)
        } else {
            BOOL(0)
        }
    }
}
