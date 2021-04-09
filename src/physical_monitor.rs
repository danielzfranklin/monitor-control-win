use crate::*;
use std::ptr;
use thiserror::Error;
use winapi::{
    ctypes::c_void,
    shared::windef::HMONITOR,
    shared::{minwindef::HKEY, windef::HDC__, winerror},
    um::{
        handleapi::INVALID_HANDLE_VALUE,
        physicalmonitorenumerationapi::{
            GetNumberOfPhysicalMonitorsFromHMONITOR, GetPhysicalMonitorsFromHMONITOR,
            PHYSICAL_MONITOR,
        },
        setupapi::*,
        wingdi::*,
        winnt::{KEY_READ, LPCWSTR},
        winreg::*,
    },
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct PhysicalMonitor {
    // ffi handle
    pub h: HMONITOR,
    pub description: String,
}

impl PhysicalMonitor {
    pub(crate) fn list(virt: &Monitor) -> Result<Vec<Self>, PhysicalMonitorError> {
        let mut len = 0;

        let is_success = unsafe { GetNumberOfPhysicalMonitorsFromHMONITOR(virt.h, &mut len) };
        if !unffi_bool(is_success) {
            return Err(PhysicalMonitorError::Listing(WinError::last()));
        }

        let mut list = vec![PHYSICAL_MONITOR::default(); len as usize];

        let is_success = unsafe { GetPhysicalMonitorsFromHMONITOR(virt.h, len, &mut list[0]) };
        if !unffi_bool(is_success) {
            return Err(PhysicalMonitorError::Listing(WinError::last()));
        }

        let list = list.into_iter().map(PhysicalMonitor::from_ffi).collect();
        Ok(list)
    }

    fn from_ffi(sys: PHYSICAL_MONITOR) -> Self {
        // We need an intermediate copy because the struct is packed.
        // See <https://github.com/rust-lang/rust/issues/46043>
        let description = sys.szPhysicalMonitorDescription;
        let description = wchars_to_string(&description);
        let h = sys.hPhysicalMonitor as HMONITOR;
        Self { h, description }
    }
}

#[derive(Debug, Error)]
pub enum PhysicalMonitorError {
    #[error("Error listing physical monitors associated with monitor")]
    Listing(#[source] WinError),
}
