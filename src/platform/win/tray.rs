use windows::{
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, POINT, WPARAM},
        UI::{
            Shell::{
                NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIIF_ERROR, NIIF_INFO,
                NIM_ADD, NIM_DELETE, NIM_MODIFY, NIM_SETVERSION, NOTIFY_ICON_INFOTIP_FLAGS,
                NOTIFY_ICON_MESSAGE, NOTIFYICON_VERSION_4, NOTIFYICONDATAW, Shell_NotifyIconW,
            },
            WindowsAndMessaging::{
                CreatePopupMenu, DestroyMenu, GWLP_HINSTANCE, GetCursorPos, GetWindowLongPtrW,
                HICON, HMENU, IMAGE_ICON, InsertMenuW, LR_SHARED, LoadImageW, MF_SEPARATOR,
                MF_STRING, PostMessageW, SW_HIDE, SW_SHOW, SetForegroundWindow, ShowWindow,
                TPM_BOTTOMALIGN, TPM_NOANIMATION, TPM_RETURNCMD, TPM_RIGHTALIGN, TPM_RIGHTBUTTON,
                TrackPopupMenu, WM_APP, WM_CLOSE,
            },
        },
    },
    core::{BOOL, PCWSTR, Result},
};

pub const WM_APP_TRAY: u32 = WM_APP + 3;
const TRAY_UID: u32 = 1;
const ID_EXIT: u32 = 1001;
const ID_SHOW_HIDE: u32 = 1002;

pub fn show_tray_context_menu(hwnd: HWND, window_visible: bool) -> Result<()> {
    unsafe {
        let hmenu = build_tray_menu(window_visible)?;
        let cmd = show_popup_menu_at_cursor(hwnd, hmenu);
        let _ = DestroyMenu(hmenu);
        handle_tray_menu_cmd(hwnd, window_visible, cmd)?;
        Ok(())
    }
}

fn build_tray_menu(window_visible: bool) -> Result<HMENU> {
    let hmenu = unsafe { CreatePopupMenu() }?;
    (unsafe { insert_show_hide_item(hmenu, window_visible) })?;
    (unsafe { insert_separator(hmenu, 1) })?;
    (unsafe { insert_exit_item(hmenu) })?;
    Ok(hmenu)
}

unsafe fn insert_show_hide_item(hmenu: HMENU, window_visible: bool) -> Result<()> {
    let text = if window_visible { "Hide\0" } else { "Show\0" };
    let wide = text.encode_utf16().collect::<Vec<u16>>();

    (unsafe {
        InsertMenuW(
            hmenu,
            0,
            MF_STRING,
            ID_SHOW_HIDE as usize,
            PCWSTR(wide.as_ptr()),
        )
    })?;

    Ok(())
}

unsafe fn insert_exit_item(hmenu: HMENU) -> Result<()> {
    let wide = "Exit\0".encode_utf16().collect::<Vec<u16>>();
    (unsafe { InsertMenuW(hmenu, 0, MF_STRING, ID_EXIT as usize, PCWSTR(wide.as_ptr())) })?;
    Ok(())
}

unsafe fn insert_separator(hmenu: HMENU, position: u32) -> Result<()> {
    (unsafe { InsertMenuW(hmenu, position, MF_SEPARATOR, 0, PCWSTR::null()) })?;
    Ok(())
}

unsafe fn show_popup_menu_at_cursor(hwnd: HWND, hmenu: HMENU) -> BOOL {
    let mut pt = POINT { x: 0, y: 0 };
    let _ = unsafe { GetCursorPos(&raw mut pt) };

    let _ = unsafe { SetForegroundWindow(hwnd) };

    unsafe {
        TrackPopupMenu(
            hmenu,
            TPM_RETURNCMD | TPM_BOTTOMALIGN | TPM_RIGHTALIGN | TPM_NOANIMATION | TPM_RIGHTBUTTON,
            pt.x,
            pt.y,
            Some(0),
            hwnd,
            None,
        )
    }
}

unsafe fn handle_tray_menu_cmd(hwnd: HWND, window_visible: bool, cmd: BOOL) -> Result<()> {
    match cmd.0.cast_unsigned() {
        ID_SHOW_HIDE => unsafe { toggle_window_visibility(hwnd, window_visible) },
        ID_EXIT => unsafe { request_window_close(hwnd) }?,
        _ => {}
    }
    Ok(())
}

unsafe fn toggle_window_visibility(hwnd: HWND, window_visible: bool) {
    if window_visible {
        let _ = unsafe { ShowWindow(hwnd, SW_HIDE) };
    } else {
        let _ = unsafe { ShowWindow(hwnd, SW_SHOW) };
        let _ = unsafe { SetForegroundWindow(hwnd) };
    }
}

unsafe fn request_window_close(hwnd: HWND) -> Result<()> {
    (unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) })?;
    Ok(())
}

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

fn shell_notify(
    action: NOTIFY_ICON_MESSAGE,
    nid: &NOTIFYICONDATAW,
    what: &str,
) -> windows::core::Result<()> {
    unsafe {
        if Shell_NotifyIconW(action, nid).as_bool() {
            Ok(())
        } else {
            Err(windows::core::Error::new(
                windows::core::HRESULT(0x8000_4005_u32.cast_signed()),
                format!("Shell_NotifyIconW returned FALSE: {what}"),
            ))
        }
    }
}

pub fn ensure_icon(hwnd: HWND) -> windows::core::Result<()> {
    unsafe {
        let mut nid = base_tray_nid(hwnd)?;
        apply_tray_identity(&mut nid, hwnd)?;
        add_or_modify_tray_icon(&nid)?;
        set_tray_version(&mut nid)?;
        Ok(())
    }
}

