use std::sync::atomic::{AtomicIsize, AtomicU32, Ordering};

use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::{
        CallNextHookEx, HC_ACTION, HHOOK, KBDLLHOOKSTRUCT, SetWindowsHookExW, UnhookWindowsHookEx,
        WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
    },
};

use crate::{config, helpers};

static HOOK_HANDLE: AtomicIsize = AtomicIsize::new(0);
static MAIN_HWND: AtomicIsize = AtomicIsize::new(0);
static MODS_DOWN: AtomicU32 = AtomicU32::new(0);

fn mod_bit_for_vk(vk: u32) -> Option<u32> {
    match vk {
        0xA2 | 0xA3 => Some(windows::Win32::UI::Input::KeyboardAndMouse::MOD_CONTROL.0), // VK_LCONTROL VK_RCONTROL
        0xA0 | 0xA1 => Some(windows::Win32::UI::Input::KeyboardAndMouse::MOD_SHIFT.0), // VK_LSHIFT VK_RSHIFT
        0xA4 | 0xA5 => Some(windows::Win32::UI::Input::KeyboardAndMouse::MOD_ALT.0), // VK_LMENU VK_RMENU
        0x5B | 0x5C => Some(windows::Win32::UI::Input::KeyboardAndMouse::MOD_WIN.0), // VK_LWIN VK_RWIN
        _ => None,
    }
}

fn is_keydown_msg(msg: u32) -> bool {
    msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN
}

fn is_keyup_msg(msg: u32) -> bool {
    msg == WM_KEYUP || msg == WM_SYSKEYUP
}

extern "system" fn proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let msg = wparam.0 as u32;
        let kb = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let vk = kb.vkCode;

        if is_keydown_msg(msg) {
            if let Some(bit) = mod_bit_for_vk(vk) {
                MODS_DOWN.fetch_or(bit, Ordering::Relaxed);
            }
        } else if is_keyup_msg(msg) {
            if let Some(bit) = mod_bit_for_vk(vk) {
                MODS_DOWN.fetch_and(!bit, Ordering::Relaxed);
            }
        }

        if is_keydown_msg(msg) {
            let hwnd_raw = MAIN_HWND.load(Ordering::Relaxed);
            if hwnd_raw != 0 {
                let hwnd = HWND(hwnd_raw as *mut _);

                super::with_state_mut_do(hwnd, |state| {
                    if !state.hotkey_capture.active {
                        return;
                    }
                    let Some(slot) = state.hotkey_capture.slot else {
                        return;
                    };

                    let mods = MODS_DOWN.load(Ordering::Relaxed);

                    let hk = if mod_bit_for_vk(vk).is_some() {
                        config::Hotkey { vk: 0, mods }
                    } else {
                        config::Hotkey { vk, mods }
                    };

                    state.hotkey_values.set(slot, Some(hk));

                    let text = super::format_hotkey(Some(hk));
                    let target = match slot {
                        crate::app::HotkeySlot::LastWord => state.hotkeys.last_word,
                        crate::app::HotkeySlot::Pause => state.hotkeys.pause,
                        crate::app::HotkeySlot::Selection => state.hotkeys.selection,
                        crate::app::HotkeySlot::SwitchLayout => state.hotkeys.switch_layout,
                    };

                    let _ = helpers::set_edit_text(target, &text);
                });

                if super::with_state_mut(hwnd, |s| s.hotkey_capture.active).unwrap_or(false) {
                    return LRESULT(1);
                }
            }
        } else if is_keyup_msg(msg) {
            let hwnd_raw = MAIN_HWND.load(Ordering::Relaxed);
            if hwnd_raw != 0 {
                let hwnd = HWND(hwnd_raw as *mut _);
                if super::with_state_mut(hwnd, |s| s.hotkey_capture.active).unwrap_or(false) {
                    return LRESULT(1);
                }
            }
        }
    }

    let h = HOOK_HANDLE.load(Ordering::Relaxed);
    let hook = if h == 0 {
        None
    } else {
        Some(HHOOK(h as *mut _))
    };
    unsafe { CallNextHookEx(hook, code, wparam, lparam) }
}

pub fn install(hwnd: HWND) {
    MAIN_HWND.store(hwnd.0 as isize, Ordering::Relaxed);

    if HOOK_HANDLE.load(Ordering::Relaxed) != 0 {
        return;
    }

    match unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(proc), None, 0) } {
        Ok(h) => {
            HOOK_HANDLE.store(h.0 as isize, Ordering::Relaxed);
            #[cfg(debug_assertions)]
            eprintln!("RustSwitcher: WH_KEYBOARD_LL installed");
        }
        Err(e) => {
            #[cfg(debug_assertions)]
            eprintln!("RustSwitcher: SetWindowsHookExW failed: {}", e);
        }
    }
}

pub fn uninstall() {
    let h = HOOK_HANDLE.swap(0, Ordering::Relaxed);
    if h == 0 {
        return;
    }

    unsafe {
        let _ = UnhookWindowsHookEx(HHOOK(h as *mut _));
    }

    #[cfg(debug_assertions)]
    eprintln!("RustSwitcher: WH_KEYBOARD_LL removed");
}
