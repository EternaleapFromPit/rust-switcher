#![cfg(debug_assertions)]

use std::sync::atomic::{AtomicIsize, Ordering};

use windows::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::{
        CallNextHookEx, HC_ACTION, HHOOK, KBDLLHOOKSTRUCT, SetWindowsHookExW, UnhookWindowsHookEx,
        WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
    },
};

static HOOK_HANDLE: AtomicIsize = AtomicIsize::new(0);

extern "system" fn proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let msg = wparam.0 as u32;
        let kb = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };

        if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
            eprintln!(
                "KBD down vk=0x{:02X} scan=0x{:02X} flags=0x{:X} time={}",
                kb.vkCode, kb.scanCode, kb.flags.0, kb.time
            );
        } else if msg == WM_KEYUP || msg == WM_SYSKEYUP {
            eprintln!(
                "KBD up   vk=0x{:02X} scan=0x{:02X} flags=0x{:X} time={}",
                kb.vkCode, kb.scanCode, kb.flags.0, kb.time
            );
        }
    }

    let h = HOOK_HANDLE.load(Ordering::Relaxed);
    let hook = if h == 0 {
        None
    } else {
        Some(HHOOK(h as *mut _))
    };
    unsafe { CallNextHookEx(hook, code, wparam, lparam) }
}

pub fn install() {
    if HOOK_HANDLE.load(Ordering::Relaxed) != 0 {
        return;
    }

    match unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(proc), None, 0) } {
        Ok(h) => {
            HOOK_HANDLE.store(h.0 as isize, Ordering::Relaxed);
            eprintln!("RustSwitcher: WH_KEYBOARD_LL installed");
        }
        Err(e) => {
            eprintln!("RustSwitcher: SetWindowsHookExW failed: {}", e);
        }
    }
}

pub fn uninstall() {
    let h = HOOK_HANDLE.swap(0, Ordering::Relaxed);
    if h == 0 {
        return;
    }

    unsafe {
        let _ = UnhookWindowsHookEx(HHOOK(h as *mut _));
    }

    eprintln!("RustSwitcher: WH_KEYBOARD_LL removed");
}
