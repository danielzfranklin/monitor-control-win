#![warn(clippy::cargo)]

use lazy_static::lazy_static;
use regex::Regex;
use registry::{Hive, RegKey, Security};
use std::{
    fmt::{Debug, Display},
    mem, ptr,
};
use thiserror::Error;
use tracing::info;
use winapi::{
    shared::{
        minwindef::{BOOL, LPARAM},
        windef::{HDC, HMONITOR, HWND, RECT},
    },
    um::{
        errhandlingapi::GetLastError,
        wingdi::DISPLAY_DEVICEW,
        winuser::{
            EnumDisplayDevicesW, EnumDisplayMonitors, GetMonitorInfoW, GetWindowDC,
            EDD_GET_DEVICE_INTERFACE_NAME, MONITORINFO, MONITORINFOEXW,
        },
    },
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Monitor {
    pub driver_id: String,
    pub id: String,
}

impl Monitor {
    const LIST_PATH: &'static str = r"SYSTEM\CurrentControlSet\Enum\DISPLAY";

    /// List all monitor-looking things we can find. Expect this to return
    /// spurious and duplicate results.
    pub fn all() -> Result<Vec<Self>, MonitorError> {
        let drivers = Hive::LocalMachine
            .open(Self::LIST_PATH, Security::Read)
            .map_err(|e| MonitorError::ListDisplayDrivers(e.into()))?;

        let mut all_monitors = vec![];
        for driver_key in drivers.keys() {
            let driver_id = driver_key
                .map_err(|e| MonitorError::ListDisplayDrivers(e.into()))?
                .to_string();
            let mut monitors = Self::for_driver(driver_id)?;
            all_monitors.append(&mut monitors);
        }

        Ok(all_monitors)
    }

    fn for_driver(driver_id: String) -> Result<Vec<Self>, MonitorError> {
        let driver_key =
            Self::driver_key(&driver_id).map_err(|e| MonitorError::ListMonitorsForDriver {
                driver_id: driver_id.clone(),
                source: e.into(),
            })?;

        let mut list = vec![];
        for monitor_key in driver_key.keys() {
            match monitor_key {
                Ok(monitor_key) => {
                    let id = monitor_key.to_string();

                    list.push(Self {
                        driver_id: driver_id.clone(),
                        id,
                    });
                }
                Err(error) => {
                    info!(
                        ?error,
                        "Can't access a sub-key of driver {}, assuming not a monitor", driver_id
                    );
                }
            }
        }

        Ok(list)
    }

    /// List all monitors a window is on.
    ///
    /// If your window is on exactly one monitor, this should return exactly
    /// one result. Unlike [`Self::all`], your should not get spurious or
    /// duplicate results.
    pub fn intersecting(window: HWND) -> Result<Vec<Self>, MonitorError> {
        assert!(!window.is_null());

        let hdc = unsafe { GetWindowDC(window) };
        if hdc.is_null() {
            return Err(MonitorError::ListIntersecting {
                window: window as usize,
                source: WinError::last(),
            });
        }
        extern "system" fn cb(h: HMONITOR, _ctx: HDC, _rect: *mut RECT, list_ptr: LPARAM) -> BOOL {
            let mut info = MONITORINFOEXW {
                cbSize: mem::size_of::<MONITORINFOEXW>() as u32,
                ..Default::default()
            };
            unsafe {
                GetMonitorInfoW(h, &mut info as *mut MONITORINFOEXW as *mut MONITORINFO);
            }

            let mut interface_name = DISPLAY_DEVICEW {
                cb: mem::size_of::<DISPLAY_DEVICEW>() as u32,
                ..Default::default()
            };
            let status = unsafe {
                EnumDisplayDevicesW(
                    &info.szDevice[0],
                    0,
                    &mut interface_name,
                    EDD_GET_DEVICE_INTERFACE_NAME,
                )
            };
            if status == 0 {
                panic!();
            }
            let interface_name = wchars_to_string(&interface_name.DeviceID);

            let list_ptr = list_ptr as *mut Vec<Result<Monitor, MonitorError>>;
            let list = unsafe { &mut *list_ptr };

            let monitor = Monitor::from_interface_name(&interface_name);
            list.push(monitor);

            BOOL::from(true) // continue enumerating
        }

        let mut monitors = Vec::<Result<Monitor, MonitorError>>::new();
        let monitors_ptr = &mut monitors as *mut Vec<_> as LPARAM;
        unsafe {
            EnumDisplayMonitors(ptr::null_mut(), ptr::null_mut(), Some(cb), monitors_ptr);
        }

        monitors.into_iter().collect()
    }

    fn from_interface_name(name: &str) -> Result<Self, MonitorError> {
        lazy_static! {
            static ref RE: Regex = Regex::new(
                r"(?x)^
                \\\\\?\\DISPLAY
                \#(?P<d>[A-Z0-9]+)
                \#(?P<m>[A-Za-z0-9&]+)
                \#\{.*?\}
                $"
            )
            .unwrap();
        }

        let caps = RE
            .captures(name)
            .ok_or_else(|| MonitorError::InvalidInterface(name.to_string()))?;

        let driver_id = caps.name("d").unwrap().as_str().to_string();
        let monitor_id = caps.name("m").unwrap().as_str().to_string();

        Ok(Self {
            driver_id,
            id: monitor_id,
        })
    }

    /// Get the Extended Device Identification Data of a monitor.
    ///
    /// You can feed this to an [EDID parser][edid-parser-crate] to get
    /// information about the display such as the model name or colorspace.
    ///
    /// [edid-parser-crate]: https://crates.io/crates/edid
    pub fn edid(&self) -> Result<Vec<u8>, MonitorError> {
        let data = self
            .params_key()?
            .value(r"EDID")
            .map_err(|err| MonitorError::GetEdid {
                monitor: self.clone(),
                source: err.into(),
            })?;

        let bytes = match data {
            registry::value::Data::Binary(bytes) => bytes,
            _ => unreachable!("EDID will always be in bytes"),
        };

        Ok(bytes)
    }

    fn params_key(&self) -> Result<RegKey, MonitorError> {
        fn helper(monitor: &Monitor) -> Result<RegKey, RegistryError> {
            let driver_key = Monitor::driver_key(&monitor.driver_id)?;
            let monitor_key = driver_key.open(&monitor.id, Security::Read)?;
            let data = monitor_key.open(r"Device Parameters", Security::Read)?;
            Ok(data)
        }

        helper(self).map_err(|source| MonitorError::GetParams {
            monitor: self.clone(),
            source,
        })
    }

    fn driver_key(driver_id: &str) -> Result<registry::RegKey, registry::key::Error> {
        let path = format!(r"{}\{}", Self::LIST_PATH, driver_id);
        Hive::LocalMachine.open(path, Security::Read)
    }
}

#[derive(Debug, Error)]
pub enum MonitorError {
    #[error("Error listing display drivers to get monitors")]
    ListDisplayDrivers(#[source] RegistryError),
    #[error("Error listing monitors for display driver {driver_id}")]
    ListMonitorsForDriver {
        driver_id: String,
        #[source]
        source: RegistryError,
    },
    #[error("Error getting EDID for monitor {monitor:?}")]
    GetEdid {
        monitor: Monitor,
        #[source]
        source: RegistryError,
    },
    #[error("Could not get monitors intersecting window with hwnd {window}. Windows error or invalid hwnd.")]
    ListIntersecting {
        window: usize,
        #[source]
        source: WinError,
    },
    #[error("Error parsing monitor interface name. Expected something like \\\\?DISPLAY#MEI96A2#4&289d...#{{...}}, got: {0}")]
    InvalidInterface(String),
    #[error("Failed to get monitor parameters from the registry")]
    GetParams {
        monitor: Monitor,
        #[source]
        source: RegistryError,
    },
}

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error(transparent)]
    Key(#[from] registry::key::Error),
    #[error(transparent)]
    Value(#[from] registry::value::Error),
    #[error(transparent)]
    KeyIter(#[from] registry::iter::keys::Error),
}

pub(crate) fn wchars_to_string(wchars: &[u16]) -> String {
    // Take up to null
    let end = wchars.iter().position(|&i| i == 0).unwrap_or(wchars.len());
    let (wchars, _) = wchars.split_at(end);

    String::from_utf16_lossy(wchars)
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
mod tests {
    use super::*;

    #[test]
    fn can_list_monitors() {
        let list = Monitor::all().unwrap();
        println!("{:#?}", list)
    }

    #[test]
    fn can_get_edids() {
        let monitors = Monitor::all().unwrap();
        let edids = monitors.iter().flat_map(Monitor::edid).collect::<Vec<_>>();
        assert!(edids.len() > 0);
        println!("{:#?}", edids);
    }

    #[test]
    fn can_list_monitors_for_hwnd() {
        use winit::{
            event_loop::{ControlFlow, EventLoop},
            platform::windows::{EventLoopExtWindows, WindowExtWindows},
            window::WindowBuilder,
        };

        let event_loop = EventLoop::<()>::new_any_thread();
        let window = WindowBuilder::new().build(&event_loop).unwrap();

        let mut already_ran = false;
        event_loop.run(move |_event, _, control_flow| {
            if !already_ran {
                let hwnd = window.hwnd() as HWND;
                let monitors = Monitor::intersecting(hwnd).unwrap();

                assert!(monitors.len() == 1);
                let monitor = &monitors[0];
                eprintln!("{:#?}", monitor);

                let edid = monitor.edid().unwrap();
                eprintln!("edid: {:?}...", &edid[..20]);

                already_ran = true;
            }

            *control_flow = ControlFlow::Exit;
        });
    }
}
