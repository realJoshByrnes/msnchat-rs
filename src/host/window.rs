use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            AppendMenuW, CB_SETITEMHEIGHT, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CallWindowProcW,
            CreateMenu, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DispatchMessageW,
            GWLP_WNDPROC, GetMessageW, MF_POPUP, MF_STRING, MSG, PostQuitMessage, RegisterClassW,
            SetMenu, TranslateMessage, WM_DESTROY, WM_SIZE, WNDCLASSW, WNDPROC,
            WS_OVERLAPPEDWINDOW, WS_VISIBLE,
        },
    },
    core::{GUID, Result, w},
};

use crate::host::OcxHost;

/// The 16 MSN Chat palette colors stored as COLORREF (0x00BBGGRR) for Win32 APIs.
const MSN_COLORS: [u32; 16] = [
    0x00000000, // 0  Black    #000000
    0x00FFFFFF, // 1  White    #FFFFFF
    0x00000080, // 2  Maroon   #800000
    0x00008000, // 3  Green    #008000
    0x00800000, // 4  Navy     #000080
    0x00008080, // 5  Olive    #808000
    0x00800080, // 6  Purple   #800080
    0x00808000, // 7  Teal     #008080
    0x00C0C0C0, // 8  Silver   #C0C0C0
    0x00808080, // 9  Gray     #808080
    0x000000FF, // 10 Red      #FF0000
    0x0000FF00, // 11 Lime     #00FF00
    0x00FF0000, // 12 Blue     #0000FF
    0x0000FFFF, // 13 Yellow   #FFFF00
    0x00FF00FF, // 14 Fuchsia  #FF00FF
    0x00FFFF00, // 15 Aqua     #00FFFF
];

unsafe fn send_message_w(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::SendMessageW(hwnd, msg, Some(wparam), Some(lparam))
    }
}

unsafe extern "system" fn enum_font_fam_ex_proc(
    lpelfe: *const windows::Win32::Graphics::Gdi::LOGFONTW,
    _lpntme: *const windows::Win32::Graphics::Gdi::TEXTMETRICW,
    fonttype: u32,
    lparam: LPARAM,
) -> i32 {
    unsafe {
        let list = &mut *(lparam.0 as *mut Vec<String>);
        let font_name = String::from_utf16_lossy(&(*lpelfe).lfFaceName);
        let font_name = font_name.trim_end_matches('\0').to_string();
        if (fonttype & windows::Win32::Graphics::Gdi::TRUETYPE_FONTTYPE) != 0
            && !font_name.is_empty()
            && !list.contains(&font_name)
            && !font_name.starts_with('@')
        {
            list.push(font_name);
        }
        1
    }
}

unsafe extern "system" fn rebar_wndproc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        let Ok(parent) = windows::Win32::UI::WindowsAndMessaging::GetParent(hwnd) else {
            return DefWindowProcW(hwnd, message, wparam, lparam);
        };
        let user_data = windows::Win32::UI::WindowsAndMessaging::GetWindowLongW(
            parent,
            windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
        );
        let mut old_wndproc: Option<WNDPROC> = None;
        if user_data != 0 {
            let this = &*(user_data as *const OcxWindow);
            old_wndproc = this.old_rebar_wndproc;
        }

        if message == windows::Win32::UI::WindowsAndMessaging::WM_SHOWWINDOW && user_data != 0 {
            let mut rc = RECT::default();
            let _ = windows::Win32::UI::WindowsAndMessaging::GetClientRect(parent, &mut rc);
            let lp = LPARAM(((rc.bottom as u32) << 16 | (rc.right as u32 & 0xFFFF)) as isize);
            let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                Some(parent),
                WM_SIZE,
                WPARAM(0),
                lp,
            );
        }

        if let Some(old) = old_wndproc {
            CallWindowProcW(old, hwnd, message, wparam, lparam)
        } else {
            DefWindowProcW(hwnd, message, wparam, lparam)
        }
    }
}

unsafe extern "system" fn ocx_wndproc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        let Ok(parent) = windows::Win32::UI::WindowsAndMessaging::GetParent(hwnd) else {
            return DefWindowProcW(hwnd, message, wparam, lparam);
        };
        let user_data = windows::Win32::UI::WindowsAndMessaging::GetWindowLongW(
            parent,
            windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
        );
        let mut old_wndproc: Option<WNDPROC> = None;
        if user_data != 0 {
            let this = &mut *(user_data as *mut OcxWindow);
            old_wndproc = this.old_ocx_wndproc;

            let wm_update_settings =
                windows::Win32::UI::WindowsAndMessaging::RegisterWindowMessageW(w!(
                    "WM_CHAT_UPDATESETTINGS"
                ));
            if message == wm_update_settings && wparam.0 != 2 {
                // Forward the setting update to the parent window, flagging it so parent doesn't post it back.
                let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                    Some(parent),
                    wm_update_settings,
                    WPARAM(1), // Flag: came from OCX subclass
                    lparam,
                );
            }
        }

        if let Some(old) = old_wndproc {
            CallWindowProcW(old, hwnd, message, wparam, lparam)
        } else {
            DefWindowProcW(hwnd, message, wparam, lparam)
        }
    }
}

pub struct OcxWindow {
    hwnd: HWND,
    host: Option<OcxHost>,
    parent: Option<HWND>,
    module: Option<std::sync::Arc<crate::patch::pe::ManualModule>>,
    rebar_hwnd: Option<HWND>,
    toolbar_hwnd: Option<HWND>,
    cb_font: Option<HWND>,
    cb_charset: Option<HWND>,
    btn_color: Option<HWND>,
    btn_bold: Option<HWND>,
    btn_italic: Option<HWND>,
    btn_underline: Option<HWND>,
    hfont_bold: Option<windows::Win32::Graphics::Gdi::HFONT>,
    hfont_italic: Option<windows::Win32::Graphics::Gdi::HFONT>,
    hfont_underline: Option<windows::Win32::Graphics::Gdi::HFONT>,
    hfont_normal: Option<windows::Win32::Graphics::Gdi::HFONT>,
    old_rebar_wndproc: Option<WNDPROC>,
    old_ocx_wndproc: Option<WNDPROC>,
}

