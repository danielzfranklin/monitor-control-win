use super::*;
use bitflags::bitflags;
use derivative::Derivative;
use std::{mem, ptr};
use thiserror::Error;
use winapi::{
    shared::windef::HDC__,
    um::{
        wingdi::{
            CreateDCW, GetColorSpace, GetLogColorSpaceW, LCS_sRGB, CIEXYZ, CIEXYZTRIPLE,
            DISPLAY_DEVICEW, DISPLAY_DEVICE_ACC_DRIVER, DISPLAY_DEVICE_ACTIVE,
            DISPLAY_DEVICE_ATTACHED, DISPLAY_DEVICE_ATTACHED_TO_DESKTOP, DISPLAY_DEVICE_DISCONNECT,
            DISPLAY_DEVICE_MIRRORING_DRIVER, DISPLAY_DEVICE_MODESPRUNED,
            DISPLAY_DEVICE_MULTI_DRIVER, DISPLAY_DEVICE_PRIMARY_DEVICE, DISPLAY_DEVICE_RDPUDD,
            DISPLAY_DEVICE_REMOTE, DISPLAY_DEVICE_REMOVABLE, DISPLAY_DEVICE_TS_COMPATIBLE,
            DISPLAY_DEVICE_UNSAFE_MODES_ON, DISPLAY_DEVICE_VGA_COMPATIBLE, LCSGAMUTMATCH,
            LCS_CALIBRATED_RGB, LCS_GM_ABS_COLORIMETRIC, LCS_GM_BUSINESS, LCS_GM_GRAPHICS,
            LCS_GM_IMAGES, LCS_WINDOWS_COLOR_SPACE, LOGCOLORSPACEW,
        },
        winnt::LPCWSTR,
        winuser::EnumDisplayDevicesW,
    },
};

#[derive(PartialEq, Clone, Derivative)]
#[derivative(Debug)]
pub struct DisplayDevice {
    pub name: String,
    pub string: String,
    pub state: State,
    pub id: String,
    pub key: String,
    #[derivative(Debug = "ignore")]
    ffi_device: [u16; 32],
}

impl DisplayDevice {
    // Inspired by <https://ofekshilon.com/2014/06/19/reading-specific-monitor-dimensions/>

    /// Get the primary device
    ///
    /// ```
    /// # use monitor_control_win::DisplayDevice;
    /// let device = DisplayDevice::primary()?;
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// For a system with a single display card, there is always a primary
    /// device so long as you are running in interactive mode. For a system with
    /// multiple display cards, only one device can be primary.
    pub fn primary() -> Result<Self, DisplayDeviceError> {
        Self::list()
            .into_iter()
            .find(|d| d.state.contains(State::PRIMARY_DEVICE))
            .ok_or(DisplayDeviceError::NoPrimaryDevice)
    }

    /// List all devices
    ///
    /// ```
    /// # use monitor_control_win::DisplayDevice;
    /// let list = DisplayDevice::list();
    /// ```
    pub fn list() -> Vec<Self> {
        let mut display = DISPLAY_DEVICEW {
            cb: mem::size_of::<DISPLAY_DEVICEW>() as u32,
            ..Default::default()
        };

        let mut list = Vec::new();
        let mut n = 0;
        while unsafe { EnumDisplayDevicesW(ptr::null(), n, &mut display, 0) } != 0 {
            let name = wchars_to_string(&display.DeviceName);
            let string = wchars_to_string(&display.DeviceString);
            let state = State::from_bits(display.StateFlags).expect("Valid device state bitflags");
            let id = wchars_to_string(&display.DeviceID);
            let key = wchars_to_string(&display.DeviceKey);

            list.push(Self {
                name,
                string,
                state,
                id,
                key,
                ffi_device: display.DeviceName,
            });

            n += 1;
        }

        list
    }

