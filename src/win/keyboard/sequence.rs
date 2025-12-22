use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    UI::WindowsAndMessaging::{PostMessageW, WM_HOTKEY},
};

use crate::config;

pub(crate) fn chord_matches(template: config::HotkeyChord, input: config::HotkeyChord) -> bool {
    if template.mods != input.mods {
        return false;
    }
    if template.vk != input.vk {
        return false;
    }
    if template.mods_vks == 0 {
        return true;
    }
    template.mods_vks == input.mods_vks
}

pub(crate) fn progress_for_slot_mut(
    state: &mut crate::app::AppState,
    slot: crate::app::HotkeySlot,
) -> &mut crate::app::SequenceProgress {
    match slot {
        crate::app::HotkeySlot::LastWord => &mut state.hotkey_sequence_progress.last_word,
        crate::app::HotkeySlot::Pause => &mut state.hotkey_sequence_progress.pause,
        crate::app::HotkeySlot::Selection => &mut state.hotkey_sequence_progress.selection,
        crate::app::HotkeySlot::SwitchLayout => &mut state.hotkey_sequence_progress.switch_layout,
    }
}

pub(crate) fn hotkey_id_for_slot(slot: crate::app::HotkeySlot) -> i32 {
    match slot {
        crate::app::HotkeySlot::LastWord => crate::hotkeys::HK_CONVERT_LAST_WORD_ID,
        crate::app::HotkeySlot::Pause => crate::hotkeys::HK_PAUSE_TOGGLE_ID,
        crate::app::HotkeySlot::Selection => crate::hotkeys::HK_CONVERT_SELECTION_ID,
        crate::app::HotkeySlot::SwitchLayout => crate::hotkeys::HK_SWITCH_LAYOUT_ID,
    }
}

pub(crate) fn effective_gap_ms(_slot: crate::app::HotkeySlot, seq: config::HotkeySequence) -> u64 {
    seq.max_gap_ms as u64
}

pub(crate) fn post_hotkey(hwnd: HWND, id: i32) -> windows::core::Result<()> {
    unsafe { PostMessageW(Some(hwnd), WM_HOTKEY, WPARAM(id as usize), LPARAM(0)) }
}

pub(crate) fn try_match_sequence(
    hwnd: HWND,
    state: &mut crate::app::AppState,
    slot: crate::app::HotkeySlot,
    chord: config::HotkeyChord,
    now_ms: u64,
) -> windows::core::Result<bool> {
    let Some(seq) = state.active_hotkey_sequences.get(slot) else {
        return Ok(false);
    };

    let first = seq.first;

    // Single chord
    let Some(second) = seq.second else {
        if chord_matches(first, chord) {
            post_hotkey(hwnd, hotkey_id_for_slot(slot))?;
            return Ok(true);
        }
        return Ok(false);
    };

    let gap_ms = effective_gap_ms(slot, seq);
    let prog = progress_for_slot_mut(state, slot);

    if prog.waiting_second {
        let elapsed_ms = now_ms.saturating_sub(prog.first_tick_ms);
        if elapsed_ms > gap_ms {
            prog.waiting_second = false;
            prog.first_tick_ms = 0;
        }
    }

    if prog.waiting_second {
        if chord_matches(second, chord) {
            prog.waiting_second = false;
            prog.first_tick_ms = 0;

            post_hotkey(hwnd, hotkey_id_for_slot(slot))?;
            return Ok(true);
        }

        if chord_matches(first, chord) {
            prog.first_tick_ms = now_ms;
            return Ok(true);
        }

        prog.waiting_second = false;
        prog.first_tick_ms = 0;
        return Ok(false);
    }

    if chord_matches(first, chord) {
        prog.waiting_second = true;
        prog.first_tick_ms = now_ms;
        return Ok(true);
    }

    Ok(false)
}

pub(crate) fn try_match_any_sequence(
    hwnd: HWND,
    state: &mut crate::app::AppState,
    chord: config::HotkeyChord,
    now_ms: u64,
) -> windows::core::Result<bool> {
    for slot in [
        crate::app::HotkeySlot::SwitchLayout,
        crate::app::HotkeySlot::LastWord,
        crate::app::HotkeySlot::Selection,
        crate::app::HotkeySlot::Pause,
    ] {
        if try_match_sequence(hwnd, state, slot, chord, now_ms)? {
            return Ok(true);
        }
    }
    Ok(false)
}
