# UTF-8 Charset Support Design Document

This document outlines the proposed changes to support UTF-8 encoding/decoding in `MsnChat45.ocx`. 

Currently, the control uses Windows-specific 8-bit code pages derived from legacy GDI Font Charsets (e.g. Western, ShiftJIS, Hangul) to convert text to and from wide strings (UTF-16) for network transmission and UI rendering. This leads to text corruption (garbling) when communicating with modern IRC clients that expect UTF-8.

We present two alternative approaches to solve this issue:
1. **Option A**: Support UTF-8 as an additional charset option (preserving legacy charset support).
2. **Option B**: Replace all charsets entirely with UTF-8.

---

## Target Code Analysis

The OCX control relies on several critical functions for character conversion and charset translation:

### 1. Character Conversion Helpers
These functions translate between wide characters (UTF-16) used internally by Windows and multi-byte strings used over the network:

*   **`sub_372124D0` (Wide -> Multi-Byte)**:
    ```c
    LPSTR __cdecl sub_372124D0(UINT CodePage, LPSTR lpMultiByteStr, LPCWCH lpWideCharStr, int cbMultiByte)
    ```
*   **`sub_37212492` (Multi-Byte -> Wide)**:
    ```c
    LPWSTR __cdecl sub_37212492(UINT CodePage, LPWSTR lpWideCharStr, LPCCH lpMultiByteStr, int cchWideChar)
    ```
*   **`sub_37205FA7` (Wide -> Multi-Byte Wrapper)**:
    ```c
    LPSTR __stdcall sub_37205FA7(LPSTR lpMultiByteStr, LPCWCH lpWideCharStr, int cbMultiByte, UINT CodePage)
    ```
*   **`sub_37205F81` (Multi-Byte -> Wide Wrapper)**:
    ```c
    LPWSTR __stdcall sub_37205F81(LPWSTR lpWideCharStr, LPCCH lpMultiByteStr, int cchWideChar, UINT CodePage)
    ```

### 2. Charset Translation Helpers
These map GDI Font Charset IDs (e.g. `128` for ShiftJIS) to Windows ANSI Code Pages (e.g. `932`):

*   **`sub_3723EF73` (Charset -> Code Page)**:
    Uses `TranslateCharsetInfo(..., TCI_SRCCHARSET)` to map GDI charsets to active code pages.
*   **`sub_3723EF49` (Code Page -> Charset)**:
    Uses `TranslateCharsetInfo(..., TCI_SRCCODEPAGE)` to map active code pages back to GDI charsets.
*   **`CodePageEnumProc` (0x3723ed97)**:
    Used in best-fit character mapping; explicitly skips UTF-8 (`65001`) and UTF-7 (`65000`) because standard GDI flags like `WC_COMPOSITECHECK` are invalid for them.

---

## Option A: Support UTF-8 as an Additional Charset

This approach adds UTF-8 to the existing list of charsets in the UI and network layer while keeping the legacy encodings intact.

### Technical Concept
Windows GDI does not have a native font charset value for UTF-8. To represent UTF-8 in the control's internal structures, we can introduce a **pseudo-charset ID** (e.g., `254` or `0xFE`). We then intercept translation routines to map this pseudo-charset to Code Page `65001` (UTF-8).

### Required Changes

1.  **Modify UI Population (`src/host/window.rs`)**:
    Add the UTF-8 option to the charset dropdown array:
    ```rust
    // src/host/window.rs:528
    let charsets = [
        (w!("Western"), 0),
        (w!("UTF-8"), 254), // Pseudo-charset ID for UTF-8
        (w!("Default"), 1),
        // ... rest of the charsets
    ];
    ```

2.  **Hook Charset-to-CodePage Translation (`sub_3723EF73`)**:
    If the input charset is `254`, bypass `TranslateCharsetInfo` and return `65001` (UTF-8):
    ```rust
    #[unsafe(no_mangle)]
    pub unsafe extern "cdecl" fn detour_sub_3723ef73(lp_src: *const u32) -> u32 {
        let charset = *lp_src;
        if charset == 254 {
            65001 // CP_UTF8
        } else {
            // Call trampoline to original sub_3723EF73
            TRAMPOLINE_SUB_3723EF73(lp_src)
        }
    }
    ```

3.  **Hook CodePage-to-Charset Translation (`sub_3723EF49`)**:
    If the input code page is `65001`, return `254`:
    ```rust
    #[unsafe(no_mangle)]
    pub unsafe extern "cdecl" fn detour_sub_3723ef49(lp_src: *const u32) -> u32 {
        let codepage = *lp_src;
        if codepage == 65001 {
            254
        } else {
            // Call trampoline
            TRAMPOLINE_SUB_3723EF49(lp_src)
        }
    }
    ```

