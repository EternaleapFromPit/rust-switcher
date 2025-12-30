# Rust Switcher - Specification (implementation aligned)

This document describes the current behavior and architecture of the repository implementation.
It is intended as onboarding documentation and as a reference for expected runtime behavior.

## Scope

- Supported OS: Windows only
- Primary UI: native Win32 window + tray icon (always on, user can hide via Windows UI)
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
- Autoconvert enabled: runtime flag controlling whether Autoconvert is active.
  Important: this flag is NOT persisted in config.
  Default on app start: disabled.

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
- Hotkeys (legacy single chord, optional):
  - hotkey_convert_last_word
  - hotkey_convert_selection
  - hotkey_switch_layout
  - hotkey_pause
- Hotkey sequences (preferred, optional):
  - hotkey_convert_last_word_sequence
  - hotkey_pause_sequence
  - hotkey_switch_layout_sequence

Notes:
- Autoconvert enabled is runtime only and is not stored in config.
- The UI displays hotkeys as read only values derived from config.
- Hotkey sequences are validated on save.

Default bindings (current defaults in code):
- Convert smart: double tap Left Shift within 1000 ms
- Autoconvert toggle: press Left Shift + Right Shift together
- Switch keyboard layout: CapsLock

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
- The UI thread handles WM_APP_AUTOCONVERT and calls autoconvert_last_word only when Autoconvert enabled is true.
- A guard prevents double conversion of the same token.

### Autoconvert toggle

- The toggle hotkey flips runtime Autoconvert enabled.
- Autoconvert enabled default is disabled on app start.
- Toggling shows an informational tray balloon.

## UI

Native Win32 UI with 2 tabs:

### Settings tab
- Start on startup (checkbox)
- Delay ms (edit box)

### Hotkeys tab
- Read only displays for:
  - Convert last word (sequence)
  - Convert selection
  - Autoconvert toggle
  - Switch layout

Buttons:
- Apply: persists config and applies runtime changes
- Cancel: reloads config from disk and applies it to UI and runtime
- Exit: closes the application
- GitHub: opens repository page (for issues and contribution)

## Tray icon

- A tray icon is always added via Shell_NotifyIconW.
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
- Error notifications are queued and drained on the UI thread via a single entry point.
- Fallback for tray failure is MessageBoxW.
- Notifications must not block hotkey critical paths.

## Logging

Current behavior:
- A tracing subscriber is installed on startup.
- Logs are written to ./logs/output.log with hourly rotation.

Planned change (tracked in roadmap):
- Disable file logging in release builds by default, or gate it behind a feature or env var.

## Known issues

These are current code behavior issues, not design goals:
- start_on_startup and autostart shortcut sync may not be applied automatically on app startup.
  They are applied after Apply or Cancel.
