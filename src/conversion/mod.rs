mod clipboard;
use clipboard as clip;

mod input;
mod mapping;

use std::{ptr::null_mut, thread, time::Duration};

use mapping::convert_ru_en_bidirectional;
use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    System::DataExchange::GetClipboardSequenceNumber,
    UI::{
        Input::KeyboardAndMouse::{
            GetAsyncKeyState, GetKeyboardLayout, GetKeyboardLayoutList, HKL, VIRTUAL_KEY,
            VK_LSHIFT, VK_RSHIFT,
        },
        WindowsAndMessaging::{
            GetForegroundWindow, GetWindowThreadProcessId, PostMessageW, WM_INPUTLANGCHANGEREQUEST,
        },
    },
};

use crate::{
    app::AppState,
    conversion::input::{
        KeySequence, reselect_last_inserted_text_utf16_units, send_ctrl_combo, send_text_unicode,
    },
};

const VK_C_KEY: VIRTUAL_KEY = VIRTUAL_KEY(0x43);
const VK_DELETE_KEY: VIRTUAL_KEY = VIRTUAL_KEY(0x2E);

#[tracing::instrument(level = "trace", skip(state))]
pub fn convert_selection_if_any(state: &mut AppState) -> bool {
    let Some(s) = copy_selection_text_with_clipboard_restore(256) else {
        tracing::trace!("no selection");
        return false;
    };

    tracing::trace!(len = s.chars().count(), "selection detected");
    convert_selection_from_text(state, &s);
    true
}

#[tracing::instrument(level = "trace", skip(state))]
pub fn convert_last_word(state: &mut AppState) {
    let fg = unsafe { GetForegroundWindow() };
    if fg.0.is_null() {
        tracing::warn!("foreground window is null");
        return;
    }

    if !wait_shift_released(150) {
        tracing::info!("wait_shift_released returned false");
        return;
    }

    let delay_ms = crate::helpers::get_edit_u32(state.edits.delay_ms).unwrap_or(100);
    tracing::trace!(delay_ms, "sleep before convert");
    std::thread::sleep(std::time::Duration::from_millis(delay_ms as u64));

    let Some((word, suffix)) = crate::input_journal::take_last_word_with_suffix() else {
        tracing::info!("journal: no last word");
        return;
    };

    let word_len = word.chars().count();
    let suffix_len = suffix.chars().count();
    tracing::trace!(%word, %suffix, word_len, suffix_len, "journal extracted");

    if word.is_empty() {
        tracing::warn!("journal returned empty word");
        return;
    }

    let converted = convert_ru_en_bidirectional(&word);
    tracing::trace!(%converted, "converted");

    let delete_count = word_len.saturating_add(suffix_len).min(4096);
    tracing::info!(delete_count, "delete_count computed");

    let mut seq = KeySequence::new();

    let backspace = VIRTUAL_KEY(0x08);

    if let Err(i) = (0..delete_count).try_for_each(|i| seq.tap(backspace).then_some(()).ok_or(i)) {
        tracing::error!(i, delete_count, "backspace tap failed");
        return;
    }

    tracing::trace!("backspaces sent");

    if !send_text_unicode(&converted) {
        tracing::error!("send_text_unicode(converted) failed");
        return;
    }
    tracing::trace!("converted text sent");

    if !suffix.is_empty() {
        if !send_text_unicode(&suffix) {
            tracing::error!("send_text_unicode(suffix) failed");
            return;
        }
        tracing::trace!("suffix sent");
    }

    crate::input_journal::push_text(&converted);
    if !suffix.is_empty() {
        crate::input_journal::push_text(&suffix);
    }
    tracing::trace!("journal updated");

    match switch_keyboard_layout() {
        Ok(()) => tracing::trace!("layout switched"),
        Err(e) => tracing::warn!(error = ?e, "layout switch failed"),
    }
}

/// Replaces currently selected text with layout-converted text.
///
/// Implementation details:
/// - waits `delay_ms` to avoid racing the target app
/// - deletes selection with Delete
/// - injects converted Unicode via SendInput
/// - reselects inserted text (best effort)
/// - switches keyboard layout (best effort)
fn convert_selection_from_text(state: &mut AppState, text: &str) {
    let delay_ms = crate::helpers::get_edit_u32(state.edits.delay_ms).unwrap_or(100);

    let converted = convert_ru_en_bidirectional(text);
    let converted_units = converted.encode_utf16().count();

    thread::sleep(Duration::from_millis(delay_ms as u64));

    let mut seq = KeySequence::new();

    let ok = seq.tap(VK_DELETE_KEY)
        && send_text_unicode(&converted)
        && reselect_with_retry(
            converted_units,
            Duration::from_millis(120),
            Duration::from_millis(5),
        );

    let _ = ok;
    let _ = switch_keyboard_layout();
}

