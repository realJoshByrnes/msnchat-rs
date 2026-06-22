# MSN Chat ActiveX Control Properties & Interfaces

This document contains a list of all parameters, properties, and methods defined in `MsnChat45.ocx`, extracted directly from its Type Library (TypeLib), including property/parameter types, **VTable slots (offsets)**, and their absolute addresses at **Active Runtime Base (`0x37200000`)**.

---

## 1. ChatFrame (`IChatFrame` Interface)
* **Interface GUID**: `125E64FA-3304-4BB9-A756-D0D44CC8CD7D`
* **Target VTable**: **`IChatFrame` VTable** (Exposed by the `MSNChatFrame` CoClass `F58E1CEF-A068-4C15-BA5E-587CAF3EE8C6`)
* **VTable RVA (Relative Virtual Address)**: `0x001AE0`
* **VTable Address (Active Runtime Base `0x37200000`)**: **`0x37201AE0`**

### Methods & Standard Functions on `IChatFrame` VTable

| VTable Index | VTable Offset | Absolute Address (Active Base `0x37200000`) | Type | Function Signature / Parameters |
| :---: | :---: | :---: | :---: | :--- |
| **0** | `0x00` | `0x37201AE0` | `METHOD` | `void QueryInterface(riid: GUID*, ppvObj: void**)` |
| **1** | `0x04` | `0x37201AE4` | `METHOD` | `unsigned long AddRef()` |
| **2** | `0x08` | `0x37201AE8` | `METHOD` | `unsigned long Release()` |
| **3** | `0x0C` | `0x37201AEC` | `METHOD` | `void GetTypeInfoCount(pctinfo: unsigned int*)` |
| **4** | `0x10` | `0x37201AF0` | `METHOD` | `void GetTypeInfo(itinfo: unsigned int, lcid: unsigned long, pptinfo: void**)` |
| **5** | `0x14` | `0x37201AF4` | `METHOD` | `void GetIDsOfNames(riid: GUID*, rgszNames: char**, cNames: unsigned int, lcid: unsigned long, rgdispid: long*)` |
| **6** | `0x18` | `0x37201AF8` | `METHOD` | `void Invoke(dispidMember: long, riid: GUID*, lcid: unsigned long, wFlags: unsigned short, pdispparams: DISPPARAMS*, pvarResult: VARIANT*, pexcepinfo: EXCEPINFO*, puArgErr: unsigned int*)` |

### Control Properties on `IChatFrame` VTable

