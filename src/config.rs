use std::{
    io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use windows::Win32::UI::Input::KeyboardAndMouse::{MOD_CONTROL, VK_CANCEL, VK_PAUSE};

const APP_DIR: &str = "RustSwitcher";
const CONFIG_FILE: &str = "config.json";

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Hotkey {
    pub vk: u32,
    pub mods: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct HotkeyChord {
    // Битовая маска MOD_*
    pub mods: u32,
    // None означает "только модификаторы" (например Shift)
    pub vk: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct HotkeySequence {
    // Всегда есть первый аккорд
    pub first: HotkeyChord,
    // Второй аккорд опционален (если None, то это обычная хоткея из одного аккорда)
    pub second: Option<HotkeyChord>,
    // Максимальная пауза между first и second
    pub max_gap_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub start_on_startup: bool,
    pub show_tray_icon: bool,
    pub delay_ms: u32,
    pub hotkey_convert_last_word: Option<Hotkey>,
    pub hotkey_convert_selection: Option<Hotkey>,
    pub hotkey_switch_layout: Option<Hotkey>,
    pub hotkey_pause: Option<Hotkey>,
    pub paused: bool,

    #[serde(default)]
    pub hotkey_convert_last_word_sequence: Option<HotkeySequence>,
    #[serde(default)]
    pub hotkey_pause_sequence: Option<HotkeySequence>,
    #[serde(default)]
    pub hotkey_convert_selection_sequence: Option<HotkeySequence>,
    #[serde(default)]
    pub hotkey_switch_layout_sequence: Option<HotkeySequence>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            start_on_startup: false,
            show_tray_icon: false,
            delay_ms: 100,

            hotkey_convert_last_word: Some(Hotkey {
                vk: VK_PAUSE.0 as u32,
                mods: 0,
            }),
            hotkey_convert_selection: Some(Hotkey {
                vk: VK_CANCEL.0 as u32,
                mods: MOD_CONTROL.0,
            }),
            hotkey_switch_layout: None,
            hotkey_pause: None,

            paused: false,

            hotkey_convert_last_word_sequence: None,
            hotkey_pause_sequence: None,
            hotkey_convert_selection_sequence: None,
            hotkey_switch_layout_sequence: None,
        }
    }
}

pub fn config_path() -> io::Result<PathBuf> {
    let appdata = std::env::var_os("APPDATA")
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "APPDATA is not set"))?;

    Ok(PathBuf::from(appdata).join(APP_DIR).join(CONFIG_FILE))
}

fn ensure_parent_dir(path: &Path) -> io::Result<()> {
    let Some(dir) = path.parent() else {
        return Ok(());
    };
    std::fs::create_dir_all(dir)
}

fn confy_err(e: confy::ConfyError) -> io::Error {
    io::Error::other(e)
}

pub fn load() -> io::Result<Config> {
    let path = config_path()?;
    ensure_parent_dir(&path)?;
    confy::load_path(path).map_err(confy_err)
}

#[allow(dead_code)]
pub fn save(cfg: &Config) -> io::Result<()> {
    let path = config_path()?;
    ensure_parent_dir(&path)?;
    confy::store_path(path, cfg).map_err(confy_err)
}