fn reselect_with_retry(units: usize, budget: Duration, step_sleep: Duration) -> bool {
    let deadline = std::time::Instant::now() + budget;

    loop {
        if reselect_last_inserted_text_utf16_units(units) {
            return true;
        }
        if std::time::Instant::now() >= deadline {
            return false;
        }
        thread::sleep(step_sleep);
    }
}

#[tracing::instrument(level = "trace", skip(state))]
pub fn convert_selection(state: &mut AppState) {
    let fg = unsafe { GetForegroundWindow() };
    if fg.0.is_null() {
        tracing::warn!("foreground window is null");
        return;
    }

    if !wait_shift_released(150) {
        tracing::info!("wait_shift_released returned false");
        return;
    }

    let Some(text) = copy_selection_text_with_clipboard_restore(256) else {
        tracing::trace!("no selection");
        return;
    };

    convert_selection_from_text(state, &text);
}

pub fn switch_keyboard_layout() -> windows::core::Result<()> {
    unsafe {
        let fg = GetForegroundWindow();
        if fg.0.is_null() {
            return Ok(());
        }

        let tid = GetWindowThreadProcessId(fg, None);
        let cur = GetKeyboardLayout(tid);

        let n = GetKeyboardLayoutList(None);
        if n <= 0 {
            return Ok(());
        }

        let mut layouts = vec![HKL(null_mut()); n as usize];

        let n2 = GetKeyboardLayoutList(Some(layouts.as_mut_slice()));
        if n2 <= 0 {
            return Ok(());
        }
        layouts.truncate(n2 as usize);

        let next = next_layout(&layouts, cur);
        post_layout_change(fg, next)?;

        Ok(())
    }
}

fn next_layout(layouts: &[HKL], cur: HKL) -> HKL {
    layouts
        .iter()
        .position(|&h| h == cur)
        .and_then(|i| layouts.get((i + 1) % layouts.len()).copied())
        .unwrap_or(cur)
}

fn post_layout_change(fg: HWND, hkl: HKL) -> windows::core::Result<()> {
    unsafe {
        PostMessageW(
            Some(fg),
            WM_INPUTLANGCHANGEREQUEST,
            WPARAM(0),
            LPARAM(hkl.0 as isize),
        )?;
    }
    Ok(())
}

fn wait_shift_released(timeout_ms: u64) -> bool {
    let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms);

    std::iter::from_fn(|| {
        let now = std::time::Instant::now();
        (now < deadline).then_some(())
    })
    .any(|_| {
        let l = unsafe { GetAsyncKeyState(VK_LSHIFT.0 as i32) } as u16;
        let r = unsafe { GetAsyncKeyState(VK_RSHIFT.0 as i32) } as u16;

        let released = (l & 0x8000) == 0 && (r & 0x8000) == 0;
        if released {
            return true;
        }

        thread::sleep(Duration::from_millis(1));
        false
    })
}

/// RAII helper that restores clipboard text on drop.
/// Used to guarantee clipboard restoration even on early returns.
struct ClipboardRestore {
    old: Option<String>,
}

impl ClipboardRestore {
    fn capture() -> Self {
        Self {
            old: clip::get_unicode_text(),
        }
    }
}

impl Drop for ClipboardRestore {
    fn drop(&mut self) {
        if let Some(old_text) = self.old.as_deref() {
            let _ = clip::set_unicode_text(old_text);
        }
    }
}

/// Copies current selection via Ctrl+C, reads Unicode text from clipboard, then restores clipboard.
///
/// Returns `None` when selection is empty, multiline, too long, or clipboard did not change.
/// `max_chars` is counted in Unicode scalar values, not UTF-16 units.
fn copy_selection_text_with_clipboard_restore(max_chars: usize) -> Option<String> {
    let _restore = ClipboardRestore::capture();
    let before_seq = unsafe { GetClipboardSequenceNumber() };

    send_ctrl_combo(VK_C_KEY)
        .then(|| clip::wait_change(before_seq, 10, 20))
        .filter(|&changed| changed)
        .and_then(|_| clip::get_unicode_text())
        .filter(|s| !s.is_empty())
        .filter(|s| !s.contains('\n') && !s.contains('\r'))
        .filter(|s| s.chars().count() <= max_chars)
}
