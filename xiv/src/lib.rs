pub mod ui;

use failure::Error;
use log;
use std::fmt;

#[cfg(windows)]
use {
    failure::format_err,
    std::ffi::CStr,
    winapi::shared::basetsd::LONG_PTR,
    winapi::shared::minwindef::BOOL,
    winapi::shared::windef::HWND,
    winapi::um::winuser::{EnumWindows, GetWindowTextA},
};

pub const JOB_CNT: usize = 8;
pub const JOBS: [&str; JOB_CNT] = ["CRP", "BSM", "ARM", "GSM", "LTW", "WVR", "ALC", "CUL"];

// The main handle passed back to library methods. The contents are kept
// private to avoid leaking any winapi dependencies to callers.
#[derive(Copy, Clone)]
pub struct XivHandle {
    #[cfg(windows)]
    hwnd: HWND, // The handle passed back by the winapi
    pub use_slow_navigation: bool, // Add more delay to XIV navigation
}

impl fmt::Debug for XivHandle {
    #[cfg(windows)]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Xivhandle {{ {} }}", self.hwnd as LONG_PTR as u64)
    }
    #[cfg(not(windows))]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "XivHandle {{ }}")
    }
}

#[cfg(windows)]
pub fn init() -> Result<XivHandle, Error> {
    let mut arg = std::ptr::null_mut();
    unsafe {
        // TODO: Figure out Rust error handling rather than just panicking inside a lib
        // method.
        match EnumWindows(Some(enum_callback), &mut arg as *mut HWND as LONG_PTR) {
            0 => Ok(XivHandle {
                hwnd: arg as HWND,
                use_slow_navigation: false,
            }),
            _ => Err(format_err!(
                "{}",
                "Unable to find XIV window! Is Final Fantasy XIV running?"
            )),
        }
    }
}

#[cfg(not(windows))]
pub fn init() -> Result<XivHandle, Error> {
    log::info!("Stub XIV lib in use!");
    Ok(XivHandle {
        use_slow_navigation: false,
    })
}

// This callback is called for every window the user32 EnumWindows call finds
// while walking the window list. It's used to find the XIV window by title.
//
// To be more foolproof checking process name might be better.
#[cfg(windows)]
unsafe extern "system" fn enum_callback(win_hwnd: HWND, arg: LONG_PTR) -> BOOL {
    let mut title: Vec<i8> = vec![0; 256];
    let xiv_hwnd = arg as *mut HWND;

    if GetWindowTextA(win_hwnd, title.as_mut_ptr(), title.len() as i32) > 0 {
        let title = CStr::from_ptr(title.as_ptr()).to_string_lossy();
        log::trace!("found {}: {:?}, arg {:?}", title, win_hwnd, xiv_hwnd);
        if title.contains("FINAL FANTASY XIV") {
            log::info!("Found FFXIV.\n");
            *xiv_hwnd = win_hwnd;
            return 0;
        }
    }
    1
}
