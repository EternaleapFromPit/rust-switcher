#![cfg(windows)]

use windows::Win32::UI::Input::KeyboardAndMouse::{MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN};

use crate::{config, platform::win::hotkey_format::format_hotkey};

#[test]
fn format_hotkey_none() {
    assert_eq!(format_hotkey(None), "None");
}

#[test]
fn format_hotkey_letter_fast_path() {
    let hk = config::Hotkey {
        mods: MOD_CONTROL.0,
        vk: u32::from(b'A'),
    };
    let s = format_hotkey(Some(hk));
    assert!(s.contains("Ctrl"));
    assert!(s.contains("A"));
}

#[test]
fn format_hotkey_multiple_mods_fast_path() {
    let hk = config::Hotkey {
        mods: MOD_CONTROL.0 | MOD_SHIFT.0 | MOD_ALT.0 | MOD_WIN.0,
        vk: u32::from(b'9'),
    };
    let s = format_hotkey(Some(hk));
    assert!(s.contains("Ctrl"));
    assert!(s.contains("Shift"));
    assert!(s.contains("Alt"));
    assert!(s.contains("Win"));
    assert!(s.contains("9"));
}
