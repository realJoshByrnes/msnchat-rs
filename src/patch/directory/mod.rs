pub mod recv;
pub mod send;

/// # Safety
///
/// This function is unsafe because it installs module hooks.
pub unsafe fn apply(info: &crate::module_info::ModuleInfo) -> Result<(), String> {
    unsafe {
        send::apply(info)?;
        recv::apply(info)?;
    }
    Ok(())
}
