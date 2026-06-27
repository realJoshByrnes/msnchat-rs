# Registry Access Reference & TOML Configuration Migration

This document maps out all registry keys, values, and access patterns identified in the MSN Chat OCX (`MsnChat45.ocx`) binary via reverse engineering, along with a design for a Rusty replacement that migrates these settings to a local `config.toml` configuration file.

---

## 1. ActiveX Control & COM Registration

These keys are registered and unregistered during `DllRegisterServer` and `DllUnregisterServer` via the ATL registrar. They define the COM classes and licensing/integrity checks for the MSN Chat controls.

### Registry Table: COM Class Registration (HKCR)

| Key Path | Value Name | Access | Storage Type | Description / Comment |
| :--- | :--- | :--- | :--- | :--- |
| `HKCR\CLSID\{F58E1CEF-A068-4c15-BA5E-587CAF3EE8C6}` | (Default) | Write | String (`REG_SZ`) | Register user-friendly name: `"MSN Chat Control 4.5"`. |
| `HKCR\CLSID\{F58E1CEF-A068-4c15-BA5E-587CAF3EE8C6}\InprocServer32` | (Default) | Read/Write | String (`REG_SZ`) | Path to the OCX module (replaced from `%MODULE%`). |
| `HKCR\CLSID\{F58E1CEF-A068-4c15-BA5E-587CAF3EE8C6}\InprocServer32` | `ThreadingModel` | Write | String (`REG_SZ`) | Threading model setting: `"Apartment"`. |
| `HKCR\CLSID\{F58E1CEF-A068-4c15-BA5E-587CAF3EE8C6}\MiscStatus\1` | (Default) | Write | String (`REG_SZ`) | Miscellaneous status flags: `"131473"`. |
| `HKCR\CLSID\{F58E1CEF-A068-4c15-BA5E-587CAF3EE8C6}\TypeLib` | (Default) | Write | String (`REG_SZ`) | Associated TypeLib GUID: `"{0F0A655C-6C6D-4e0b-8038-F980B36F9C78}"`. |
| `HKCR\CLSID\{FA980E7E-9E44-4d2f-B3C2-9A5BE42525F8}` | (Default) | Write | String (`REG_SZ`) | Register user-friendly name: `"MSN Chat Control 4.5 Settings"`. |
| `HKCR\CLSID\{FA980E7E-9E44-4d2f-B3C2-9A5BE42525F8}\InprocServer32` | (Default) | Write | String (`REG_SZ`) | Path to the OCX module. |
| `HKCR\CLSID\{FA980E7E-9E44-4d2f-B3C2-9A5BE42525F8}\InprocServer32` | `ThreadingModel` | Write | String (`REG_SZ`) | Threading model setting: `"Apartment"`. |

### Registry Table: Integrity & Licensing (HKCR)

During registration/initialization (`sub_3721DC47`), a unique GUID is generated and stored alongside a hash of the file modification time. This is validated at runtime (`sub_3721DA6C`) to ensure the control hasn't been tampered with.

| Key Path | Value Name | Access | Storage Type | Description / Comment |
| :--- | :--- | :--- | :--- | :--- |
| `HKCR\CLSID\{F58E1CEF-A068-4c15-BA5E-587CAF3EE8C6}` | `{E113C6A6-D44A-4639-A40E-3B6DE32A1A40}` | Read/Write | Binary (`REG_BINARY`) | 16-byte random GUID generated during registration via `CoCreateGuid`. |
| `HKCR\CLSID\{F58E1CEF-A068-4c15-BA5E-587CAF3EE8C6}` | `{5954F421-4768-46bc-B331-3DC37B1E7048}` | Read/Write | Binary (`REG_BINARY`) | 16-byte hash combining the generated GUID and the module's file creation/modification time. |

---

## 2. ActiveX Killbit/Compatibility (HKLM)

To unregister or disable the control, or configure compatibility options, HKLM compatibility flags are managed.

