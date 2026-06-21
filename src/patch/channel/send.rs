use crate::patch::module_info::ModuleInfo;
use std::ffi::c_void;
use windows::Win32::System::Threading::CRITICAL_SECTION;
use windows::core::PCSTR;

type Sub37230EB3 = unsafe extern "thiscall" fn(
    this: *mut *mut i32,
    a2: *mut c_void,
    lp_critical_section: *mut CRITICAL_SECTION,
    lp_string: PCSTR,
    a5: PCSTR,
    a6: PCSTR,
    a7: PCSTR,
    a8: PCSTR,
    a9: i32,
    a10: i32,
    a11: i32,
) -> bool;

static mut TRAMPOLINE: Option<Sub37230EB3> = None;

/// # Safety
///
/// This function is unsafe because it resolves module pointers and installs hooks.
pub unsafe fn apply(info: &ModuleInfo) -> Result<(), String> {
    let target = info.resolve(0x37230eb3);
    let trampoline = unsafe { crate::patch::hook(target, hook_sub_37230eb3 as *mut c_void) }?;

    unsafe {
        TRAMPOLINE = Some(std::mem::transmute::<*mut c_void, Sub37230EB3>(trampoline));
    }
    Ok(())
}

#[unsafe(no_mangle)]
unsafe extern "thiscall" fn hook_sub_37230eb3(
    this: *mut *mut i32,
    a2: *mut c_void,
    lp_critical_section: *mut CRITICAL_SECTION,
    lp_string: PCSTR,
    a5: PCSTR,
    a6: PCSTR,
    a7: PCSTR,
    a8: PCSTR,
    a9: i32,
    a10: i32,
    a11: i32,
) -> bool {
    let p_lp = unsafe { pcstr_to_opt(lp_string) };
    let p_a5 = unsafe { pcstr_to_opt(a5) };
    let p_a6 = unsafe { pcstr_to_opt(a6) };
    let p_a7 = unsafe { pcstr_to_opt(a7) };
    let p_a8 = unsafe { pcstr_to_opt(a8) };

    let build_command = || -> Option<String> {
        let cmd = match a2 as usize {
            0 => {
                // ACCESS ADD
                let lp = p_lp?;
                let a5 = p_a5?;
                let a6 = p_a6?;
                let mut s = format!("ACCESS {} ADD {} {}", lp, a5, a6);
                if let Some(a7) = p_a7 {
                    s.push_str(&format!(" {}", a7));
                    if let Some(a8) = p_a8 {
                        s.push_str(&format!(" :{}", a8));
                    }
                }
                s
            }
            1 => {
                // ACCESS DELETE
                let lp = p_lp?;
                let a5 = p_a5?;
                let a6 = p_a6?;
                let mut s = format!("ACCESS {} DELETE {} {}", lp, a5, a6);
                if let Some(a7) = p_a7 {
                    s.push_str(&format!(" {}", a7));
                    if let Some(a8) = p_a8 {
                        s.push_str(&format!(" :{}", a8));
                    }
                }
                s
            }
            2 => format!("ACCESS {} CLEAR", p_lp?),
            3 => format!("ACCESS {} LIST", p_lp?),
            4 => {
                // AUTH
                let lp = p_lp?;
                let a5 = p_a5?;
                let mut s = format!("AUTH {} {}", lp, a5);
                if let Some(a6) = p_a6 {
                    s.push_str(&format!(" {}", a6));
                }
                s
            }
            5 => {
                // AWAY
                if let Some(lp) = p_lp {
                    format!("AWAY :{}", lp)
                } else {
                    "AWAY".to_string()
                }
            }
            6 => format!("DATA {} {} :{}", p_lp?, p_a5?, p_a6?),
            7 => {
                // EVENT ADD
                let lp = p_lp?;
                if let Some(a5) = p_a5 {
                    format!("EVENT ADD {} {}", lp, a5)
                } else {
                    format!("EVENT ADD {}", lp)
                }
            }
            8 => {
                // EVENT DELETE
                let lp = p_lp?;
                if let Some(a5) = p_a5 {
                    format!("EVENT DELETE {} {}", lp, a5)
                } else {
                    format!("EVENT DELETE {}", lp)
                }
            }
            9 => format!("EVENT LIST {}", p_lp?),
            10 => format!("EPRIVMSG {} :{}", p_lp?, p_a5?),
            11 => format!("EQUESTION {} {} {} :{}", p_lp?, p_a5?, p_a6?, p_a7?),
            12 => format!("ESUBMIT {} :{}", p_lp?, p_a5?),
            13 => format!("GOTO {} :{}", p_lp?, p_a5?),
            14 => "INFO".to_string(),
            15 => format!("INVITE {}", p_lp?),
            16 => {
                // IRCVERS
                let lp = p_lp?;
                let mut s = format!("IRCVERS {}", lp);
                if let Some(a5) = p_a5 {
                    s.push_str(&format!(" {}", a5));
                    if let Some(a6) = p_a6 {
                        s.push_str(&format!(" {}", a6));
                        if let Some(a7) = p_a7 {
                            s.push_str(&format!(" :{}", a7));
                        }
                    }
                }
                s
            }
            17 => {
                // JOIN
                let lp = p_lp?;
                if let Some(a5) = p_a5 {
                    format!("JOIN {} {}", lp, a5)
                } else {
                    format!("JOIN {}", lp)
                }
            }
            18 => {
                // KICK
                let lp = p_lp?;
                let a5 = p_a5?;
                if let Some(a6) = p_a6 {
                    format!("KICK {} {} :{}", lp, a5, a6)
                } else {
                    format!("KICK {} {}", lp, a5)
                }
            }
            19 => format!("KILL {} :{}", p_lp?, p_a5?),
            20 => "LINKS".to_string(),
            21 => format!("LIST {}", p_lp?),
            22 => format!("LISTX {}", p_lp?),
            23 => "LUSERS".to_string(),
            24 => format!("MESSAGE {} :{}", p_lp?, p_a5?),
            25 => {
                // MODE
                let lp = p_lp?;
                match (p_a5, p_a6) {
                    (Some(a5), Some(a6)) => format!("MODE {} {} {}", lp, a5, a6),
                    (Some(a5), None) => format!("MODE {} {}", lp, a5),
                    _ => format!("MODE {}", lp),
                }
            }
            26 => "MOTD".to_string(),
            27 => format!("NAMES {}", p_lp?),
            28 => format!("NICK {}", p_lp?),
            29 => format!("NOTICE {} :{}", p_lp?, p_a5?),
            30 => format!("OPER {} :{}", p_lp?, p_a5?),
            31 => format!("PART {} :{}", p_lp?, p_a5?),
            32 => format!("PASS {}", p_lp?),
            33 => format!("PING {}", p_lp?),
            34 => "PONG ".to_string(),
            35 => format!("PRIVMSG {} :{}", p_lp?, p_a5?),
            36 => {
                // PROP
                let lp = p_lp?;
                let a5 = p_a5?;
                let mut s = format!("PROP {} {}", lp, a5);
                if let Some(a6) = p_a6 {
                    s.push_str(&format!(" :{}", a6));
                }
                s
            }
            37 => {
                // QUIT
                if let Some(lp) = p_lp {
                    format!("QUIT :{}", lp)
                } else {
                    "QUIT".to_string()
                }
            }
            38 => format!("REPLY {} {} :{}", p_lp?, p_a5?, p_a6?),
            39 => format!("REQUEST {} {} :{}", p_lp?, p_a5?, p_a6?),
            40 => format!("SILENCE {}", p_lp?),
            41 => "TIME".to_string(),
            42 => {
                // TOPIC
                let lp = p_lp?;
                if let Some(a5) = p_a5 {
                    format!("TOPIC {} {}", lp, a5)
                } else {
                    format!("TOPIC {}", lp)
                }
            }
            43 => format!("USER {} {} {} :{}", p_lp?, p_a5?, p_a6?, p_a7?),
            44 => format!("USERHOST {}", p_lp?),
            45 => "VERSION".to_string(),
            46 => format!("WALLOPS {}", p_lp?),
            47 => format!("WALLUSERS {}", p_lp?),
            48 => format!("WHISPER {} {} :{}", p_lp?, p_a5?, p_a6?),
            49 => {
                // WHO
                let lp = p_lp?;
                if let Some(a5) = p_a5 {
                    format!("WHO {} {}", lp, a5)
                } else {
                    format!("WHO {}", lp)
                }
            }
            50 => {
                // WHOIS
                let lp = p_lp?;
                if let Some(a5) = p_a5 {
                    format!("WHOIS {} {}", lp, a5)
                } else {
                    format!("WHOIS {}", lp)
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
