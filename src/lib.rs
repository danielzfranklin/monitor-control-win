use std::mem;
use std::ptr;
use winapi::um::{wingdi::DISPLAY_DEVICEA, winuser::{EnumDisplayDevicesA, EDD_GET_DEVICE_INTERFACE_NAME}};

pub struct Monitor;

impl Monitor {
    pub fn list() {
        let mut out = DISPLAY_DEVICEA {
            cb: 0,
            DeviceName: [0; 32],
            DeviceString: [0; 128],
            StateFlags: 0,
            DeviceID: [0; 128],
            DeviceKey: [0; 128],
        };
        out.cb = mem::size_of::<DISPLAY_DEVICEA>() as u32;

        let mut n = 0;
        while unsafe { EnumDisplayDevicesA(ptr::null_mut(), n, &mut out, 0) } != 0 {
            n += 1;
        }
        panic!("{:?}", out.DeviceName);
    }

    // pub fn _list() {
    //     extern "system" fn cb(
    //         // A handle to the display monitor. This value will always be non-NULL.
    //         monitor: HMONITOR,
    //         // This value is NULL if the hdc parameter of EnumDisplayMonitors was NULL.
    //         _ctx: HDC,
    //         // If hdcMonitor is NULL, this rectangle is the display monitor rectangle.
    //         _rect: *mut RECT,
    //         // Application-defined data that EnumDisplayMonitors passes directly to the enumeration function.
    //         data: LPARAM,
    //         // To continue the enumeration, return TRUE.
    //     ) -> BOOL {
    //         let list = unsafe { &mut *(data.0 as *mut Vec<Monitor>) };
    //
    //         BOOL::from(true)
    //     }
    //
    //     let list: Vec<Monitor> = Vec::new();
    //     let data = Box::into_raw(Box::new(list));
    //
    //     unsafe {
    //         EnumDisplayMonitors(HDC(0), ptr::null_mut(), Some(cb), LPARAM(data as _));
    //     }
    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_list() {
        Monitor::list();
    }
}
