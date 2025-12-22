mod mods;
mod sequence;
mod vk;

use std::sync::atomic::{AtomicIsize, Ordering};

use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    System::SystemInformation::GetTickCount64,
    UI::WindowsAndMessaging::{
        CallNextHookEx, HC_ACTION, HHOOK, KBDLLHOOKSTRUCT, SetWindowsHookExW, WH_KEYBOARD_LL,
    },
};

use self::vk::{is_keydown_msg, is_keyup_msg, mod_bit_for_vk, normalize_vk};
use crate::{
    config, helpers,
    win::keyboard::{
        mods::{chord_from_vk, mods_now, update_mods_down_press, update_mods_down_release},
        sequence::try_match_any_sequence,
    },
};

static HOOK_HANDLE: AtomicIsize = AtomicIsize::new(0);
static MAIN_HWND: AtomicIsize = AtomicIsize::new(0);

fn now_tick_ms() -> u64 {
    unsafe { GetTickCount64() }
}

fn chord_to_hotkey(ch: config::HotkeyChord) -> config::Hotkey {
    config::Hotkey {
        vk: ch.vk.unwrap_or(0),
        mods: ch.mods,
    }
}

fn main_hwnd() -> Option<HWND> {
    let raw = MAIN_HWND.load(Ordering::Relaxed);
    if raw == 0 {
        None
    } else {
        Some(HWND(raw as *mut _))
    }
}

#[allow(dead_code)]
fn should_swallow(hwnd: HWND) -> bool {
    super::with_state_mut(hwnd, |s| s.hotkey_capture.active).unwrap_or(false)
}

