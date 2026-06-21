// This patch fixes a bug in the MSN Chat control where the fn at 0x3720e0a5 would execute memory that was not executable.

use std::ffi::c_void;
use windows::{
    Win32::System::Memory::{PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS, VirtualProtect},
    core::BOOL,
};

use crate::module_info::ModuleInfo;

// BOOL __thiscall sub_3720E0A5(_DWORD *lpBaseAddress, int a2, int a3)
// {
//   HANDLE CurrentProcess; // eax
//
//   *(lpBaseAddress + 1) = a3;
//   *lpBaseAddress = 69485767;
//   *((_BYTE *)lpBaseAddress + 8) = -23;
//   *(_DWORD *)((char *)lpBaseAddress + 9) = a2 - (_DWORD)lpBaseAddress - 13;
//   CurrentProcess = GetCurrentProcess();
//   return FlushInstructionCache(CurrentProcess, lpBaseAddress, 0xDu);
// }

type Sub3720E0A5 =
    unsafe extern "thiscall" fn(lp_base_address: *mut c_void, a2: i32, a3: i32) -> BOOL;

static mut TRAMPOLINE: Option<Sub3720E0A5> = None;

/// # Safety
///
/// This function is unsafe because it installs hooks on module load.
pub unsafe fn apply(info: &ModuleInfo) -> Result<(), String> {
    let target = info.resolve(0x3720e0a5);
    let trampoline = unsafe { super::hook(target, fixed_3720e0a5 as *mut c_void) }?;

    // SAFETY: Single threaded init
    unsafe { TRAMPOLINE = Some(std::mem::transmute::<*mut c_void, Sub3720E0A5>(trampoline)) };
    Ok(())
}

#[unsafe(no_mangle)]
unsafe extern "thiscall" fn fixed_3720e0a5(lp_base_address: *mut c_void, a2: i32, a3: i32) -> BOOL {
    let mut old_protect = PAGE_PROTECTION_FLAGS::default();
    // Make the memory executable (13 bytes written by original fn)
    unsafe {
        let _ = VirtualProtect(
            lp_base_address,
            13,
            PAGE_EXECUTE_READWRITE,
            &mut old_protect,
        );
    }

    let result = if let Some(trampoline) = unsafe { TRAMPOLINE } {
        unsafe { trampoline(lp_base_address, a2, a3) }
    } else {
        BOOL(0)
    };

    // Restore original protection
    unsafe {
        let _ = VirtualProtect(lp_base_address, 13, old_protect, std::ptr::null_mut());
    }

    result
}
