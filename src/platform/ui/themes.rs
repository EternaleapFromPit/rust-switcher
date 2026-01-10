use windows::Win32::Graphics::Gdi::{
    SetBkColor, // For SetBkColor
};
use windows::{
    Win32::{
        Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::{
            Dwm::{DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute},
            Gdi::{
                COLOR_WINDOW, COLOR_WINDOWTEXT, CreateSolidBrush, DeleteObject, FillRect,
                GetSysColor, GetSysColorBrush, HBRUSH, HDC, HGDIOBJ, SetBkMode, SetTextColor,
                TRANSPARENT,
            },
        },
        UI::{Controls::SetWindowTheme, WindowsAndMessaging::GetClientRect},
    },
    core::{BOOL, w},
};

use crate::platform::win::state::{get_state, with_state_mut_do};

// Define RGNDATA structure (simplified - we only need a pointer to it)
#[repr(C)]
#[allow(clippy::upper_case_acronyms)]
pub struct RGNDATA {
    _private: [u8; 0], // Zero-sized type since we only pass null
}

#[link(name = "user32")]
unsafe extern "system" {
    pub fn InvalidateRect(hWnd: HWND, lpRect: *const RECT, bErase: BOOL) -> BOOL;
    pub fn UpdateWindow(hWnd: HWND) -> BOOL;
    pub fn RedrawWindow(
        hWnd: HWND,
        lprcUpdate: *const RECT,
        hrgnUpdate: *const RGNDATA,
        flags: u32,
    ) -> BOOL;
}

const RDW_INVALIDATE: u32 = 0x0001;
const RDW_ALLCHILDREN: u32 = 0x0080;

/// Handles `WM_CTLCOLOR*` style messages by configuring the device context and returning a brush.
///
/// Expected usage:
/// - called from a window procedure when processing control color messages
/// - `wparam` is interpreted as an `HDC` for the control being painted
///
/// What it does:
/// - sets the text color to the system `COLOR_WINDOWTEXT` color
/// - sets background mode to `TRANSPARENT` so the parent background shows through
/// - returns a system brush for `COLOR_WINDOW` to paint the control background
///
/// Return value:
/// - `LRESULT` containing an `HBRUSH` handle, as required by `WM_CTLCOLOR*`
///
/// Safety:
/// - this function performs raw handle casts (`WPARAM` to `HDC`) and calls Win32 APIs
///   that assume a valid device context.
pub fn on_ctlcolor(wparam: WPARAM, _lparam: LPARAM) -> LRESULT {
    unsafe {
        let hdc = HDC(wparam.0 as *mut core::ffi::c_void);
        SetTextColor(hdc, COLORREF(GetSysColor(COLOR_WINDOWTEXT)));
        SetBkMode(hdc, TRANSPARENT);

        let brush: HBRUSH = GetSysColorBrush(COLOR_WINDOW);
        LRESULT(brush.0 as isize)
    }
}

#[allow(clippy::unnecessary_cast)]
#[allow(clippy::needless_return)]
pub fn on_color_dialog(hwnd: HWND, wparam: WPARAM, _lparam: LPARAM) -> LRESULT {
    if let Some(state) = get_state(hwnd) && state.current_theme_dark {
        let hdc = HDC(wparam.0 as *mut std::ffi::c_void);
        unsafe {
            SetBkColor(hdc, COLORREF(0x002D2D30));
            SetTextColor(hdc, COLORREF(0x00FFFFFF));
            return LRESULT(
                CreateSolidBrush(COLORREF(0x002D2D30)).0 as *mut std::ffi::c_void as isize,
            );
        }
    }
    return on_ctlcolor(wparam, _lparam);
}

#[allow(clippy::unnecessary_cast)]
#[allow(clippy::needless_return)]
pub fn on_color_static(hwnd: HWND, wparam: WPARAM, _lparam: LPARAM) -> LRESULT {
    if let Some(state) = get_state(hwnd) && state.current_theme_dark {
        let hdc = HDC(wparam.0 as *mut std::ffi::c_void);
        unsafe {
            SetBkColor(hdc, COLORREF(0x002D2D30)); // 0x00BBGGRR
            SetTextColor(hdc, COLORREF(0x00FFFFFF));
            return LRESULT(
                CreateSolidBrush(COLORREF(0x002D2D30)).0 as *mut std::ffi::c_void as isize,
            );
        }
    }
    return on_ctlcolor(wparam, _lparam);
}