    /// Get the colorspace of a device
    ///
    /// ```
    /// # use monitor_control_win::DisplayDevice;
    /// let device = DisplayDevice::primary()?;
    /// let colorspace = device.colorspace()?;
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn colorspace(&self) -> Result<ColorSpace, DisplayDeviceError> {
        let hdc = self.hdc()?;
        let ident = ptr::NonNull::new(unsafe { GetColorSpace(hdc.as_ptr()) })
            .ok_or(DisplayDeviceError::GetColorSpace)?;

        // NOTE: Log stands for logical
        let mut space = LOGCOLORSPACEW::default();

        unsafe {
            let status = GetLogColorSpaceW(
                ident.as_ptr(),
                &mut space,
                mem::size_of::<LOGCOLORSPACEW>() as u32,
            );
            if status != 1 {
                return Err(DisplayDeviceError::GetColorSpace);
            }
        }

        Ok(space.into())
    }

    fn hdc(&self) -> Result<ptr::NonNull<HDC__>, DisplayDeviceError> {
        let device = &self.ffi_device as *const _ as LPCWSTR;
        ptr::NonNull::new(unsafe { CreateDCW(device, device, ptr::null(), ptr::null()) })
            .ok_or(DisplayDeviceError::CreateCtx)
    }
}

bitflags! {
    pub struct State: u32 {
        /// DISPLAY_DEVICE_ACTIVE specifies whether a monitor is presented as
        /// being "on" by the respective GDI view.
        const ACTIVE = DISPLAY_DEVICE_ACTIVE;
        const RDPUDD = DISPLAY_DEVICE_RDPUDD;
        const REMOTE = DISPLAY_DEVICE_REMOTE;
        const ATTACHED = DISPLAY_DEVICE_ATTACHED;
        /// The device is removable; it cannot be the primary display.
        const REMOVABLE = DISPLAY_DEVICE_REMOVABLE;
        const ACC_DRIVER = DISPLAY_DEVICE_ACC_DRIVER;
        const DISCONNECT = DISPLAY_DEVICE_DISCONNECT;
        /// The device has more display modes than its output devices support.
        const MODES_PRUNED = DISPLAY_DEVICE_MODESPRUNED;
        const MULTI_DRIVER = DISPLAY_DEVICE_MULTI_DRIVER;
        const TS_COMPATIBLE = DISPLAY_DEVICE_TS_COMPATIBLE;
        /// The primary desktop is on the device. For a system with a single
        /// display card, this is always set. For a system with multiple display
        /// cards, only one device can have this set.
        const PRIMARY_DEVICE = DISPLAY_DEVICE_PRIMARY_DEVICE;
        /// The device is VGA compatible.
        const VGA_COMPATIBLE = DISPLAY_DEVICE_VGA_COMPATIBLE;
        const UNSAFE_MODES_ON = DISPLAY_DEVICE_UNSAFE_MODES_ON;
        /// Represents a pseudo device used to mirror application drawing for
        /// remoting or other purposes. An invisible pseudo monitor is
        /// associated with this device.
        const MIRRORING_DRIVER = DISPLAY_DEVICE_MIRRORING_DRIVER;
        const ATTACHED_TO_DESKTOP = DISPLAY_DEVICE_ATTACHED_TO_DESKTOP;
    }
}

// See docs <https://docs.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-logcolorspacea>

#[derive(Debug, PartialEq, Clone)]
pub struct ColorSpace {
    space_type: ColorSpaceType,
    intent: ColorSpaceIntent,
    gamma: ColorSpaceGamma,
    filename: String,
}

