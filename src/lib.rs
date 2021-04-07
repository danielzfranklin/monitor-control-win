#![warn(clippy::cargo)]
use winapi::shared::windef::RECT;

pub mod display;
pub mod monitor;

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

pub(crate) fn wchars_to_string(wchars: &[u16]) -> String {
    // Take up to null
    let end = wchars.iter().position(|&i| i == 0).unwrap_or(wchars.len());
    let (wchars, _) = wchars.split_at(end);

    String::from_utf16_lossy(wchars)
}

#[cfg(test)]
mod tests {}
