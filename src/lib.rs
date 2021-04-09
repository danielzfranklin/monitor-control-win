#![warn(clippy::cargo)]
use const_fn_assert::cfn_assert;
use std::fmt::{Debug, Display};
use thiserror::Error;
use winapi::{
    shared::{
        guiddef::GUID,
        minwindef::{BOOL, FALSE, TRUE},
        windef::RECT,
    },
    um::errhandlingapi::GetLastError,
};

pub mod display;
pub mod monitor;
pub mod physical_monitor;

pub use display::DisplayDevice;
pub use monitor::Monitor;
pub use physical_monitor::PhysicalMonitor;

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub(crate) fn from(sys: RECT) -> Self {
        Self {
            left: sys.left,
            top: sys.top,
            right: sys.right,
            bottom: sys.bottom,
        }
    }
}

pub(crate) fn unffi_bool(val: BOOL) -> bool {
    match val {
        TRUE => true,
        FALSE => false,
        n => panic!("Invalid BOOL of value {}", n),
    }
}

pub(crate) fn wchars_to_string(wchars: &[u16]) -> String {
    // Take up to null
    let end = wchars.iter().position(|&i| i == 0).unwrap_or(wchars.len());
    let (wchars, _) = wchars.split_at(end);

    String::from_utf16_lossy(wchars)
}

#[allow(clippy::clippy::many_single_char_names)]
pub(crate) const fn guid(a: u32, b: u16, c: u16, d: u16, e: u64) -> GUID {
    let [last_1, last_2] = d.to_le_bytes();
    let [last_3, last_4, last_5, last_6, last_7, last_8, unused_1, unused_2] = e.to_le_bytes();
    let last = [
        last_1, last_2, last_3, last_4, last_5, last_6, last_7, last_8,
    ];
    cfn_assert!(unused_1 == 0 && unused_2 == 0);
    GUID {
        Data1: a,
        Data2: b,
        Data3: c,
        Data4: last,
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Error)]
pub struct WinError(u32);

impl WinError {
    pub fn code(&self) -> u32 {
        self.0
    }

    pub fn last() -> Self {
        let code = unsafe { GetLastError() };
        Self(code)
    }
}

impl Display for WinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = self.0;
        f.debug_tuple("WinError")
            .field(&format!("0x{:X}", code))
            .finish()
    }
}

impl Debug for WinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl From<u32> for WinError {
    fn from(code: u32) -> Self {
        Self(code)
    }
}

impl From<i32> for WinError {
    fn from(code: i32) -> Self {
        assert!(code > 0);
        Self(code as u32)
    }
}

#[cfg(test)]
mod tests {}