impl OcxWindow {
    pub fn new() -> Result<Self> {
        let instance = unsafe { GetModuleHandleW(None)? };

        // Define window class
        let class_name = w!("MsnChatOcxHostClass");
        let wc = WNDCLASSW {
            hCursor: unsafe {
                windows::Win32::UI::WindowsAndMessaging::LoadCursorW(
                    None,
                    windows::Win32::UI::WindowsAndMessaging::IDC_ARROW,
                )?
            },
            hInstance: instance.into(),
            lpszClassName: class_name,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(Self::wndproc),
            ..Default::default()
        };

        // Register class
        let _atom = unsafe { RegisterClassW(&wc) };

        // Create window
        let hwnd = unsafe {
            CreateWindowExW(
                windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE::default(),
                class_name,
                w!("MsnChat OCX Host"),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                800,
                600,
                None,
                None,
                Some(instance.into()),
                None,
            )?
        };

        // Create menu
        unsafe {
            let hmenu = CreateMenu()?;
            let hsubmenu = CreatePopupMenu()?;

            AppendMenuW(hsubmenu, MF_STRING, 1001, w!("&Options"))?;
            AppendMenuW(hsubmenu, MF_STRING, 1002, w!("E&xit"))?;

            AppendMenuW(hmenu, MF_POPUP, hsubmenu.0 as usize, w!("&File"))?;
            SetMenu(hwnd, Some(hmenu))?;
        }

        // Ensure common controls are initialized (needed for Rebar)
        unsafe {
            let icc = windows::Win32::UI::Controls::INITCOMMONCONTROLSEX {
                dwSize: std::mem::size_of::<windows::Win32::UI::Controls::INITCOMMONCONTROLSEX>()
                    as u32,
                dwICC: windows::Win32::UI::Controls::ICC_COOL_CLASSES
                    | windows::Win32::UI::Controls::ICC_BAR_CLASSES,
            };
            let _ = windows::Win32::UI::Controls::InitCommonControlsEx(&icc);
        }

        // Create child controls for toolbar, hosted inside a rebar
        let (
            rebar_hwnd,
            toolbar_hwnd,
            cb_font,
            cb_charset,
            btn_color,
            btn_bold,
            btn_italic,
            btn_underline,
            hfont_bold,
            hfont_italic,
            hfont_underline,
            hfont_normal,
            old_rebar_wndproc,
        ) = unsafe {
            // Create the rebar control
            let rebar = CreateWindowExW(
                windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE::default(),
                w!("ReBarWindow32"),
                None,
                windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(
                    windows::Win32::UI::WindowsAndMessaging::WS_CHILD.0
                        | windows::Win32::UI::WindowsAndMessaging::WS_VISIBLE.0
                        | windows::Win32::UI::WindowsAndMessaging::WS_CLIPSIBLINGS.0
                        | windows::Win32::UI::WindowsAndMessaging::WS_CLIPCHILDREN.0
                        | windows::Win32::UI::Controls::CCS_NODIVIDER as u32
                        | windows::Win32::UI::Controls::RBS_VARHEIGHT
                        | windows::Win32::UI::Controls::RBS_BANDBORDERS,
                ),
                0,
                0,
                0,
                0,
                Some(hwnd),
                None,
                Some(instance.into()),
                None,
            )?;

            // Create a toolbar to host the font controls
            let toolbar = CreateWindowExW(
                windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE::default(),
                w!("ToolbarWindow32"),
                None,
                windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(
                    windows::Win32::UI::WindowsAndMessaging::WS_CHILD.0
                        | windows::Win32::UI::WindowsAndMessaging::WS_VISIBLE.0
                        | windows::Win32::UI::Controls::CCS_NODIVIDER as u32
                        | windows::Win32::UI::Controls::TBSTYLE_FLAT,
                ),
                0,
                0,
                0,
                0,
                Some(hwnd),
                None,
                Some(instance.into()),
                None,
            )?;
            let _ = send_message_w(
                toolbar,
                windows::Win32::UI::Controls::TB_BUTTONSTRUCTSIZE,
                WPARAM(std::mem::size_of::<windows::Win32::UI::Controls::TBBUTTON>() as usize),
                LPARAM(0),
            );

            let cb_font = CreateWindowExW(
                windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE::default(),
                w!("COMBOBOX"),
                None,
                windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(
                    windows::Win32::UI::WindowsAndMessaging::WS_CHILD.0
                        | windows::Win32::UI::WindowsAndMessaging::WS_VISIBLE.0
                        | windows::Win32::UI::WindowsAndMessaging::CBS_DROPDOWNLIST as u32
                        | windows::Win32::UI::WindowsAndMessaging::CBS_OWNERDRAWFIXED as u32
                        | windows::Win32::UI::WindowsAndMessaging::CBS_HASSTRINGS as u32
                        | windows::Win32::UI::WindowsAndMessaging::WS_VSCROLL.0
                        | windows::Win32::UI::WindowsAndMessaging::WS_TABSTOP.0,
                ),
                5,
                2,
                150,
                200, // Positioned on toolbar
                Some(toolbar),
                Some(windows::Win32::UI::WindowsAndMessaging::HMENU(
                    2001 as *mut std::ffi::c_void,
                )),
                Some(instance.into()),
                None,
            )?;

            let cb_charset = CreateWindowExW(
                windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE::default(),
                w!("COMBOBOX"),
                None,
                windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(
                    windows::Win32::UI::WindowsAndMessaging::WS_CHILD.0
                        | windows::Win32::UI::WindowsAndMessaging::WS_VISIBLE.0
                        | windows::Win32::UI::WindowsAndMessaging::CBS_DROPDOWNLIST as u32
                        | windows::Win32::UI::WindowsAndMessaging::WS_TABSTOP.0,
                ),
                160,
                2,
                120,
                200, // Positioned on toolbar
                Some(toolbar),
                Some(windows::Win32::UI::WindowsAndMessaging::HMENU(
                    2002 as *mut std::ffi::c_void,
                )),
                Some(instance.into()),
                None,
            )?;

            // Set height of selection fields to match the buttons (24px)
            // (cb_font is owner-draw and gets measured to 24px automatically in WM_MEASUREITEM)
            let _ = send_message_w(cb_charset, CB_SETITEMHEIGHT, WPARAM(usize::MAX), LPARAM(24));

            let btn_color = CreateWindowExW(
                windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE::default(),
                w!("BUTTON"),
                None,
                windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(
                    windows::Win32::UI::WindowsAndMessaging::WS_CHILD.0
                        | windows::Win32::UI::WindowsAndMessaging::WS_VISIBLE.0
                        | windows::Win32::UI::WindowsAndMessaging::BS_OWNERDRAW as u32,
                ),
                285,
                2,
                26,
                24, // Positioned on toolbar (24px height)
                Some(toolbar),
                Some(windows::Win32::UI::WindowsAndMessaging::HMENU(
                    2005 as *mut std::ffi::c_void,
                )),
                Some(instance.into()),
                None,
            )?;

            let btn_bold = CreateWindowExW(
                windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE::default(),
                w!("BUTTON"),
                w!("B"),
                windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(
                    windows::Win32::UI::WindowsAndMessaging::WS_CHILD.0
                        | windows::Win32::UI::WindowsAndMessaging::WS_VISIBLE.0
                        | windows::Win32::UI::WindowsAndMessaging::BS_AUTOCHECKBOX as u32
                        | windows::Win32::UI::WindowsAndMessaging::BS_PUSHLIKE as u32,
                ),
                316,
                2,
                26,
                24, // Positioned on toolbar (24px height)
                Some(toolbar),
                Some(windows::Win32::UI::WindowsAndMessaging::HMENU(
                    2003 as *mut std::ffi::c_void,
                )),
                Some(instance.into()),
                None,
            )?;

            let btn_italic = CreateWindowExW(
                windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE::default(),
                w!("BUTTON"),
                w!("I"),
                windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(
                    windows::Win32::UI::WindowsAndMessaging::WS_CHILD.0
                        | windows::Win32::UI::WindowsAndMessaging::WS_VISIBLE.0
                        | windows::Win32::UI::WindowsAndMessaging::BS_AUTOCHECKBOX as u32
                        | windows::Win32::UI::WindowsAndMessaging::BS_PUSHLIKE as u32,
                ),
                347,
                2,
                26,
                24, // Positioned on toolbar (24px height)
                Some(toolbar),
                Some(windows::Win32::UI::WindowsAndMessaging::HMENU(
                    2004 as *mut std::ffi::c_void,
                )),
                Some(instance.into()),
                None,
            )?;

            let btn_underline = CreateWindowExW(
                windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE::default(),
                w!("BUTTON"),
                w!("U"),
                windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(
                    windows::Win32::UI::WindowsAndMessaging::WS_CHILD.0
                        | windows::Win32::UI::WindowsAndMessaging::WS_VISIBLE.0
                        | windows::Win32::UI::WindowsAndMessaging::BS_AUTOCHECKBOX as u32
                        | windows::Win32::UI::WindowsAndMessaging::BS_PUSHLIKE as u32,
                ),
                378,
                2,
                26,
                24, // Positioned on toolbar (24px height)
                Some(toolbar),
                Some(windows::Win32::UI::WindowsAndMessaging::HMENU(
                    2006 as *mut std::ffi::c_void,
                )),
                Some(instance.into()),
                None,
            )?;

            // Create bold, italic, underline, and normal fonts with height -14 to match the font dropdown
            let gui_font = windows::Win32::Graphics::Gdi::GetStockObject(
                windows::Win32::Graphics::Gdi::DEFAULT_GUI_FONT,
            );
            let mut lf: windows::Win32::Graphics::Gdi::LOGFONTW = std::mem::zeroed();
            windows::Win32::Graphics::Gdi::GetObjectW(
                gui_font,
                std::mem::size_of::<windows::Win32::Graphics::Gdi::LOGFONTW>() as i32,
                Some(&mut lf as *mut _ as *mut std::ffi::c_void),
            );

            lf.lfHeight = -14;
            let font_name = w!("Tahoma");
            let copy_len = font_name.len().min(lf.lfFaceName.len() - 1);
            lf.lfFaceName[..copy_len].copy_from_slice(font_name.as_wide());
            lf.lfFaceName[copy_len] = 0;

            let hfont_normal = windows::Win32::Graphics::Gdi::CreateFontIndirectW(&lf);

            let mut lf_bold = lf;
            lf_bold.lfWeight = windows::Win32::Graphics::Gdi::FW_BOLD.0 as i32;
            let hfont_bold = windows::Win32::Graphics::Gdi::CreateFontIndirectW(&lf_bold);

            let mut lf_italic = lf;
            lf_italic.lfItalic = 1; // TRUE
            let hfont_italic = windows::Win32::Graphics::Gdi::CreateFontIndirectW(&lf_italic);

            let mut lf_underline = lf;
            lf_underline.lfUnderline = 1; // TRUE
            let hfont_underline = windows::Win32::Graphics::Gdi::CreateFontIndirectW(&lf_underline);

            let _ = send_message_w(
                cb_font,
                windows::Win32::UI::WindowsAndMessaging::WM_SETFONT,
                WPARAM(hfont_normal.0 as usize),
                LPARAM(0),
            );
            let _ = send_message_w(
                cb_charset,
                windows::Win32::UI::WindowsAndMessaging::WM_SETFONT,
                WPARAM(hfont_normal.0 as usize),
                LPARAM(0),
            );

            let _ = send_message_w(
                btn_bold,
                windows::Win32::UI::WindowsAndMessaging::WM_SETFONT,
                WPARAM(hfont_bold.0 as usize),
                LPARAM(1), // Redraw
            );
            let _ = send_message_w(
                btn_italic,
                windows::Win32::UI::WindowsAndMessaging::WM_SETFONT,
                WPARAM(hfont_italic.0 as usize),
                LPARAM(1), // Redraw
            );
            let _ = send_message_w(
                btn_underline,
                windows::Win32::UI::WindowsAndMessaging::WM_SETFONT,
                WPARAM(hfont_underline.0 as usize),
                LPARAM(1), // Redraw
            );

            // Populate Fonts from OS
            let mut font_names: Vec<String> = Vec::new();
            let hdc = windows::Win32::Graphics::Gdi::GetDC(None);
            let lf = windows::Win32::Graphics::Gdi::LOGFONTW {
                lfCharSet: windows::Win32::Graphics::Gdi::DEFAULT_CHARSET,
                ..Default::default()
            };
            let _ = windows::Win32::Graphics::Gdi::EnumFontFamiliesExW(
                hdc,
                &lf,
                Some(enum_font_fam_ex_proc),
                LPARAM(&mut font_names as *mut _ as isize),
                0,
            );
            windows::Win32::Graphics::Gdi::ReleaseDC(None, hdc);
            font_names.sort();

            for name in font_names {
                let f_wstr = windows::core::HSTRING::from(&name);
                let _ = send_message_w(
                    cb_font,
                    windows::Win32::UI::WindowsAndMessaging::CB_ADDSTRING,
                    WPARAM(0),
                    LPARAM(f_wstr.as_ptr() as isize),
                );
            }

            // Populate Charsets
            let charsets = [
                (w!("Western"), 0),
                (w!("Default"), 1),
                (w!("Symbol"), 2),
                (w!("ShiftJIS"), 128),
                (w!("Hangul"), 129),
                (w!("GB2312"), 134),
                (w!("Big5"), 136),
                (w!("Greek"), 161),
                (w!("Turkish"), 162),
                (w!("Hebrew"), 177),
                (w!("Arabic"), 178),
                (w!("Baltic"), 186),
                (w!("Russian"), 204),
                (w!("Thai"), 222),
                (w!("Eastern Europe"), 238),
                (w!("OEM"), 255),
            ];
            for (name, val) in &charsets {
                let idx = send_message_w(
                    cb_charset,
                    windows::Win32::UI::WindowsAndMessaging::CB_ADDSTRING,
                    WPARAM(0),
                    LPARAM(name.as_ptr() as isize),
                )
                .0 as usize;
                let _ = send_message_w(
                    cb_charset,
                    windows::Win32::UI::WindowsAndMessaging::CB_SETITEMDATA,
                    WPARAM(idx),
                    LPARAM(*val as isize),
                );
            }

            // (Color is now an owner-draw button, no population needed)

            // Load from config
            let manager = crate::config::MSNConfigManager::new(std::path::Path::new("config.toml"));
            let config = manager.load().unwrap_or_default();

            // Fontname config format: "<fontfamily>;<charset>"
            let full_font_name = config
                .settings
                .fontname
                .clone()
                .unwrap_or_else(|| "Tahoma;0".to_string());
            let parts: Vec<&str> = full_font_name.split(';').collect();
            let font_family = parts.first().copied().unwrap_or("Tahoma");
            let charset_val = parts
                .get(1)
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0);

            let font_wstr = windows::core::HSTRING::from(font_family);
            let idx = send_message_w(
                cb_font,
                windows::Win32::UI::WindowsAndMessaging::CB_FINDSTRINGEXACT,
                WPARAM(usize::MAX),
                LPARAM(font_wstr.as_ptr() as isize),
            )
            .0 as i32;
            if idx >= 0 {
                let _ = send_message_w(
                    cb_font,
                    windows::Win32::UI::WindowsAndMessaging::CB_SETCURSEL,
                    WPARAM(idx as usize),
                    LPARAM(0),
                );
                let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(cb_font), None, true);
            }

