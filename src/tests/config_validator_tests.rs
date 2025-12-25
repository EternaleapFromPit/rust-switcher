use windows::Win32::UI::Input::KeyboardAndMouse::{MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN};

use crate::{
    config::{Config, HotkeyChord, HotkeySequence},
    constants::{CONVERT_LAST_WORD, CONVERT_SELECTION, PAUSE, SWITCH_LAYOUT},
};

fn chord(mods: u32, vk: u32) -> HotkeyChord {
    HotkeyChord {
        mods,
        mods_vks: 0,
        vk: Some(vk),
    }
}

fn seq(mods: u32, vk: u32) -> HotkeySequence {
    HotkeySequence {
        first: chord(mods, vk),
        second: None,
        max_gap_ms: 250,
    }
}

fn cfg(
    last_word: Option<HotkeySequence>,
    pause: Option<HotkeySequence>,
    selection: Option<HotkeySequence>,
    layout: Option<HotkeySequence>,
) -> Config {
    Config {
        hotkey_convert_last_word_sequence: last_word,
        hotkey_pause_sequence: pause,
        hotkey_convert_selection_sequence: selection,
        hotkey_switch_layout_sequence: layout,
        ..Default::default()
    }
}

#[test]
fn no_hotkeys_set_ok() {
    let c = cfg(None, None, None, None);
    assert!(c.validate_hotkey_sequences().is_ok());
}

#[test]
fn no_duplicates_ok() {
    let c = cfg(
        Some(seq(MOD_CONTROL.0, b'A' as u32)),
        Some(seq(MOD_ALT.0, b'B' as u32)),
        Some(seq(MOD_SHIFT.0, b'C' as u32)),
        Some(seq(MOD_WIN.0, b'D' as u32)),
    );
    assert!(c.validate_hotkey_sequences().is_ok());
}

#[test]
fn allowed_duplicate_selection_and_last_word_ok() {
    let same = seq(MOD_CONTROL.0, b'A' as u32);
    let c = cfg(
        Some(same),
        Some(seq(MOD_ALT.0, b'B' as u32)),
        Some(seq(MOD_CONTROL.0, b'A' as u32)),
        Some(seq(MOD_SHIFT.0, b'C' as u32)),
    );
    assert!(c.validate_hotkey_sequences().is_ok());
}

#[test]
fn duplicate_pause_and_layout_err() {
    let dup = seq(MOD_ALT.0, b'B' as u32);
    let c = cfg(
        Some(seq(MOD_CONTROL.0, b'A' as u32)),
        Some(dup),
        Some(seq(MOD_SHIFT.0, b'C' as u32)),
        Some(seq(MOD_ALT.0, b'B' as u32)),
    );

    let err = c.validate_hotkey_sequences().unwrap_err();
    assert!(err.contains("Duplicate hotkey sequences"));
    assert!(err.contains(PAUSE));
    assert!(err.contains(SWITCH_LAYOUT));
}

#[test]
fn duplicate_last_word_and_pause_err() {
    let dup = seq(MOD_CONTROL.0, b'A' as u32);
    let c = cfg(
        Some(dup),
        Some(seq(MOD_CONTROL.0, b'A' as u32)),
        Some(seq(MOD_SHIFT.0, b'C' as u32)),
        Some(seq(MOD_ALT.0, b'B' as u32)),
    );

    let err = c.validate_hotkey_sequences().unwrap_err();
    assert!(err.contains(CONVERT_LAST_WORD));
    assert!(err.contains(PAUSE));
}

#[test]
fn duplicate_selection_and_pause_err() {
    let dup = seq(MOD_CONTROL.0, b'A' as u32);
    let c = cfg(
        Some(seq(MOD_SHIFT.0, b'C' as u32)),
        Some(dup),
        Some(seq(MOD_CONTROL.0, b'A' as u32)),
        Some(seq(MOD_ALT.0, b'B' as u32)),
    );

    let err = c.validate_hotkey_sequences().unwrap_err();
    assert!(err.contains(CONVERT_SELECTION));
    assert!(err.contains(PAUSE));
}