| Key Path | Value Name | Access | Storage Type | Description / Comment |
| :--- | :--- | :--- | :--- | :--- |
| `HKLM\Software\Microsoft\Internet Explorer\ActiveX Compatibility\<CLSID>` | `Compatibility Flags` | Read/Write | DWORD (`REG_DWORD`) | Killbit flag (`0x400` / `COMPAT_EVIL_DONT_LOAD`) used to block the control in Internet Explorer. |

---

## 3. Sound Scheme Registration & Event Configuration (HKCU)

The MSN Chat Control registers custom system event sounds (like whispered messages or user arrival) under Windows AppEvents so the OS or the control itself can play them.

### Registry Table: Sound Event Labels (HKCU)

Stored under `HKCU\AppEvents\EventLabels\<EventName>`. The default value defines the user-friendly event name loaded from the resource string table (IDs 350-358).

| Key Path | Value Name | Access | Storage Type | Description / Comment |
| :--- | :--- | :--- | :--- | :--- |
| `HKCU\AppEvents\EventLabels\msnchat_Whisper` | (Default) | Write | String (`REG_SZ`) | "MSN Chat Whisper" label (extracted to `ChatWhsp.wav` from resource ID 350). |
| `HKCU\AppEvents\EventLabels\msnchat_Arrival` | (Default) | Write | String (`REG_SZ`) | "MSN Chat Arrival" label (extracted to `ChatJoin.wav` from resource ID 351). |
| `HKCU\AppEvents\EventLabels\msnchat_Departure` | (Default) | Write | String (`REG_SZ`) | "MSN Chat Departure" label (no wav). |
| `HKCU\AppEvents\EventLabels\msnchat_HostMessage` | (Default) | Write | String (`REG_SZ`) | "MSN Chat Host Message" label (no wav). |
| `HKCU\AppEvents\EventLabels\msnchat_TagMessage` | (Default) | Write | String (`REG_SZ`) | "MSN Chat Tag Message" label (extracted to `ChatTag.wav` from resource ID 354). |
| `HKCU\AppEvents\EventLabels\msnchat_HostWhisper` | (Default) | Write | String (`REG_SZ`) | "MSN Chat Host Whisper" label (extracted to `ChatWhsp.wav` from resource ID 350). |
| `HKCU\AppEvents\EventLabels\msnchat_TagWhisper` | (Default) | Write | String (`REG_SZ`) | "MSN Chat Tag Whisper" label (extracted to `ChatWhsp.wav` from resource ID 350). |
| `HKCU\AppEvents\EventLabels\msnchat_Kick` | (Default) | Write | String (`REG_SZ`) | "MSN Chat Kick" label (extracted to `ChatKick.wav` from resource ID 357). |
| `HKCU\AppEvents\EventLabels\msnchat_Invitation` | (Default) | Write | String (`REG_SZ`) | "MSN Chat Invitation" label (extracted to `ChatInvt.wav` from resource ID 358). |

---

## 4. App/Runtime State & Session Identifiers (HKCU)

The control uses a registry key under MSNChat to persist runtime tracking data, including a daily session identifier token that rotates every 24 hours.

| Key Path | Value Name | Access | Storage Type | Description / Comment |
| :--- | :--- | :--- | :--- | :--- |
| `HKCU\Software\Microsoft\MSNChat\4.0` | `UserData1` | Read/Write | String (`REG_SZ`) | 31-character randomized session identifier token generated using LCG. |
| `HKCU\Software\Microsoft\MSNChat\4.0` | `UserData2` | Read/Write | int32 (`REG_DWORD`) | Timestamp (in seconds) representing when `UserData1` was generated. Used to check if 24 hours have elapsed. |

---

## 5. Safe/Installed Resource DLL Directory (HKCU)

Tracks installed resource DLL paths to clean them up or delete them safely when unregistering.

| Key Path | Value Name | Access | Storage Type | Description / Comment |
| :--- | :--- | :--- | :--- | :--- |
| `HKCU\Software\Microsoft\MSNChat\4.0\ResDLLInstalled` | `<DLL File Path>` | Read/Write | int32 (`REG_DWORD`) | tracks the absolute file path of the installed ResDLL. Value is generally 0. Enumerated during unregistration to call `DeleteFileA` on the file paths before deleting the key. |

