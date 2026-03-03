use crate::patch::module_info::ModuleInfo;
use std::ffi::c_void;
use windows::Win32::Foundation::HMODULE;
use windows::core::PCWSTR;

type LoadLibraryWType = unsafe extern "system" fn(PCWSTR) -> HMODULE;

static mut O_LOAD_LIBRARY_W: Option<LoadLibraryWType> = None;

#[unsafe(no_mangle)]
unsafe extern "system" fn h_load_library_w(lp_lib_file_name: PCWSTR) -> HMODULE {
    let result = if let Some(trampoline) = unsafe { O_LOAD_LIBRARY_W } {
        unsafe { trampoline(lp_lib_file_name) }
    } else {
        HMODULE::default()
    };

    if !result.is_invalid() {
        let name = unsafe { lp_lib_file_name.display().to_string().to_lowercase() };
        if name.ends_with("msnchat45.ocx") {
            log::info!("Intercepted MsnChat45.ocx load, applying patches...");

            let module_info = ModuleInfo::new(result.0 as usize);
            if let Err(e) = unsafe { crate::patch::gatekeeper::apply(&module_info) } {
                log::error!("Failed to apply gatekeeper patch: {}", e);
            }

            // Apply queued hooks
            if let Err(status) = unsafe { minhook::MinHook::apply_queued() } {
                log::error!("Failed to apply queued hooks: {:?}", status);
            }
        }
    }

    result
}

// Ensure MinHook is set up and queues hook internally
unsafe fn hook(target: *mut c_void, detour: *mut c_void) -> Result<*mut c_void, String> {
    let hook_addr = unsafe { minhook::MinHook::create_hook(target, detour) }
        .map_err(|e| format!("MinHook create hook error: {:?}", e))?;

    unsafe { minhook::MinHook::queue_enable_hook(target) }
        .map_err(|e| format!("MinHook queue enable error: {:?}", e))?;

    Ok(hook_addr)
}

/// # Safety
/// This function relies on MinHook functioning correctly and hooking into Win32 APIs.
pub unsafe fn init_dll_hooks() -> Result<(), String> {
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::LibraryLoader::GetProcAddress;

    let kernel32 = unsafe { GetModuleHandleW(windows::core::w!("kernel32.dll")) }
        .map_err(|e| format!("GetModuleHandleW failed: {}", e))?;

    let load_library_w_ptr = unsafe {
        let proc = GetProcAddress(kernel32, windows::core::s!("LoadLibraryW"));
        if let Some(p) = proc {
            p as *mut c_void
        } else {
            return Err("GetProcAddress failed to find LoadLibraryW".to_string());
        }
    };

    let original = unsafe { hook(load_library_w_ptr, h_load_library_w as *mut c_void)? };
    unsafe {
        O_LOAD_LIBRARY_W = Some(std::mem::transmute::<
            *mut c_void,
            unsafe extern "system" fn(windows::core::PCWSTR) -> windows::Win32::Foundation::HMODULE,
        >(original))
    };

    unsafe { minhook::MinHook::apply_queued().map_err(|_| "Failed to apply_queued".to_string())? };

    Ok(())
}