| Property Name | DISPID (Dec) | Property Type | VTable Get Slot | Get Addr (Active Base) | VTable Put Slot | Put Addr (Active Base) |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| **BackColor** | `-501` | `OLE_COLOR` | Index **8** (`0x20`) | `0x37201B00` | Index **7** (`0x1C`) | `0x37201AFC` |
| **RoomName** | `2` | `BSTR` | Index **9** (`0x24`) | `0x37201B04` | Index **10** (`0x28`) | `0x37201B08` |
| **HexRoomName** | `3` | `BSTR` | Index **11** (`0x2C`) | `0x37201B0C` | Index **12** (`0x30`) | `0x37201B10` |
| **NickName** | `4` | `BSTR` | Index **13** (`0x34`) | `0x37201B14` | Index **14** (`0x38`) | `0x37201B18` |
| **Server** | `5` | `BSTR` | Index **15** (`0x3C`) | `0x37201B1C` | Index **16** (`0x40`) | `0x37201B20` |
| **BackHighlightColor** | `6` | `OLE_COLOR` | Index **17** (`0x44`) | `0x37201B24` | Index **18** (`0x48`) | `0x37201B28` |
| **ButtonFrameColor** | `7` | `OLE_COLOR` | Index **19** (`0x4C`) | `0x37201B2C` | Index **20** (`0x50`) | `0x37201B30` |
| **TopBackHighlightColor** | `8` | `OLE_COLOR` | Index **21** (`0x54`) | `0x37201B34` | Index **22** (`0x58`) | `0x37201B38` |
| **ChatMode** | `9` | `long` | Index **23** (`0x5C`) | `0x37201B3C` | Index **24** (`0x60`) | `0x37201B40` |
| **URLBack** | `10` | `BSTR` | Index **25** (`0x64`) | `0x37201B44` | Index **26** (`0x68`) | `0x37201B48` |
| **Category** | `11` | `BSTR` | Index **27** (`0x6C`) | `0x37201B4C` | Index **28** (`0x70`) | `0x37201B50` |
| **Topic** | `12` | `BSTR` | Index **29** (`0x74`) | `0x37201B54` | Index **30** (`0x78`) | `0x37201B58` |
| **WelcomeMsg** | `13` | `BSTR` | Index **31** (`0x7C`) | `0x37201B5C` | Index **32** (`0x80`) | `0x37201B60` |
| **BaseURL** | `15` | `BSTR` | Index **33** (`0x84`) | `0x37201B64` | Index **34** (`0x88`) | `0x37201B68` |
| **InputBorderColor** | `16` | `OLE_COLOR` | Index **35** (`0x8C`) | `0x37201B6C` | Index **36** (`0x90`) | `0x37201B70` |
| **CreateRoom** | `17` | `BSTR` | Index **37** (`0x94`) | `0x37201B74` | Index **38** (`0x98`) | `0x37201B78` |
| **ChatHome** | `19` | `BSTR` | Index **39** (`0x9C`) | `0x37201B7C` | Index **40** (`0xA0`) | `0x37201B80` |
| **Locale** | `20` | `BSTR` | Index **41** (`0xA4`) | `0x37201B84` | Index **42** (`0xA8`) | `0x37201B88` |
| **ResDLL** | `21` | `BSTR` | Index **43** (`0xAC`) | `0x37201B8C` | Index **44** (`0xB0`) | `0x37201B90` |
| **ButtonTextColor** | `22` | `OLE_COLOR` | Index **45** (`0xB4`) | `0x37201B94` | Index **46** (`0xB8`) | `0x37201B98` |
| **ButtonBackColor** | `23` | `OLE_COLOR` | Index **47** (`0xBC`) | `0x37201B9C` | Index **48** (`0xC0`) | `0x37201BA0` |
| **PassportTicket** | `24` | `BSTR` | Index **49** (`0xC4`) | `0x37201BA4` | Index **50** (`0xC8`) | `0x37201BA8` |
| **PassportProfile** | `25` | `BSTR` | Index **51** (`0xCC`) | `0x37201BAC` | Index **52** (`0xD0`) | `0x37201BB0` |
| **Feature** | `26` | `unsigned long` | Index **53** (`0xD4`) | `0x37201BB4` | Index **54** (`0xD8`) | `0x37201BB8` |
| **MessageOfTheDay** | `27` | `BSTR` | Index **55** (`0xDC`) | `0x37201BBC` | Index **56** (`0xE0`) | `0x37201BC0` |
| **ChannelLanguage** | `28` | `BSTR` | Index **57** (`0xE4`) | `0x37201BC4` | Index **58** (`0xE8`) | `0x37201BC8` |
| **InvitationCode** | `29` | `BSTR` | Index **59** (`0xEC`) | `0x37201BCC` | Index **60** (`0xF0`) | `0x37201BD0` |
| **NicknameToInvite** | `30` | `BSTR` | Index **61** (`0xF4`) | `0x37201BD4` | Index **62** (`0xF8`) | `0x37201BD8` |
| **MSNREGCookie** | `31` | `BSTR` | Index **63** (`0xFC`) | `0x37201BDC` | Index **64** (`0x100`) | `0x37201BE0` |
| **CreationModes** | `32` | `BSTR` | Index **65** (`0x104`) | `0x37201BE4` | Index **66** (`0x108`) | `0x37201BE8` |
| **MSNProfile** | `33` | `BSTR` | Index **67** (`0x10C`) | `0x37201BEC` | Index **68** (`0x110`) | `0x37201BF0` |
| **Market** | `34` | `BSTR` | Index **69** (`0x114`) | `0x37201BF4` | Index **70** (`0x118`) | `0x37201BF8` |
| **WhisperContent** | `35` | `BSTR` | Index **71** (`0x11C`) | `0x37201BFC` | Index **72** (`0x120`) | `0x37201C00` |
| **UserRole** | `36` | `BSTR` | Index **73** (`0x124`) | `0x37201C04` | Index **74** (`0x128`) | `0x37201C08` |
| **AuditMessage** | `37` | `BSTR` | Index **75** (`0x12C`) | `0x37201C0C` | Index **76** (`0x130`) | `0x37201C10` |
| **SubscriberInfo** | `38` | `BSTR` | Index **77** (`0x134`) | `0x37201C14` | Index **78** (`0x138`) | `0x37201C18` |
| **UpsellURL** | `39` | `BSTR` | Index **79** (`0x13C`) | `0x37201C1C` | Index **80** (`0x140`) | `0x37201C20` |

