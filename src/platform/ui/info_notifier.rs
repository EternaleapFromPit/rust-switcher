use windows::Win32::Foundation::HWND;

use crate::app::AppState;

#[allow(dead_code)]
pub fn push(hwnd: HWND, _state: &mut AppState, title: &str, text: &str) {
    crate::platform::ui::notify::push_info(hwnd, title, text);
}
