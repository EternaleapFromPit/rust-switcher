use crate::app::{AppState, UiError};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_APP};

pub const WM_APP_ERROR: u32 = WM_APP + 1;

pub fn push(
    hwnd: HWND,
    state: &mut AppState,
    title: &str,
    user_text: &str,
    err: &windows::core::Error,
) {
    let debug_text = format!("{:?}", err);

    state.errors.push_back(UiError {
        title: title.to_string(),
        user_text: user_text.to_string(),
        debug_text,
    });

    unsafe {
        let _ = PostMessageW(Some(hwnd), WM_APP_ERROR, WPARAM(0), LPARAM(0));
    }
}

pub fn drain_one(state: &mut AppState) -> Option<UiError> {
    state.errors.pop_front()
}
