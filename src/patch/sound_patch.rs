//! Sound patch and hook implementations for `MsnChat45.ocx`.
//!
//! Exposes:
//! - Hook for `PlaySoundA` in `winmm.dll` (simplified to stop audio only).
//! - Hook for the event-index based sound player `sub_3721D4D3`.

use crate::audio;
use std::ffi::c_void;
use windows::Win32::Foundation::HMODULE;
use windows::core::{BOOL, PCSTR};

type Sub3721D4D3Type = unsafe extern "cdecl" fn(file_part: *const u8) -> BOOL;

static mut TRAMPOLINE: Option<Sub3721D4D3Type> = None;

/// # Safety
///
/// This function is unsafe because it installs hooks.
pub unsafe fn apply(info: &super::module_info::ModuleInfo) -> Result<(), String> {
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    use windows::core::w;

    // 1. Hook sub_3721D4D3
    let target = info.resolve(0x3721d4d3);
    let trampoline = unsafe { super::hook(target, detour_sub_3721d4d3 as *mut c_void)? };

    // SAFETY: Single threaded initialization during DLL loader hook execution
    unsafe {
        TRAMPOLINE = Some(std::mem::transmute::<*mut c_void, Sub3721D4D3Type>(
            trampoline,
        ));
    }

    // 2. Hook PlaySoundA (stop audio only)
    let winmm = unsafe { LoadLibraryW(w!("winmm.dll")) }
        .map_err(|e| format!("Failed to load winmm.dll: {}", e))?;

    let playsound_ptr = unsafe {
        let proc = GetProcAddress(
            winmm,
            windows::core::PCSTR::from_raw(c"PlaySoundA".as_ptr() as *const u8),
        );
        if let Some(p) = proc {
            p as *mut c_void
        } else {
            return Err("GetProcAddress failed to find PlaySoundA in winmm.dll".to_string());
        }
    };

    // We do not need a trampoline for PlaySoundA since we fully detour/suppress it
    let _ = unsafe { super::hook(playsound_ptr, detour_playsound_a as *mut c_void)? };

    Ok(())
}

/// Detour for `PlaySoundA` in `winmm.dll`.
///
/// Since sound playback is intercepted at `sub_3721D4D3` and handled by our Rust audio player,
/// we handle `PlaySoundA` calls exclusively to stop the audio when `psz_sound` is NULL.
///
/// # Safety
///
/// This function is unsafe because it is called via a raw DLL function hook and handles raw pointers/system conventions.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn detour_playsound_a(
    _psz_sound: PCSTR,
    _h_mod: HMODULE,
    _fdw_sound: u32,
) -> BOOL {
    log::info!("PlaySoundA detour: stopping audio");
    audio::stop_sound();
    BOOL::from(true)
}

/// Detour for `sub_3721D4D3` in the OCX.
///
/// Intercepts sound play indices and maps them directly to the Relative Virtual Addresses (RVA)
/// of the WAV files embedded inside `MsnChat45.ocx`, then plays them via Rust.
///
/// # Safety
///
/// This function is unsafe because it treats a raw pointer parameter `file_part` as an integer index value and performs unsafe memory pointer operations.
#[unsafe(no_mangle)]
pub unsafe extern "cdecl" fn detour_sub_3721d4d3(file_part: *const u8) -> BOOL {
    let index = file_part as usize;
    log::info!("sub_3721D4D3 called with sound index: {}", index);

    // Map sound index directly to (RVA, size) of the WAV files embedded in the OCX image
    let sound_info = match index {
        // msnchat_Whisper -> ChatWhsp.wav (ID 350)
        0 => Some((0x6ad40, 9818)),
        // msnchat_Arrival -> ChatJoin.wav (ID 351)
        1 => Some((0x6d3a0, 4698)),
        // msnchat_Departure -> (Silent/None)
        2 => None,
        // msnchat_HostMessage -> (Silent/None)
        3 => None,
        // msnchat_TagMessage -> ChatTag.wav (ID 354)
        4 => Some((0x6e600, 7770)),
        // msnchat_HostWhisper -> ChatWhsp.wav (ID 350)
        5 => Some((0x6ad40, 9818)),
        // msnchat_TagWhisper -> ChatWhsp.wav (ID 350)
        6 => Some((0x6ad40, 9818)),
        // msnchat_Kick -> ChatKick.wav (ID 357)
        7 => Some((0x716c0, 5210)),
        // msnchat_Invitation -> ChatInvt.wav (ID 358)
        8 => Some((0x70460, 4698)),
        _ => None,
    };

    if let Some((rva, size)) = sound_info {
        log::info!(
            "Playing sound index {} mapping -> RVA: {:#x}, size: {}",
            index,
            rva,
            size
        );

        let h_module = unsafe { crate::patch::loader_hook::OCX_MODULE };
        if let Some(module) = h_module {
            let module_base = module.0 as *const u8;
            // SAFETY: Memory is within the loaded OCX image and is read-only static data
            let bytes = unsafe { std::slice::from_raw_parts(module_base.add(rva), size) }.to_vec();
            audio::play_sound(bytes);
        } else {
            log::error!("Cannot play sound: OCX_MODULE handle is not set");
        }
    } else {
        log::info!("Sound index {} is mapped to silent/none", index);
    }

    BOOL::from(true)
}
