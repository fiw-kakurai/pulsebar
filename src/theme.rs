use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};

use crate::util::wide;

/// A straight-alpha color; premultiplication happens at draw time.
#[derive(Clone, Copy, PartialEq)]
pub struct Rgba {
    pub a: u8,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub struct Palette {
    /// Nearly invisible layer covering the whole widget for hit-testing
    /// (alpha-0 pixels would let clicks fall through to the taskbar).
    pub hit: Rgba,
    pub track: Rgba,
    pub cpu: Rgba,
    pub mem: Rgba,
    pub warn_mid: Rgba, // >= 80%
    pub warn_hi: Rgba,  // >= 90%
}

pub fn system_uses_light_theme() -> bool {
    unsafe {
        let sub = wide("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
        let val = wide("SystemUsesLightTheme");
        let mut data: u32 = 0;
        let mut size: u32 = std::mem::size_of::<u32>() as u32;
        let r = RegGetValueW(
            HKEY_CURRENT_USER,
            sub.as_ptr(),
            val.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            &mut data as *mut u32 as *mut _,
            &mut size,
        );
        // Treat a read failure as dark (the Windows 11 default).
        r == 0 && data != 0
    }
}

pub fn palette(light: bool) -> Palette {
    if light {
        Palette {
            hit: Rgba {
                a: 2,
                r: 0,
                g: 0,
                b: 0,
            },
            track: Rgba {
                a: 46,
                r: 0,
                g: 0,
                b: 0,
            },
            cpu: Rgba {
                a: 232,
                r: 0,
                g: 103,
                b: 192,
            },
            mem: Rgba {
                a: 232,
                r: 122,
                g: 68,
                b: 200,
            },
            warn_mid: Rgba {
                a: 240,
                r: 200,
                g: 130,
                b: 0,
            },
            warn_hi: Rgba {
                a: 240,
                r: 216,
                g: 43,
                b: 43,
            },
        }
    } else {
        Palette {
            hit: Rgba {
                a: 2,
                r: 0,
                g: 0,
                b: 0,
            },
            track: Rgba {
                a: 42,
                r: 255,
                g: 255,
                b: 255,
            },
            cpu: Rgba {
                a: 232,
                r: 96,
                g: 205,
                b: 255,
            },
            mem: Rgba {
                a: 232,
                r: 187,
                g: 154,
                b: 255,
            },
            warn_mid: Rgba {
                a: 240,
                r: 255,
                g: 200,
                b: 80,
            },
            warn_hi: Rgba {
                a: 240,
                r: 255,
                g: 99,
                b: 99,
            },
        }
    }
}
