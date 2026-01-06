#![cfg(windows)]

use crate::{config, platform::win::keyboard::sequence::chord_matches};

fn ch(mods: u32, mods_vks: u32, vk: Option<u32>) -> config::HotkeyChord {
    config::HotkeyChord { mods, mods_vks, vk }
}

#[test]
fn chord_matches_requires_same_mods() {
    let t = ch(1, 0, Some(65));
    let i = ch(2, 0, Some(65));
    assert!(!chord_matches(t, i));
}

#[test]
fn chord_matches_requires_same_mods_vks() {
    let t = ch(0, 4, Some(65));
    let i = ch(0, 8, Some(65));
    assert!(!chord_matches(t, i));
}

#[test]
fn chord_matches_vk_none_requires_none() {
    let t = ch(1, 0, None);

    let i_some = ch(1, 0, Some(70));
    assert!(!chord_matches(t, i_some));

    let i_none = ch(1, 0, None);
    assert!(chord_matches(t, i_none));
}

#[test]
fn chord_matches_vk_some_requires_exact() {
    let t = ch(1, 0, Some(65));
    let i = ch(1, 0, Some(66));
    assert!(!chord_matches(t, i));
}
