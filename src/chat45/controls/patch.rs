use super::msnchatedit4;
use crate::patch::module_info::ModuleInfo;

/// Applies all control-specific lifecycle patches.
///
/// # Safety
/// Relies on accurately resolving offsets inside the `msnchat45.ocx` module.
pub unsafe fn apply(info: &ModuleInfo) -> Result<(), String> {
    log::info!("Patching Controls Lifecycle methods...");
    unsafe {
        msnchatedit4::hooks::apply(info)?;
    }
    Ok(())
}