---

## 2. ChatSettings (`IChatSettings` Interface)
* **Interface GUID**: `D5EF4299-12F1-474D-98C5-3C658FD2E343`
* **Target VTable**: **`IChatSettings` VTable** (Exposed by the `ChatSettings` CoClass `FA980E7E-9E44-4D2F-B3C2-9A5BE42525F8`)
* **VTable RVA (Relative Virtual Address)**: `0x001D90`
* **VTable Address (Active Runtime Base `0x37200000`)**: **`0x37201D90`**

### Methods & Standard Functions on `IChatSettings` VTable

| VTable Index | VTable Offset | Absolute Address (Active Base `0x37200000`) | Type | Function Signature / Parameters |
| :---: | :---: | :---: | :---: | :--- |
| **0** | `0x00` | `0x37201D90` | `METHOD` | `void QueryInterface(riid: GUID*, ppvObj: void**)` |
| **1** | `0x04` | `0x37201D94` | `METHOD` | `unsigned long AddRef()` |
| **2** | `0x08` | `0x37201D98` | `METHOD` | `unsigned long Release()` |
| **3** | `0x0C` | `0x37201D9C` | `METHOD` | `void GetTypeInfoCount(pctinfo: unsigned int*)` |
| **4** | `0x10` | `0x37201DA0` | `METHOD` | `void GetTypeInfo(itinfo: unsigned int, lcid: unsigned long, pptinfo: void**)` |
| **5** | `0x14` | `0x37201DA4` | `METHOD` | `void GetIDsOfNames(riid: GUID*, rgszNames: char**, cNames: unsigned int, lcid: unsigned long, rgdispid: long*)` |
| **6** | `0x18` | `0x37201DA8` | `METHOD` | `void Invoke(dispidMember: long, riid: GUID*, lcid: unsigned long, wFlags: unsigned short, pdispparams: DISPPARAMS*, pvarResult: VARIANT*, pexcepinfo: EXCEPINFO*, puArgErr: unsigned int*)` |

### Control Properties on `IChatSettings` VTable

| Property Name | DISPID (Dec) | Property Type | VTable Get Slot | Get Addr (Active Base) | VTable Put Slot | Put Addr (Active Base) |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| **BackColor** | `-501` | `OLE_COLOR` | Index **8** (`0x20`) | `0x37201DB0` | Index **7** (`0x1C`) | `0x37201DAC` |
| **ForeColor** | `-513` | `OLE_COLOR` | Index **10** (`0x28`) | `0x37201DBC` | Index **9** (`0x24`) | `0x37201DB8` |
| **RedirectURL** | `1` | `BSTR` | Index **12** (`0x30`) | `0x37201DC0` | Index **11** (`0x2C`) | `0x37201DBC` |
| **ResDLL** | `2` | `BSTR` | Index **14** (`0x38`) | `0x37201DC8` | Index **13** (`0x34`) | `0x37201DC4` |

---

## 3. Events (`_ICChatFrameEvents` Interface)
* **Interface GUID**: `5EEB8014-53B2-448B-9F3B-C553424832E1`
* **Target VTable**: **Host/Container Event Sink VTable**
* **Description**: _ICChatHolderEvents Interface

> [!NOTE]
> `_ICChatFrameEvents` is an **outgoing (source)** interface. 
> The virtual table for this interface is **implemented dynamically by the client host/container application** at runtime. Therefore, there is no static vtable or function implementation address for this interface inside `MsnChat45.ocx`. The control calls these functions via the container's sink pointer.

### Callbacks on Event Sink VTable

| VTable Slot | Type | Function Signature / Parameters |
| :---: | :---: | :--- |
| **Index 0** (`0x00`) | `METHOD` | `HRESULT OnRedirect(strUrl: BSTR)` (DISPID `1`) |
