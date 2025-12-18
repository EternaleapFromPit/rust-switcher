use std::ptr::null_mut;

use crate::app::AppState;
use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    UI::{
        Input::KeyboardAndMouse::{GetKeyboardLayout, GetKeyboardLayoutList, HKL},
        WindowsAndMessaging::{
            GetForegroundWindow, GetWindowThreadProcessId, PostMessageW, WM_INPUTLANGCHANGEREQUEST,
        },
    },
};

pub fn convert_last_word(state: &mut AppState, _hwnd: HWND) {
    let delay = unsafe { crate::helpers::get_edit_u32(state.edits.delay_ms).unwrap_or(100) };
    println!("convert_last_word delay={}", delay);
}

pub fn convert_selection(state: &mut AppState, _hwnd: HWND) {
    let delay = unsafe { crate::helpers::get_edit_u32(state.edits.delay_ms).unwrap_or(100) };
    println!("convert_selection delay={}", delay);
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
    if layouts.is_empty() {
        return cur;
    }

    let mut it = layouts.iter().copied().cycle();

    while let Some(h) = it.next() {
        if h == cur {
            return it.next().unwrap_or(cur);
        }

        if h == layouts[layouts.len() - 1] {
            return layouts[0];
        }
    }

    cur
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
