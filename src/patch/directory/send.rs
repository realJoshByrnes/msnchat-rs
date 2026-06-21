use crate::module_info::ModuleInfo;
use std::ffi::c_void;
use windows::Win32::System::Threading::CRITICAL_SECTION;
use windows::core::PCSTR;

type Sub372321AE = unsafe extern "thiscall" fn(
    this: *mut *mut i32,
    a2: *mut c_void,
    lp_critical_section: *mut CRITICAL_SECTION,
    lp_string: PCSTR,
    a5: PCSTR,
    a6: PCSTR,
    a7: PCSTR,
    a8: PCSTR,
    a9: PCSTR,
    a10: PCSTR,
    a11: PCSTR,
    a12: PCSTR,
) -> bool;

static mut TRAMPOLINE: Option<Sub372321AE> = None;

/// # Safety
///
/// This function is unsafe because it resolves module pointers and installs hooks.
pub unsafe fn apply(info: &ModuleInfo) -> Result<(), String> {
    let target = info.resolve(0x372321ae);
    let trampoline = unsafe { crate::patch::hook(target, hook_sub_372321ae as *mut c_void) }?;

    unsafe {
        TRAMPOLINE = Some(std::mem::transmute::<*mut c_void, Sub372321AE>(trampoline));
    }
    Ok(())
}

#[unsafe(no_mangle)]
unsafe extern "thiscall" fn hook_sub_372321ae(
    this: *mut *mut i32,
    a2: *mut c_void,
    lp_critical_section: *mut CRITICAL_SECTION,
    lp_string: PCSTR,
    a5: PCSTR,
    a6: PCSTR,
    a7: PCSTR,
    a8: PCSTR,
    a9: PCSTR,
    a10: PCSTR,
    a11: PCSTR,
    a12: PCSTR,
) -> bool {
    let p_lp = unsafe { pcstr_to_opt(lp_string) };
    let p_a5 = unsafe { pcstr_to_opt(a5) };
    let p_a6 = unsafe { pcstr_to_opt(a6) };
    let p_a7 = unsafe { pcstr_to_opt(a7) };
    let p_a8 = unsafe { pcstr_to_opt(a8) };
    let p_a9 = unsafe { pcstr_to_opt(a9) };
    let p_a10 = unsafe { pcstr_to_opt(a10) };
    let p_a11 = unsafe { pcstr_to_opt(a11) };
    let p_a12 = unsafe { pcstr_to_opt(a12) };

    let build_command = || -> Option<String> {
        let cmd = match a2 as usize {
            0 => {
                // AUTH
                let lp = p_lp?;
                let a5 = p_a5?;
                if let Some(a6) = p_a6 {
                    format!("AUTH {} {} {}", lp, a5, a6)
                } else {
                    format!("AUTH {} {}", lp, a5)
                }
            }
            1 => {
                // CREATE
                let lp = p_lp?;
                let a5 = p_a5?;
                let a6 = p_a6?;
                let a7 = p_a7?;
                let a8 = p_a8?;
                let a9 = p_a9?;
                let a10 = p_a10?;
                let a11 = p_a11?;
                if let Some(a12) = p_a12 {
                    format!(
                        "CREATE {} {} {} {} {} {} {} {} {}",
                        lp, a5, a6, a7, a8, a9, a10, a11, a12
                    )
                } else {
                    format!(
                        "CREATE {} {} {} {} {} {} {} {}",
                        lp, a5, a6, a7, a8, a9, a10, a11
                    )
                }
            }
            2 => "CREDITS".to_string(),
            3 => format!("FINDS {}", p_lp?),
            4 => format!("FINDU {}", p_lp?),
            5 => {
                // IRCVERS
                let lp = p_lp?;
                if let Some(a5) = p_a5 {
                    format!("IRCVERS {} {}", lp, a5)
                } else {
                    format!("IRCVERS {}", lp)
                }
            }
            6 => "LINKSX".to_string(),
            7 => {
                // LIST
                let lp = p_lp?;
                if let Some(a5) = p_a5 {
                    format!("LIST {} {}", lp, a5)
                } else {
                    format!("LIST {}", lp)
                }
            }
            8 => "LISTC".to_string(),
            9 => format!("LISTR {}", p_lp?),
            10 => "LISTU".to_string(),
            11 => {
                // LISTX
                let lp = p_lp?;
                if let Some(a5) = p_a5 {
                    format!("LISTX {} {}", lp, a5)
                } else {
                    format!("LISTX {}", lp)
                }
            }
            12 => {
                // LISTZ
                let lp = p_lp?;
                if let Some(a5) = p_a5 {
                    format!("LISTZ {} {}", lp, a5)
                } else {
                    format!("LISTZ {}", lp)
                }
            }
            14 => format!("NICK {}", p_lp?),
            15 => format!("MOVE {} {}", p_lp?, p_a5?),
            16 => format!("PASS {}", p_lp?),
            17 => "STATS".to_string(),
            18 => "STATSD".to_string(),
            19 => "STATSG".to_string(),
            20 => "STATSGD".to_string(),
            21 => "UPTIME".to_string(),
            22 => format!("USER {} {} {} {}", p_lp?, p_a5?, p_a6?, p_a7?),
            23 => "VERSION".to_string(),
            24 => {
                // PROP
                let lp = p_lp?;
                let a5 = p_a5?;
                if let Some(a6) = p_a6 {
                    format!("PROP {} {} :{}", lp, a5, a6)
                } else {
                    format!("PROP {} {}", lp, a5)
                }
            }
            _ => return None,
        };
        Some(cmd)
    };

    if let Some(cmd_string) = build_command() {
        log::info!("{}", cmd_string);
    }

    if let Some(orig) = unsafe { TRAMPOLINE } {
        unsafe {
            orig(
                this,
                a2,
                lp_critical_section,
                lp_string,
                a5,
                a6,
                a7,
                a8,
                a9,
                a10,
                a11,
                a12,
            )
        }
    } else {
        false
    }
}

unsafe fn pcstr_to_opt<'a>(p: PCSTR) -> Option<&'a str> {
    if p.is_null() {
        None
    } else {
        unsafe {
            std::ffi::CStr::from_ptr(p.as_ptr() as *const std::ffi::c_char)
                .to_str()
                .ok()
        }
    }
}
