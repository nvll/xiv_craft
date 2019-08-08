#[cfg(windows)]
use {winapi::shared::minwindef::UINT, winapi::um::winuser::PostMessageA};

use log;
use std::thread::sleep;
use std::time::Duration;

// This module handles all interactions with the game UI.

// Delay for WM_CHAR events. In testing, even with low fps or
// higher latency this value is still safe because of the game's
// input buffer.
const CHAR_DELAY: f32 = 0.05;
// Delay for window navigation sent via KEYDOWN / KEYUP events.
// These are affect by latency and in testing 150 milliseconds
// seems safe in laggier conditions.
const UI_DELAY: f32 = 0.05;
const UI_DELAY_SLOW: f32 = 0.15;

#[cfg(windows)]
mod constants {
    use winapi::um::winuser::*;
    pub const KEY_UP: i32 = VK_NUMPAD8;
    pub const KEY_DOWN: i32 = VK_NUMPAD2;
    pub const KEY_LEFT: i32 = VK_NUMPAD4;
    pub const KEY_RIGHT: i32 = VK_NUMPAD6;
    pub const KEY_CONFIRM: i32 = VK_NUMPAD0;
    pub const KEY_FORWARD: i32 = VK_NUMPAD9;
    pub const KEY_BACKWARD: i32 = VK_NUMPAD7;
    pub const KEY_CANCEL: i32 = VK_DECIMAL;
    pub const KEY_ENTER: i32 = VK_RETURN;
    pub const KEY_ESCAPE: i32 = VK_ESCAPE;
    pub const KEY_BACKSPACE: i32 = VK_BACK;
    pub const MSG_KEY_UP: u32 = WM_KEYUP;
    pub const MSG_KEY_DOWN: u32 = WM_KEYDOWN;
    pub const MSG_KEY_CHAR: u32 = WM_CHAR;
}

#[cfg(not(windows))]
mod constants {
    pub const KEY_UP: i32 = 0;
    pub const KEY_DOWN: i32 = 0;
    pub const KEY_LEFT: i32 = 0;
    pub const KEY_RIGHT: i32 = 0;
    pub const KEY_CONFIRM: i32 = 0;
    pub const KEY_FORWARD: i32 = 0;
    pub const KEY_BACKWARD: i32 = 0;
    pub const KEY_CANCEL: i32 = 0;
    pub const KEY_ENTER: i32 = 0;
    pub const KEY_BACKSPACE: i32 = 0;
    pub const KEY_ESCAPE: i32 = 0;
    pub const MSG_KEY_UP: u32 = 0;
    pub const MSG_KEY_DOWN: u32 = 0;
    pub const MSG_KEY_CHAR: u32 = 0;
}

// Wait |s| seconds, fractions permitted.
pub fn wait(s: f32) {
    let ms = (s * 1000_f32) as u64;
    sleep(Duration::from_millis(ms));
}

pub fn cursor_down(xiv_handle: super::XivHandle) {
    log::trace!("[down]");
    send_key(xiv_handle, constants::KEY_DOWN);
}

pub fn cursor_up(xiv_handle: super::XivHandle) {
    log::trace!("[up]");
    send_key(xiv_handle, constants::KEY_UP);
}

pub fn cursor_left(xiv_handle: super::XivHandle) {
    log::trace!("[left]");
    send_key(xiv_handle, constants::KEY_LEFT);
}

pub fn cursor_right(xiv_handle: super::XivHandle) {
    log::trace!("[right]");
    send_key(xiv_handle, constants::KEY_RIGHT);
}

pub fn cursor_backward(xiv_handle: super::XivHandle) {
    log::trace!("[ui back]");
    send_key(xiv_handle, constants::KEY_BACKWARD)
}

pub fn cursor_forward(xiv_handle: super::XivHandle) {
    log::trace!("[ui forward]");
    send_key(xiv_handle, constants::KEY_FORWARD);
}

pub fn press_backspace(xiv_handle: super::XivHandle) {
    log::trace!("[backspace]");
    send_key(xiv_handle, constants::KEY_BACKSPACE);
}

pub fn press_confirm(xiv_handle: super::XivHandle) {
    log::trace!("[confirm]");
    send_key(xiv_handle, constants::KEY_CONFIRM);
}

pub fn press_cancel(xiv_handle: super::XivHandle) {
    log::trace!("[cancel]");
    send_key(xiv_handle, constants::KEY_CANCEL);
}

pub fn press_enter(xiv_handle: super::XivHandle) {
    log::trace!("[enter]");
    send_key(xiv_handle, constants::KEY_ENTER);
}

pub fn press_escape(xiv_handle: super::XivHandle) {
    log::trace!("[esc]");
    send_key(xiv_handle, constants::KEY_ESCAPE);
}

pub fn target_nearest_npc(xiv_handle: super::XivHandle) {
    press_enter(xiv_handle);
    send_string(xiv_handle, "/tnpc");
    press_enter(xiv_handle);
}

pub fn send_string(xiv_handle: super::XivHandle, s: &str) {
    log::trace!("sending string: '{}'\n", s);
    for c in s.chars() {
        send_char(xiv_handle, c);
    }
}

pub fn send_action(xiv_handle: super::XivHandle, s: &str, _delay: Option<i64>) {
    send_string(xiv_handle, s);
    wait(0.5);
    press_enter(xiv_handle);
}

// Clear all dialog windows and the text input so we can get
// the game into a state we can trust. If someone kills a craft or
// Talan midway then the UI can be in an inconsistent state, this
// attempts to deal with that. This has been tested in environments
// as low as 11 fps.
pub fn clear_window(xiv_handle: super::XivHandle) {
    log::debug!("clearing the game window");
    // If the text input has focus, try clearing the text to prevent
    // saying junk in a linkshell, /say, etc.
    for _ in 0..32 {
        press_backspace(xiv_handle);
    }
    press_enter(xiv_handle);

    // If we didn't have focus before, we do now and we clear the
    // test this time.
    for _ in 0..32 {
        press_backspace(xiv_handle);
    }
    press_enter(xiv_handle);

    for _ in 0..4 {
        press_escape(xiv_handle);
    }
    press_cancel(xiv_handle);

    // Each press of escape clears out one window, or removes the input focus
    for _ in 0..10 {
        press_cancel(xiv_handle);
    }

    // Cancelling twice will close the System menu if it is open, as well as any
    // remaining text input focus.
    press_cancel(xiv_handle);
    press_cancel(xiv_handle);
}

pub fn send_char(xiv_handle: super::XivHandle, c: char) {
    log::trace!("char: {}", c);
    send_msg(xiv_handle, constants::MSG_KEY_CHAR, c as i32);
    // TODO: Redo this when we have a better timing system
    wait(CHAR_DELAY);
}

pub fn send_key(xiv_handle: super::XivHandle, c: i32) {
    log::trace!("key {:x}", c);
    send_msg(xiv_handle, constants::MSG_KEY_DOWN, c);
    send_msg(xiv_handle, constants::MSG_KEY_UP, c);
    if xiv_handle.use_slow_navigation {
        wait(UI_DELAY_SLOW);
    } else {
        wait(UI_DELAY);
    }
}

// Send a character/key to the XIV window
fn send_msg(_xiv_handle: super::XivHandle, _msg: u32, _key: i32) {
    #[cfg(windows)]
    unsafe {
        PostMessageA(_xiv_handle.hwnd, _msg as UINT, _key as usize, 0);
    }
}
