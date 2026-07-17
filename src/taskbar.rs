use std::ptr::null_mut;

use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::UI::HiDpi::{GetDpiForSystem, GetDpiForWindow};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, FindWindowW, GetWindowRect, SystemParametersInfoW, SPI_GETWORKAREA,
};

use crate::util::{scale, wide};

/// Base widget width at 96 dpi.
pub const BASE_W: i32 = 46;

#[derive(Clone, Copy, PartialEq)]
pub struct Placement {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub dpi: u32,
}

/// Find (Shell_TrayWnd, TrayNotifyWnd). This is the one place that depends
/// on the internal window structure of the Windows 11 taskbar.
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

/// Placement in the parent's (Shell_TrayWnd) client coordinates, just left of
/// the notification area. Shell_TrayWnd is borderless, so its window-rect
/// origin can be treated as the client origin.
pub fn placement_in_tray(tray: HWND, notify: HWND) -> Option<Placement> {
    unsafe {
        let mut tr: RECT = std::mem::zeroed();
        let mut nr: RECT = std::mem::zeroed();
        if GetWindowRect(tray, &mut tr) == 0 || GetWindowRect(notify, &mut nr) == 0 {
            return None;
        }
        let tray_h = tr.bottom - tr.top;
        if tray_h <= 0 {
            return None;
        }
        let dpi = match GetDpiForWindow(tray) {
            0 => GetDpiForSystem(),
            d => d,
        };
        let w = scale(BASE_W, dpi);
        let margin = scale(6, dpi);
        let h = (tray_h - scale(12, dpi)).max(scale(16, dpi)).min(tray_h);
        let x = (nr.left - tr.left) - margin - w;
        let y = (tray_h - h) / 2;
        if x < 0 {
            return None;
        }
        Some(Placement { x, y, w, h, dpi })
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
