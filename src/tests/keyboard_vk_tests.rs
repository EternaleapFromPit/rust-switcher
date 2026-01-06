#![cfg(windows)]

use windows::Win32::UI::WindowsAndMessaging::{WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP};

use crate::platform::win::keyboard::vk;

#[test]
fn keydown_keyup_msg_classification() {
    assert!(vk::is_keydown_msg(WM_KEYDOWN));
    assert!(vk::is_keydown_msg(WM_SYSKEYDOWN));
    assert!(!vk::is_keydown_msg(WM_KEYUP));
    assert!(!vk::is_keydown_msg(WM_SYSKEYUP));

    assert!(vk::is_keyup_msg(WM_KEYUP));
    assert!(vk::is_keyup_msg(WM_SYSKEYUP));
    assert!(!vk::is_keyup_msg(WM_KEYDOWN));
    assert!(!vk::is_keyup_msg(WM_SYSKEYDOWN));
}

#[test]
fn mod_bit_for_vk_known_values() {
    assert!(vk::mod_bit_for_vk(0xA2).is_some()); // LCtrl
    assert!(vk::mod_bit_for_vk(0xA3).is_some()); // RCtrl
    assert!(vk::mod_bit_for_vk(0xA0).is_some()); // LShift
    assert!(vk::mod_bit_for_vk(0xA1).is_some()); // RShift
    assert!(vk::mod_bit_for_vk(0xA4).is_some()); // LAlt
    assert!(vk::mod_bit_for_vk(0xA5).is_some()); // RAlt
    assert!(vk::mod_bit_for_vk(0x5B).is_some()); // LWin
    assert!(vk::mod_bit_for_vk(0x5C).is_some()); // RWin
    assert!(vk::mod_bit_for_vk(0x30).is_none()); // 0
}

#[test]
fn mod_vk_bit_for_vk_known_values() {
    assert!(vk::mod_vk_bit_for_vk(0xA2).is_some());
    assert!(vk::mod_vk_bit_for_vk(0xA3).is_some());
    assert!(vk::mod_vk_bit_for_vk(0xA0).is_some());
    assert!(vk::mod_vk_bit_for_vk(0xA1).is_some());
    assert!(vk::mod_vk_bit_for_vk(0xA4).is_some());
    assert!(vk::mod_vk_bit_for_vk(0xA5).is_some());
    assert!(vk::mod_vk_bit_for_vk(0x5B).is_some());
    assert!(vk::mod_vk_bit_for_vk(0x5C).is_some());
    assert!(vk::mod_vk_bit_for_vk(0x30).is_none());
}
