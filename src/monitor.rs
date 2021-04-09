use crate::*;
use physical_monitor::PhysicalMonitorError;
use std::{mem, ptr};
use thiserror::Error;
use winapi::{
    shared::{
        minwindef::{BOOL, LPARAM},
        windef::{HDC, HMONITOR, RECT},
    },
    um::winuser::{
        EnumDisplayMonitors, GetMonitorInfoW, MONITORINFO, MONITORINFOEXW, MONITORINFOF_PRIMARY,
    },
};

// Note that if the monitor is not the primary display monitor, some of the
// coordinates may be negative values.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// Get the primary monitor.
    ///
    /// The implimentation involves looking through every monitor returned by
    /// [`Self::list`].
    ///
    /// ```
    /// # use monitor_control_win::Monitor;
    /// let monitor = Monitor::primary()?;
    /// println!("{:#?}", monitor);
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn primary() -> Result<Self, MonitorError> {
        Self::list()?
            .into_iter()
            .find(|m| m.is_primary)
            .ok_or(MonitorError::NoPrimary)
    }

    /// List all monitors.
    ///
    /// ```
    /// # use monitor_control_win::Monitor;
    /// let monitors = Monitor::list()?;
    /// println!("{:#?}", monitors);
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn list() -> Result<Vec<Self>, MonitorError> {
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
            let list = unsafe { &mut *(data as *mut Vec<Result<Monitor, MonitorError>>) };
            list.push(Monitor::get(h));

            BOOL::from(true)
        }

        let mut list: Vec<Result<Monitor, MonitorError>> = Vec::new();
        let list_ptr = &mut list as *mut Vec<_> as LPARAM;

        unsafe {
            EnumDisplayMonitors(ptr::null_mut(), ptr::null_mut(), Some(cb), list_ptr);
        }

        list.into_iter().collect()
    }

    /// Get associated physical monitors.
    ///
    /// ```
    /// # use monitor_control_win::Monitor;
    /// let monitor = Monitor::primary()?;
    /// let physical_monitors = monitor.physical_monitors()?;
    /// println!("{:#?}", physical_monitors);
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn physical_monitors(&self) -> Result<Vec<PhysicalMonitor>, PhysicalMonitorError> {
        PhysicalMonitor::list(self)
    }

    fn get(h: HMONITOR) -> Result<Self, MonitorError> {
        let mut info = MONITORINFOEXW {
            cbSize: mem::size_of::<MONITORINFOEXW>() as u32,
            ..Default::default()
        };

        unsafe {
            GetMonitorInfoW(h, &mut info as *mut MONITORINFOEXW as *mut MONITORINFO);
        }

        let name = wchars_to_string(&info.szDevice);
        if name == "WinDisc" {
            return Err(MonitorError::GotPlaceholder);
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

#[derive(Error, Debug, Eq, PartialEq, Clone)]
pub enum MonitorError {
    #[error("Got placeholder monitor (WinDisc). Are you running in a non-interactive session?")]
    GotPlaceholder,
    #[error("No primary monitor")]
    NoPrimary,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_list() {
        let list = Monitor::list().unwrap();
        assert!(!list.is_empty());
    }
}