4.  **Hook String Conversion Helpers (`sub_372124D0`, `sub_37212492`, `sub_37205FA7`, `sub_37205F81`)**:
    Ensure that when Code Page `65001` is passed, the conversion flags are compatible (since `WideCharToMultiByte` raises errors if flags other than `0` or `WC_ERR_INVALID_CHARS` are passed with `CP_UTF8`).
    ```rust
    // Example detour for sub_372124D0
    #[unsafe(no_mangle)]
    pub unsafe extern "cdecl" fn detour_sub_372124d0(
        mut codepage: u32,
        lp_multibyte_str: *mut u8,
        lp_widechar_str: *const u16,
        cb_multibyte: i32,
    ) -> *mut u8 {
        if codepage == 65001 {
            // Force safe flags: WideCharToMultiByte(CP_UTF8, 0, ...)
            *lp_multibyte_str = 0;
            windows::Win32::Globalization::WideCharToMultiByte(
                windows::Win32::Globalization::CP_UTF8,
                0, // Must use 0 for CP_UTF8 flags in legacy Windows versions
                lp_widechar_str,
                -1,
                Some(std::slice::from_raw_parts_mut(lp_multibyte_str, cb_multibyte as usize)),
                None,
                None,
            );
            lp_multibyte_str
        } else {
            TRAMPOLINE_SUB_372124D0(codepage, lp_multibyte_str, lp_widechar_str, cb_multibyte)
        }
    }
    ```

### Pros & Cons
*   **Pros**: Highly flexible; retains backward compatibility for connecting to legacy servers or channels that still communicate in regional encodings (e.g., ShiftJIS or Cyrillic).
*   **Cons**: More complex implementation (requires hooking translation procedures and managing a pseudo-charset mapping).

---

## Option B: Force UTF-8 Globally (Replace All Charsets)

This approach forces all network communications to use UTF-8 (`CP_UTF8`), effectively ignoring the user-selected charset for network translation while letting GDI render using Unicode fonts.

### Technical Concept
Rather than handling custom pseudo-charset mappings, we intercept the low-level string conversion functions and force their `CodePage` parameter to `65001`. The UI charset dropdown can either be disabled or simplified to just "UTF-8".

### Required Changes

1.  **Modify UI Population (`src/host/window.rs`)**:
    Simplify the charset selection list or force it to default to UTF-8:
    ```rust
    let charsets = [
        (w!("UTF-8"), 0), // Map to 0 (which hooks will force to 65001)
    ];
    ```

2.  **Hook String Conversion Helpers (`sub_372124D0`, `sub_37212492`, `sub_37205FA7`, `sub_37205F81`)**:
    Completely replace the conversion calls to force `CP_UTF8` and correct conversion flags:
    ```rust
    #[unsafe(no_mangle)]
    pub unsafe extern "cdecl" fn detour_sub_372124d0(
        _codepage: u32, // Ignore OCX-provided code page
        lp_multibyte_str: *mut u8,
        lp_widechar_str: *const u16,
        cb_multibyte: i32,
    ) -> *mut u8 {
        *lp_multibyte_str = 0;
        windows::Win32::Globalization::WideCharToMultiByte(
            windows::Win32::Globalization::CP_UTF8,
            0,
            lp_widechar_str,
            -1,
            Some(std::slice::from_raw_parts_mut(lp_multibyte_str, cb_multibyte as usize)),
            None,
            None,
        );
        lp_multibyte_str
    }
    ```

3.  **Hook Code Page / Charset Queries (`sub_3723EF73`, `sub_3723EF49`)**:
    Force charset translation queries to always return UTF-safe defaults (e.g., return `65001` for code page queries, and `DEFAULT_CHARSET` (1) for font selection, ensuring GDI selects Unicode-capable fonts).

### Pros & Cons
*   **Pros**: Much simpler implementation; eliminates charset-related bugs entirely; guarantees compatibility with modern IRC standards.
*   **Cons**: Legacy charsets will no longer be available for network transmission (all traffic is converted to/from UTF-8).

---

## Recommendation

We recommend **Option B (Force UTF-8 Globally)** if the control is intended to connect exclusively to modern IRC servers, as virtually all modern IRC networks expect UTF-8. 

If backward compatibility with legacy servers using regional encodings is required, **Option A** is the preferred approach.
