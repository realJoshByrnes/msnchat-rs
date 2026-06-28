use std::sync::Mutex;
use lazy_static::lazy_static;
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            FindWindowW, GetClientRect, PostMessageW,
            WM_SIZE, CreateWindowExW, WS_VISIBLE, CW_USEDEFAULT,
        },
        UI::Shell::{SetWindowSubclass, DefSubclassProc},
        UI::HiDpi::GetDpiForWindow,
    },
    core::{GUID, Result, w},
};
use windows_reactor::*;

use crate::host::OcxHost;

#[derive(Clone, Copy, Debug, PartialEq)]
struct SendHwnd(HWND);
unsafe impl Send for SendHwnd {}
unsafe impl Sync for SendHwnd {}

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

const CHARSETS: [(&str, i32); 16] = [
    ("Western", 0),
    ("Default", 1),
    ("Symbol", 2),
    ("ShiftJIS", 128),
    ("Hangul", 129),
    ("GB2312", 134),
    ("Big5", 136),
    ("Greek", 161),
    ("Turkish", 162),
    ("Hebrew", 177),
    ("Arabic", 178),
    ("Baltic", 186),
    ("Russian", 204),
    ("Thai", 222),
    ("Eastern Europe", 238),
    ("OEM", 255),
];

lazy_static! {
    static ref ACTIVE_OCX_HWND: Mutex<Option<SendHwnd>> = Mutex::new(None);
    static ref ACTIVE_OCX_HOST: Mutex<Option<OcxHost>> = Mutex::new(None);
    static ref ACTIVE_MODULE: Mutex<Option<std::sync::Arc<crate::patch::pe::ManualModule>>> = Mutex::new(None);
}

fn get_active_ocx_hwnd() -> Option<HWND> {
    ACTIVE_OCX_HWND.lock().unwrap().map(|sh| sh.0)
}

unsafe extern "system" fn clip_sibling_proc(hwnd: HWND, _lparam: LPARAM) -> windows::core::BOOL {
    // Add WS_CLIPSIBLINGS to sibling windows so they don't draw over the OCX
    unsafe {
        let style = windows::Win32::UI::WindowsAndMessaging::GetWindowLongW(hwnd, windows::Win32::UI::WindowsAndMessaging::GWL_STYLE);
        windows::Win32::UI::WindowsAndMessaging::SetWindowLongW(
            hwnd,
            windows::Win32::UI::WindowsAndMessaging::GWL_STYLE,
            style | windows::Win32::UI::WindowsAndMessaging::WS_CLIPSIBLINGS.0 as i32,
        );
    }
    true.into()
}

fn apply_clipping_to_children(parent: HWND) {
    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::EnumChildWindows(
            Some(parent),
            Some(clip_sibling_proc),
            LPARAM(0),
        );
    }
}

