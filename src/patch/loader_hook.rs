use super::module_info::ModuleInfo;
use std::ffi::c_void;
use windows::Win32::Foundation::HMODULE;
use windows::core::{PCSTR, PCWSTR};

type LoadLibraryWType = unsafe extern "system" fn(PCWSTR) -> HMODULE;

static mut O_LOAD_LIBRARY_W: Option<LoadLibraryWType> = None;
pub static mut OCX_MODULE: Option<HMODULE> = None;

#[unsafe(no_mangle)]
unsafe extern "system" fn h_load_library_w(lp_lib_file_name: PCWSTR) -> HMODULE {
    let result = if let Some(trampoline) = unsafe { O_LOAD_LIBRARY_W } {
        unsafe { trampoline(lp_lib_file_name) }
    } else {
        HMODULE::default()
    };

    if !result.is_invalid() {
        let name = unsafe { lp_lib_file_name.display().to_string().to_lowercase() };
        // The file string is often an absolute path, we'll check if it ends with the name
        if name.ends_with("msnchat45.ocx") {
            log::info!("Intercepted MsnChat45.ocx load, applying patches...");
            unsafe {
                OCX_MODULE = Some(result);
            }

            let module_info = ModuleInfo::new(result.0 as usize);
            if let Err(e) = unsafe { crate::patch::gatekeeper_id::apply(&module_info) } {
                log::error!("Failed to apply gatekeeper_id patch: {}", e);
            }
            if let Err(e) = unsafe { crate::patch::virtual_protect::apply(&module_info) } {
                log::error!("Failed to apply virtual_protect patch: {}", e);
            }
            if let Err(e) = unsafe { crate::patch::directory::apply(&module_info) } {
                log::error!("Failed to apply Directory Server patches: {}", e);
            }
            if let Err(e) = unsafe { crate::patch::channel::apply(&module_info) } {
                log::error!("Failed to apply Channel Server patches: {}", e);
            }
            if let Err(e) = unsafe { crate::patch::sound_patch::apply(&module_info) } {
                log::error!("Failed to apply sound patches: {}", e);
            }

            // Apply queued hooks
            if let Err(status) = unsafe { minhook::MinHook::apply_queued() } {
                log::error!("Failed to apply queued hooks: {:?}", status);
            }
        }
    }

    result
}

/// # Safety
///
/// This function is unsafe because it modifies global state and installs hooks.
pub unsafe fn init_dll_hooks() -> Result<(), String> {
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::LibraryLoader::GetProcAddress;

    let kernel32 = unsafe { GetModuleHandleW(windows::core::w!("kernel32.dll")) }
        .map_err(|e| format!("GetModuleHandleW failed: {}", e))?;

    let load_library_w_ptr = unsafe {
        let proc = GetProcAddress(
            kernel32,
            PCSTR::from_raw(c"LoadLibraryW".as_ptr() as *const u8),
        );
        if let Some(p) = proc {
            p as *mut c_void
        } else {
            return Err("GetProcAddress failed to find LoadLibraryW".to_string());
        }
    };

    let original = unsafe { super::hook(load_library_w_ptr, h_load_library_w as *mut c_void)? };
    unsafe {
        O_LOAD_LIBRARY_W = Some(std::mem::transmute::<*mut c_void, LoadLibraryWType>(
            original,
        ))
    };

    unsafe { minhook::MinHook::apply_queued().map_err(|_| "Failed to apply_queued".to_string())? };

    Ok(())
}