#[allow(clippy::useless_format)]
#[allow(clippy::needless_return)]
#[allow(clippy::unnecessary_cast)]
pub fn on_color_edit(hwnd: HWND, wparam: WPARAM, _lparam: LPARAM) -> LRESULT {
    if let Some(state) = get_state(hwnd) && state.current_theme_dark {
        let hdc = HDC(wparam.0 as *mut std::ffi::c_void);
        unsafe {
            SetBkColor(hdc, COLORREF(0x001E1E1E)); // Dark gray for dark theme
            SetTextColor(hdc, COLORREF(0x00FFFFFF)); // White text for dark theme
            return LRESULT(
                CreateSolidBrush(COLORREF(0x002D2D30)).0 as *mut std::ffi::c_void as isize,
            );
        }
    }
    return on_ctlcolor(wparam, _lparam);
}

#[allow(clippy::useless_format)]
#[allow(clippy::needless_return)]
pub fn on_erase_background(hwnd: HWND, wparam: WPARAM, _lparam: LPARAM) -> LRESULT {
    if let Some(state) = get_state(hwnd) && state.current_theme_dark {
        // Paint main window background
        let hdc = HDC(wparam.0 as *mut std::ffi::c_void);
        let mut rect = RECT::default();
        unsafe {
            let _ = GetClientRect(hwnd, &mut rect);
            let brush = CreateSolidBrush(COLORREF(0x002D2D30));
            FillRect(hdc, &rect, brush);
            let _ = DeleteObject(HGDIOBJ::from(brush));
        }
        return LRESULT(1);
    } else {
        // Explicit light theme background (white)
        let hdc = HDC(wparam.0 as *mut std::ffi::c_void);
        let mut rect = RECT::default();
        unsafe {
            let _ = GetClientRect(hwnd, &mut rect);
            let brush = CreateSolidBrush(COLORREF(0x00FFFFFF)); // White
            // Or use system color: let brush = GetSysColorBrush(COLOR_WINDOW as i32);
            FillRect(hdc, &rect, brush);
            let _ = DeleteObject(HGDIOBJ::from(brush));
        }
        return LRESULT(1);
    }
}

#[allow(clippy::useless_format)]
pub fn set_window_theme(hwnd_main: HWND, current_theme_dark: bool) {
    crate::utils::helpers::debug_log(&format!("dark={current_theme_dark}"));

    unsafe {
        if !current_theme_dark {
            let mut dark_mode: BOOL = BOOL(1);

            let _ = DwmSetWindowAttribute(
                hwnd_main,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                &mut dark_mode as *mut _ as *const _,
                std::mem::size_of::<BOOL>() as u32,
            );

            let _ = SetWindowTheme(
                hwnd_main,
                w!("DarkMode_Explorer"),
                windows::core::PCWSTR::null(),
            );
            with_state_mut_do(hwnd_main, |state| {
                state.current_theme_dark = true;
            });
            crate::utils::helpers::debug_log(&format!("dark mode set to TRUE"));
        } else {
            // Revert to light mode
            let mut light_mode: BOOL = BOOL(0);
            let _ = DwmSetWindowAttribute(
                hwnd_main,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                &mut light_mode as *mut _ as *const _,
                std::mem::size_of::<BOOL>() as u32,
            );
            let _ = SetWindowTheme(hwnd_main, w!("Explorer"), windows::core::PCWSTR::null());
            with_state_mut_do(hwnd_main, |state| {
                state.current_theme_dark = false;
            });
            crate::utils::helpers::debug_log(&format!("dark mode set to FALSE"));
        }

        // Force window repaint to apply the theme changes
        let _ = InvalidateRect(hwnd_main, std::ptr::null(), BOOL(1));
        let _ = UpdateWindow(hwnd_main);

        // Also redraw child controls
        let _ = RedrawWindow(
            hwnd_main,
            std::ptr::null(),
            std::ptr::null(),
            RDW_INVALIDATE | RDW_ALLCHILDREN,
        );
    }
}