fn parent_and_show_ocx(parent_hwnd: HWND) {
    unsafe {
        // Add WS_CLIPCHILDREN to the top-level parent window so it clips child windows correctly.
        let style = windows::Win32::UI::WindowsAndMessaging::GetWindowLongW(parent_hwnd, windows::Win32::UI::WindowsAndMessaging::GWL_STYLE);
        windows::Win32::UI::WindowsAndMessaging::SetWindowLongW(
            parent_hwnd,
            windows::Win32::UI::WindowsAndMessaging::GWL_STYLE,
            style | windows::Win32::UI::WindowsAndMessaging::WS_CLIPCHILDREN.0 as i32,
        );
    }
    
    apply_clipping_to_children(parent_hwnd);
    
    if let Some(host) = ACTIVE_OCX_HOST.lock().unwrap().as_mut() {
        let _ = host.attach(parent_hwnd);
        match host.get_control_hwnd() {
            Ok(ocx_hwnd) => {
                *ACTIVE_OCX_HWND.lock().unwrap() = Some(SendHwnd(ocx_hwnd));
                
                unsafe {
                    // Explicitly set parent to be absolutely sure
                    let _ = windows::Win32::UI::WindowsAndMessaging::SetParent(ocx_hwnd, Some(parent_hwnd));
                    // Explicitly show the window
                    let _ = windows::Win32::UI::WindowsAndMessaging::ShowWindow(ocx_hwnd, windows::Win32::UI::WindowsAndMessaging::SW_SHOW);
                }

                // Subclass the WinUI 3 window to intercept resize and perform resizing of OCX window.
                unsafe {
                    let _ = SetWindowSubclass(
                        parent_hwnd,
                        Some(winui_subclass_proc),
                        101, // Subclass ID
                        0,   // Ref data
                    );
                }
                
                // Force an initial resize
                let mut rc = RECT::default();
                unsafe {
                    let _ = GetClientRect(parent_hwnd, &mut rc);
                }
                let mut width = rc.right - rc.left;
                let mut height = rc.bottom - rc.top;
                log::info!("parent_and_show_ocx size calculation: initial ClientRect is width={} height={}", width, height);
                if width <= 0 || height <= 0 {
                    width = 800;
                    height = 600;
                    log::info!("parent_and_show_ocx size calculation: falling back to default size width={} height={}", width, height);
                }
                
                let dpi = unsafe { GetDpiForWindow(parent_hwnd) };
                let dpi_scale: f64 = if dpi == 0 { 1.0 } else { dpi as f64 / 96.0 };
                
                let px_left = 0;
                let px_top = (48.0f64 * dpi_scale).round() as i32; // Top toolbar is 48 DIPs
                let px_width = width;
                let px_height = height - px_top;
                
                let rect = RECT {
                    left: px_left,
                    top: px_top,
                    right: px_left + px_width,
                    bottom: px_top + px_height,
                };
                let _ = host.resize(&rect);
                unsafe {
                    // Bring the OCX window to the top of the Z-order
                    let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowPos(
                        ocx_hwnd,
                        Some(windows::Win32::UI::WindowsAndMessaging::HWND_TOP),
                        rect.left,
                        rect.top,
                        rect.right - rect.left,
                        rect.bottom - rect.top,
                        windows::Win32::UI::WindowsAndMessaging::SWP_SHOWWINDOW,
                    );
                }
            }
            Err(e) => log::error!("host.get_control_hwnd failed: {:?}", e),
        }
    } else {
        log::error!("parent_and_show_ocx: ACTIVE_OCX_HOST is None!");
    }
}

fn resize_active_ocx(parent_hwnd: HWND, left: i32, top: i32, width: i32, height: i32) {
    log::info!("resize_active_ocx: left={} top={} width={} height={}", left, top, width, height);
    if let Some(host) = ACTIVE_OCX_HOST.lock().unwrap().as_ref() {
        let rect = RECT {
            left,
            top,
            right: left + width,
            bottom: top + height,
        };
        let _ = host.resize(&rect);
        if let Ok(ocx_hwnd) = host.get_control_hwnd() {
            unsafe {
                // Ensure parent is still set
                let _ = windows::Win32::UI::WindowsAndMessaging::SetParent(ocx_hwnd, Some(parent_hwnd));
                // Bring it to the top of the Z-order
                let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowPos(
                    ocx_hwnd,
                    Some(windows::Win32::UI::WindowsAndMessaging::HWND_TOP),
                    rect.left,
                    rect.top,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                    windows::Win32::UI::WindowsAndMessaging::SWP_SHOWWINDOW,
                );
            }
        }
    }
}

