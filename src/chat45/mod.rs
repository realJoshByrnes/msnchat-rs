pub mod controls;
pub mod registry;
pub mod sockets;
use crate::patch::module_info::ModuleInfo;

/// # Safety
/// Relies on accurate ModuleInfo mapping for the PE image.
pub unsafe fn apply_patches(info: &ModuleInfo) -> Result<(), String> {
    if let Err(e) = unsafe { controls::patch::apply(info) } {
        log::error!("Failed to apply controls patch: {}", e);
    }

    if let Err(e) = unsafe { registry::apply(info) } {
        log::error!("Failed to apply registry patch: {}", e);
    }

    if let Err(e) = unsafe { sockets::patch::apply(info) } {
        log::error!("Failed to apply sockets patch: {}", e);
    }

    // Future chat45 specific patches can be added here

    Ok(())
}
