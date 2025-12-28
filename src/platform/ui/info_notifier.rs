#[cfg(debug_assertions)]
pub fn push(
    hwnd: windows::Win32::Foundation::HWND,
    _state: &mut crate::app::AppState,
    title: &str,
    text: &str,
) {
    use crate::platform::win::tray::balloon_info;

    let _ = balloon_info(hwnd, title, text);
}