unsafe extern "system" fn winui_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _ref_data: usize,
) -> LRESULT {
    match msg {
        WM_SIZE => {
            let width = (lparam.0 & 0xFFFF) as i32;
            let height = ((lparam.0 >> 16) & 0xFFFF) as i32;
            log::info!("winui_subclass_proc: WM_SIZE width={} height={}", width, height);
            
            let dpi = unsafe { GetDpiForWindow(hwnd) };
            let dpi_scale: f64 = if dpi == 0 { 1.0 } else { dpi as f64 / 96.0 };
            
            let px_left = 0;
            let px_top = (48.0f64 * dpi_scale).round() as i32; // Top toolbar is 48 DIPs
            let px_width = width;
            let px_height = height - px_top;
            
            resize_active_ocx(hwnd, px_left, px_top, px_width, px_height);
        }
        windows::Win32::UI::WindowsAndMessaging::WM_WINDOWPOSCHANGED | windows::Win32::UI::WindowsAndMessaging::WM_PAINT => {
            let mut rc = RECT::default();
            unsafe {
                let _ = GetClientRect(hwnd, &mut rc);
            }
            let width = rc.right - rc.left;
            let height = rc.bottom - rc.top;
            if width > 0 && height > 0 {
                let dpi = unsafe { GetDpiForWindow(hwnd) };
                let dpi_scale: f64 = if dpi == 0 { 1.0 } else { dpi as f64 / 96.0 };
                let px_left = 0;
                let px_top = (48.0f64 * dpi_scale).round() as i32;
                resize_active_ocx(hwnd, px_left, px_top, width, height - px_top);
            }
        }
        _ => {}
    }
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

fn update_settings(font_name: &str, charset_val: i32, is_bold: bool, is_italic: bool, is_underline: bool, color_idx: usize) {
    let fontstyle = (if is_bold { 1 } else { 0 }) 
        | (if is_italic { 2 } else { 0 })
        | (if is_underline { 4 } else { 0 });
    let fontname_comb = format!("{};{}", font_name, charset_val);
    
    let manager = crate::config::MSNConfigManager::new(std::path::Path::new("config.toml"));
    if let Ok(mut config) = manager.load() {
        config.settings.fontname = Some(fontname_comb);
        config.settings.fontstyle = Some(fontstyle);
        config.settings.fontcolor = Some(color_idx as u32);
        let _ = manager.save(&config);
    }
    
    if let Some(ocx_hwnd) = get_active_ocx_hwnd() {
        unsafe {
            let wm = windows::Win32::UI::WindowsAndMessaging::RegisterWindowMessageW(w!("WM_CHAT_UPDATESETTINGS"));
            let _ = PostMessageW(
                Some(ocx_hwnd),
                wm,
                WPARAM(0),
                LPARAM(0),
            );
        }
    }
}

fn show_color_picker() -> Option<usize> {
    let hwnd = unsafe { FindWindowW(None, w!("MsnChat WinUI")).unwrap_or_default() };
    if hwnd.0.is_null() {
        return None;
    }
    
    let manager = crate::config::MSNConfigManager::new(std::path::Path::new("config.toml"));
    let current_idx = manager
        .load()
        .ok()
        .and_then(|c| c.settings.fontcolor)
        .unwrap_or(0) as usize;
        
    let current_colorref = if current_idx < MSN_COLORS.len() {
        MSN_COLORS[current_idx]
    } else {
        MSN_COLORS[0]
    };

    let mut cust_colors: [u32; 16] = MSN_COLORS;
    let mut cc = windows::Win32::UI::Controls::Dialogs::CHOOSECOLORW {
        lStructSize: std::mem::size_of::<windows::Win32::UI::Controls::Dialogs::CHOOSECOLORW>() as u32,
        hwndOwner: hwnd,
        rgbResult: windows::Win32::Foundation::COLORREF(current_colorref),
        lpCustColors: cust_colors.as_mut_ptr() as *mut windows::Win32::Foundation::COLORREF,
        Flags: windows::Win32::UI::Controls::Dialogs::CC_RGBINIT
            | windows::Win32::UI::Controls::Dialogs::CC_PREVENTFULLOPEN
            | windows::Win32::UI::Controls::Dialogs::CC_ANYCOLOR,
        ..Default::default()
    };

    if unsafe { windows::Win32::UI::Controls::Dialogs::ChooseColorW(&mut cc) }.as_bool() {
        let chosen = cc.rgbResult.0;
        let new_idx = MSN_COLORS.iter().position(|&c| c == chosen).unwrap_or(0);
        Some(new_idx)
    } else {
        None
    }
}

fn get_system_font_names() -> Vec<String> {
    unsafe {
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
        font_names
    }
}

unsafe extern "system" fn enum_font_fam_ex_proc(
    lpelfe: *const windows::Win32::Graphics::Gdi::LOGFONTW,
    _lpntme: *const windows::Win32::Graphics::Gdi::TEXTMETRICW,
    _fonttype: u32,
    lparam: LPARAM,
) -> i32 {
    unsafe {
        let list = &mut *(lparam.0 as *mut Vec<String>);
        let font_name = String::from_utf16_lossy(&(*lpelfe).lfFaceName);
        let font_name = font_name.trim_end_matches('\0').to_string();
        if !font_name.is_empty() && !list.contains(&font_name) && !font_name.starts_with('@') {
            list.push(font_name);
        }
        1
    }
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

        let mut host = OcxHost::new(module.clone(), &clsid_settings)?;
        let _ = host.put_property("BackColor", "16777215");

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

        let _ = windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow(parent, false);

        let instance = GetModuleHandleW(None)?;
        let class_name = w!("MsnChatOcxHostClass");

        // Define window class if not already registered
        let wc = windows::Win32::UI::WindowsAndMessaging::WNDCLASSW {
            hCursor: windows::Win32::UI::WindowsAndMessaging::LoadCursorW(
                None,
                windows::Win32::UI::WindowsAndMessaging::IDC_ARROW,
            )?,
            hInstance: instance.into(),
            lpszClassName: class_name,
            lpfnWndProc: Some(settings_wndproc),
            ..Default::default()
        };
        let _ = windows::Win32::UI::WindowsAndMessaging::RegisterClassW(&wc);

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

        // Center settings window
        let mut parent_rect = RECT::default();
        let mut child_rect = RECT::default();
        let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowRect(parent, &mut parent_rect);
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

        host.attach(hwnd)?;

        let settings_win = Box::into_raw(Box::new((host, parent)));
        windows::Win32::UI::WindowsAndMessaging::SetWindowLongW(
            hwnd,
            windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
            settings_win as isize as i32,
        );
    }
    Ok(())
}

