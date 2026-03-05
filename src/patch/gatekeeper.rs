use crate::patch::module_info::ModuleInfo;
use std::ffi::c_void;
use uuid::Uuid;
use windows::core::GUID;

type Sub3721da6c = unsafe extern "cdecl" fn(a1: *mut GUID) -> u8;
static mut TRAMPOLINE: Option<Sub3721da6c> = None;

/// # Safety
/// This function relies on an accurate ModuleInfo representing the mapped PE image.
pub unsafe fn apply(info: &ModuleInfo) -> Result<(), String> {
    let target = info.resolve(0x3721da6c);

    // Using minhook from loader hook
    let hook_addr =
        unsafe { minhook::MinHook::create_hook(target, gatekeeper_hook as *mut c_void) }
            .map_err(|e| format!("MinHook create hook error for gatekeeper: {:?}", e))?;

    unsafe { minhook::MinHook::queue_enable_hook(target) }
        .map_err(|e| format!("MinHook queue enable error for gatekeeper: {:?}", e))?;

    unsafe {
        TRAMPOLINE = Some(std::mem::transmute::<
            *mut c_void,
            unsafe extern "cdecl" fn(*mut GUID) -> u8,
        >(hook_addr))
    };
    Ok(())
}

#[unsafe(no_mangle)]
unsafe extern "cdecl" fn gatekeeper_hook(a1: *mut GUID) -> u8 {
    let mut result = if let Some(trampoline) = unsafe { TRAMPOLINE } {
        unsafe { trampoline(a1) }
    } else {
        0
    };

    if !a1.is_null() && result == 0 {
        log::info!(
            "Gatekeeper original function failed to read registry. Providing newly generated GUID."
        );

        // Generate a new UUIDv4 to mimic successful GUID instantiation
        match generate_id() {
            Ok(new_guid) => {
                unsafe { *a1 = new_guid };
                result = 1;
            }
            Err(e) => {
                log::error!("Failed to generate fallback Gatekeeper GUID: {:?}", e);
            }
        }
    } else if !a1.is_null() && result != 0 {
        let guid = unsafe { *a1 };
        log::info!("Gatekeeper GUID loaded from registry correctly: {:?}", guid);
    }

    result
}

fn generate_id() -> Result<GUID, String> {
    let uuid = Uuid::new_v4();
    let (d1, d2, d3, d4) = uuid.as_fields();
    Ok(GUID::from_values(d1, d2, d3, *d4))
}
