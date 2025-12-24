mod clipboard;
use clipboard as clip;

mod input;
mod last_word;

pub use last_word::convert_last_word;

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

/// Virtual key code for the `C` key.
///
/// Used together with Ctrl to trigger the standard Copy shortcut.
const VK_C_KEY: VIRTUAL_KEY = VIRTUAL_KEY(0x43);

/// Virtual key code for the Delete key.
///
/// Used to remove the current selection before inserting converted text.
const VK_DELETE_KEY: VIRTUAL_KEY = VIRTUAL_KEY(0x2E);

/// Converts the currently selected text, if there is any selection.
///
/// Selection is obtained via clipboard:
/// - sends Ctrl+C to the foreground window
/// - waits until `GetClipboardSequenceNumber` changes
/// - reads Unicode text from the clipboard
/// - restores the previous clipboard text on scope exit via RAII
///
/// Returns `true` if a non empty selection was converted, otherwise `false`.
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

/// Replaces currently selected text with layout converted text.
///
/// Implementation details:
/// - waits `delay_ms` to avoid racing the target application
/// - deletes selection with Delete
/// - injects converted Unicode via `SendInput`
/// - reselects inserted text using bounded retries (best effort)
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

/// Attempts to reselect the last inserted text using bounded retries.
///
/// Rationale:
/// Some target applications update caret position and selection state asynchronously
/// relative to `SendInput`. A single fixed delay is either too long for fast apps or
/// too short for slow ones. Retrying for a short budget reduces median latency while
/// keeping behavior stable under load.
///
/// Returns `true` if reselect succeeds within `budget`, otherwise `false`.
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

/// Converts the current selection, if it exists.
///
/// This is the same pipeline as `convert_selection_if_any`, but it is `()` and
/// simply returns when there is no selection.
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

/// Switches the keyboard layout for the current foreground window to the next installed layout.
///
/// How it works:
/// - identifies the foreground window and its thread id
/// - reads the current keyboard layout for that thread
/// - enumerates all installed layouts
/// - picks the next one cyclically
/// - posts `WM_INPUTLANGCHANGEREQUEST` to the target window
///
/// Returns `Ok(())` if the operation is completed or skipped (no window or no layouts),
/// or an error if posting the message fails.
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

/// Returns the next layout in `layouts` after `cur`, cycling back to the first.
///
/// If `cur` is not found, returns `cur`.
fn next_layout(layouts: &[HKL], cur: HKL) -> HKL {
    layouts
        .iter()
        .position(|&h| h == cur)
        .and_then(|i| layouts.get((i + 1) % layouts.len()).copied())
        .unwrap_or(cur)
}

/// Posts a layout change request message to the foreground window.
///
/// Uses `WM_INPUTLANGCHANGEREQUEST`. The `hkl` is passed through `LPARAM`.
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

/// Waits until both left and right Shift keys are released or the timeout elapses.
///
/// Returns `true` as soon as neither Shift key is currently pressed.
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

/// RAII helper that restores clipboard Unicode text on drop.
///
/// The conversion pipeline uses the clipboard for reading the current selection.
/// This guard preserves the previous clipboard text and restores it even on early
/// returns or errors.
struct ClipboardRestore {
    old: Option<String>,
}

impl ClipboardRestore {
    /// Captures current clipboard Unicode text.
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
/// The function relies on `GetClipboardSequenceNumber` to detect whether Ctrl+C produced
/// a new clipboard payload.
///
/// Returns `None` when:
/// - clipboard sequence number did not change after Ctrl+C
/// - clipboard text is empty
/// - clipboard text is multiline (contains CR or LF)
/// - clipboard text exceeds `max_chars` measured in Unicode scalar values (`chars().count()`)
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
