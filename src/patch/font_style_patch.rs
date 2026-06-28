use super::module_info::ModuleInfo;
use std::ffi::c_void;
use windows::Win32::System::Memory::{
    PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS, VirtualProtect,
};

/// # Safety
///
/// This function is unsafe because it modifies executable code in memory.
pub unsafe fn apply(info: &ModuleInfo) -> Result<(), String> {
    unsafe {
        // 1. RVA 0x17608: FontStyle mask when loading settings (originally 0x03).
        // Changing this to 0x07 enables the Underline style bit (bit 2, value 4) to be loaded.
        {
            let target = info.resolve(0x37217608) as *mut u8;
            let mut old_protect = PAGE_PROTECTION_FLAGS::default();
            VirtualProtect(
                target as *const c_void,
                1,
                PAGE_EXECUTE_READWRITE,
                &mut old_protect,
            )
            .map_err(|e| format!("VirtualProtect failed: {}", e))?;

            *target = 0x07;

            let mut temp = PAGE_PROTECTION_FLAGS::default();
            let _ = VirtualProtect(target as *const c_void, 1, old_protect, &mut temp);
            log::info!("Patched FontStyle settings mask to 7 at RVA 0x17608");
        }

        // 2. RVA 0x25a7e: CHARFORMAT2 dwMask in ChatEdit::sub_37225A29 (originally 0x03 for bold/italic).
        // Changing this to 0x07 (CFM_BOLD | CFM_ITALIC | CFM_UNDERLINE) tells the RichEdit control
        // to also apply/update the Underline effect.
        {
            let target = info.resolve(0x37225a7e) as *mut u8;
            let mut old_protect = PAGE_PROTECTION_FLAGS::default();
            VirtualProtect(
                target as *const c_void,
                1,
                PAGE_EXECUTE_READWRITE,
                &mut old_protect,
            )
            .map_err(|e| format!("VirtualProtect failed: {}", e))?;

            *target = 0x07;

            let mut temp = PAGE_PROTECTION_FLAGS::default();
            let _ = VirtualProtect(target as *const c_void, 1, old_protect, &mut temp);
            log::info!("Patched ChatEdit dwMask to 7 at RVA 0x25a7e");
        }
    }
    Ok(())
}
