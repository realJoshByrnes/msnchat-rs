use std::ffi::c_void;
use windows::Win32::Graphics::Gdi::{CreateSolidBrush, GetSysColor, SYS_COLOR_INDEX};
use windows::Win32::UI::WindowsAndMessaging::DefWindowProcA;

const OFFSET_HWND_PARENT: usize = 4;
const OFFSET_UNK_18: usize = 24;
const OFFSET_DEF_WINDOW_PROC: usize = 28;

const OFFSET_CR_TEXT_COLOR: usize = 32;
const OFFSET_CR_BG_COLOR: usize = 36;
const OFFSET_ATL_FLAGS: usize = 40;
const OFFSET_WINDOW_ONLY_A: usize = 48;
const OFFSET_WINDOW_ONLY_B: usize = 52;
const OFFSET_MARGIN: usize = 56;
const OFFSET_FACE_NAME: usize = 60;
const OFFSET_BYTE_FLAGS: usize = 124;
const OFFSET_PITCH_FAMILY: usize = 140;
const OFFSET_CHARSET: usize = 144;
const OFFSET_FONT_SIZE: usize = 148;
const OFFSET_FONT_EFFECTS: usize = 152;
const OFFSET_IS_RICHEDIT20: usize = 156;

const OFFSET_CONTAINED_WINDOW: usize = 160;
const OFFSET_EVENT_SINK: usize = 200;
const OFFSET_HMODULE_SLOT: usize = 208;
const OFFSET_BG_BRUSH_SLOT: usize = 212;
const OFFSET_CONTEXT_MENU_SLOT: usize = 216;

const OFFSET_STATE_COUNT: usize = 300;
const OFFSET_STATE_INDEX: usize = 304;
const OFFSET_TRACKING_A: usize = 820;
const OFFSET_TRACKING_B: usize = 824;

const CONTAINED_OFFSET_CLASS_NAME: usize = 0x14;
const CONTAINED_OFFSET_SUPER_WNDPROC: usize = 0x18;
const CONTAINED_OFFSET_OBJECT_PTR: usize = 0x1C;
const CONTAINED_OFFSET_MSG_MAP_ID: usize = 0x20;
const CONTAINED_OFFSET_CURRENT_MSG: usize = 0x24;

const ATL_FLAG_INTERNAL_BITMASK: u32 = 46_976_204;
const DEFAULT_MARGIN: i32 = 2;
const DEFAULT_PITCH_FAMILY: u8 = 1;
const DEFAULT_CHARSET_ANSI_1252: i32 = 1252;
const DEFAULT_FONT_SIZE_PT: i32 = 9;

/// Handles the exact byte-level memory initialization of the MSNChatEdit4 C++ object.
///
/// MSNChatEdit4 relies on an ATL `CComControlBase` and `CContainedWindow` layout.
/// Because we bypassed the native C++ constructor to prevent early crashes during OLE
/// activation, we must manually reconstruct the expected memory state. Without these
/// specific flags (like `m_bWindowOnly` at offset 48) and the `CContainedWindow`
/// initialization offsets, the container refuses to render the child HWND.
pub struct MemoryLayout;