unsafe extern "system" fn settings_wndproc(
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
            let data = &mut *(user_data as *mut (OcxHost, HWND));
            match message {
                WM_SIZE => {
                    let width = (lparam.0 & 0xFFFF) as i32;
                    let height = ((lparam.0 >> 16) & 0xFFFF) as i32;
                    let rect = RECT { left: 0, top: 0, right: width, bottom: height };
                    let _ = data.0.resize(&rect);
                }
                windows::Win32::UI::WindowsAndMessaging::WM_DESTROY => {
                    let _ = windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow(data.1, true);
                    let _ = windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(data.1);
                    drop(Box::from_raw(data as *mut (OcxHost, HWND)));
                }
                _ => {}
            }
        }
        windows::Win32::UI::WindowsAndMessaging::DefWindowProcW(window, message, wparam, lparam)
    }
}

fn app(cx: &mut RenderCx) -> Element {
    // Load config initially
    let manager = crate::config::MSNConfigManager::new(std::path::Path::new("config.toml"));
    let config = manager.load().unwrap_or_default();
    
    let fontname_comb = config.settings.fontname.clone().unwrap_or_else(|| "Tahoma;0".to_string());
    let parts: Vec<&str> = fontname_comb.split(';').collect();
    let initial_font_name = parts.first().copied().unwrap_or("Tahoma").to_string();
    let initial_charset_val = parts.get(1).and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
    
    let fontstyle = config.settings.fontstyle.unwrap_or(0);
    let initial_is_bold = (fontstyle & 1) != 0;
    let initial_is_italic = (fontstyle & 2) != 0;
    let initial_is_underline = (fontstyle & 4) != 0;
    let initial_color_idx = config.settings.fontcolor.unwrap_or(0) as usize;

    let (font_name, set_font_name) = cx.use_state(initial_font_name);
    let (charset_val, set_charset_val) = cx.use_state(initial_charset_val);
    let (is_bold, set_is_bold) = cx.use_state(initial_is_bold);
    let (is_italic, set_is_italic) = cx.use_state(initial_is_italic);
    let (is_underline, set_is_underline) = cx.use_state(initial_is_underline);
    let (color_idx, set_color_idx) = cx.use_state(initial_color_idx);
    
    let font_names = cx.use_memo((), || get_system_font_names());
    let font_index = font_names.iter().position(|name| name == &font_name).unwrap_or(0) as i32;
    
    let charset_names: Vec<String> = CHARSETS.iter().map(|(name, _)| name.to_string()).collect();
    let charset_index = CHARSETS.iter().position(|(_, val)| *val == charset_val).unwrap_or(0) as i32;
    
    let on_font_changed = {
        let font_names = font_names.clone();
        let set_font_name = set_font_name.clone();
        move |idx: i32| {
            if idx >= 0 && (idx as usize) < font_names.len() {
                let name = font_names[idx as usize].clone();
                set_font_name.call(name.clone());
                update_settings(&name, charset_val, is_bold, is_italic, is_underline, color_idx);
            }
        }
    };
    
    let on_charset_changed = {
        let set_charset_val = set_charset_val.clone();
        let font_name = font_name.clone();
        move |idx: i32| {
            if idx >= 0 && (idx as usize) < CHARSETS.len() {
                let val = CHARSETS[idx as usize].1;
                set_charset_val.call(val);
                update_settings(&font_name, val, is_bold, is_italic, is_underline, color_idx);
            }
        }
    };
    
    let on_bold_toggled = {
        let set_is_bold = set_is_bold.clone();
        let font_name = font_name.clone();
        move |checked: bool| {
            set_is_bold.call(checked);
            update_settings(&font_name, charset_val, checked, is_italic, is_underline, color_idx);
        }
    };
    
    let on_italic_toggled = {
        let set_is_italic = set_is_italic.clone();
        let font_name = font_name.clone();
        move |checked: bool| {
            set_is_italic.call(checked);
            update_settings(&font_name, charset_val, is_bold, checked, is_underline, color_idx);
        }
    };

    let on_underline_toggled = {
        let set_is_underline = set_is_underline.clone();
        let font_name = font_name.clone();
        move |checked: bool| {
            set_is_underline.call(checked);
            update_settings(&font_name, charset_val, is_bold, is_italic, checked, color_idx);
        }
    };
    
    let on_color_clicked = {
        let set_color_idx = set_color_idx.clone();
        let font_name = font_name.clone();
        move || {
            if let Some(new_idx) = show_color_picker() {
                set_color_idx.call(new_idx);
                update_settings(&font_name, charset_val, is_bold, is_italic, is_underline, new_idx);
            }
        }
    };

    let on_options_clicked = move || {
        let hwnd = unsafe { FindWindowW(None, w!("MsnChat WinUI")).unwrap_or_default() };
        if !hwnd.0.is_null() {
            if let Some(module) = ACTIVE_MODULE.lock().unwrap().as_ref() {
                let _ = show_settings_modal(hwnd, module.clone());
            }
        }
    };

    vstack((
        // Top Toolbar
        hstack((
            ComboBox::new(font_names.clone())
                .selected_index(font_index)
                .on_selection_changed(on_font_changed)
                .width(180.0),
            
            ComboBox::new(charset_names)
                .selected_index(charset_index)
                .on_selection_changed(on_charset_changed)
                .width(120.0),
                
            toggle_button("𝐁", is_bold)
                .on_checked(on_bold_toggled)
                .width(36.0),
                
            toggle_button("𝑰", is_italic)
                .on_checked(on_italic_toggled)
                .width(36.0),

            toggle_button("U\u{0332}", is_underline)
                .on_checked(on_underline_toggled)
                .width(36.0),
                
            button("Color")
                .on_click(on_color_clicked)
                .width(70.0),

            button("Options")
                .on_click(on_options_clicked)
                .width(80.0),
        ))
        .spacing(8.0)
        .height(48.0)
        .padding(Thickness::uniform(6.0)),
        
        // Chat control area container
        swap_chain_panel()
            .on_mounted(move |_handle| {
                let hwnd = unsafe { FindWindowW(None, w!("MsnChat WinUI")).unwrap_or_default() };
                log::info!("on_mounted called. FindWindowW('MsnChat WinUI') returned HWND: {:?}", hwnd.0);
                if !hwnd.0.is_null() {
                    parent_and_show_ocx(hwnd);
                } else {
                    log::error!("on_mounted: Could not find window 'MsnChat WinUI'!");
                }
            })
    ))
    .spacing(0.0)
    .into()
}