fn push_chord_capture(
    existing: Option<config::HotkeySequence>,
    chord: config::HotkeyChord,
    now_ms: u64,
    last_input_tick_ms: &mut u64,
) -> config::HotkeySequence {
    const DEFAULT_GAP_MS: u32 = 1000;
    const RESET_AFTER_MS: u64 = 2000;

    let existing = match (*last_input_tick_ms, existing) {
        (0, s) => s,
        (prev, s) if now_ms.saturating_sub(prev) > RESET_AFTER_MS => None,
        (_, s) => s,
    };

    let seq = match existing {
        None => config::HotkeySequence {
            first: chord,
            second: None,
            max_gap_ms: DEFAULT_GAP_MS,
        },
        Some(mut s) => match s.second {
            None => {
                s.second = Some(chord);
                s
            }
            Some(prev_second) => {
                s.first = prev_second;
                s.second = Some(chord);
                s
            }
        },
    };

    *last_input_tick_ms = now_ms;
    seq
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum HookDecision {
    Pass,
    Swallow,
}

impl HookDecision {
    fn should_swallow(self) -> bool {
        matches!(self, Self::Swallow)
    }
}

fn report_hook_error(hwnd: HWND, state: &mut crate::app::AppState, e: &windows::core::Error) {
    crate::ui::error_notifier::push(
        hwnd,
        state,
        crate::ui::error_notifier::T_UI,
        "Hotkey handling failed",
        e,
    );
}

fn handle_keydown(vk: u32, is_mod: bool) -> windows::core::Result<HookDecision> {
    update_mods_down_press(vk);

    let Some(hwnd) = main_hwnd() else {
        return Ok(HookDecision::Pass);
    };

    let now_ms = now_tick_ms();

    super::with_state_mut(hwnd, |state| {
        handle_keydown_in_state(hwnd, state, vk, is_mod, now_ms)
    })
    .unwrap_or(Ok(HookDecision::Pass))
}

fn handle_keydown_in_state(
    hwnd: HWND,
    state: &mut crate::app::AppState,
    vk: u32,
    is_mod: bool,
    now_ms: u64,
) -> windows::core::Result<HookDecision> {
    let chord = chord_from_vk(vk);

    if state.hotkey_capture.active {
        return handle_keydown_capture(state, chord, is_mod, now_ms);
    }

    handle_keydown_runtime(hwnd, state, chord, is_mod, now_ms)
}

fn handle_keydown_capture(
    state: &mut crate::app::AppState,
    chord: config::HotkeyChord,
    is_mod: bool,
    now_ms: u64,
) -> windows::core::Result<HookDecision> {
    let Some(slot) = state.hotkey_capture.slot else {
        return Ok(HookDecision::Pass);
    };

    if is_mod {
        state.hotkey_capture.pending_mods = chord.mods;
        state.hotkey_capture.pending_mods_vks = chord.mods_vks;
        state.hotkey_capture.pending_mods_valid = true;
        state.hotkey_capture.saw_non_mod = false;
        return Ok(HookDecision::Swallow);
    }

    state.hotkey_capture.saw_non_mod = true;
    state.hotkey_capture.pending_mods_valid = false;

    let prev = state.hotkey_sequence_values.get(slot);
    let seq = push_chord_capture(
        prev,
        chord,
        now_ms,
        &mut state.hotkey_capture.last_input_tick_ms,
    );

    store_captured_hotkey(state, slot, chord, seq)?;
    Ok(HookDecision::Swallow)
}

fn store_captured_hotkey(
    state: &mut crate::app::AppState,
    slot: crate::app::HotkeySlot,
    chord: config::HotkeyChord,
    seq: config::HotkeySequence,
) -> windows::core::Result<()> {
    state.hotkey_sequence_values.set(slot, Some(seq));
    state.hotkey_values.set(slot, Some(chord_to_hotkey(chord)));

    let text = super::format_hotkey_sequence(Some(seq));
    let target = ui_hotkey_target(state, slot);

    helpers::set_edit_text(target, &text)?;
    Ok(())
}

fn ui_hotkey_target(state: &crate::app::AppState, slot: crate::app::HotkeySlot) -> HWND {
    match slot {
        crate::app::HotkeySlot::LastWord => state.hotkeys.last_word,
        crate::app::HotkeySlot::Pause => state.hotkeys.pause,
        crate::app::HotkeySlot::Selection => state.hotkeys.selection,
        crate::app::HotkeySlot::SwitchLayout => state.hotkeys.switch_layout,
    }
}

fn handle_keydown_runtime(
    hwnd: HWND,
    state: &mut crate::app::AppState,
    chord: config::HotkeyChord,
    is_mod: bool,
    now_ms: u64,
) -> windows::core::Result<HookDecision> {
    if is_mod {
        state.runtime_chord_capture.pending_mods = chord.mods;
        state.runtime_chord_capture.pending_mods_vks = chord.mods_vks;
        state.runtime_chord_capture.pending_mods_valid = true;
        state.runtime_chord_capture.saw_non_mod = false;
        return Ok(HookDecision::Pass);
    }

    state.runtime_chord_capture.saw_non_mod = true;
    state.runtime_chord_capture.pending_mods_valid = false;

    let matched = try_match_any_sequence(hwnd, state, chord, now_ms)?;
    Ok(if matched {
        HookDecision::Swallow
    } else {
        HookDecision::Pass
    })
}

fn handle_keyup(vk: u32, is_mod: bool) -> windows::core::Result<HookDecision> {
    update_mods_down_release(vk);

    let Some(hwnd) = main_hwnd() else {
        return Ok(HookDecision::Pass);
    };

    let now_ms = now_tick_ms();

    super::with_state_mut(hwnd, |state| {
        handle_keyup_in_state(hwnd, state, vk, is_mod, now_ms)
    })
    .unwrap_or(Ok(HookDecision::Pass))
}

fn handle_keyup_in_state(
    hwnd: HWND,
    state: &mut crate::app::AppState,
    _vk: u32,
    is_mod: bool,
    now_ms: u64,
) -> windows::core::Result<HookDecision> {
    if state.hotkey_capture.active {
        return handle_keyup_capture(state, is_mod, now_ms);
    }

    handle_keyup_runtime(hwnd, state, is_mod, now_ms)
}

fn handle_keyup_capture(
    state: &mut crate::app::AppState,
    is_mod: bool,
    now_ms: u64,
) -> windows::core::Result<HookDecision> {
    let Some(slot) = state.hotkey_capture.slot else {
        return Ok(HookDecision::Pass);
    };

    if !is_mod {
        return Ok(HookDecision::Swallow);
    }

    if !state.hotkey_capture.pending_mods_valid {
        return Ok(HookDecision::Swallow);
    }
    if state.hotkey_capture.saw_non_mod {
        return Ok(HookDecision::Swallow);
    }

    let mods_now = mods_now();
    if mods_now != 0 {
        return Ok(HookDecision::Swallow);
    }

    let chord = config::HotkeyChord {
        mods: state.hotkey_capture.pending_mods,
        mods_vks: state.hotkey_capture.pending_mods_vks,
        vk: None,
    };

    let prev = state.hotkey_sequence_values.get(slot);
    let seq = push_chord_capture(
        prev,
        chord,
        now_ms,
        &mut state.hotkey_capture.last_input_tick_ms,
    );

    state.hotkey_capture.pending_mods_valid = false;
    state.hotkey_capture.pending_mods = 0;
    state.hotkey_capture.pending_mods_vks = 0;

    store_captured_hotkey(state, slot, chord, seq)?;
    Ok(HookDecision::Swallow)
}

fn handle_keyup_runtime(
    hwnd: HWND,
    state: &mut crate::app::AppState,
    is_mod: bool,
    now_ms: u64,
) -> windows::core::Result<HookDecision> {
    if !is_mod {
        return Ok(HookDecision::Pass);
    }

    if !state.runtime_chord_capture.pending_mods_valid {
        return Ok(HookDecision::Pass);
    }
    if state.runtime_chord_capture.saw_non_mod {
        return Ok(HookDecision::Pass);
    }

    let mods_now = mods_now();
    if mods_now != 0 {
        return Ok(HookDecision::Pass);
    }

    let chord = config::HotkeyChord {
        mods: state.runtime_chord_capture.pending_mods,
        mods_vks: state.runtime_chord_capture.pending_mods_vks,
        vk: None,
    };

    state.runtime_chord_capture = crate::app::RuntimeChordCapture::default();

    let matched = try_match_any_sequence(hwnd, state, chord, now_ms)?;
    Ok(if matched {
        HookDecision::Swallow
    } else {
        HookDecision::Pass
    })
}

extern "system" fn proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code != HC_ACTION as i32 {
        let h = HOOK_HANDLE.load(Ordering::Relaxed);
        let hook = (h != 0).then_some(HHOOK(h as *mut _));
        return unsafe { CallNextHookEx(hook, code, wparam, lparam) };
    }

    let msg = wparam.0 as u32;
    let kb = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
    let vk = normalize_vk(kb);
    let is_mod = mod_bit_for_vk(vk).is_some();

    let decision = if is_keydown_msg(msg) {
        handle_keydown(vk, is_mod)
    } else if is_keyup_msg(msg) {
        handle_keyup(vk, is_mod)
    } else {
        Ok(HookDecision::Pass)
    };

    match decision {
        Ok(d) if d.should_swallow() && !(is_mod && is_keyup_msg(msg)) => return LRESULT(1),
        Ok(_) => {}
        Err(e) => {
            if let Some(hwnd) = main_hwnd() {
                super::with_state_mut_do(hwnd, |state| {
                    report_hook_error(hwnd, state, &e);
                });
            }
        }
    }

    let h = HOOK_HANDLE.load(Ordering::Relaxed);
    let hook = (h != 0).then_some(HHOOK(h as *mut _));
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
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("RustSwitcher: SetWindowsHookExW failed: {}", _e);
        }
    }
}