impl MemoryLayout {
    /// Applies the expected memory layout to the raw C++ object pointer.
    ///
    /// # Safety
    /// `this` must be a valid pointer to the allocated `MSNChatEdit4` object instance.
    pub unsafe fn apply(this: *mut c_void) {
        unsafe {
            let base = this as usize;

            // VTable is initialized externally before calling this.

            *((base + OFFSET_HWND_PARENT) as *mut u32) = 0;
            *((base + OFFSET_UNK_18) as *mut u32) = 0;
            *((base + OFFSET_DEF_WINDOW_PROC) as *mut usize) = DefWindowProcA as usize;

            // CContainedWindow initialization at this+160 (offset 40 DWORDs)
            let contained_base = base + OFFSET_CONTAINED_WINDOW;
            *((contained_base) as *mut u32) = 0; // m_hWnd
            *((contained_base + CONTAINED_OFFSET_CURRENT_MSG) as *mut u32) = 0; // m_pCurrentMsg

            static MSN_CLASS: &[u8] = b"MSNChatEdit4\0";
            *((contained_base + CONTAINED_OFFSET_CLASS_NAME) as *mut usize) =
                MSN_CLASS.as_ptr() as usize; // m_lpszClassName
            *((contained_base + CONTAINED_OFFSET_SUPER_WNDPROC) as *mut usize) =
                DefWindowProcA as usize; // m_pfnSuperWindowProc
            *((contained_base + CONTAINED_OFFSET_OBJECT_PTR) as *mut usize) = base; // m_pObject
            *((contained_base + CONTAINED_OFFSET_MSG_MAP_ID) as *mut u32) = 1; // m_dwMsgMapID

            *((base + OFFSET_EVENT_SINK) as *mut u32) = 0;
            *((base + OFFSET_HMODULE_SLOT) as *mut u32) = 0;

            // COLOR_WINDOW (5) - Background brush and color
            let bg_color = GetSysColor(SYS_COLOR_INDEX(5));
            *((base + OFFSET_BG_BRUSH_SLOT) as *mut *mut std::ffi::c_void) =
                CreateSolidBrush(windows::Win32::Foundation::COLORREF(bg_color)).0;
            *((base + OFFSET_CONTEXT_MENU_SLOT) as *mut u32) = 0;

            // COLOR_WINDOWTEXT (8) - Text color
            let text_color = GetSysColor(SYS_COLOR_INDEX(8));
            *((base + OFFSET_CR_TEXT_COLOR) as *mut u32) = text_color;
            *((base + OFFSET_CR_BG_COLOR) as *mut u32) = bg_color;

            // Critical OLE Window/Activation flags (e.g., m_bWindowOnly)
            *((base + OFFSET_WINDOW_ONLY_A) as *mut u32) = 1;
            *((base + OFFSET_WINDOW_ONLY_B) as *mut u32) = 1;
            *((base + OFFSET_ATL_FLAGS) as *mut u32) = ATL_FLAG_INTERNAL_BITMASK;

            // Control Margins
            *((base + OFFSET_MARGIN) as *mut i32) = DEFAULT_MARGIN;

            // Font Face (UTF-16 "Arial")
            let font_face = "Arial\0".encode_utf16().collect::<Vec<u16>>();
            let font_face_ptr = (base + OFFSET_FACE_NAME) as *mut u16;
            for (i, &c) in font_face.iter().enumerate() {
                *font_face_ptr.add(i) = c;
            }

            // Pitch and Family
            *((base + OFFSET_PITCH_FAMILY) as *mut u8) = DEFAULT_PITCH_FAMILY;

            // Charset (1252 = ANSI_CHARSET / Western European)
            *((base + OFFSET_CHARSET) as *mut i32) = DEFAULT_CHARSET_ANSI_1252;

            // Font Size
            *((base + OFFSET_FONT_SIZE) as *mut i32) = DEFAULT_FONT_SIZE_PT;

            // Font Effects (bold, italic, etc.)
            *((base + OFFSET_FONT_EFFECTS) as *mut u32) = 0;

            // 1-byte flags
            *((base + OFFSET_BYTE_FLAGS) as *mut u8) = 0;

            // is_richedit20 flag (0x9C)
            *((base + OFFSET_IS_RICHEDIT20) as *mut u32) = 1;

            // Other zero-initialized state tracking fields
            *((base + OFFSET_STATE_COUNT) as *mut u32) = 0;
            *((base + OFFSET_STATE_INDEX) as *mut u32) = 0;
            *((base + OFFSET_TRACKING_A) as *mut u32) = 0;
            *((base + OFFSET_TRACKING_B) as *mut u32) = 0;
        }
    }
}
