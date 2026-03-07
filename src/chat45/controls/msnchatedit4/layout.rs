use std::ffi::c_void;
use windows::Win32::Graphics::Gdi::{CreateSolidBrush, GetSysColor, SYS_COLOR_INDEX};
use windows::Win32::UI::WindowsAndMessaging::DefWindowProcA;

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

            *((base + 4) as *mut u32) = 0;
            *((base + 24) as *mut u32) = 0;
            *((base + 28) as *mut usize) = DefWindowProcA as usize;

            // CContainedWindow initialization at this+160 (offset 40 DWORDs)
            let contained_base = base + 160;
            *((contained_base) as *mut u32) = 0; // m_hWnd
            *((contained_base + 0x24) as *mut u32) = 0; // m_pCurrentMsg

            static MSN_CLASS: &[u8] = b"MSNChatEdit4\0";
            *((contained_base + 0x14) as *mut usize) = MSN_CLASS.as_ptr() as usize; // m_lpszClassName
            *((contained_base + 0x18) as *mut usize) = DefWindowProcA as usize; // m_pfnSuperWindowProc
            *((contained_base + 0x1C) as *mut usize) = base; // m_pObject
            *((contained_base + 0x20) as *mut u32) = 1; // m_dwMsgMapID

            *((base + 200) as *mut u32) = 0;
            *((base + 208) as *mut u32) = 0;

            // COLOR_WINDOW (5) - Background brush and color
            let bg_color = GetSysColor(SYS_COLOR_INDEX(5));
            *((base + 212) as *mut *mut std::ffi::c_void) =
                CreateSolidBrush(windows::Win32::Foundation::COLORREF(bg_color)).0;
            *((base + 216) as *mut u32) = 0;

            // COLOR_WINDOWTEXT (8) - Text color
            let text_color = GetSysColor(SYS_COLOR_INDEX(8));
            *((base + 32) as *mut u32) = text_color;
            *((base + 36) as *mut u32) = bg_color;

            // Critical OLE Window/Activation flags (e.g., m_bWindowOnly)
            *((base + 48) as *mut u32) = 1;
            *((base + 52) as *mut u32) = 1;
            *((base + 40) as *mut u32) = 46976204; // Internal bitmask ATL flag

            // Control Margins
            *((base + 56) as *mut i32) = 2;

            // Font Face (UTF-16 "Arial")
            let font_face = "Arial\0".encode_utf16().collect::<Vec<u16>>();
            let font_face_ptr = (base + 60) as *mut u16;
            for (i, &c) in font_face.iter().enumerate() {
                *font_face_ptr.add(i) = c;
            }

            // Pitch and Family
            *((base + 140) as *mut u8) = 1;

            // Charset (1252 = ANSI_CHARSET / Western European)
            *((base + 144) as *mut i32) = 1252;

            // Font Size
            *((base + 148) as *mut i32) = 9;

            // Font Effects (bold, italic, etc.)
            *((base + 152) as *mut u32) = 0;

            // 1-byte flags
            *((base + 124) as *mut u8) = 0;

            // is_richedit20 flag (0x9C)
            *((base + 156) as *mut u32) = 1;

            // Other zero-initialized state tracking fields
            *((base + 300) as *mut u32) = 0;
            *((base + 304) as *mut u32) = 0;
            *((base + 820) as *mut u32) = 0;
            *((base + 824) as *mut u32) = 0;
        }
    }
}
