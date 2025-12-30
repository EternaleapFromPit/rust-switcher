# Rust Switcher - Specification (code driven)

This document describes the current behavior and architecture of the repository implementation.
It is intended as onboarding documentation and as a reference for expected runtime behavior.

## Scope

- Supported OS: Windows only
- Primary UI: native Win32 window + optional tray icon
- Linux: out of scope for current implementation, tracked in roadmap

## Core user goals

- Convert text typed in the wrong keyboard layout (RU <-> EN) quickly and reliably.
- Work without relying on fragile clipboard only flows for the main path.
- Provide global hotkeys and light UI for configuration.

## Terminology

- Convert: map characters between keyboard layouts (RU <-> EN) using the built in mapping table.
- Selection: currently selected text in the active application.
- Last word: the last token captured by the keyboard hook input journal.
- Autoconvert: automatic conversion triggered by typed delimiter characters (for example Space).
- Paused: disables autoconvert only. Hotkeys still work.

## High level architecture

- UI boundary (Win32):
  - Window procedure, message loop, controls, Apply and Cancel logic
  - Tray icon integration
- Input boundary:
  - Low level keyboard hook (WH_KEYBOARD_LL)
  - Input journal and ring buffer for tokenization and last word extraction
- Domain logic:
  - text conversion, replacement of selection, insertion via SendInput
- Platform integration:
  - global hotkeys (RegisterHotKey)
  - keyboard layout switching
  - autostart shortcut in Startup folder
  - notifications (tray balloon, MessageBox fallback)
- Persistence:
  - config stored under %APPDATA%\RustSwitcher\config.json via confy

## Configuration

Config fields (see src/config.rs):
- start_on_startup: bool
- delay_ms: u32
- paused: bool
- Hotkeys (legacy single chord, optional):
  - hotkey_convert_last_word
  - hotkey_convert_selection
  - hotkey_switch_layout
  - hotkey_pause
- Hotkey sequences (preferred, optional):
  - hotkey_convert_last_word_sequence
  - hotkey_pause_sequence
  - hotkey_switch_layout_sequence

Default bindings (current defaults in code):
- Convert smart: double tap Left Shift within 1000 ms
- Pause toggle (autoconvert only): press Left Shift + Right Shift together
- Switch keyboard layout: CapsLock

Notes:
- The UI currently displays hotkeys as read only values derived from config.
- Hotkey sequences are validated on save.

## Actions and behavior

### Convert smart

This is the primary conversion action.
Behavior:
- If there is a non empty selection, it converts the selection.
- Otherwise it converts last word using the input journal.

### Convert selection

Algorithm (domain/text/convert.rs):
- Copy selection text while restoring clipboard afterwards (best effort).
- Sleep for delay_ms before conversion and replacement.
- Convert the copied text via mapping.
- Replace selection by:
  - Send Delete to remove the selection
  - Inject Unicode text via SendInput
  - Attempt to reselect the inserted text within a retry budget

This intentionally avoids paste via Ctrl+V to reduce interference with application specific paste behavior.

### Convert last word

Algorithm (domain/text/last_word.rs):
- Uses the input journal tokenization to determine the last word.
- Sleep for delay_ms before conversion and replacement.
- Applies an input based replacement strategy (backspace and Unicode injection via SendInput).
- Clipboard is not used as the primary mechanism.

### Switch keyboard layout

Switches keyboard layout (Windows) for the current thread using the platform API.

### Autoconvert

- The low level keyboard hook maintains a ring buffer of recent tokens.
- When a trigger delimiter is typed, the hook posts a window message WM_APP_AUTOCONVERT.
- The UI thread handles WM_APP_AUTOCONVERT and calls autoconvert_last_word when not paused.
- A guard prevents double conversion of the same token.

### Pause

- Pause toggles state.paused.
- When paused:
  - autoconvert is disabled
  - hotkeys and manual conversion actions still work
- Pause toggling currently shows an informational tray balloon if the tray is available.

## UI

Native Win32 UI with 2 tabs:

### Settings tab
- Start on startup (checkbox)
- Show tray icon (checkbox)
- Delay ms (edit box)

### Hotkeys tab
- Read only displays for:
  - Convert last word (sequence)
  - Convert selection
  - Pause
  - Switch layout

Buttons:
- Apply: persists config and applies runtime changes
- Cancel: reloads config from disk and applies it to UI and runtime
- Exit: closes the application

## Tray icon

- When enabled, a tray icon is added via Shell_NotifyIconW.
- Right click shows a context menu:
  - Show or Hide (toggles window visibility)
  - Exit
- Left click behavior is not implemented at the moment.

## Autostart

- Implemented by creating a shortcut RustSwitcher.lnk in the user Startup folder.
- The shortcut points to the current executable path.
- Moving or deleting the executable breaks autostart.

## Notifications and errors

- Info notifications use tray balloon where possible.
- Error notifications are queued and drained on the UI thread.
- Fallback for tray failure is MessageBoxW.

## Logging

Current behavior:
- A tracing subscriber is installed on startup.
- Logs are written to ./logs/output.log with hourly rotation.
- This currently happens in all build modes.

Planned change (tracked in roadmap):
- Disable file logging in release builds by default, or gate it behind a feature or env var.

## Known issues

These are current code behavior issues, not design goals:
- start_on_startup and are not applied automatically on app startup.
  They are applied after Apply or Cancel.
- Tray notifications may be attempted even when tray icon is disabled, which can lead to failures.
- Notifications can block the UI thread (MessageBox fallback), which can slow hotkey handling.
