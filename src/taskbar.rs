use std::ptr::null_mut;

use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::UI::HiDpi::{GetDpiForSystem, GetDpiForWindow};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, FindWindowW, GetWindowRect, SystemParametersInfoW, SPI_GETWORKAREA,
};

use crate::util::{scale, wide};

/// Base widget width at 96 dpi.
pub const BASE_W: i32 = 46;

/// Width reserved for the secondary-taskbar clock at 96 dpi when the clock
/// has no window of its own to measure (Win11 draws it in XAML).
const SECONDARY_CLOCK_W: i32 = 100;

#[derive(Clone, Copy, PartialEq)]
pub struct Placement {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub dpi: u32,
}

/// Find (Shell_TrayWnd, TrayNotifyWnd). This and the secondary-taskbar
/// lookups below are the only places that depend on the internal window
/// structure of the Windows 11 taskbar.
pub fn find_tray() -> Option<(HWND, HWND)> {
    unsafe {
        let tray = FindWindowW(wide("Shell_TrayWnd").as_ptr(), std::ptr::null());
        if tray.is_null() {
            return None;
        }
        let notify = FindWindowExW(
            tray,
            null_mut(),
            wide("TrayNotifyWnd").as_ptr(),
            std::ptr::null(),
        );
        if notify.is_null() {
            return None;
        }
        Some((tray, notify))
    }
}

/// All secondary taskbars (one per extra monitor; empty on single-display
/// setups or while Explorer is restarting).
pub fn find_secondary_trays() -> Vec<HWND> {
    let cls = wide("Shell_SecondaryTrayWnd");
    let mut trays = Vec::new();
    unsafe {
        let mut h: HWND = null_mut();
        loop {
            h = FindWindowExW(null_mut(), h, cls.as_ptr(), std::ptr::null());
            if h.is_null() {
                break;
            }
            trays.push(h);
        }
    }
    trays
}

fn dpi_for(hwnd: HWND) -> u32 {
    unsafe {
        match GetDpiForWindow(hwnd) {
            0 => GetDpiForSystem(),
            d => d,
        }
    }
}

/// Shared geometry: a BASE_W-wide widget vertically centered in the bar,
/// placed just left of `anchor_x` (a screen coordinate), expressed in the
/// tray's client coordinates. The taskbar windows are borderless, so their
/// window-rect origin can be treated as the client origin.
fn placement_left_of(tr: &RECT, anchor_x: i32, dpi: u32) -> Option<Placement> {
    let tray_h = tr.bottom - tr.top;
    if tray_h <= 0 {
        return None;
    }
    let w = scale(BASE_W, dpi);
    let margin = scale(6, dpi);
    let h = (tray_h - scale(12, dpi)).max(scale(16, dpi)).min(tray_h);
    let x = (anchor_x - tr.left) - margin - w;
    let y = (tray_h - h) / 2;
    if x < 0 {
        return None;
    }
    Some(Placement { x, y, w, h, dpi })
}

/// Placement in the parent's (Shell_TrayWnd) client coordinates, just left of
/// the notification area.
pub fn placement_in_tray(tray: HWND, notify: HWND) -> Option<Placement> {
    unsafe {
        let mut tr: RECT = std::mem::zeroed();
        let mut nr: RECT = std::mem::zeroed();
        if GetWindowRect(tray, &mut tr) == 0 || GetWindowRect(notify, &mut nr) == 0 {
            return None;
        }
        placement_left_of(&tr, nr.left, dpi_for(tray))
    }
}

/// Placement in a secondary taskbar's (Shell_SecondaryTrayWnd) client
/// coordinates. Secondary taskbars have no TrayNotifyWnd: anchor to the
/// clock window when one exists (classic-style taskbars), otherwise reserve
/// a fixed clock width, since the Win11 clock is XAML-drawn and exposes no
/// HWND to measure.
pub fn placement_in_secondary(tray: HWND) -> Option<Placement> {
    unsafe {
        let mut tr: RECT = std::mem::zeroed();
        if GetWindowRect(tray, &mut tr) == 0 {
            return None;
        }
        let dpi = dpi_for(tray);
        let mut anchor = tr.right - scale(SECONDARY_CLOCK_W, dpi);
        let clock = FindWindowExW(
            tray,
            null_mut(),
            wide("ClockButton").as_ptr(),
            std::ptr::null(),
        );
        if !clock.is_null() {
            let mut cr: RECT = std::mem::zeroed();
            if GetWindowRect(clock, &mut cr) != 0 && cr.left > tr.left {
                anchor = cr.left;
            }
        }
        placement_left_of(&tr, anchor, dpi)
    }
}

/// Compatibility mode: bottom-right corner of the work area (screen coords).
pub fn placement_fallback() -> Placement {
    unsafe {
        let dpi = GetDpiForSystem();
        let mut wa = RECT {
            left: 0,
            top: 0,
            right: 1280,
            bottom: 720,
        };
        SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut wa as *mut RECT as *mut _, 0);
        let w = scale(BASE_W, dpi);
        let h = scale(28, dpi);
        let m = scale(8, dpi);
        Placement {
            x: wa.right - w - m,
            y: wa.bottom - h - m,
            w,
            h,
            dpi,
        }
    }
}
