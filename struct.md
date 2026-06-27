---

## 6. Integration into the Global Struct

To manage the life-cycle and coordinate settings/frames, we define how these interfaces integrate into a global container/session struct.

This layout maps to our existing container implementation (e.g., `SharedSiteState` in [mod.rs](file:///c:/Users/jd/Desktop/MSN%20Chat%20Control/redmond-chat/src/host/site/mod.rs)):

```rust
use windows::Win32::Foundation::HWND;

#[repr(C)]
pub struct MsnChatActiveXInstance {
    /// Window handle of the hosted control parent or container
    pub hwnd: HWND,
    
    /// Reference count of the overall container site
    pub ref_count: u32,
    
    /// Pointer to the hosted control's `IChatFrame` interface (if queried/active)
    pub chat_frame: *mut IChatFrame,
    
    /// Pointer to the hosted control's `IChatSettings` interface (if queried/active)
    pub chat_settings: *mut IChatSettings,
    
    /// Event sink for outgoing callbacks from the Chat Control
    pub event_sink: *mut ICChatFrameEvents,
}
```
---

## 3. IChatSettings Interface (`ChatSettings` CoClass)

* **Interface GUID**: `D5EF4299-12F1-474D-98C5-3C658FD2E343`
* **CoClass GUID**: `FA980E7E-9E44-4D2F-B3C2-9A5BE42525F8`
* **VTable RVA**: `0x001D90`

### Property Constraints & Offsets

| Property Name | VTable Get Slot | VTable Put Slot | Internal Struct Offset | Type | Notes / Constraints |
| :--- | :---: | :---: | :---: | :---: | :--- |
| **BackColor** | Slot 8 (`0x20`) | Slot 7 (`0x1C`) | `148` (`0x94`) | `OLE_COLOR` | Triggers invalidate/repaint on set. |
| **ForeColor** | Slot 10 (`0x28`) | Slot 9 (`0x24`) | `152` (`0x98`) | `OLE_COLOR` | Triggers invalidate/repaint on set. |
| **RedirectURL** | Slot 12 (`0x30`) | Slot 11 (`0x2C`) | `156` (`0x9C`) | `BSTR` | Max 512 chars. |
| **ResDLL** | Slot 14 (`0x38`) | Slot 13 (`0x34`) | `160` (`0xA0`) | `BSTR` | Max 512 chars. |

---

## 4. Rust Representation of the Internal MSN Chat Structs

We can model the internal control structures representing the memory layouts of `MSNChatFrame` and `ChatSettings` instances inside `MsnChat45.ocx`. 

### ChatSettingsInstance Layout

```rust
#[repr(C)]
pub struct ChatSettingsInstance {
    /// Pointer to the IChatSettings virtual table
    pub lp_vtbl: *const IChatSettings_Vtbl,
    
    // Internal state padding up to flags
    pub padding1: [u8; 72],
    pub flags: u8,                // Offset 72 (e.g. BackColor/ForeColor dirty flags)
    
    // Alignment padding up to properties
    pub padding2: [u8; 75],       
    pub back_color: OleColor,     // Offset 148 (0x94)
    pub fore_color: OleColor,     // Offset 152 (0x98)
    pub redirect_url: BSTR,       // Offset 156 (0x9C)
    pub res_dll: BSTR,            // Offset 160 (0xA0)
}
```

### ChatFrameInstance Layout

See [Section 5: ChatFrameInstance Layout](#chatframeinstance-layout) below for the complete, fully-resolved memory layout of the `ChatFrameInstance` structure.
```

### Rust VTable Struct Definitions

```rust
#[repr(C)]
pub struct IChatFrame_Vtbl {
    // 0-6: IDispatch Base Methods
    pub base: IDispatch_Vtbl,

    // 7-80: Control Properties Getters and Setters
    pub put_BackColor: unsafe extern "system" fn(this: *mut c_void, color: OleColor) -> HRESULT,
    pub get_BackColor: unsafe extern "system" fn(this: *mut c_void, color: *mut OleColor) -> HRESULT,
    pub get_RoomName: unsafe extern "system" fn(this: *mut c_void, name: *mut BSTR) -> HRESULT,
    pub put_RoomName: unsafe extern "system" fn(this: *mut c_void, name: BSTR) -> HRESULT,
    pub get_HexRoomName: unsafe extern "system" fn(this: *mut c_void, name: *mut BSTR) -> HRESULT,
    pub put_HexRoomName: unsafe extern "system" fn(this: *mut c_void, name: BSTR) -> HRESULT,
    pub get_NickName: unsafe extern "system" fn(this: *mut c_void, name: *mut BSTR) -> HRESULT,
    pub put_NickName: unsafe extern "system" fn(this: *mut c_void, name: BSTR) -> HRESULT,
    pub get_Server: unsafe extern "system" fn(this: *mut c_void, server: *mut BSTR) -> HRESULT,
    pub put_Server: unsafe extern "system" fn(this: *mut c_void, server: BSTR) -> HRESULT,
    pub get_BackHighlightColor: unsafe extern "system" fn(this: *mut c_void, color: *mut OleColor) -> HRESULT,
    pub put_BackHighlightColor: unsafe extern "system" fn(this: *mut c_void, color: OleColor) -> HRESULT,
    pub get_ButtonFrameColor: unsafe extern "system" fn(this: *mut c_void, color: *mut OleColor) -> HRESULT,
    pub put_ButtonFrameColor: unsafe extern "system" fn(this: *mut c_void, color: OleColor) -> HRESULT,
    pub get_TopBackHighlightColor: unsafe extern "system" fn(this: *mut c_void, color: *mut OleColor) -> HRESULT,
    pub put_TopBackHighlightColor: unsafe extern "system" fn(this: *mut c_void, color: OleColor) -> HRESULT,
    pub get_ChatMode: unsafe extern "system" fn(this: *mut c_void, mode: *mut i32) -> HRESULT,
    pub put_ChatMode: unsafe extern "system" fn(this: *mut c_void, mode: i32) -> HRESULT,
    pub get_URLBack: unsafe extern "system" fn(this: *mut c_void, url: *mut BSTR) -> HRESULT,
    pub put_URLBack: unsafe extern "system" fn(this: *mut c_void, url: BSTR) -> HRESULT,
    pub get_Category: unsafe extern "system" fn(this: *mut c_void, category: *mut BSTR) -> HRESULT,
    pub put_Category: unsafe extern "system" fn(this: *mut c_void, category: BSTR) -> HRESULT,
    pub get_Topic: unsafe extern "system" fn(this: *mut c_void, topic: *mut BSTR) -> HRESULT,
    pub put_Topic: unsafe extern "system" fn(this: *mut c_void, topic: BSTR) -> HRESULT,
    pub get_WelcomeMsg: unsafe extern "system" fn(this: *mut c_void, msg: *mut BSTR) -> HRESULT,
    pub put_WelcomeMsg: unsafe extern "system" fn(this: *mut c_void, msg: BSTR) -> HRESULT,
    pub get_BaseURL: unsafe extern "system" fn(this: *mut c_void, url: *mut BSTR) -> HRESULT,
    pub put_BaseURL: unsafe extern "system" fn(this: *mut c_void, url: BSTR) -> HRESULT,
    pub get_InputBorderColor: unsafe extern "system" fn(this: *mut c_void, color: *mut OleColor) -> HRESULT,
    pub put_InputBorderColor: unsafe extern "system" fn(this: *mut c_void, color: OleColor) -> HRESULT,
    pub get_CreateRoom: unsafe extern "system" fn(this: *mut c_void, room: *mut BSTR) -> HRESULT,
    pub put_CreateRoom: unsafe extern "system" fn(this: *mut c_void, room: BSTR) -> HRESULT,
    pub get_ChatHome: unsafe extern "system" fn(this: *mut c_void, home: *mut BSTR) -> HRESULT,
    pub put_ChatHome: unsafe extern "system" fn(this: *mut c_void, home: BSTR) -> HRESULT,
    pub get_Locale: unsafe extern "system" fn(this: *mut c_void, locale: *mut BSTR) -> HRESULT,
    pub put_Locale: unsafe extern "system" fn(this: *mut c_void, locale: BSTR) -> HRESULT,
    pub get_ResDLL: unsafe extern "system" fn(this: *mut c_void, dll: *mut BSTR) -> HRESULT,
    pub put_ResDLL: unsafe extern "system" fn(this: *mut c_void, dll: BSTR) -> HRESULT,
    pub get_ButtonTextColor: unsafe extern "system" fn(this: *mut c_void, color: *mut OleColor) -> HRESULT,
    pub put_ButtonTextColor: unsafe extern "system" fn(this: *mut c_void, color: OleColor) -> HRESULT,
    pub get_ButtonBackColor: unsafe extern "system" fn(this: *mut c_void, color: *mut OleColor) -> HRESULT,
    pub put_ButtonBackColor: unsafe extern "system" fn(this: *mut c_void, color: OleColor) -> HRESULT,
    pub get_PassportTicket: unsafe extern "system" fn(this: *mut c_void, ticket: *mut BSTR) -> HRESULT,
    pub put_PassportTicket: unsafe extern "system" fn(this: *mut c_void, ticket: BSTR) -> HRESULT,
    pub get_PassportProfile: unsafe extern "system" fn(this: *mut c_void, profile: *mut BSTR) -> HRESULT,
    pub put_PassportProfile: unsafe extern "system" fn(this: *mut c_void, profile: BSTR) -> HRESULT,
    pub get_Feature: unsafe extern "system" fn(this: *mut c_void, feature: *mut u32) -> HRESULT,
    pub put_Feature: unsafe extern "system" fn(this: *mut c_void, feature: u32) -> HRESULT,
    pub get_MessageOfTheDay: unsafe extern "system" fn(this: *mut c_void, motd: *mut BSTR) -> HRESULT,
    pub put_MessageOfTheDay: unsafe extern "system" fn(this: *mut c_void, motd: BSTR) -> HRESULT,
    pub get_ChannelLanguage: unsafe extern "system" fn(this: *mut c_void, lang: *mut BSTR) -> HRESULT,
    pub put_ChannelLanguage: unsafe extern "system" fn(this: *mut c_void, lang: BSTR) -> HRESULT,
    pub get_InvitationCode: unsafe extern "system" fn(this: *mut c_void, code: *mut BSTR) -> HRESULT,
    pub put_InvitationCode: unsafe extern "system" fn(this: *mut c_void, code: BSTR) -> HRESULT,
    pub get_NicknameToInvite: unsafe extern "system" fn(this: *mut c_void, nick: *mut BSTR) -> HRESULT,
    pub put_NicknameToInvite: unsafe extern "system" fn(this: *mut c_void, nick: BSTR) -> HRESULT,
    pub get_MSNREGCookie: unsafe extern "system" fn(this: *mut c_void, cookie: *mut BSTR) -> HRESULT,
    pub put_MSNREGCookie: unsafe extern "system" fn(this: *mut c_void, cookie: BSTR) -> HRESULT,
    pub get_CreationModes: unsafe extern "system" fn(this: *mut c_void, modes: *mut BSTR) -> HRESULT,
    pub put_CreationModes: unsafe extern "system" fn(this: *mut c_void, modes: BSTR) -> HRESULT,
    pub get_MSNProfile: unsafe extern "system" fn(this: *mut c_void, profile: *mut BSTR) -> HRESULT,
    pub put_MSNProfile: unsafe extern "system" fn(this: *mut c_void, profile: BSTR) -> HRESULT,
    pub get_Market: unsafe extern "system" fn(this: *mut c_void, market: *mut BSTR) -> HRESULT,
    pub put_Market: unsafe extern "system" fn(this: *mut c_void, market: BSTR) -> HRESULT,
    pub get_WhisperContent: unsafe extern "system" fn(this: *mut c_void, content: *mut BSTR) -> HRESULT,
    pub put_WhisperContent: unsafe extern "system" fn(this: *mut c_void, content: BSTR) -> HRESULT,
    pub get_UserRole: unsafe extern "system" fn(this: *mut c_void, role: *mut BSTR) -> HRESULT,
    pub put_UserRole: unsafe extern "system" fn(this: *mut c_void, role: BSTR) -> HRESULT,
    pub get_AuditMessage: unsafe extern "system" fn(this: *mut c_void, msg: *mut BSTR) -> HRESULT,
    pub put_AuditMessage: unsafe extern "system" fn(this: *mut c_void, msg: BSTR) -> HRESULT,
    pub get_SubscriberInfo: unsafe extern "system" fn(this: *mut c_void, info: *mut BSTR) -> HRESULT,
    pub put_SubscriberInfo: unsafe extern "system" fn(this: *mut c_void, info: BSTR) -> HRESULT,
    pub get_UpsellURL: unsafe extern "system" fn(this: *mut c_void, url: *mut BSTR) -> HRESULT,
    pub put_UpsellURL: unsafe extern "system" fn(this: *mut c_void, url: BSTR) -> HRESULT,
}

#[repr(C)]
pub struct IChatFrame {
    pub lp_vtbl: *const IChatFrame_Vtbl,
}
```

---

## 3. IChatSettings Interface (`ChatSettings` CoClass)

* **Interface GUID**: `D5EF4299-12F1-474D-98C5-3C658FD2E343`
* **CoClass GUID**: `FA980E7E-9E44-4D2F-B3C2-9A5BE42525F8`
* **VTable RVA**: `0x001D90`

### Property Constraints & Offsets

| Property Name | VTable Get Slot | VTable Put Slot | Internal Struct Offset | Type | Notes / Constraints |
| :--- | :---: | :---: | :---: | :---: | :--- |
| **BackColor** | Slot 8 (`0x20`) | Slot 7 (`0x1C`) | `148` (`0x94`) | `OLE_COLOR` | Triggers invalidate/repaint on set. |
| **ForeColor** | Slot 10 (`0x28`) | Slot 9 (`0x24`) | `152` (`0x98`) | `OLE_COLOR` | Triggers invalidate/repaint on set. |
| **RedirectURL** | Slot 12 (`0x30`) | Slot 11 (`0x2C`) | `156` (`0x9C`) | `BSTR` | Max 512 chars. |
| **ResDLL** | Slot 14 (`0x38`) | Slot 13 (`0x34`) | `160` (`0xA0`) | `BSTR` | Max 512 chars. |

### Rust VTable Struct Definitions

```rust
#[repr(C)]
pub struct IChatSettings_Vtbl {
    // 0-6: IDispatch Base Methods
    pub base: IDispatch_Vtbl,

    // 7-14: Control Properties Getters and Setters
    pub put_BackColor: unsafe extern "system" fn(this: *mut c_void, color: OleColor) -> HRESULT,
    pub get_BackColor: unsafe extern "system" fn(this: *mut c_void, color: *mut OleColor) -> HRESULT,
    pub put_ForeColor: unsafe extern "system" fn(this: *mut c_void, color: OleColor) -> HRESULT,
    pub get_ForeColor: unsafe extern "system" fn(this: *mut c_void, color: *mut OleColor) -> HRESULT,
    pub put_RedirectURL: unsafe extern "system" fn(this: *mut c_void, url: BSTR) -> HRESULT,
    pub get_RedirectURL: unsafe extern "system" fn(this: *mut c_void, url: *mut BSTR) -> HRESULT,
    pub put_ResDLL: unsafe extern "system" fn(this: *mut c_void, dll: BSTR) -> HRESULT,
    pub get_ResDLL: unsafe extern "system" fn(this: *mut c_void, dll: *mut BSTR) -> HRESULT,
}

#[repr(C)]
pub struct IChatSettings {
    pub lp_vtbl: *const IChatSettings_Vtbl,
}
```

---

## 4. _ICChatFrameEvents Interface (Event Sink)

* **Interface GUID**: `5EEB8014-53B2-448B-9F3B-C553424832E1`
* **Type**: Outgoing Source Interface

Because event interfaces are outgoing (implemented by the client container), the client defines the structure to handle these callbacks.

```rust
#[repr(C)]
pub struct ICChatFrameEvents_Vtbl {
    pub base: IUnknown_Vtbl,
    pub OnRedirect: unsafe extern "system" fn(this: *mut c_void, url: BSTR) -> HRESULT,
}

#[repr(C)]
pub struct ICChatFrameEvents {
    pub lp_vtbl: *const ICChatFrameEvents_Vtbl,
}
```

---

## 5. Rust Representation of the Internal MSN Chat Structs

We can model the internal control structures representing the memory layouts of `MSNChatFrame` and `ChatSettings` instances inside `MsnChat45.ocx`, including the ATL base class `CComControlBase`.

### CComControlBase Layout (ATL Base Class)

Size: 72 bytes.

```rust
#[repr(C)]
pub struct CComControlBase {
    pub m_spAdviseSink: *mut c_void,               // Offset 0
    pub m_spInPlaceSiteWindowless: *mut c_void,    // Offset 4
    pub m_spClientSite: *mut c_void,               // Offset 8
    pub m_spDataAdviseHolder: *mut c_void,         // Offset 12
    pub m_spOleAdviseHolder: *mut c_void,          // Offset 16
    pub m_spInPlaceSite: *mut c_void,              // Offset 20
    pub m_spDataObject: *mut c_void,               // Offset 24
    pub m_spActiveIPObject: *mut c_void,           // Offset 28
    pub m_pcontrolBorder: *mut c_void,             // Offset 32
    pub m_dwSafety: u32,                           // Offset 36
    pub m_spPropertyNotifySink: *mut c_void,       // Offset 40
    pub m_sizeExtent_cx: u32,                      // Offset 44
    pub m_sizeExtent_cy: u32,                      // Offset 48
    pub m_sizeNatural_cx: u32,                     // Offset 52
    pub m_sizeNatural_cy: u32,                     // Offset 56
    pub m_phwnd: *mut *mut c_void,                 // Offset 60
    pub m_dwBrandNewFlag: u32,                     // Offset 64
    pub m_flags: u32,                              // Offset 68
}
```


### ChatSettingsInstance Layout

```rust
#[repr(C)]
pub struct ChatSettingsInstance {
    /// Pointer to the IChatSettings virtual table
    pub lp_vtbl: *const IChatSettings_Vtbl,
    
    /// ATL Control Base class variables
    pub control_base: CComControlBase,            // Offset 4 (ends at 76)
    
    pub flags: u8,                                // Offset 76 (e.g. BackColor/ForeColor dirty flags)
    pub pad_align1: [u8; 3],                      // Alignment to 4-byte boundary
    
    // CWindowImpl variables
    pub m_pfnSuperWindowProc: *mut c_void,        // Offset 80
    pub m_hwnd: HWND,                             // Offset 84
    pub m_thunk: [u8; 12],                        // Offset 88 (ATL window proc assembly thunk)
    
    // Additional COM interface VTable pointers implemented by ChatSettings
    pub other_vtables: [*const c_void; 11],       // Offset 100 (44 bytes, e.g. IOleObject, IViewObject)
    
    pub pad_align2: [u8; 4],                      // Alignment padding up to properties (Offset 144)
    pub back_color: OleColor,                     // Offset 148 (0x94)
    pub fore_color: OleColor,                     // Offset 152 (0x98)
    pub redirect_url: BSTR,                       // Offset 156 (0x9C)
    pub res_dll: BSTR,                            // Offset 160 (0xA0)
}
```


### ChatFrameInstance Layout

```rust
#[repr(C)]
pub struct MsnChatString {
    pub ptr: *mut c_void,
    pub len: i32,
}

#[repr(C)]
pub struct RTL_CRITICAL_SECTION {
    pub debug_info: *mut c_void,
    pub lock_count: i32,
    pub recursion_count: i32,
    pub owning_thread: *mut c_void,
    pub lock_semaphore: *mut c_void,
    pub spin_count: usize,
}

#[repr(C)]
pub struct ChatRichEdit {
    pub padding: [u8; 136],
    pub m_hwnd: HWND,                             // Offset 136 (0x88)
    pub padding_end: [u8; 396],
}

#[repr(C)]
pub struct ChatListView {
    pub padding: [u8; 504],
}

#[repr(C)]
pub struct ChatEdit {
    pub padding: [u8; 828],
}

#[repr(C)]
pub struct ChatFrameInstance {
    /// Pointer to the IChatFrame virtual table
    pub lp_vtbl: *const IChatFrame_Vtbl,
    
    /// ATL Control Base class variables
    pub control_base: CComControlBase,            // Offset 4 (ends at 76)
    
    pub flags: u8,                                // Offset 76 (e.g. BackColor/ForeColor dirty flags)
    pub pad_align1: [u8; 3],                      // Alignment to 4-byte boundary
    
    // CWindowImpl variables
    pub m_pfnSuperWindowProc: *mut c_void,        // Offset 80
    pub m_hwnd: HWND,                             // Offset 84
    pub m_thunk: [u8; 12],                        // Offset 88 (ATL window proc assembly thunk)
    
    // Additional COM interface VTable pointers implemented by ChatFrame
    pub other_vtables: [*const c_void; 11],       // Offset 100 (44 bytes, e.g. IOleObject, IViewObject)
    
    pub padding_pre_sock: [u8; 64],               // Offset 144 to 208
    pub connection_session: CChatSock,            // Offset 208 (size 16816)
    pub padding_post_sock_1: [u8; 44],            // Offset 17024 to 17068
    pub nick_list: ChatListView,                  // Offset 17068 (size 504)
    pub chat_input: ChatEdit,                     // Offset 17572 (size 828)
    pub chat_output: ChatRichEdit,                // Offset 18400 (size 536)
    pub chat_special_output: ChatRichEdit,        // Offset 18936 (size 536)
    pub padding_post_rich: [u8; 792],             // Offset 19472 to 20264
    
    // Internal initialization flags and variables at the end of the massive block
    pub unknown_val_20264: i32,                   // Offset 20264 (0x4F28)
    pub show_arrivals: i32,                       // Offset 20268 (0x4F2C)
    pub show_departures: i32,                     // Offset 20272 (0x4F30)
    pub show_activity: i32,                       // Offset 20276 (0x4F34)
    pub disable_invites: i32,                     // Offset 20280 (0x4F38)
    pub disable_urls: i32,                        // Offset 20284 (0x4F3C)
    pub show_emoticons: i32,                      // Offset 20288 (0x4F40)
    pub unknown_val_20292: i32,                   // Offset 20292 (0x4F44)
    pub unknown_val_20296: i32,                   // Offset 20296 (0x4F4C)
    pub unknown_val_20300: i32,                   // Offset 20300 (0x4F50)
    pub unknown_val_20304: u8,                    // Offset 20304 (0x4F54)
    pub pad_align_20305: [u8; 3],                 // Offset 20305 (0x4F55)
    pub unknown_val_20308: i32,                   // Offset 20308 (0x4F58)
    pub unknown_val_20312: i32,                   // Offset 20312 (0x4F5C)
    pub unknown_val_20316: i32,                   // Offset 20316 (0x4F60)
    pub unknown_val_20320: i32,                   // Offset 20320 (0x4F64)
    pub unknown_val_20324: i32,                   // Offset 20324 (0x4F68)
    pub unknown_val_20328: u8,                    // Offset 20328 (0x4F6C)
    pub unknown_val_20329: u8,                    // Offset 20329 (0x4F6D)
    pub pad_align_20330: [u8; 2],                 // Offset 20330 (0x4F6E)
    pub unknown_val_20332: i32,                   // Offset 20332 (0x4F70)
    pub unknown_val_20336: i32,                   // Offset 20336 (0x4F74)
    pub unknown_val_20340: i32,                   // Offset 20340 (0x4F78)
    pub unknown_val_20344: i32,                   // Offset 20344 (0x4F7C)
    pub unknown_val_20348: i32,                   // Offset 20348 (0x4F80)
    pub unknown_val_20352: i32,                   // Offset 20352 (0x4F84)
    pub unknown_val_20356: i32,                   // Offset 20356 (0x4F88)
    pub unknown_val_20360: i32,                   // Offset 20360 (0x4F8C)
    pub unknown_val_20364: i32,                   // Offset 20364 (0x4F90)

    
    // Properties area starting from offset 20368
    pub room_name_bstr: MsnChatString,            // Offset 20368 (0x4F90) - converted to MsnChatString
    pub room_name_conv: MsnChatString,            // Offset 20376 (0x4F98) - pointer and length of converted room name
    pub room_name_is_valid: i32,                  // Offset 20384 (0x4FA0)
    pub topic: MsnChatString,                     // Offset 20388 (0x4FA4)
    pub server_host: MsnChatString,               // Offset 20396 (0x4FAC)
    pub server_host_conv: i32,                    // Offset 20404 (0x4FB4)
    pub server_port: u32,                         // Offset 20408 (0x4FB8)
    pub nickname: BSTR,                           // Offset 20412 (0x4FBC)
    pub category: MsnChatString,                  // Offset 20416 (0x4FC0)
    pub locale: MsnChatString,                    // Offset 20424 (0x4FC8)
    pub channel_language: i32,                    // Offset 20432 (0x4FD0)
    pub welcome_msg: BSTR,                        // Offset 20436 (0x4FD4)
    pub base_url: BSTR,                           // Offset 20440 (0x4FD8)
    pub market: BSTR,                             // Offset 20444 (0x4FDC)
    pub url_back: BSTR,                           // Offset 20448 (0x4FE0)
    pub create_room: BSTR,                        // Offset 20452 (0x4FE4)
    pub chat_home: BSTR,                          // Offset 20456 (0x4FE8)
    pub upsell_url: BSTR,                         // Offset 20460 (0x4FEC)
    pub whisper_content: BSTR,                    // Offset 20464 (0x4FF0)
    pub padding8: i32,                            // Offset 20468 (0x4FF4)
    pub message_of_the_day: BSTR,                 // Offset 20472 (0x4FF8)
    pub audit_message: BSTR,                      // Offset 20476 (0x4FFC)
    
    // Auth & profile settings
    pub padding9: [u32; 12],                      // Offset 20480 (0x5000)
    pub passport_ticket: MsnChatString,           // Offset 20528 (0x5030)
    pub passport_profile: MsnChatString,          // Offset 20536 (0x5038)
    pub msn_reg_cookie: MsnChatString,            // Offset 20544 (0x5040)
    pub msn_profile: MsnChatString,               // Offset 20552 (0x5048)
    pub subscriber_info: MsnChatString,           // Offset 20560 (0x5050)
    pub creation_modes: MsnChatString,            // Offset 20568 (0x5058)
    pub creation_modes_bitmask: u32,              // Offset 20576 (0x5060)
    pub user_role: MsnChatString,                 // Offset 20580 (0x5064)
    pub critical_section: RTL_CRITICAL_SECTION,   // Offset 20588 (0x506C)
    pub unknown_val_20612: i32,                   // Offset 20612 (0x5084)
    
    // Color configurations
    pub back_color: OleColor,                     // Offset 20616 (0x5088)
    pub back_highlight_color: OleColor,           // Offset 20620 (0x508C)
    pub top_back_highlight_color: OleColor,       // Offset 20624 (0x5090)
    pub input_border_color: OleColor,             // Offset 20628 (0x5094)
    pub button_text_color: OleColor,              // Offset 20632 (0x5098)
    pub button_back_color: OleColor,              // Offset 20636 (0x509C)
    pub button_frame_color: OleColor,             // Offset 20640 (0x50A0)
    pub button_text_highlight_color: OleColor,    // Offset 20644 (0x50A4)
    
    // ResDLL configuration and other state
    pub unknown_val_20648: i32,                   // Offset 20648 (0x50A8)
    pub ignore_fonts: i32,                        // Offset 20652 (0x50AC)
    pub font_style: i32,                          // Offset 20656 (0x50B0)
    pub font_name: BSTR,                          // Offset 20660 (0x50B4)
    pub font_color: OleColor,                     // Offset 20664 (0x50B8)
    pub font_size: i32,                           // Offset 20668 (0x50BC)
    pub res_dll_url: BSTR,                        // Offset 20672 (0x50C0)
    pub res_dll_loaded: u8,                       // Offset 20676 (0x50C4)
    pub res_dll_something: u8,                    // Offset 20677 (0x50C5)
    pub pad_align_res_dll: [u8; 2],               // Offset 20678 (0x50C6)
    pub res_dll_timeout: u32,                     // Offset 20680 (0x50C8)
    pub some_flag_20684: u8,                      // Offset 20684 (0x50CC)
    pub res_dll_path: [u8; 260],                  // Offset 20685 (0x50CD)
    pub pad_align_res_dll2: [u8; 3],              // Offset 20945 (0x51D1)
    pub urlmon_hmodule: *mut c_void,              // Offset 20948 (0x51D4)
    pub pull_stream_cb: *mut c_void,              // Offset 20952 (0x51D8)
    pub chat_mode: i32,                           // Offset 20956 (0x51DC)
    
    // Remaining trailing fields (old padding17)
    pub unknown_val_20960: i32,                   // Offset 20960 (0x51E0)
    pub registry_manager: *mut MsnChatRegistryManager, // Offset 20964 (0x51E4)
    pub unknown_val_20968: i32,                   // Offset 20968 (0x51E8)
    pub unknown_val_20972: i32,                   // Offset 20972 (0x51EC)
    pub unknown_val_20976: i32,                   // Offset 20976 (0x51F0)
    pub use_whisper_window: i32,                  // Offset 20980 (0x51F4)
    pub unknown_block_20984: [u8; 56],            // Offset 20984 (0x51F8)
    pub unknown_val_21040: u8,                    // Offset 21040 (0x5230)
    pub unknown_val_21041: u8,                    // Offset 21041 (0x5231)
    pub disable_whisper: u8,                      // Offset 21042 (0x5232)
    pub pad_align_last: [u8; 1],                  // Offset 21043 (0x5233)
    pub feature: u32,                             // Offset 21044 (0x5234)
    pub invitation_code: u32,                     // Offset 21048 (0x5238)
    pub nickname_to_invite: MsnChatString,        // Offset 21052 (0x523C)
    pub unknown_block_21060: [u8; 8],             // Offset 21060 (0x5244)
    pub msg_id_invitedata: i32,                   // Offset 21068 (0x524C)
}
```
```

---

## 6. Integration into the Global "this" (`SharedSiteState`)

In our hosting container implementation (defined in [mod.rs](file:///c:/Users/jd/Desktop/MSN%20Chat%20Control/redmond-chat/src/host/site/mod.rs)), the container's global context is represented by the `SharedSiteState` struct. This structure functions as the global `this` context for all of our COM site wrappers (such as `MyOleClientSite`, `MyOleInPlaceFrame`, and `MyChatFrameEvents`), which retrieve it via their own `this.shared` pointer.

To interface directly with the hosted `MsnChat45.ocx` control, we store the queried interface pointers directly inside this global `SharedSiteState`.

### Updated Global State Structure (`SharedSiteState`)

```rust
use windows::Win32::Foundation::HWND;

#[repr(C)]
pub struct SharedSiteState {
    // --- Container Lifetime & Windowing ---
    pub ref_count: u32,
    pub hwnd: HWND,

    // --- Container Site COM Wrappers (our local 'this' structures) ---
    pub client_site: *mut MyOleClientSite,
    pub inplace_site: *mut MyOleInPlaceSite,
    pub frame: *mut MyOleInPlaceFrame,
    pub events: *mut events::MyChatFrameEvents,
    pub navigate: *mut navigate::MyOleNavigate,
    pub browser: *mut browser::MyWebBrowser,
    pub provider: *mut provider::MyServiceProvider,

    // --- Hosted Control Interface Pointers (MsnChatActiveXInstance equivalent) ---
    /// Pointer to the control's active `IChatFrame` COM interface
    pub chat_frame: *mut IChatFrame,
    
    /// Pointer to the control's active `IChatSettings` COM interface
    pub chat_settings: *mut IChatSettings,
}
```

### Direct Memory Access / Hooking Relationship

```
                     +----------------------------------------+
                     |        Container's Global state        |
                     |           (SharedSiteState)            |
                     +-------------------+--------------------+
                                         |
                                         |  holds pointers to
                                         v
        +--------------------------------+--------------------------------+
        |                                                                 |
        v                                                                 v
+-------------------------------+                                 +-------------------------------+
|         chat_settings         |                                 |          chat_frame           |
|      (*mut IChatSettings)       |                                 |       (*mut IChatFrame)       |
+---------------+---------------+                                 +---------------+---------------+
                |                                                                 |
                |  points to VTable at offset 0 of                                |  points to VTable at offset 0 of
                v                                                                 v
+-------------------------------+                                 +-------------------------------+
|     ChatSettingsInstance      |                                 |       ChatFrameInstance       |
|    (Control's internal mem)   |                                 |    (Control's internal mem)   |
+-------------------------------+                                 +-------------------------------+
| Offset 0   : lp_vtbl          |                                 | Offset 0   : lp_vtbl          |
| Offset 148 : back_color       |                                 | Offset 20368: room_name_bstr  |
| Offset 152 : fore_color       |                                 | Offset 20616: back_color      |
| Offset 156 : redirect_url     |                                 | Offset 20956: chat_mode       |
+-------------------------------+                                 +-------------------------------+
```

If we are intercepting or directly reading/modifying control properties under the hood, we can safely cast the hosted control's interface pointers stored in `SharedSiteState` directly to their internal layout representations:

```rust
// Accessing internal control variables directly by pointer casting
unsafe {
    let settings_ptr = (*shared_state).chat_settings as *mut ChatSettingsInstance;
    let current_back_color = (*settings_ptr).back_color;
    
    let frame_ptr = (*shared_state).chat_frame as *mut ChatFrameInstance;
    let active_room_name = (*frame_ptr).room_name_bstr;
}
```

---

## 7. Socket & Connection Classes Layout

We can model the internal protocol-handling classes of the MSN Chat Control. These classes are instantiated inside the `ChatFrameInstance` (offset `208`) to manage the IRC network connection.

### ChatSocket Layout
Each socket connection is managed by a `ChatSocket` instance (size approximately `7244` bytes). This struct holds raw ASCII command strings transmitted directly over the IRC socket.

```rust
#[repr(C)]
pub struct ChatSocket {
    // 0..4185: Socket state, descriptors, buffer pointers
    pub padding_init: [u8; 4185],
    
    /// Market identifier
    pub market: [u8; 100],                         // Offset 4185 (0x1059)
    
    pub padding_mid1: [u8; 371],                   // Offset 4285 (0x10BD)
    
    /// Converted MSN Passport profile info
    pub passport_profile: [u8; 401],               // Offset 4656 (0x1230)
    
    /// Converted MSN Passport ticket info
    pub passport_ticket: [u8; 401],                // Offset 5057 (0x13C1)
    
    /// Converted MSN registration cookie
    pub msn_reg_cookie: [u8; 401],                 // Offset 5458 (0x1552)
    
    /// Converted MSN user profile
    pub msn_profile: [u8; 401],                    // Offset 5859 (0x16E3)
    
    /// Converted MSN subscriber info
    pub subscriber_info: [u8; 401],                // Offset 6260 (0x1874)
    
    /// Converted user role
    pub user_role: [u8; 420],                      // Offset 6661 (0x1A05)
    
    /// Active nickname
    pub nickname: [u8; 100],                       // Offset 7081 (0x1BA9)
    pub padding_end: [u8; 59],
    
    /// Callback or Parent Frame Interface pointer
    pub lp_callback: *mut c_void,                  // Offset 7240 (0x1C48)
}
```

### CChatSock Layout (Inline Connection Class)
Located at offset `208` inside the `ChatFrameInstance` structure. It contains two instances of `ChatSocket` (likely representing a primary chat connection and a secondary whisper or backup socket).

```rust
#[repr(C)]
pub struct CChatSock {
    pub lp_vtbl: *const c_void,                    // Offset 0
    pub padding_init: [u8; 24],                    // Offset 4
    
    /// Primary Chat Connection Socket
    pub primary_socket: ChatSocket,                // Offset 28 (size 7244)
    
    /// Secondary/Whisper Connection Socket
    pub secondary_socket: ChatSocket,              // Offset 7272 (size 7244)
    
    pub padding_end: [u8; 2300],                   // Trailing connection state variables
}
```

---

## 8. Registry Settings Manager Layout

The Registry Settings Manager is instantiated dynamically and stored in `ChatFrameInstance->unknown_val_20964`. It acts as a wrapper around the Windows Registry APIs to read and write MSN Chat settings under `HKEY_CURRENT_USER\Software\Microsoft\MSNChat\4.0`.

### VTable Definition (`off_37203224`)

```rust
#[repr(C)]
pub struct IMsnChatRegistry_Vtbl {
    /// Destructor (closes HKEY if open)
    pub Destructor: unsafe extern "thiscall" fn(this: *mut c_void, free_memory: u8) -> *mut c_void,

    /// Raw RegQueryValueExA wrapper
    pub RegQueryValueRaw: unsafe extern "thiscall" fn(
        this: *mut c_void,
        value_name: PCSTR,
        lp_type: *mut u32,
        lp_data: *mut u8,
        lpcb_data: *mut u32,
    ) -> i32,

    /// Raw RegSetValueExA wrapper
    pub RegSetValueRaw: unsafe extern "thiscall" fn(
        this: *mut c_void,
        value_name: PCSTR,
        dw_type: u32,
        lp_data: *const u8,
        cb_data: u32,
    ) -> i32,

    /// Reads registry value as a boolean (wrapper around RegGetDword)
    pub RegGetBool: unsafe extern "thiscall" fn(this: *mut c_void, value_name: PCSTR, out_value: *mut i32) -> i32,

    /// Writes registry value as a boolean (wrapper around RegSetDword)
    pub RegSetBool: unsafe extern "thiscall" fn(this: *mut c_void, value_name: PCSTR, value: i32) -> i32,

    /// Reads registry value as a string (REG_SZ)
    pub RegGetString: unsafe extern "thiscall" fn(
        this: *mut c_void,
        value_name: PCSTR,
        out_buf: *mut u8,
        out_buf_len: *mut u32,
    ) -> i32,

    /// Writes registry value as a string (REG_SZ)
    pub RegSetString: unsafe extern "thiscall" fn(this: *mut c_void, value_name: PCSTR, value: PCSTR) -> i32,

    /// Reads registry value as a DWORD
    pub RegGetDword: unsafe extern "thiscall" fn(this: *mut c_void, value_name: PCSTR, out_value: *mut i32) -> i32,

    /// Writes registry value as a DWORD
    pub RegSetDword: unsafe extern "thiscall" fn(this: *mut c_void, value_name: PCSTR, value: u32) -> i32,

    /// Returns the open HKEY handle
    pub GetHKey: unsafe extern "thiscall" fn(this: *mut c_void) -> HKEY,
}

#[repr(C)]
pub struct MsnChatRegistryManager {
    pub lp_vtbl: *const IMsnChatRegistry_Vtbl,
    pub hkey: HKEY, // Offset 4
}
```


