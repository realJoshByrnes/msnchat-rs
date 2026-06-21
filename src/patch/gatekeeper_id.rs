use std::ffi::c_void;
use windows::core::{BOOL, GUID};

use super::module_info::ModuleInfo;

type Sub3721DA6C = unsafe extern "cdecl" fn(a1: *const GUID) -> BOOL;

static mut TRAMPOLINE: Option<Sub3721DA6C> = None;

/// # Safety
///
/// This function is unsafe because it installs hooks on module load.
pub unsafe fn apply(info: &ModuleInfo) -> Result<(), String> {
    let target = info.resolve(0x3721da6c);
    let trampoline = unsafe { super::hook(target, detour_gatekeeper_id as *mut c_void) }?;

    // SAFETY: Single threaded init
    unsafe { TRAMPOLINE = Some(std::mem::transmute::<*mut c_void, Sub3721DA6C>(trampoline)) };
    Ok(())
}

#[unsafe(no_mangle)]
unsafe extern "cdecl" fn detour_gatekeeper_id(a1: *const GUID) -> BOOL {
    let mut result = if let Some(trampoline) = unsafe { TRAMPOLINE } {
        unsafe { trampoline(a1) }
    } else {
        BOOL(0)
    };

    if !a1.is_null() {
        // SAFETY: We checked for null, assuming valid pointer if not null
        let guid_val = unsafe { *a1 };

        // The MSN Chat Control typically retrieves the GateKeeper ID from the Windows Registry.
        // Since this implementation does not require component registration, the registry entry
        // may be missing, resulting in a zeroed GUID and a failure code.
        //
        // This patch intercepts the failure case where the GUID is zero. It generates a new,
        // valid GUID on the fly and writes it to the output parameter, simulating a successful
        // retrieval. This ensures the control can initialize correctly without external dependencies.
        if result == BOOL(false.into()) {
            match crate::auth::gatekeeper::GateKeeperProvider::generate_id() {
                Ok(new_guid) => {
                    // Write GUID to a1
                    // SAFETY: a1 is a valid pointer to a GUID
                    unsafe { *(a1 as *mut GUID) = new_guid };
                    result = BOOL(1);
                    log::info!("Gatekeeper ID (Generated): {:?}", new_guid);
                }
                Err(e) => {
                    log::error!("Failed to create GUID: {:?}", e);
                    result = BOOL(0);
                }
            }
        } else {
            log::info!("Gatekeeper ID: {:?}", guid_val);
        }
    }

    result
}