unsafe fn base_tray_nid(hwnd: HWND) -> windows::core::Result<NOTIFYICONDATAW> {
    Ok(NOTIFYICONDATAW {
        cbSize: u32::try_from(core::mem::size_of::<NOTIFYICONDATAW>())?,
        hWnd: hwnd,
        uID: TRAY_UID,
        ..Default::default()
    })
}

unsafe fn apply_tray_identity(nid: &mut NOTIFYICONDATAW, hwnd: HWND) -> windows::core::Result<()> {
    nid.uCallbackMessage = WM_APP_TRAY;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP;

    nid.hIcon = unsafe { default_icon(hwnd) }?;
    fill_wide(&mut nid.szTip, "RustSwitcher");

    Ok(())
}

unsafe fn add_or_modify_tray_icon(nid: &NOTIFYICONDATAW) -> windows::core::Result<()> {
    if unsafe { Shell_NotifyIconW(NIM_ADD, &raw const *nid).as_bool() } {
        return Ok(());
    }

    shell_notify(
        NIM_MODIFY,
        nid,
        "ensure_icon: NIM_MODIFY after NIM_ADD failure",
    )
}

unsafe fn set_tray_version(nid: &mut NOTIFYICONDATAW) -> windows::core::Result<()> {
    nid.Anonymous.uVersion = NOTIFYICON_VERSION_4;

    if unsafe { Shell_NotifyIconW(NIM_SETVERSION, &raw const *nid).as_bool() } {
        Ok(())
    } else {
        Err(windows::core::Error::new(
            windows::core::HRESULT(0x8000_4005_u32.cast_signed()),
            "Shell_NotifyIconW returned FALSE: ensure_icon NIM_SETVERSION",
        ))
    }
}

fn balloon_common(
    hwnd: HWND,
    title: &str,
    text: &str,
    flags: u32,
    what: &str,
) -> windows::core::Result<()> {
    ensure_icon(hwnd)?;

    let mut nid = NOTIFYICONDATAW {
        cbSize: u32::try_from(core::mem::size_of::<NOTIFYICONDATAW>())?,
        hWnd: hwnd,
        uID: TRAY_UID,
        ..Default::default()
    };

    nid.uCallbackMessage = WM_APP_TRAY;
    nid.uFlags = NIF_INFO | NIF_MESSAGE;
    nid.dwInfoFlags = NOTIFY_ICON_INFOTIP_FLAGS(flags);
    nid.Anonymous.uTimeout = 10_000;

    fill_wide(&mut nid.szInfoTitle, title);
    fill_wide(&mut nid.szInfo, text);

    if unsafe { Shell_NotifyIconW(NIM_MODIFY, &raw const nid).as_bool() } {
        return Ok(());
    }

    remove_icon(hwnd);
    ensure_icon(hwnd)?;

    shell_notify(NIM_MODIFY, &nid, what)
}

pub fn balloon_error(hwnd: HWND, title: &str, text: &str) -> windows::core::Result<()> {
    balloon_common(hwnd, title, text, NIIF_ERROR.0, "balloon_error: NIM_MODIFY")
}

pub fn balloon_info(hwnd: HWND, title: &str, text: &str) -> windows::core::Result<()> {
    balloon_common(hwnd, title, text, NIIF_INFO.0, "balloon_info: NIM_MODIFY")
}

pub fn switch_tray_icon(hwnd: HWND, use_green: bool) -> windows::core::Result<()> {
    unsafe {
        let icon = if use_green {
            green_icon(hwnd)?
        } else {
            default_icon(hwnd)?
        };

        let mut nid = NOTIFYICONDATAW {
            cbSize: u32::try_from(core::mem::size_of::<NOTIFYICONDATAW>())?,
            hWnd: hwnd,
            uID: TRAY_UID,
            ..Default::default()
        };

        nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP | NIF_SHOWTIP;
        nid.uCallbackMessage = WM_APP_TRAY;
        nid.hIcon = icon;

        shell_notify(NIM_MODIFY, &nid, "switch_tray_icon")
    }
}

unsafe fn window_hinstance(hwnd: HWND) -> HINSTANCE {
    let raw = unsafe { GetWindowLongPtrW(hwnd, GWLP_HINSTANCE) };
    HINSTANCE(raw as *mut core::ffi::c_void)
}

unsafe fn green_icon(hwnd: HWND) -> windows::core::Result<HICON> {
    let hinst = unsafe { window_hinstance(hwnd) };

    let h = unsafe {
        LoadImageW(
            Some(hinst),
            #[allow(clippy::manual_dangling_ptr)]
            PCWSTR(2usize as *const u16),
            IMAGE_ICON,
            0,
            0,
            LR_SHARED,
        )
    }?;

    Ok(HICON(h.0))
}

unsafe fn default_icon(hwnd: HWND) -> windows::core::Result<HICON> {
    let hinst = unsafe { window_hinstance(hwnd) };

    let h = unsafe {
        LoadImageW(
            Some(hinst),
            #[allow(clippy::manual_dangling_ptr)]
            PCWSTR(1usize as *const u16),
            IMAGE_ICON,
            0,
            0,
            LR_SHARED,
        )
    }?;

    Ok(HICON(h.0))
}

pub fn remove_icon(hwnd: HWND) {
    unsafe {
        let nid = NOTIFYICONDATAW {
            cbSize: core::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: TRAY_UID,
            ..Default::default()
        };

        let _ = Shell_NotifyIconW(NIM_DELETE, &raw const nid);
    }
}
