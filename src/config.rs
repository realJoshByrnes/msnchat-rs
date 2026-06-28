use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct SessionConfig {
    pub token: String,
    pub last_rotated: u32,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct PathsConfig {
    pub resource_dlls: Vec<PathBuf>,
}

// Sounds configuration is bypassed by sound_patch.rs, so it is removed from the config
// #[derive(Serialize, Deserialize, Default, Debug, Clone)]
// pub struct SoundsConfig {
//     pub enabled: bool,
//     pub media_dir: PathBuf,
//     pub events: HashMap<String, String>,
// }

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct SettingsConfig {
    #[serde(default)]
    pub showactivity: Option<bool>,
    #[serde(default)]
    pub showdepartures: Option<bool>,
    #[serde(default)]
    pub showarrivals: Option<bool>,
    #[serde(default)]
    pub disableinvites: Option<bool>,
    #[serde(default)]
    pub disableurls: Option<bool>,
    #[serde(default)]
    pub showemoticons: Option<bool>,
    #[serde(default)]
    pub ignorefonts: Option<bool>,
    #[serde(default)]
    pub playsounds: Option<bool>,
    #[serde(default)]
    pub usewhisperwindow: Option<bool>,
    #[serde(default)]
    pub disablewhisper: Option<bool>,
    #[serde(default)]
    pub fontname: Option<String>,
    #[serde(default)]
    pub fontstyle: Option<u32>,
    #[serde(default)]
    pub fontsize: Option<u32>,
    #[serde(default)]
    pub fontcolor: Option<u32>,
}

impl SettingsConfig {
    pub fn get_value(&self, name: &str) -> Option<(u32, Vec<u8>)> {
        match name.to_lowercase().as_str() {
            "showactivity" => self
                .showactivity
                .map(|b| (4, (b as u32).to_ne_bytes().to_vec())),
            "showdepartures" => self
                .showdepartures
                .map(|b| (4, (b as u32).to_ne_bytes().to_vec())),
            "showarrivals" => self
                .showarrivals
                .map(|b| (4, (b as u32).to_ne_bytes().to_vec())),
            "disableinvites" => self
                .disableinvites
                .map(|b| (4, (b as u32).to_ne_bytes().to_vec())),
            "disableurls" => self
                .disableurls
                .map(|b| (4, (b as u32).to_ne_bytes().to_vec())),
            "showemoticons" => self
                .showemoticons
                .map(|b| (4, (b as u32).to_ne_bytes().to_vec())),
            "ignorefonts" => self
                .ignorefonts
                .map(|b| (4, (b as u32).to_ne_bytes().to_vec())),
            "playsounds" => self
                .playsounds
                .map(|b| (4, (b as u32).to_ne_bytes().to_vec())),
            "usewhisperwindow" => self
                .usewhisperwindow
                .map(|b| (4, (b as u32).to_ne_bytes().to_vec())),
            "disablewhisper" => self
                .disablewhisper
                .map(|b| (4, (b as u32).to_ne_bytes().to_vec())),
            "fontname" => self.fontname.as_ref().map(|s| {
                let mut b = s.as_bytes().to_vec();
                b.push(0);
                (1, b)
            }),
            "fontstyle" => self.fontstyle.map(|v| (4, v.to_ne_bytes().to_vec())),
            "fontsize" => self.fontsize.map(|v| (4, v.to_ne_bytes().to_vec())),
            "fontcolor" => self.fontcolor.map(|v| (4, v.to_ne_bytes().to_vec())),
            _ => None,
        }
    }

    pub fn set_value(&mut self, name: &str, value_type: u32, data: &[u8]) -> bool {
        let name_lower = name.to_lowercase();
        match name_lower.as_str() {
            "showactivity" | "showdepartures" | "showarrivals" | "disableinvites"
            | "disableurls" | "showemoticons" | "ignorefonts" | "playsounds"
            | "usewhisperwindow" | "disablewhisper" => {
                let val = if value_type == 4 && data.len() == 4 {
                    u32::from_ne_bytes([data[0], data[1], data[2], data[3]]) != 0
                } else if value_type == 1 {
                    let s = String::from_utf8_lossy(data)
                        .trim_end_matches('\0')
                        .to_string();
                    s == "1" || s.eq_ignore_ascii_case("true")
                } else {
                    false
                };
                match name_lower.as_str() {
                    "showactivity" => self.showactivity = Some(val),
                    "showdepartures" => self.showdepartures = Some(val),
                    "showarrivals" => self.showarrivals = Some(val),
                    "disableinvites" => self.disableinvites = Some(val),
                    "disableurls" => self.disableurls = Some(val),
                    "showemoticons" => self.showemoticons = Some(val),
                    "ignorefonts" => self.ignorefonts = Some(val),
                    "playsounds" => self.playsounds = Some(val),
                    "usewhisperwindow" => self.usewhisperwindow = Some(val),
                    "disablewhisper" => self.disablewhisper = Some(val),
                    _ => unreachable!(),
                }
                true
            }
            "fontname" => {
                let s = String::from_utf8_lossy(data)
                    .trim_end_matches('\0')
                    .to_string();
                self.fontname = Some(s);
                true
            }
            "fontstyle" | "fontsize" | "fontcolor" => {
                let val = if value_type == 4 && data.len() == 4 {
                    u32::from_ne_bytes([data[0], data[1], data[2], data[3]])
                } else if value_type == 1 {
                    String::from_utf8_lossy(data)
                        .trim_end_matches('\0')
                        .parse()
                        .unwrap_or(0)
                } else {
                    0
                };
                match name_lower.as_str() {
                    "fontstyle" => self.fontstyle = Some(val),
                    "fontsize" => self.fontsize = Some(val),
                    "fontcolor" => self.fontcolor = Some(val),
                    _ => unreachable!(),
                }
                true
            }
            _ => false,
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct LicensingConfig {
    pub guid: String, // Hex string of {E113C6A6-D44A-4639-A40E-3B6DE32A1A40}
    pub hash: String, // Hex string of {5954F421-4768-46bc-B331-3DC37B1E7048}
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct MSNConfig {
    pub session: SessionConfig,
    pub paths: PathsConfig,
    pub licensing: LicensingConfig,
    #[serde(default)]
    pub settings: SettingsConfig,
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

        toml::from_str(&contents).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Save configuration back to config.toml
    pub fn save(&self, config: &MSNConfig) -> io::Result<()> {
        let serialized =
            toml::to_string(config).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(&self.config_path, serialized)
    }

    /// Rotates the daily unique session token if expired (older than 24 hours)
    pub fn update_user_session(&self) -> io::Result<String> {
        let mut config = self.load()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as u32;
        let day_seconds = 24 * 3600;

        let token_expired = config.session.token.is_empty()
            || config.session.token.len() < 10
            || config.session.last_rotated == 0
            || (now - config.session.last_rotated) >= day_seconds;

        if token_expired {
            let new_token = Uuid::new_v4().simple().to_string();
            config.session.token = new_token.clone();
            config.session.last_rotated = now;
            self.save(&config)?;
            Ok(new_token)
        } else {
            Ok(config.session.token.clone())
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

    // register_sounds is no longer needed since sounds are bypassed by sound_patch.rs
    // pub fn register_sounds(&self, media_dir: &Path, sound_events: &[(&str, &str)]) -> io::Result<()> {
    //     let mut config = self.load()?;
    //     config.sounds.enabled = true;
    //     config.sounds.media_dir = media_dir.to_path_buf();
    //
    //     for &(event_id, wav_name) in sound_events {
    //         config.sounds.events.insert(event_id.to_string(), wav_name.to_string());
    //     }
    //
    //     self.save(&config)
    // }
}
