use windows::Win32::Foundation::{COLORREF, FreeLibrary, HINSTANCE, HMODULE, HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateSolidBrush, DeleteObject, GetSysColor, HBRUSH, SYS_COLOR_INDEX,
};
use windows::Win32::System::LibraryLoader::LoadLibraryA;
use windows::Win32::UI::WindowsAndMessaging::{DestroyMenu, HMENU, LoadMenuA, SendMessageA};
use windows::core::{PCSTR, s};

pub mod hooks;
pub mod layout;

pub struct MSNChatEdit4 {
    pub is_richedit20: bool, // offset 39
    pub hwnd: HWND,          // offset 40 (inside inner object/thunk)
    // ... padding ...
    pub hmodule: HMODULE,    // offset 51
    pub gdiobj52: HBRUSH,    // offset 52
    pub bg_brush: HBRUSH,    // offset 53
    pub context_menu: HMENU, // offset 54
}

// These are necessary because GDI/Window handles are technically not Send/Sync natively in windows-rs.
unsafe impl Send for MSNChatEdit4 {}
unsafe impl Sync for MSNChatEdit4 {}

#[repr(C)]
pub struct MSNChatEdit4Layout {
    pub vtable: usize,                   // 0 / 0x00
    pub hwnd_parent: HWND,               // 4 / 0x04
    pub unk_08: usize,                   // 8 / 0x08
    pub unk_0c: usize,                   // 12 / 0x0C
    pub unk_10: usize,                   // 16 / 0x10
    pub unk_14: usize,                   // 20 / 0x14
    pub unk_18: usize,                   // 24 / 0x18
    pub unk_1c: usize,                   // 28 / 0x1C
    pub cr_text_color: u32,              // 32 / 0x20
    pub unk_24: usize,                   // 36 / 0x24
    pub unk_28: usize,                   // 40 / 0x28
    pub unk_2c: usize,                   // 44 / 0x2C
    pub unk_30: usize,                   // 48 / 0x30
    pub unk_34: usize,                   // 52 / 0x34
    pub margin: i32, // 56 / 0x38 (0x3C in C++ = 60 face name, so this is roughly before it)
    pub facename: [u16; 32], // 60 / 0x3C
    pub unk_7c: [u8; 32], // 124 / 0x7C
    pub unk_9c: usize, // 156 / 0x9C  (is_richedit20 flag check)
    pub hwnd_self: HWND, // 160 / 0xA0
    pub padding: [usize; 9], // 164 - 196
    pub event_sink: *const *const usize, // 200 / 0xC8
}

impl MSNChatEdit4 {
    pub const CLASS_NAME: PCSTR = s!("MSNChatEdit4");

    /// Corresponds to sub_37226403
    ///
    /// # Safety
    /// This function calls native Win32 APIs (LoadLibraryA, GetClassInfoExA) which may have unsafe behavior.
    pub unsafe fn new(h_instance: HINSTANCE) -> Self {
        unsafe {
            let mut lib = LoadLibraryA(s!("RICHED20.DLL"));
            let mut is_richedit20 = true;

            if lib.is_err() {
                lib = LoadLibraryA(s!("RICHED32.DLL"));
                is_richedit20 = false;
            }

            let base_class = if is_richedit20 {
                s!("RichEdit20W")
            } else {
                s!("RICHEDIT")
            };

            // Initialize the superclass
            crate::chat45::controls::utils::superclass_window(
                h_instance,
                base_class,
                Self::CLASS_NAME,
                true,
                None,
            );

            let sys_color_val = GetSysColor(SYS_COLOR_INDEX(5)); // COLOR_WINDOW
            let bg_brush = CreateSolidBrush(COLORREF(sys_color_val));

            Self {
                is_richedit20,
                hwnd: HWND::default(),
                hmodule: lib.unwrap_or_default(),
                gdiobj52: HBRUSH::default(),
                bg_brush,
                context_menu: HMENU::default(),
            }
        }
    }

