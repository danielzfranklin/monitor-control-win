use std::{mem, ptr};
use thiserror::Error;
use winapi::shared::minwindef::{BOOL, DWORD, LPARAM};
use winapi::shared::windef::{HDC, HMONITOR, RECT};
use winapi::um::wingdi::DISPLAY_DEVICEW;
use winapi::um::winnt::LPCWSTR;
use winapi::um::winuser::{
    EnumDisplayDevicesW, EnumDisplayMonitors, GetMonitorInfoW, MONITORINFO, MONITORINFOEXW,
    MONITORINFOF_PRIMARY,
};

// Note that if the monitor is not the primary display monitor, some of the
// coordinates may be negative values.
#[derive(Debug)]
pub struct Monitor {
    // ffi handle
    pub h: HMONITOR,
    pub name: String,
    // The display monitor rectangle in virtual-screen coordinates.
    pub rect: Rect,
    // The work area rectangle of the display monitor in virtual-screen coordinates.
    pub work_area: Rect,
    // If this is the primary display monitor.
    pub is_primary: bool,
}

impl Monitor {
    pub fn list() -> Result<Vec<Self>, GetMonitorError> {
        extern "system" fn cb(
            // A handle to the display monitor. This value will always be non-NULL.
            h: HMONITOR,
            // This value is NULL if the hdc parameter of EnumDisplayMonitors was NULL.
            _ctx: HDC,
            // If hdcMonitor is NULL, this rectangle is the display monitor rectangle.
            _rect: *mut RECT,
            // Application-defined data that EnumDisplayMonitors passes directly to the enumeration function.
            data: LPARAM,
            // To continue the enumeration, return TRUE.
        ) -> BOOL {
            let list = unsafe { &mut *(data as *mut Vec<Result<Monitor, GetMonitorError>>) };
            list.push(Monitor::get(h));

            BOOL::from(true)
        }

        let mut list: Vec<Result<Monitor, GetMonitorError>> = Vec::new();
        let list_ptr = &mut list as *mut Vec<_> as LPARAM;

        unsafe {
            EnumDisplayMonitors(ptr::null_mut(), ptr::null_mut(), Some(cb), list_ptr);
        }

        list.into_iter().collect()
    }

    /// Requires elevated permissions
    pub fn display_device(&self) -> DisplayDevice {
        DisplayDevice::get(self)
    }

    fn get(h: HMONITOR) -> Result<Self, GetMonitorError> {
        let mut info = MONITORINFOEXW {
            cbSize: Default::default(),
            rcMonitor: Rect::default_sys(),
            rcWork: Rect::default_sys(),
            dwFlags: Default::default(),
            szDevice: Default::default(),
        };
        info.cbSize = mem::size_of::<MONITORINFOEXW>() as u32;

        unsafe {
            GetMonitorInfoW(h, &mut info as *mut MONITORINFOEXW as *mut MONITORINFO);
        }

        let name = wchars_to_string(&info.szDevice);
        if name == "WinDisc" {
            return Err(GetMonitorError::GotPlaceholder);
        }

        let rect = Rect::from(info.rcMonitor);
        let work_area = Rect::from(info.rcWork);
        let is_primary = info.dwFlags & MONITORINFOF_PRIMARY != 0;

        Ok(Self {
            h,
            name,
            rect,
            work_area,
            is_primary,
        })
    }
}

#[derive(Error, Debug)]
pub enum GetMonitorError {
    #[error("Got placeholder monitor (WinDisc). Are you running in a non-interactive session?")]
    GotPlaceholder,
}

#[derive(Debug)]
pub struct DisplayDevice {
    pub name: String,
    pub key: String,
}

impl DisplayDevice {
    /// Requires elevated permissions
    fn get(monitor: &Monitor) -> Self {
        let name = monitor.name.encode_utf16().collect::<Vec<_>>();
        let lp_device = name.as_slice() as *const _ as LPCWSTR;

        let mut display = DISPLAY_DEVICEW {
            cb: 0,
            DeviceName: [0; 32],
            DeviceString: [0; 128],
            StateFlags: 0,
            DeviceID: [0; 128],
            DeviceKey: [0; 128],
        };
        display.cb = mem::size_of::<DISPLAY_DEVICEW>() as u32;

        unsafe {
            EnumDisplayDevicesW(lp_device, 0, &mut display, 0);
        }

        let key = wchars_to_string(&display.DeviceKey);
        let name = wchars_to_string(&display.DeviceName);

        Self { name, key }
    }
}

#[derive(Debug)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    fn from(sys: RECT) -> Self {
        Self {
            left: sys.left,
            top: sys.top,
            right: sys.right,
            bottom: sys.bottom,
        }
    }

    fn default_sys() -> RECT {
        RECT {
            left: Default::default(),
            top: Default::default(),
            right: Default::default(),
            bottom: Default::default(),
        }
    }
}

fn wchars_to_string(wchars: &[u16]) -> String {
    // Take up to null
    let end = wchars.iter().position(|&i| i == 0).unwrap_or(wchars.len());
    let (wchars, _) = wchars.split_at(end);

    String::from_utf16_lossy(wchars)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_list() {
        let list = Monitor::list().unwrap();
        assert!(!list.is_empty());
    }

    #[test]
    fn can_get_display() {
        let monitors = Monitor::list().unwrap();
        let monitor = monitors.first().unwrap();
        let display = monitor.display_device();
        panic!("{:#?}", display);
    }
}