pub fn run_winui_app(module: std::sync::Arc<crate::patch::pe::ManualModule>) -> Result<()> {
    // Save module globally so the settings callback can access it
    *ACTIVE_MODULE.lock().unwrap() = Some(module.clone());
    
    // Attach MSN Chat OCX
    let clsid = GUID::from_values(
        0xF58E1CEF,
        0xA068,
        0x4c15,
        [0xBA, 0x5E, 0x58, 0x7C, 0xAF, 0x3E, 0xE8, 0xC6],
    );

    let host = OcxHost::new(module.clone(), &clsid)?;

    let random_id = (uuid::Uuid::new_v4().as_u128() % 10000) as u32;
    let nickname = format!("JD{:04}", random_id);
    let _ = host.put_property("BaseURL", "http://chat.msn.com/");
    let _ = host.put_property("Market", "en-au");
    let _ = host.put_property("AuditMessage", "Note: MSN has detected that you are connected to this chat session from the IP address <b>%1</b>.");
    let _ = host.put_property("ChatMode", "0");
    let _ = host.put_property("InvitationCode", "5355");
    let _ = host.put_property("MessageOfTheDay", "Welcome to MSN Chat. Important: MSN does not control or endorse the content, messages or information found in chat. MSN specifically disclaims any liability with regard to these areas. To review the guidelines for use of MSN Chat, go to http://chat.msn.com/conduct.asp.");
    let _ = host.put_property("NickName", &nickname);
    let _ = host.put_property("RoomName", "The Lobby");
    let _ = host.put_property("Server", "dir.irc7.com");
    let _ = host.put_property("WhisperContent", "http://test.example.com/whisper");

    *ACTIVE_OCX_HOST.lock().unwrap() = Some(host);

    App::new()
        .title("MsnChat WinUI")
        .inner_size(800.0, 600.0)
        .render(app)
        .map_err(|e| windows::core::Error::new(windows::core::HRESULT(0x80004005u32 as i32), format!("{:?}", e)))?;

    Ok(())
}