impl From<LOGCOLORSPACEW> for ColorSpace {
    #[allow(non_upper_case_globals)] // needed for LCS_sRGB
    fn from(ffi: LOGCOLORSPACEW) -> Self {
        let space_type = match ffi.lcsCSType {
            LCS_CALIBRATED_RGB => {
                let endpoints = ffi.lcsEndpoints.into();
                ColorSpaceType::CalibratedRgb(endpoints)
            }
            LCS_sRGB => ColorSpaceType::Srgb,
            LCS_WINDOWS_COLOR_SPACE => ColorSpaceType::Windows,
            _ => unreachable!("Unexpected lcsCSType"),
        };

        let intent = ffi.lcsIntent.into();

        let gamma = ColorSpaceGamma {
            red: fxp_8dot8_to_f32(ffi.lcsGammaRed),
            green: fxp_8dot8_to_f32(ffi.lcsGammaGreen),
            blue: fxp_8dot8_to_f32(ffi.lcsGammaBlue),
        };

        let filename = wchars_to_string(&ffi.lcsFilename);

        Self {
            space_type,
            intent,
            gamma,
            filename,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct ColorSpaceGamma {
    red: f32,
    green: f32,
    blue: f32,
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum ColorSpaceIntent {
    /// Absolute Colorimetric: Maintain the white point. Match the colors to
    /// their nearest color in the destination gamut.
    Match,
    /// Saturation: Maintain saturation. Used for business charts and other
    /// situations in which undithered colors are required.
    Graphic,
    /// Relative Colorimetric: Maintain colorimetric match. Used for graphic
    /// designs and named colors.
    Proof,
    /// Perceptual: Maintain contrast. Used for photographs and natural images.
    Picture,
}

impl From<LCSGAMUTMATCH> for ColorSpaceIntent {
    fn from(ffi: LCSGAMUTMATCH) -> Self {
        match ffi {
            LCS_GM_ABS_COLORIMETRIC => Self::Match,
            LCS_GM_BUSINESS => Self::Graphic,
            LCS_GM_GRAPHICS => Self::Proof,
            LCS_GM_IMAGES => Self::Picture,
            _ => unreachable!("Unexpected lcsIntent"),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ColorSpaceType {
    /// Color values are calibrated RGB values. The values are translated using
    /// the endpoints specified by the lcsEndpoints member before being passed
    /// to the device.
    CalibratedRgb(ColorSpaceEndpoints),
    /// Color values are values are sRGB values.
    Srgb,
    /// Color values are Windows default color space color values.
    Windows,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ColorSpaceEndpoints {
    red: CieXyz,
    green: CieXyz,
    blue: CieXyz,
}

impl From<CIEXYZTRIPLE> for ColorSpaceEndpoints {
    fn from(ffi: CIEXYZTRIPLE) -> Self {
        let red = ffi.ciexyzRed.into();
        let green = ffi.ciexyzGreen.into();
        let blue = ffi.ciexyzBlue.into();

        Self { red, green, blue }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct CieXyz {
    x: f32,
    y: f32,
    z: f32,
}

impl From<CIEXYZ> for CieXyz {
    fn from(ffi: CIEXYZ) -> Self {
        let x = fxp230_to_f32(ffi.ciexyzX);
        let y = fxp230_to_f32(ffi.ciexyzY);
        let z = fxp230_to_f32(ffi.ciexyzZ);
        Self { x, y, z }
    }
}

fn fxp230_to_f32(fxp: i32) -> f32 {
    const ONE_BILLION: u64 = 1_000_000_000;
    const TWO_THIRTY: u64 = 1 << 30;

    let n = (fxp as u64 * ONE_BILLION) / TWO_THIRTY;
    n as f32 / ONE_BILLION as f32
}

fn fxp_8dot8_to_f32(fxp: u32) -> f32 {
    const EIGHT_ZEROS: f32 = 1_000_000_000_f32;
    (fxp as f32) / EIGHT_ZEROS
}

#[derive(Error, Debug, Eq, PartialEq, Clone)]
pub enum DisplayDeviceError {
    #[error("Failed to create device context")]
    CreateCtx,
    #[error("Failed to get color space")]
    GetColorSpace,
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    #[test]
    fn can_get_hdc() {
        let devs = DisplayDevice::list();
        let dev = devs.first().unwrap();
        dev.hdc().unwrap();
    }

    #[test]
    fn fxp_math() {
        // Test cases from <https://stackoverflow.com/a/57454169>
        let cases = [
            (0, 0f32),
            (1, 0f32),
            (2, 0.000_000_001),
            (5, 0.000_000_004),
            (20, 0.000_000_018),
            (740329, 0.000_689_485),
            (1073741823, 0.999_999_999),
        ];

        for (input, output) in cases.iter() {
            eprintln!("input = {}", input);
            assert_relative_eq!(fxp230_to_f32(*input), output, epsilon = f32::EPSILON);
        }
    }
}