            let mut charset_idx = 0;
            loop {
                let val = send_message_w(
                    cb_charset,
                    windows::Win32::UI::WindowsAndMessaging::CB_GETITEMDATA,
                    WPARAM(charset_idx),
                    LPARAM(0),
                )
                .0 as i32;
                if val == -1 {
                    break;
                }
                if val == charset_val as i32 {
                    let _ = send_message_w(
                        cb_charset,
                        windows::Win32::UI::WindowsAndMessaging::CB_SETCURSEL,
                        WPARAM(charset_idx),
                        LPARAM(0),
                    );
                    break;
                }
                charset_idx += 1;
            }

            let fontstyle = config.settings.fontstyle.unwrap_or(0);
            let bold_checked = (fontstyle & 1) != 0;
            let italic_checked = (fontstyle & 2) != 0;
            let underline_checked = (fontstyle & 4) != 0;

            let _ = send_message_w(
                btn_bold,
                windows::Win32::UI::WindowsAndMessaging::BM_SETCHECK,
                WPARAM(if bold_checked { 1 } else { 0 }),
                LPARAM(0),
            );
            let _ = send_message_w(
                btn_italic,
                windows::Win32::UI::WindowsAndMessaging::BM_SETCHECK,
                WPARAM(if italic_checked { 1 } else { 0 }),
                LPARAM(0),
            );
            let _ = send_message_w(
                btn_underline,
                windows::Win32::UI::WindowsAndMessaging::BM_SETCHECK,
                WPARAM(if underline_checked { 1 } else { 0 }),
                LPARAM(0),
            );