---

# TOML Config Replacement Plan

Rather than interacting with the Windows registry, the Rust implementation will utilize a clean, cross-platform `config.toml` file to manage MSN Chat configurations and state. This bypasses registry restrictions/permissions entirely and keeps the application lightweight.

### Proposed Cargo Dependencies

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
uuid = { version = "1.0", features = ["v4"] }
chrono = "0.4"
```

### Proposed `config.toml` Layout

```toml
[session]
token = "4d6d678d2b3846e9b4625b6a71e8d42c"
last_rotated = 1782635746

[paths]
resource_dlls = [
    "C:\\Program Files\\MSNChat\\ResDll01.dll"
]

[sounds]
enabled = true
media_dir = "C:\\Windows\\Media"
events = { msnchat_Whisper = "ChatWhsp.wav", msnchat_Arrival = "ChatJoin.wav" }
```

### Proposed Rust Implementation

```rust
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::Utc;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct SessionConfig {
    pub token: String,
    pub last_rotated: u32,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct PathsConfig {
    pub resource_dlls: Vec<PathBuf>,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct SoundsConfig {
    pub enabled: bool,
    pub media_dir: PathBuf,
    pub events: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct MSNConfig {
    pub session: SessionConfig,
    pub paths: PathsConfig,
    pub sounds: SoundsConfig,
}

pub struct MSNConfigManager {
    config_path: PathBuf,
}

impl MSNConfigManager {
    pub fn new(config_path: &Path) -> Self {
        Self {
            config_path: config_path.to_path_buf(),
        }
    }

    /// Load configuration from config.toml, or initialize with defaults if missing
    pub fn load(&self) -> io::Result<MSNConfig> {
        if !self.config_path.exists() {
            let default_config = MSNConfig::default();
            self.save(&default_config)?;
            return Ok(default_config);
        }

        let mut file = File::open(&self.config_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        
        toml::from_str(&contents)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Save configuration back to config.toml
    pub fn save(&self) -> io::Result<()> {
        let serialized = toml::to_string(config)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(&self.config_path, serialized)
    }

    /// Rotates the daily unique session token if expired (older than 24 hours)
    pub fn update_user_session(&self) -> io::Result<String> {
        let mut config = self.load()?;
        let now = Utc::now().timestamp() as u32;
        let day_seconds = 24 * 3600;

        let token_expired = config.session.token.is_empty() 
            || config.session.token.len() < 10 
            || config.session.last_rotated == 0 
            || (now - config.session.last_rotated) >= day_seconds;

        if token_expired {
            let new_token = Uuid::new_v4().to_simple().to_string();
            config.session.token = new_token.clone();
            config.session.last_rotated = now;
            self.save(&config)?;
            Ok(new_token)
        } else {
            Ok(config.session.token)
        }
    }

    /// Registers a resource DLL path to track for clean uninstallation
    pub fn register_res_dll(&self, dll_path: &Path) -> io::Result<()> {
        let mut config = self.load()?;
        let path_buf = dll_path.to_path_buf();
        if !config.paths.resource_dlls.contains(&path_buf) {
            config.paths.resource_dlls.push(path_buf);
            self.save(&config)?;
        }
        Ok(())
    }

    /// Cleans up registered resource DLL files and removes config paths
    pub fn clean_and_unregister(&self) -> io::Result<()> {
        let mut config = self.load()?;
        for dll_path in &config.paths.resource_dlls {
            let _ = fs::remove_file(dll_path);
        }
        config.paths.resource_dlls.clear();
        self.save(&config)?;
        Ok(())
    }

    /// Setup app sound events and configs
    pub fn register_sounds(&self, media_dir: &Path, sound_events: &[(&str, &str)]) -> io::Result<()> {
        let mut config = self.load()?;
        config.sounds.enabled = true;
        config.sounds.media_dir = media_dir.to_path_buf();
        
        for &(event_id, wav_name) in sound_events {
            config.sounds.events.insert(event_id.to_string(), wav_name.to_string());
        }
        
        self.save(&config)
    }
}
```
