use crate::helpers;

use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_ERROR, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GWLP_HINSTANCE, GetWindowLongPtrW, IMAGE_ICON, LR_SHARED, LoadImageW, WM_APP,
};

use windows::core::PCWSTR;

pub const WM_APP_TRAY: u32 = WM_APP + 2;
const TRAY_UID: u32 = 1;

fn fill_wide(dst: &mut [u16], s: &str) {
    if let Some((last, body)) = dst.split_last_mut() {
        for (d, ch) in body
            .iter_mut()
            .zip(s.encode_utf16().chain(std::iter::repeat(0)))
        {
            *d = ch;
        }
        *last = 0;
    }
}

unsafe fn default_icon(
    hwnd: HWND,
) -> windows::core::Result<windows::Win32::UI::WindowsAndMessaging::HICON> {
    let hinst = unsafe { GetWindowLongPtrW(hwnd, GWLP_HINSTANCE) } as isize;
    let hinst = HINSTANCE(hinst as *mut core::ffi::c_void);

    let h = unsafe {
        LoadImageW(
            Some(hinst),
            PCWSTR(1usize as *const u16),
            IMAGE_ICON,
            0,
            0,
            LR_SHARED,
        )
        .map(|h| windows::Win32::UI::WindowsAndMessaging::HICON(h.0))
    }?;

    Ok(h)
}

pub fn ensure_icon(hwnd: HWND) -> windows::core::Result<()> {
    unsafe {
        let mut nid = NOTIFYICONDATAW::default();
        nid.cbSize = core::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = TRAY_UID;
        nid.uCallbackMessage = WM_APP_TRAY;
        nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;

        nid.hIcon = default_icon(hwnd)?;

        fill_wide(&mut nid.szTip, "RustSwitcher");

        Shell_NotifyIconW(NIM_ADD, &nid)
            .ok()
            .map_err(|_| helpers::last_error())?;
        Ok(())
    }
}

pub fn remove_icon(hwnd: HWND) {
    unsafe {
        let mut nid = NOTIFYICONDATAW::default();
        nid.cbSize = core::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = TRAY_UID;
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}

pub fn balloon_error(hwnd: HWND, title: &str, text: &str) -> windows::core::Result<()> {
    unsafe {
        // Гарантируем, что иконка существует
        let _ = ensure_icon(hwnd);

        let mut nid = NOTIFYICONDATAW::default();
        nid.cbSize = core::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = TRAY_UID;
        nid.uFlags = NIF_INFO;
        nid.dwInfoFlags = NIIF_ERROR;

        fill_wide(&mut nid.szInfoTitle, title);
        fill_wide(&mut nid.szInfo, text);

        Shell_NotifyIconW(NIM_MODIFY, &nid)
            .ok()
            .map_err(|_| helpers::last_error())?;
        Ok(())
    }
}
