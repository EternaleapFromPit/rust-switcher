use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput, VIRTUAL_KEY, VK_CONTROL,
};

const VK_LEFT_KEY: VIRTUAL_KEY = VIRTUAL_KEY(0x25);
const VK_RIGHT_KEY: VIRTUAL_KEY = VIRTUAL_KEY(0x27);
const VK_SHIFT_KEY: VIRTUAL_KEY = VIRTUAL_KEY(0x10);

pub fn send_ctrl_combo(vk: VIRTUAL_KEY) -> bool {
    let mut seq = KeySequence::new();
    if !seq.down(VK_CONTROL) {
        return false;
    }
    seq.tap(vk)
}

pub struct KeySequence {
    pressed: Vec<VIRTUAL_KEY>,
}

impl KeySequence {
    pub fn new() -> Self {
        Self {
            pressed: Vec::new(),
        }
    }

    pub fn down(&mut self, vk: VIRTUAL_KEY) -> bool {
        if send_key(vk, false) {
            self.pressed.push(vk);
            true
        } else {
            false
        }
    }

    pub fn tap(&mut self, vk: VIRTUAL_KEY) -> bool {
        send_key(vk, false) && send_key(vk, true)
    }
}

impl Drop for KeySequence {
    fn drop(&mut self) {
        for vk in self.pressed.drain(..).rev() {
            let _ = send_key(vk, true);
        }
    }
}

pub fn send_text_unicode(text: &str) -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::KEYEVENTF_UNICODE;

    let units: Vec<u16> = text.encode_utf16().collect();
    if units.is_empty() {
        return true;
    }

    let mut inputs = Vec::with_capacity(units.len() * 2);
    for u in units {
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: u,
                    dwFlags: KEYEVENTF_UNICODE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        });

        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: u,
                    dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        });
    }

    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) } as usize;
    sent == inputs.len()
}

pub fn reselect_last_inserted_text_utf16_units(units: usize) -> bool {
    if units == 0 {
        return true;
    }

    let units = units.min(4096);
    let mut seq = KeySequence::new();

    if !(0..units).all(|_| seq.tap(VK_LEFT_KEY)) {
        return false;
    }

    if !seq.down(VK_SHIFT_KEY) {
        return false;
    }

    (0..units).all(|_| seq.tap(VK_RIGHT_KEY))
}

fn send_key(vk: VIRTUAL_KEY, key_up: bool) -> bool {
    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if key_up {
                    KEYEVENTF_KEYUP
                } else {
                    Default::default()
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    let sent = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
    sent != 0
}
