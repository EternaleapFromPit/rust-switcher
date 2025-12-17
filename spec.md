# Rust Switcher - Specification

## Purpose

Rust Switcher is a small background utility for Windows 11 that performs explicit, user triggered keyboard layout conversion.

The application does not try to automatically fix input. All actions are executed only by user commands.

Supported actions:
- Convert last word: convert the last word to the left of the caret
- Convert selection: convert the currently selected text
- Switch keyboard layout: switch the system layout only, without converting text
- Pause: temporarily disable all hotkey handling

---

## Target Platform and Scope

- OS: Windows 11
- Works in regular windowed applications:
  - browsers
  - IDEs
  - messengers
- Not required to work in:
  - fullscreen applications
  - games
  - borderless fullscreen windows

---

## Hotkeys

The application uses a set of hotkeys configurable via the GUI.

### Actions

1) Convert last word
- If there is a selection, behaves like Convert selection
- If there is no selection, selects the last word to the left of the caret and converts it

2) Convert selection
- Requires a non empty selection, otherwise does nothing

3) Switch keyboard layout
- Always switches the system keyboard layout to the next layout
- Does not modify any text

4) Pause
- Toggles pause state on or off
- When paused, other hotkeys do nothing

### Default Hotkeys

Defaults match the reference UI:
- Convert last word: Pause
- Convert selection: Ctrl + Break (sometimes shown as Control + Cancel)
- Switch keyboard layout: None
- Pause: None

Notes:
- On assignment, if the hotkey is unavailable or conflicts, the action must not become active and the previous value remains.
- The application does not override Alt + Shift or Win + Space and relies on standard Windows layout switching.

---

## Execution Delay

Setting: Delay before switching (ms)
- A delay before running the copy, switch layout, paste sequence
- Needed for stability in apps with slow selection or clipboard updates
- Configured as an integer in milliseconds
- Default value: 100 ms

---

## Text Conversion

- Simple character mapping based on keyboard layouts
- No language detection
- No heuristics
- No guessing of the correct layout

Logic:
- take the existing text
- switch the system layout
- transform characters according to the new layout
- insert the result

Supports:
- any number of layouts
- layout cycling via the system round robin

---

## Action Flows

### Convert selection

1) Send Ctrl + C
2) Read text from the system clipboard
3) Wait Delay before switching
4) Switch system keyboard layout to the next layout
5) Convert the text according to the new layout
6) Send Ctrl + V
7) Restore original clipboard contents

Requirements:
- Clipboard is always backed up and restored
- No per character backspace logic

### Convert last word

If there is no selection:
1) Select the last word to the left of the caret
   - stop at whitespace or start of line
2) Then run the same flow as Convert selection

---

## Tray and GUI

### Tray Icon

Setting: Show tray icon
- If enabled, the tray icon is always present while the application is running
- If disabled, there is no tray icon, and the GUI can only be opened from the window

Tray context menu:
- Pause or Resume
- Exit

Tray click:
- Show or hide GUI

### GUI

The settings window is minimal and matches the reference layout.

Left side:
- Checkbox: Start on windows startup
- Checkbox: Show tray icon
- Input: Delay before switching (ms)
- Button: Report an issue
- Button: Exit program

Right side, Hotkeys group:
- Read only display fields showing current hotkeys:
  - Convert last word
  - Pause
  - Convert selection
  - Switch keyboard layout
- Must support the value None

Bottom buttons:
- Apply
  - atomically writes the config and applies settings
- Cancel
  - discards UI changes and restores values from the current config

### Report an issue

- Opens the project issues page using the system shell
- The application itself does not send any data and has no telemetry

---

## Autostart

Portable application model.

When Start on windows startup is enabled:
- the executable is copied to `%APPDATA%\RustSwitcher\`
- `config.json` is stored there
- autostart points to that copied executable

When disabled:
- the autostart entry is removed
- files under `%APPDATA%\RustSwitcher\` are not removed automatically

---

## Configuration

- Path: `%APPDATA%\RustSwitcher\config.json`
- Format: JSON
- Atomic writes (write to temp then rename)
- Settings:
  - start_on_startup: bool
  - show_tray_icon: bool
  - delay_ms: u32
  - hotkey_convert_last_word: Hotkey or None
  - hotkey_convert_selection: Hotkey or None
  - hotkey_switch_layout: Hotkey or None
  - hotkey_pause: Hotkey or None
  - paused: bool

---

## Logging and Networking

- Release builds:
  - no logs
  - no networking
  - no telemetry
- Debug builds:
  - optional debug logging

---

## Stability and Safety

- Single instance only
- Self generated input events are ignored
- When paused, no hotkeys except Pause are executed
- Clipboard is always restored even on errors (best effort)

---

## Out of Scope

- AI features
- API based translation
- smart language detection
- macOS or Linux (for now)

---

## Definition of Done

- Convert last word works reliably in browsers and IDEs
- Convert selection works reliably in browsers and IDEs
- Switch keyboard layout actually switches the system layout
- Tray, pause or resume, and autostart work
- GUI matches the spec, Apply or Cancel works correctly
- No unnecessary behavior