            // Invalidate the color button so it draws the initial color
            let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(btn_color), None, true);

            // Insert the toolbar as a single band into the rebar
            let rbbi = windows::Win32::UI::Controls::REBARBANDINFOW {
                cbSize: std::mem::size_of::<windows::Win32::UI::Controls::REBARBANDINFOW>() as u32,
                fMask: windows::Win32::UI::Controls::RBBIM_STYLE
                    | windows::Win32::UI::Controls::RBBIM_CHILD
                    | windows::Win32::UI::Controls::RBBIM_CHILDSIZE
                    | windows::Win32::UI::Controls::RBBIM_SIZE,
                fStyle: windows::Win32::UI::Controls::RBBS_CHILDEDGE,
                hwndChild: toolbar,
                cxMinChild: 430, // Approx width of all controls
                cyMinChild: 28,  // Approx height of controls (height 24 + padding)
                cx: 430,
                ..Default::default()
            };
            let _ = send_message_w(
                rebar,
                windows::Win32::UI::Controls::RB_INSERTBANDW,
                WPARAM(u32::MAX as usize),
                LPARAM(&rbbi as *const _ as isize),
            );

            // Force the rebar to lay itself out to the parent's client width
            {
                let mut rc = RECT::default();
                let _ = windows::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut rc);
                let lp = LPARAM(((rc.bottom as u32) << 16 | (rc.right as u32 & 0xFFFF)) as isize);
                let _ = send_message_w(rebar, WM_SIZE, WPARAM(0), lp);
            }

            // Subclass the rebar
            let old_wndproc_val =
                windows::Win32::UI::WindowsAndMessaging::GetWindowLongW(rebar, GWLP_WNDPROC);
            let old_rebar_wndproc: WNDPROC = std::mem::transmute(old_wndproc_val as isize);
            windows::Win32::UI::WindowsAndMessaging::SetWindowLongW(
                rebar,
                GWLP_WNDPROC,
                rebar_wndproc as *const () as isize as i32,
            );

            (
                Some(rebar),
                Some(toolbar),
                Some(cb_font),
                Some(cb_charset),
                Some(btn_color),
                Some(btn_bold),
                Some(btn_italic),
                Some(btn_underline),
                hfont_bold,
                hfont_italic,
                hfont_underline,
                hfont_normal,
                Some(old_rebar_wndproc),
            )
        };

        Ok(Self {
            hwnd,
            host: None,
            parent: None,
            module: None,
            rebar_hwnd,
            toolbar_hwnd,
            cb_font,
            cb_charset,
            btn_color,
            btn_bold,
            btn_italic,
            btn_underline,
            hfont_bold: Some(hfont_bold),
            hfont_italic: Some(hfont_italic),
            hfont_underline: Some(hfont_underline),
            hfont_normal: Some(hfont_normal),
            old_rebar_wndproc,
            old_ocx_wndproc: None,
        })
    }

    pub fn attach_ocx<F>(
        &mut self,
        module: std::sync::Arc<crate::patch::pe::ManualModule>,
        clsid: &GUID,
        setup: F,
    ) -> Result<()>
    where
        F: FnOnce(&mut OcxHost),
    {
        self.module = Some(module.clone());
        let mut host = OcxHost::new(module, clsid)?;
        setup(&mut host);

        let mut rect = RECT::default();
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::GetClientRect(self.hwnd, &mut rect);
        }
        if self.parent.is_none() {
            let rebar_h = self
                .rebar_hwnd
                .filter(|&r| unsafe {
                    windows::Win32::UI::WindowsAndMessaging::IsWindowVisible(r).as_bool()
                })
                .map(|r| {
                    let h = unsafe {
                        send_message_w(
                            r,
                            windows::Win32::UI::Controls::RB_GETBARHEIGHT,
                            WPARAM(0),
                            LPARAM(0),
                        )
                    }
                    .0 as i32;
                    if h == 0 { 32 } else { h }
                })
                .unwrap_or(0);
            rect.top = rebar_h;
        }

        host.attach(self.hwnd, &rect)?;
        self.host = Some(host);
        self.old_ocx_wndproc = None;

        // Subclass the OCX control window to monitor settings changes
        if let Some(host) = &self.host
            && let Ok(ocx_hwnd) = host.get_control_hwnd()
        {
            unsafe {
                let old_wndproc_val =
                    windows::Win32::UI::WindowsAndMessaging::GetWindowLongW(ocx_hwnd, GWLP_WNDPROC);
                let old_ocx_wndproc: WNDPROC = std::mem::transmute(old_wndproc_val as isize);
                windows::Win32::UI::WindowsAndMessaging::SetWindowLongW(
                    ocx_hwnd,
                    GWLP_WNDPROC,
                    ocx_wndproc as *const () as isize as i32,
                );
                self.old_ocx_wndproc = Some(old_ocx_wndproc);
            }
        }

        // Set control window position explicitly just in case
        if self.parent.is_none() {
            if let Some(host) = &self.host
                && let Ok(ocx_hwnd) = host.get_control_hwnd()
            {
                unsafe {
                    let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowPos(
                        ocx_hwnd,
                        None,
                        rect.left,
                        rect.top,
                        rect.right - rect.left,
                        rect.bottom - rect.top,
                        windows::Win32::UI::WindowsAndMessaging::SWP_NOZORDER
                            | windows::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE,
                    );
                }
            }

            // Bring rebar above the OCX in z-order so it isn't covered
            if let Some(rebar) = self.rebar_hwnd {
                unsafe {
                    let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowPos(
                        rebar,
                        Some(windows::Win32::UI::WindowsAndMessaging::HWND_TOP),
                        0,
                        0,
                        0,
                        0,
                        windows::Win32::UI::WindowsAndMessaging::SWP_NOMOVE
                            | windows::Win32::UI::WindowsAndMessaging::SWP_NOSIZE
                            | windows::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE,
                    );
                }
            }
        }

        // Store self pointer in window user data for the wndproc
        unsafe {
            // we use the _ convention to cast safely for 32 bit pointers
            windows::Win32::UI::WindowsAndMessaging::SetWindowLongW(
                self.hwnd,
                windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
                (self as *const _ as isize) as i32,
            );
        }

        Ok(())
    }

    pub fn host(&self) -> Option<&OcxHost> {
        self.host.as_ref()
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    pub fn run_message_loop() -> Result<()> {
        unsafe {
            let mut message = MSG::default();
            while GetMessageW(&mut message, None, 0, 0).into() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        Ok(())
    }

    pub fn show_settings_modal(
        parent: HWND,
        module: std::sync::Arc<crate::patch::pe::ManualModule>,
    ) -> Result<()> {
        unsafe {
            let clsid_settings = GUID::from_values(
                0xFA980E7E,
                0x9E44,
                0x4D2F,
                [0xB3, 0xC2, 0x9A, 0x5B, 0xE4, 0x25, 0x25, 0xF8],
            );

            // 1. Create host first to get preferred extent size
            let mut host = OcxHost::new(module.clone(), &clsid_settings)?;
            let _ = host.put_property("BackColor", "16777215"); // White background

            // 2. Query preferred client size from control's extent
            let mut client_width = 400;
            let mut client_height = 350;
            if let Ok(size) = host.get_extent() {
                let hdc = windows::Win32::Graphics::Gdi::GetDC(None);
                let dpi_x = windows::Win32::Graphics::Gdi::GetDeviceCaps(
                    Some(hdc),
                    windows::Win32::Graphics::Gdi::LOGPIXELSX,
                );
                let dpi_y = windows::Win32::Graphics::Gdi::GetDeviceCaps(
                    Some(hdc),
                    windows::Win32::Graphics::Gdi::LOGPIXELSY,
                );
                let _ = windows::Win32::Graphics::Gdi::ReleaseDC(None, hdc);
                client_width = (size.cx * dpi_x) / 2540;
                client_height = (size.cy * dpi_y) / 2540;
            }

            // 3. Adjust window size to fit the client area perfectly
            let mut rect = RECT {
                left: 0,
                top: 0,
                right: client_width,
                bottom: client_height,
            };
            let _ = windows::Win32::UI::WindowsAndMessaging::AdjustWindowRectEx(
                &mut rect,
                windows::Win32::UI::WindowsAndMessaging::WS_POPUPWINDOW
                    | windows::Win32::UI::WindowsAndMessaging::WS_CAPTION,
                false,
                windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE::default(),
            );
            let win_width = rect.right - rect.left;
            let win_height = rect.bottom - rect.top;

            // 4. Disable parent window to make this modal
            let _ = windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow(parent, false);

            let instance = GetModuleHandleW(None)?;
            let class_name = w!("MsnChatOcxHostClass"); // Use same class

            // Create settings window
            let hwnd = CreateWindowExW(
                windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE::default(),
                class_name,
                w!("Chat Settings"),
                windows::Win32::UI::WindowsAndMessaging::WS_POPUPWINDOW
                    | windows::Win32::UI::WindowsAndMessaging::WS_CAPTION
                    | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                win_width,
                win_height,
                Some(parent),
                None,
                Some(instance.into()),
                None,
            )?;

            // Center settings window relative to parent
            let mut parent_rect = RECT::default();
            let mut child_rect = RECT::default();
            let _ =
                windows::Win32::UI::WindowsAndMessaging::GetWindowRect(parent, &mut parent_rect);
            let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut child_rect);
            let parent_width = parent_rect.right - parent_rect.left;
            let parent_height = parent_rect.bottom - parent_rect.top;
            let child_width = child_rect.right - child_rect.left;
            let child_height = child_rect.bottom - child_rect.top;
            let x = parent_rect.left + (parent_width - child_width) / 2;
            let y = parent_rect.top + (parent_height - child_height) / 2;
            let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowPos(
                hwnd,
                Some(HWND(std::ptr::null_mut())),
                x,
                y,
                0,
                0,
                windows::Win32::UI::WindowsAndMessaging::SWP_NOSIZE
                    | windows::Win32::UI::WindowsAndMessaging::SWP_NOZORDER,
            );

            // 5. Attach the pre-created host to the window
            let mut client_rect = RECT::default();
            let _ = windows::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut client_rect);
            host.attach(hwnd, &client_rect)?;

            let settings_win = Box::new(OcxWindow {
                hwnd,
                host: Some(host),
                parent: Some(parent),
                module: Some(module),
                rebar_hwnd: None,
                toolbar_hwnd: None,
                cb_font: None,
                cb_charset: None,
                btn_color: None,
                btn_bold: None,
                btn_italic: None,
                btn_underline: None,
                hfont_bold: None,
                hfont_italic: None,
                hfont_underline: None,
                hfont_normal: None,
                old_rebar_wndproc: None,
                old_ocx_wndproc: None,
            });

            // Leak the box pointer into GWLP_USERDATA
            windows::Win32::UI::WindowsAndMessaging::SetWindowLongW(
                hwnd,
                windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
                (Box::into_raw(settings_win) as isize) as i32,
            );
        }
        Ok(())
    }

    extern "system" fn wndproc(
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        unsafe {
            let user_data = windows::Win32::UI::WindowsAndMessaging::GetWindowLongW(
                window,
                windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
            );

            if user_data != 0 {
                let this = &mut *(user_data as *mut Self);
                let wm_update_settings =
                    windows::Win32::UI::WindowsAndMessaging::RegisterWindowMessageW(w!(
                        "WM_CHAT_UPDATESETTINGS"
                    ));
                if message == wm_update_settings {
                    let manager =
                        crate::config::MSNConfigManager::new(std::path::Path::new("config.toml"));
                    if let Ok(config) = manager.load() {
                        // 1. Update font combo
                        let full_font_name = config
                            .settings
                            .fontname
                            .clone()
                            .unwrap_or_else(|| "Tahoma;0".to_string());
                        let parts: Vec<&str> = full_font_name.split(';').collect();
                        let font_family = parts.first().copied().unwrap_or("Tahoma");
                        let charset_val = parts
                            .get(1)
                            .and_then(|s| s.parse::<u32>().ok())
                            .unwrap_or(0);

                        if let Some(cb_font) = this.cb_font {
                            let font_wstr = windows::core::HSTRING::from(font_family);
                            let idx = send_message_w(
                                cb_font,
                                windows::Win32::UI::WindowsAndMessaging::CB_FINDSTRINGEXACT,
                                WPARAM(usize::MAX),
                                LPARAM(font_wstr.as_ptr() as isize),
                            )
                            .0 as i32;
                            if idx >= 0 {
                                let _ = send_message_w(
                                    cb_font,
                                    windows::Win32::UI::WindowsAndMessaging::CB_SETCURSEL,
                                    WPARAM(idx as usize),
                                    LPARAM(0),
                                );
                                let _ = windows::Win32::Graphics::Gdi::InvalidateRect(
                                    Some(cb_font),
                                    None,
                                    true,
                                );
                            }
                        }

                        // 2. Update charset combo
                        if let Some(cb_charset) = this.cb_charset {
                            let mut charset_idx = 0;
                            loop {
                                let val = send_message_w(
                                    cb_charset,
                                    windows::Win32::UI::WindowsAndMessaging::CB_GETITEMDATA,
                                    WPARAM(charset_idx),
                                    LPARAM(0),
                                )
                                .0 as i32;
                                if val == -1 {
                                    break;
                                }
                                if val == charset_val as i32 {
                                    let _ = send_message_w(
                                        cb_charset,
                                        windows::Win32::UI::WindowsAndMessaging::CB_SETCURSEL,
                                        WPARAM(charset_idx),
                                        LPARAM(0),
                                    );
                                    break;
                                }
                                charset_idx += 1;
                            }
                        }

                        // 3. Update style buttons
                        let fontstyle = config.settings.fontstyle.unwrap_or(0);
                        let bold_checked = (fontstyle & 1) != 0;
                        let italic_checked = (fontstyle & 2) != 0;
                        let underline_checked = (fontstyle & 4) != 0;

                        if let Some(btn_bold) = this.btn_bold {
                            let _ = send_message_w(
                                btn_bold,
                                windows::Win32::UI::WindowsAndMessaging::BM_SETCHECK,
                                WPARAM(if bold_checked { 1 } else { 0 }),
                                LPARAM(0),
                            );
                        }
                        if let Some(btn_italic) = this.btn_italic {
                            let _ = send_message_w(
                                btn_italic,
                                windows::Win32::UI::WindowsAndMessaging::BM_SETCHECK,
                                WPARAM(if italic_checked { 1 } else { 0 }),
                                LPARAM(0),
                            );
                        }
                        if let Some(btn_underline) = this.btn_underline {
                            let _ = send_message_w(
                                btn_underline,
                                windows::Win32::UI::WindowsAndMessaging::BM_SETCHECK,
                                WPARAM(if underline_checked { 1 } else { 0 }),
                                LPARAM(0),
                            );
                        }

                        // 4. Redraw color button
                        if let Some(btn_color) = this.btn_color {
                            let _ = windows::Win32::Graphics::Gdi::InvalidateRect(
                                Some(btn_color),
                                None,
                                true,
                            );
                        }

                        // 5. Forward to OCX if it did not come from OCX subclass
                        if wparam.0 != 1
                            && let Some(host) = &this.host
                            && let Ok(hwnd_control) = host.get_control_hwnd()
                        {
                            let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                                Some(hwnd_control),
                                wm_update_settings,
                                WPARAM(2), // Flag: came from parent
                                lparam,
                            );
                        }
                    }
                    return LRESULT(0);
                }

                match message {
                    windows::Win32::UI::WindowsAndMessaging::WM_MEASUREITEM => {
                        let mi = &mut *(lparam.0
                            as *mut windows::Win32::UI::Controls::MEASUREITEMSTRUCT);
                        if mi.CtlID == 2001 {
                            mi.itemHeight = 24;
                            return LRESULT(1);
                        }
                    }
                    windows::Win32::UI::WindowsAndMessaging::WM_DRAWITEM => {
                        let di =
                            &*(lparam.0 as *const windows::Win32::UI::Controls::DRAWITEMSTRUCT);

                        // Owner-draw color button (ID 2005)
                        if di.CtlID == 2005 {
                            let hdc = di.hDC;
                            let rc = di.rcItem;

                            // Read current color index from config
                            let manager = crate::config::MSNConfigManager::new(
                                std::path::Path::new("config.toml"),
                            );
                            let color_idx = manager
                                .load()
                                .ok()
                                .and_then(|c| c.settings.fontcolor)
                                .unwrap_or(0) as usize;
                            let colorref = if color_idx < MSN_COLORS.len() {
                                MSN_COLORS[color_idx]
                            } else {
                                MSN_COLORS[0]
                            };

                            // Fill with active color
                            let brush = windows::Win32::Graphics::Gdi::CreateSolidBrush(
                                windows::Win32::Foundation::COLORREF(colorref),
                            );
                            windows::Win32::Graphics::Gdi::FillRect(hdc, &rc, brush);
                            let _ = windows::Win32::Graphics::Gdi::DeleteObject(brush.into());

                            // Draw a thin border
                            let _ = windows::Win32::Graphics::Gdi::DrawEdge(
                                hdc,
                                &mut rc.clone(),
                                windows::Win32::Graphics::Gdi::EDGE_SUNKEN,
                                windows::Win32::Graphics::Gdi::BF_RECT,
                            );

                            return LRESULT(1);
                        }
                        if di.CtlID == 2001 {
                            let state = di.itemState;
                            let hdc = di.hDC;
                            let rc = di.rcItem;

                            // Determine which item index to draw. For the edit control part, we get the current selection.
                            let item_idx = if di.itemID != u32::MAX {
                                di.itemID as usize
                            } else {
                                send_message_w(
                                    di.hwndItem,
                                    windows::Win32::UI::WindowsAndMessaging::CB_GETCURSEL,
                                    WPARAM(0),
                                    LPARAM(0),
                                )
                                .0 as usize
                            };

                            let is_edit_control =
                                (state.0 & windows::Win32::UI::Controls::ODS_COMBOBOXEDIT.0) != 0;
                            let is_selected =
                                (state.0 & windows::Win32::UI::Controls::ODS_SELECTED.0) != 0;

                            let brush_color = if is_selected && !is_edit_control {
                                windows::Win32::Graphics::Gdi::COLOR_HIGHLIGHT
                            } else {
                                windows::Win32::Graphics::Gdi::COLOR_WINDOW
                            };
                            windows::Win32::Graphics::Gdi::FillRect(
                                hdc,
                                &rc,
                                windows::Win32::Graphics::Gdi::GetSysColorBrush(brush_color),
                            );

                            if item_idx != usize::MAX {
                                // CB_ERR
                                let mut font_name_buf = [0u16; 128];
                                let _ = send_message_w(
                                    di.hwndItem,
                                    windows::Win32::UI::WindowsAndMessaging::CB_GETLBTEXT,
                                    WPARAM(item_idx),
                                    LPARAM(font_name_buf.as_mut_ptr() as isize),
                                );
                                let len = font_name_buf
                                    .iter()
                                    .position(|&x| x == 0)
                                    .unwrap_or(font_name_buf.len());
                                let font_name = String::from_utf16_lossy(&font_name_buf[..len]);

                                let old_bk_mode = windows::Win32::Graphics::Gdi::SetBkMode(
                                    hdc,
                                    windows::Win32::Graphics::Gdi::BACKGROUND_MODE(1),
                                );
                                let text_color = if is_selected && !is_edit_control {
                                    windows::Win32::Graphics::Gdi::GetSysColor(
                                        windows::Win32::Graphics::Gdi::COLOR_HIGHLIGHTTEXT,
                                    )
                                } else {
                                    windows::Win32::Graphics::Gdi::GetSysColor(
                                        windows::Win32::Graphics::Gdi::COLOR_WINDOWTEXT,
                                    )
                                };
                                let _ = windows::Win32::Graphics::Gdi::SetTextColor(
                                    hdc,
                                    windows::Win32::Foundation::COLORREF(text_color),
                                );

                                let mut lf = windows::Win32::Graphics::Gdi::LOGFONTW {
                                    lfHeight: -14,
                                    lfWeight: windows::Win32::Graphics::Gdi::FW_NORMAL.0 as i32,
                                    ..Default::default()
                                };

                                let font_wstr: Vec<u16> = font_name.encode_utf16().collect();
                                let copy_len = font_wstr.len().min(lf.lfFaceName.len() - 1);
                                lf.lfFaceName[..copy_len].copy_from_slice(&font_wstr[..copy_len]);

                                let hfont = windows::Win32::Graphics::Gdi::CreateFontIndirectW(&lf);
                                let old_font =
                                    windows::Win32::Graphics::Gdi::SelectObject(hdc, hfont.into());

                                let mut draw_rc = rc;
                                draw_rc.left += 4;
                                let _ = windows::Win32::Graphics::Gdi::DrawTextW(
                                    hdc,
                                    &mut font_name_buf[..len],
                                    &mut draw_rc,
                                    windows::Win32::Graphics::Gdi::DT_SINGLELINE
                                        | windows::Win32::Graphics::Gdi::DT_VCENTER,
                                );

                                windows::Win32::Graphics::Gdi::SelectObject(hdc, old_font);
                                let _ = windows::Win32::Graphics::Gdi::DeleteObject(hfont.into());
                                windows::Win32::Graphics::Gdi::SetBkMode(
                                    hdc,
                                    windows::Win32::Graphics::Gdi::BACKGROUND_MODE(
                                        old_bk_mode as u32,
                                    ),
                                );
                            }
                            return LRESULT(1);
                        }
                    }
                    windows::Win32::UI::WindowsAndMessaging::WM_NOTIFY => {
                        // Forward rebar size changes
                        let nmhdr = &*(lparam.0 as *const windows::Win32::UI::Controls::NMHDR);
                        if nmhdr.code == windows::Win32::UI::Controls::RBN_HEIGHTCHANGE {
                            // Trigger a resize to reflow the OCX below the rebar
                            let _ = send_message_w(window, WM_SIZE, WPARAM(0), {
                                let mut rc = RECT::default();
                                let _ = windows::Win32::UI::WindowsAndMessaging::GetClientRect(
                                    window, &mut rc,
                                );
                                LPARAM(
                                    ((rc.bottom as u32) << 16 | (rc.right as u32 & 0xFFFF))
                                        as isize,
                                )
                            });
                        }
                    }
                    WM_SIZE => {
                        let width = (lparam.0 & 0xFFFF) as i32;
                        let height = ((lparam.0 >> 16) & 0xFFFF) as i32;

                        // Forward WM_SIZE to the rebar so it resizes itself
                        if let Some(rebar) = this.rebar_hwnd {
                            let _ = send_message_w(rebar, WM_SIZE, wparam, lparam);
                            let _ = windows::Win32::Graphics::Gdi::InvalidateRect(
                                Some(rebar),
                                None,
                                true,
                            );
                        }

                        if let Some(toolbar) = this.toolbar_hwnd {
                            let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowPos(
                                toolbar,
                                None,
                                0,
                                0,
                                width,
                                28,
                                windows::Win32::UI::WindowsAndMessaging::SWP_NOZORDER
                                    | windows::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE,
                            );
                            let _ = windows::Win32::Graphics::Gdi::InvalidateRect(
                                Some(toolbar),
                                None,
                                true,
                            );
                        }

                        if let Some(host) = &this.host {
                            if this.parent.is_none() {
                                // Position OCX below the rebar
                                let rebar_h = this
                                    .rebar_hwnd
                                    .filter(|&r| unsafe {
                                        windows::Win32::UI::WindowsAndMessaging::IsWindowVisible(r)
                                            .as_bool()
                                    })
                                    .map(|r| {
                                        let h = send_message_w(
                                            r,
                                            windows::Win32::UI::Controls::RB_GETBARHEIGHT,
                                            WPARAM(0),
                                            LPARAM(0),
                                        )
                                        .0 as i32;
                                        if h == 0 { 32 } else { h }
                                    })
                                    .unwrap_or(0);
                                let rect = RECT {
                                    left: 0,
                                    top: rebar_h,
                                    right: width,
                                    bottom: height,
                                };
                                let _ = host.resize(&rect);
                                if let Ok(ocx_hwnd) = host.get_control_hwnd() {
                                    let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowPos(
                                        ocx_hwnd,
                                        None,
                                        rect.left,
                                        rect.top,
                                        rect.right - rect.left,
                                        rect.bottom - rect.top,
                                        windows::Win32::UI::WindowsAndMessaging::SWP_NOZORDER
                                            | windows::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE,
                                    );
                                }

                                // Bring rebar above the OCX in z-order
                                if let Some(rebar) = this.rebar_hwnd {
                                    let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowPos(
                                        rebar,
                                        Some(windows::Win32::UI::WindowsAndMessaging::HWND_TOP),
                                        0, 0, 0, 0,
                                        windows::Win32::UI::WindowsAndMessaging::SWP_NOMOVE
                                            | windows::Win32::UI::WindowsAndMessaging::SWP_NOSIZE
                                            | windows::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE,
                                    );
                                }
                            } else {
                                let rect = RECT {
                                    left: 0,
                                    top: 0,
                                    right: width,
                                    bottom: height,
                                };
                                let _ = host.resize(&rect);
                            }
                        }
                        return LRESULT(0);
                    }
                    windows::Win32::UI::WindowsAndMessaging::WM_COMMAND => {
                        let id = (wparam.0 & 0xFFFF) as u32;
                        let code = ((wparam.0 >> 16) & 0xFFFF) as u32;
                        match id {
                            1001 => {
                                if let Some(module) = &this.module
                                    && let Err(e) =
                                        Self::show_settings_modal(this.hwnd, module.clone())
                                {
                                    log::error!("Failed to show settings dialog: {}", e);
                                }
                            }
                            1002 => {
                                let _ = windows::Win32::UI::WindowsAndMessaging::DestroyWindow(
                                    this.hwnd,
                                );
                            }
                            2005 if code == windows::Win32::UI::WindowsAndMessaging::BN_CLICKED => {
                                // Color button clicked — open ChooseColor dialog
                                let manager = crate::config::MSNConfigManager::new(
                                    std::path::Path::new("config.toml"),
                                );
                                let current_idx = manager
                                    .load()
                                    .ok()
                                    .and_then(|c| c.settings.fontcolor)
                                    .unwrap_or(0)
                                    as usize;
                                let current_colorref = if current_idx < MSN_COLORS.len() {
                                    MSN_COLORS[current_idx]
                                } else {
                                    MSN_COLORS[0]
                                };

                                let mut cust_colors: [u32; 16] = MSN_COLORS;
                                let mut cc = windows::Win32::UI::Controls::Dialogs::CHOOSECOLORW {
                                    lStructSize: std::mem::size_of::<
                                        windows::Win32::UI::Controls::Dialogs::CHOOSECOLORW,
                                    >() as u32,
                                    hwndOwner: this.hwnd,
                                    rgbResult: windows::Win32::Foundation::COLORREF(
                                        current_colorref,
                                    ),
                                    lpCustColors: cust_colors.as_mut_ptr()
                                        as *mut windows::Win32::Foundation::COLORREF,
                                    Flags: windows::Win32::UI::Controls::Dialogs::CC_RGBINIT
                                        | windows::Win32::UI::Controls::Dialogs::CC_PREVENTFULLOPEN
                                        | windows::Win32::UI::Controls::Dialogs::CC_ANYCOLOR,
                                    ..Default::default()
                                };

                                if windows::Win32::UI::Controls::Dialogs::ChooseColorW(&mut cc)
                                    .as_bool()
                                {
                                    let chosen = cc.rgbResult.0;
                                    // Find the matching palette index
                                    let new_idx =
                                        MSN_COLORS.iter().position(|&c| c == chosen).unwrap_or(0)
                                            as u32;

                                    if let Ok(mut config) = manager.load() {
                                        config.settings.fontcolor = Some(new_idx);
                                        let _ = manager.save(&config);
                                    }

                                    // Redraw color button
                                    let _ = windows::Win32::Graphics::Gdi::InvalidateRect(
                                        this.btn_color,
                                        None,
                                        true,
                                    );

                                    // Broadcast update to the OCX
                                    if let Some(host) = &this.host
                                        && let Ok(hwnd_control) = host.get_control_hwnd()
                                    {
                                        let wm = windows::Win32::UI::WindowsAndMessaging::RegisterWindowMessageW(w!("WM_CHAT_UPDATESETTINGS"));
                                        let _ =
                                            windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                                                Some(hwnd_control),
                                                wm,
                                                WPARAM(0),
                                                LPARAM(0),
                                            );
                                    }
                                }
                            }
                            2001 | 2002
                                if code
                                    == windows::Win32::UI::WindowsAndMessaging::CBN_SELCHANGE =>
                            {
                                // Font / charset combo changed — save settings
                                let mut font_name_buf = [0u16; 128];
                                let font_idx = send_message_w(
                                    this.cb_font.unwrap(),
                                    windows::Win32::UI::WindowsAndMessaging::CB_GETCURSEL,
                                    WPARAM(0),
                                    LPARAM(0),
                                )
                                .0 as i32;

                                let font_name = if font_idx >= 0 {
                                    let _ = send_message_w(
                                        this.cb_font.unwrap(),
                                        windows::Win32::UI::WindowsAndMessaging::CB_GETLBTEXT,
                                        WPARAM(font_idx as usize),
                                        LPARAM(font_name_buf.as_mut_ptr() as isize),
                                    );
                                    let len = font_name_buf
                                        .iter()
                                        .position(|&x| x == 0)
                                        .unwrap_or(font_name_buf.len());
                                    String::from_utf16_lossy(&font_name_buf[..len])
                                } else {
                                    "Tahoma".to_string()
                                };

                                let charset_idx = send_message_w(
                                    this.cb_charset.unwrap(),
                                    windows::Win32::UI::WindowsAndMessaging::CB_GETCURSEL,
                                    WPARAM(0),
                                    LPARAM(0),
                                )
                                .0 as i32;

                                let charset_val = if charset_idx >= 0 {
                                    send_message_w(
                                        this.cb_charset.unwrap(),
                                        windows::Win32::UI::WindowsAndMessaging::CB_GETITEMDATA,
                                        WPARAM(charset_idx as usize),
                                        LPARAM(0),
                                    )
                                    .0 as i32
                                } else {
                                    0
                                };

                                // Read color from config (not from a combo anymore)
                                let manager = crate::config::MSNConfigManager::new(
                                    std::path::Path::new("config.toml"),
                                );
                                let color_val = manager
                                    .load()
                                    .ok()
                                    .and_then(|c| c.settings.fontcolor)
                                    .unwrap_or(0)
                                    as i32;

                                let bold_checked = send_message_w(
                                    this.btn_bold.unwrap(),
                                    windows::Win32::UI::WindowsAndMessaging::BM_GETCHECK,
                                    WPARAM(0),
                                    LPARAM(0),
                                )
                                .0 == 1;

                                let italic_checked = send_message_w(
                                    this.btn_italic.unwrap(),
                                    windows::Win32::UI::WindowsAndMessaging::BM_GETCHECK,
                                    WPARAM(0),
                                    LPARAM(0),
                                )
                                .0 == 1;

                                let underline_checked = send_message_w(
                                    this.btn_underline.unwrap(),
                                    windows::Win32::UI::WindowsAndMessaging::BM_GETCHECK,
                                    WPARAM(0),
                                    LPARAM(0),
                                )
                                .0 == 1;

                                let fontstyle = (if bold_checked { 1 } else { 0 })
                                    | (if italic_checked { 2 } else { 0 })
                                    | (if underline_checked { 4 } else { 0 });
                                let fontname_comb = format!("{};{}", font_name, charset_val);

                                if let Ok(mut config) = manager.load() {
                                    config.settings.fontname = Some(fontname_comb);
                                    config.settings.fontstyle = Some(fontstyle);
                                    config.settings.fontcolor = Some(color_val as u32);
                                    let _ = manager.save(&config);
                                }

                                if let Some(host) = &this.host
                                    && let Ok(hwnd_control) = host.get_control_hwnd()
                                {
                                    let wm = windows::Win32::UI::WindowsAndMessaging::RegisterWindowMessageW(w!("WM_CHAT_UPDATESETTINGS"));
                                    let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                                        Some(hwnd_control),
                                        wm,
                                        WPARAM(0),
                                        LPARAM(0),
                                    );
                                }
                            }
                            2003 | 2004 | 2006
                                if code == windows::Win32::UI::WindowsAndMessaging::BN_CLICKED =>
                            {
                                // Bold/Italic/Underline buttons — read all state and save
                                let mut font_name_buf = [0u16; 128];
                                let font_idx = send_message_w(
                                    this.cb_font.unwrap(),
                                    windows::Win32::UI::WindowsAndMessaging::CB_GETCURSEL,
                                    WPARAM(0),
                                    LPARAM(0),
                                )
                                .0 as i32;

                                let font_name = if font_idx >= 0 {
                                    let _ = send_message_w(
                                        this.cb_font.unwrap(),
                                        windows::Win32::UI::WindowsAndMessaging::CB_GETLBTEXT,
                                        WPARAM(font_idx as usize),
                                        LPARAM(font_name_buf.as_mut_ptr() as isize),
                                    );
                                    let len = font_name_buf
                                        .iter()
                                        .position(|&x| x == 0)
                                        .unwrap_or(font_name_buf.len());
                                    String::from_utf16_lossy(&font_name_buf[..len])
                                } else {
                                    "Tahoma".to_string()
                                };

                                let charset_idx = send_message_w(
                                    this.cb_charset.unwrap(),
                                    windows::Win32::UI::WindowsAndMessaging::CB_GETCURSEL,
                                    WPARAM(0),
                                    LPARAM(0),
                                )
                                .0 as i32;

                                let charset_val = if charset_idx >= 0 {
                                    send_message_w(
                                        this.cb_charset.unwrap(),
                                        windows::Win32::UI::WindowsAndMessaging::CB_GETITEMDATA,
                                        WPARAM(charset_idx as usize),
                                        LPARAM(0),
                                    )
                                    .0 as i32
                                } else {
                                    0
                                };

                                let manager = crate::config::MSNConfigManager::new(
                                    std::path::Path::new("config.toml"),
                                );
                                let color_val = manager
                                    .load()
                                    .ok()
                                    .and_then(|c| c.settings.fontcolor)
                                    .unwrap_or(0)
                                    as i32;

                                let bold_checked = send_message_w(
                                    this.btn_bold.unwrap(),
                                    windows::Win32::UI::WindowsAndMessaging::BM_GETCHECK,
                                    WPARAM(0),
                                    LPARAM(0),
                                )
                                .0 == 1;

                                let italic_checked = send_message_w(
                                    this.btn_italic.unwrap(),
                                    windows::Win32::UI::WindowsAndMessaging::BM_GETCHECK,
                                    WPARAM(0),
                                    LPARAM(0),
                                )
                                .0 == 1;

                                let underline_checked = send_message_w(
                                    this.btn_underline.unwrap(),
                                    windows::Win32::UI::WindowsAndMessaging::BM_GETCHECK,
                                    WPARAM(0),
                                    LPARAM(0),
                                )
                                .0 == 1;

                                let fontstyle = (if bold_checked { 1 } else { 0 })
                                    | (if italic_checked { 2 } else { 0 })
                                    | (if underline_checked { 4 } else { 0 });
                                let fontname_comb = format!("{};{}", font_name, charset_val);

                                if let Ok(mut config) = manager.load() {
                                    config.settings.fontname = Some(fontname_comb);
                                    config.settings.fontstyle = Some(fontstyle);
                                    config.settings.fontcolor = Some(color_val as u32);
                                    let _ = manager.save(&config);
                                }

                                if let Some(host) = &this.host
                                    && let Ok(hwnd_control) = host.get_control_hwnd()
                                {
                                    let wm = windows::Win32::UI::WindowsAndMessaging::RegisterWindowMessageW(w!("WM_CHAT_UPDATESETTINGS"));
                                    let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                                        Some(hwnd_control),
                                        wm,
                                        WPARAM(0),
                                        LPARAM(0),
                                    );
                                }
                            }
                            _ => {}
                        }
                        return LRESULT(0);
                    }
                    WM_DESTROY => {
                        let parent = this.parent;
                        this.host = None;

                        if let Some(parent) = parent {
                            // Clear USERDATA first to prevent use-after-free
                            windows::Win32::UI::WindowsAndMessaging::SetWindowLongW(
                                window,
                                windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
                                0,
                            );
                            let _boxed = Box::from_raw(this as *mut Self);
                            let _ = windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow(
                                parent, true,
                            );
                            let _ = windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(
                                parent,
                            );
                            let _ =
                                windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(Some(parent));
                        } else {
                            PostQuitMessage(0);
                        }
                        return LRESULT(0);
                    }
                    _ => {}
                }
            } else if message == WM_DESTROY {
                PostQuitMessage(0);
                return LRESULT(0);
            }

            DefWindowProcW(window, message, wparam, lparam)
        }
    }
}

impl Drop for OcxWindow {
    fn drop(&mut self) {
        if let Some(hfont) = self.hfont_bold.take() {
            unsafe {
                let _ = windows::Win32::Graphics::Gdi::DeleteObject(hfont.into());
            }
        }
        if let Some(hfont) = self.hfont_italic.take() {
            unsafe {
                let _ = windows::Win32::Graphics::Gdi::DeleteObject(hfont.into());
            }
        }
        if let Some(hfont) = self.hfont_underline.take() {
            unsafe {
                let _ = windows::Win32::Graphics::Gdi::DeleteObject(hfont.into());
            }
        }
        if let Some(hfont) = self.hfont_normal.take() {
            unsafe {
                let _ = windows::Win32::Graphics::Gdi::DeleteObject(hfont.into());
            }
        }
    }
}
