use windows::Win32::{
    Foundation::{COLORREF, LPARAM, LRESULT, WPARAM},
    Graphics::Gdi::{
        COLOR_WINDOW, COLOR_WINDOWTEXT, GetSysColor, GetSysColorBrush, HBRUSH, HDC, SetBkMode,
        SetTextColor, TRANSPARENT,
    },
};

pub fn on_ctlcolor(wparam: WPARAM, _lparam: LPARAM) -> LRESULT {
    unsafe {
        let hdc = HDC(wparam.0 as *mut core::ffi::c_void);
        SetTextColor(hdc, COLORREF(GetSysColor(COLOR_WINDOWTEXT)));
        SetBkMode(hdc, TRANSPARENT);
        let brush: HBRUSH = GetSysColorBrush(COLOR_WINDOW);
        LRESULT(brush.0 as isize)
    }
}