    /// # Safety
    /// This function performs numerous unsafe FFI calls to interact with the underlying C++ OCX environment.
    pub unsafe fn create_window(&mut self, parent: HWND, id: isize, h_instance: HINSTANCE) -> bool {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{CreateWindowExA, CreateWindowExW};
            use windows::core::PCWSTR;

            log::trace!(
                "Calling CreateWindow for MSNChatEdit4, parent: {:?}, id: {}",
                parent,
                id
            );

            let style = windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(0x50001084);

            let window = if self.is_richedit20 {
                let mut class_w: Vec<u16> = Vec::new();
                let mut ptr = Self::CLASS_NAME.0;
                while *ptr != 0 {
                    class_w.push(*ptr as u16);
                    ptr = ptr.add(1);
                }
                class_w.push(0);

                CreateWindowExW(
                    Default::default(),
                    PCWSTR(class_w.as_ptr()),
                    PCWSTR(std::ptr::null()),
                    style,
                    0,
                    0,
                    1,
                    1,
                    Some(parent),
                    Some(HMENU(id as _)),
                    Some(h_instance),
                    None,
                )
            } else {
                CreateWindowExA(
                    Default::default(),
                    Self::CLASS_NAME,
                    PCSTR::null(),
                    style,
                    0,
                    0,
                    1,
                    1,
                    Some(parent),
                    Some(HMENU(id as _)),
                    Some(h_instance),
                    None,
                )
            }
            .unwrap_or_default();

            if window.is_invalid() {
                log::error!("CreateWindowExA failed for MSNChatEdit4 class!");
                return false;
            }

            log::trace!(
                "CreateWindowExA succeeded for MSNChatEdit4, HWND: {:?}",
                window
            );
            self.hwnd = window;

            // Subclass hook would typically go here (sub_372212D6)

            // EM_LIMITTEXT
            SendMessageA(self.hwnd, 0x00C5, WPARAM(0xFF), LPARAM(0));
            // EM_GETEVENTMASK
            let mask = SendMessageA(self.hwnd, 0x043B, WPARAM(0), LPARAM(0));
            // EM_SETEVENTMASK (add ENM_CHANGE | ENM_SELCHANGE)
            SendMessageA(self.hwnd, 0x0445, WPARAM(0), LPARAM(mask.0 | 0x20001));

            if self.is_richedit20 {
                SendMessageA(self.hwnd, 0x045B, WPARAM(0), LPARAM(0));
                let v9 = SendMessageA(self.hwnd, 0x0479, WPARAM(0), LPARAM(0));
                SendMessageA(self.hwnd, 0x0478, WPARAM(0), LPARAM(v9.0 & !3));
                SendMessageA(self.hwnd, 0x04CC, WPARAM(0x10000), LPARAM(0x10000)); // EM_SETTEXTMODE?
            } else {
                SendMessageA(self.hwnd, 0x046A, WPARAM(2), LPARAM(258)); // EM_AUTOURLDETECT
            }

            self.context_menu = LoadMenuA(
                Some(crate::patch::pe::get_ocx_hinstance()),
                PCSTR::from_raw((0x25A & 0xFFFF) as *const u8),
            )
            .unwrap_or_default();
            true
        }
    }

    /// # Safety
    /// This function performs numerous unsafe FFI calls to interact with Win32 GDI objects and the DC.
    pub unsafe fn calculate_font_height(&self, this: *mut std::ffi::c_void) -> i32 {
        unsafe {
            use windows::Win32::Graphics::Gdi::{
                CreateFontIndirectA, DeleteObject, GetDC, GetDeviceCaps, GetTextExtentPoint32A,
                LOGFONTA, LOGPIXELSY, ReleaseDC, SelectObject,
            };
            use windows::Win32::UI::Controls::RichEdit::CHARFORMATA;

            let mut lf = LOGFONTA::default();
            let mut cfa = CHARFORMATA {
                cbSize: std::mem::size_of::<CHARFORMATA>() as u32,
                ..Default::default()
            };

            SendMessageA(
                self.hwnd,
                0x043A,    // EM_GETCHARFORMAT
                WPARAM(1), // SCF_DEFAULT
                LPARAM(&mut cfa as *mut _ as isize),
            );

            lf.lfCharSet = cfa.bCharSet;
            lf.lfPitchAndFamily = cfa.bPitchAndFamily;
            lf.lfHeight = -(cfa.yHeight);
            let effects = cfa.dwEffects.0;
            lf.lfWeight = if (effects & 1) != 0 { 700 } else { 400 }; // CFE_BOLD
            lf.lfItalic = if (effects & 2) != 0 { 1 } else { 0 }; // CFE_ITALIC

            for i in 0..32 {
                lf.lfFaceName[i] = cfa.szFaceName[i];
                if cfa.szFaceName[i] == 0 {
                    break;
                }
            }

            let mut font_height = 0;
            let h_font = CreateFontIndirectA(&lf);
            if !h_font.is_invalid() {
                let dc = GetDC(Some(self.hwnd));
                if !dc.is_invalid() {
                    let old_font = SelectObject(dc, h_font.into());
                    let mut size = windows::Win32::Foundation::SIZE::default();
                    let _ = GetTextExtentPoint32A(dc, "Xy".as_bytes(), &mut size);
                    let logic_y = GetDeviceCaps(Some(dc), LOGPIXELSY);
                    font_height = (size.cy * logic_y) / 1440;
                    SelectObject(dc, old_font);
                    ReleaseDC(Some(self.hwnd), dc);

                    let layout = &*(this as *const MSNChatEdit4Layout);
                    let margin = layout.margin;
                    font_height += 2 * margin;
                }
                let _ = DeleteObject(h_font.into());
            }
            font_height
        }
    }

    /// # Safety
    /// Modifies native memory layouts using hardcoded offset calculations.
    pub unsafe fn format_layout(&mut self, this: *mut std::ffi::c_void) {
        unsafe {
            use windows::Win32::Graphics::Gdi::InflateRect;
            use windows::Win32::UI::WindowsAndMessaging::{
                GetClientRect, SET_WINDOW_POS_FLAGS, SetWindowPos,
            };

            let layout = &*(this as *const MSNChatEdit4Layout);
            let parent_hwnd = layout.hwnd_parent;
            let margin = layout.margin;

            let mut rect = windows::Win32::Foundation::RECT::default();
            let _ = GetClientRect(parent_hwnd, &mut rect);
            let _ = InflateRect(&mut rect, -margin, -margin);

            let mut v2 = 0;
            let font_height = self.calculate_font_height(this);

            if rect.bottom - rect.top > font_height {
                v2 = (rect.bottom - rect.top - font_height) / 2;
            }

            let a3 = v2 + margin;

            let mut final_rect = windows::Win32::Foundation::RECT::default();
            let _ = GetClientRect(parent_hwnd, &mut final_rect);
            let _ = InflateRect(&mut final_rect, -a3, -a3);

            let _ = SetWindowPos(
                self.hwnd,
                Some(HWND::default()),
                final_rect.left,
                final_rect.top,
                final_rect.right - final_rect.left,
                final_rect.bottom - final_rect.top,
                SET_WINDOW_POS_FLAGS(0x14), // SWP_NOACTIVATE | SWP_NOZORDER
            );
        }
    }

    /// # Safety
    /// Modifies native memory layouts and calls native GDI APIs.
    pub unsafe fn format_font(&mut self, this: *mut std::ffi::c_void) {
        unsafe {
            if self.hwnd.is_invalid() {
                return;
            }

            use windows::Win32::UI::Controls::RichEdit::{
                CFE_EFFECTS, CFM_COLOR, CFM_EFFECTS, CFM_FACE, CFM_MASK, CFM_SIZE, CHARFORMATA,
                SCF_ALL,
            };

            let layout = &mut *(this as *mut MSNChatEdit4Layout);
            let y_height = layout.unk_18 as i32; // assuming offset 24 maps
            let dw_effects = *((this as usize + 152) as *const u32);
            let b_pitch_and_family = *((this as usize + 140) as *const u8);
            let cr_text_color = layout.cr_text_color;
            let facename_ptr = layout.facename.as_ptr();
            let is_richedit20 = layout.unk_9c == 0;

            let mut cfa = CHARFORMATA {
                cbSize: std::mem::size_of::<CHARFORMATA>() as u32,
                dwMask: CFM_MASK(CFM_EFFECTS.0 | CFM_COLOR.0 | CFM_FACE.0 | CFM_SIZE.0),
                yHeight: y_height * 20,
                dwEffects: CFE_EFFECTS(dw_effects),
                crTextColor: windows::Win32::Foundation::COLORREF(cr_text_color),
                bPitchAndFamily: b_pitch_and_family,
                ..Default::default()
            };

            for i in 0..31 {
                let c = *facename_ptr.add(i);
                if c == 0 {
                    break;
                }
                cfa.szFaceName[i] = c as i8;
            }

            let em_setcharformat = 0x0444;
            SendMessageA(
                self.hwnd,
                em_setcharformat,
                WPARAM(SCF_ALL as usize),
                LPARAM(&mut cfa as *mut _ as isize),
            );

            if is_richedit20 {
                let bg_color = *((this as usize + 36) as *const u32);
                let mut lp = [cr_text_color, bg_color, dw_effects, 0];
                // Sending a custom proprietary message 0x468
                SendMessageA(
                    self.hwnd,
                    0x0468,
                    WPARAM(0),
                    LPARAM(lp.as_mut_ptr() as isize),
                );
            }
        }
    }
}

impl Drop for MSNChatEdit4 {
    /// Corresponds to sub_37225931
    fn drop(&mut self) {
        unsafe {
            let base_class = if self.is_richedit20 {
                s!("RichEdit20W")
            } else {
                s!("RICHEDIT")
            };

            // Unregister class. Assuming we just use an arbitrary null-like HINSTANCE for cleanup
            // as original code passes hModule which is likely the DLL instance handle.
            let h_instance = HINSTANCE::default();
            crate::chat45::controls::utils::superclass_window(
                h_instance,
                base_class,
                Self::CLASS_NAME,
                false,
                None,
            );

            if !self.gdiobj52.is_invalid() {
                let _ = DeleteObject(self.gdiobj52.into());
            }
            if !self.bg_brush.is_invalid() {
                let _ = DeleteObject(self.bg_brush.into());
            }
            if !self.context_menu.is_invalid() {
                let _ = DestroyMenu(self.context_menu);
            }

            if !self.hmodule.is_invalid() {
                let _ = FreeLibrary(self.hmodule);
            }
        }
    }
}
