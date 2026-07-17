use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{HWND, POINT, SIZE};
use windows_sys::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS,
    HBITMAP, HDC, HGDIOBJ,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{UpdateLayeredWindow, ULW_ALPHA};

use crate::theme::{Palette, Rgba};
use crate::util::scale;

/// UpdateLayeredWindow requires premultiplied alpha.
fn premul(c: Rgba) -> u32 {
    let a = c.a as u32;
    let r = c.r as u32 * a / 255;
    let g = c.g as u32 * a / 255;
    let b = c.b as u32 * a / 255;
    (a << 24) | (r << 16) | (g << 8) | b
}

/// Writes pixels directly into a 32bpp top-down DIB and transfers it with
/// UpdateLayeredWindow. Only two bars are drawn, so no GDI drawing calls are
/// used at all (GDI would destroy the alpha channel anyway).
pub struct Renderer {
    hdc: HDC,
    bmp: HBITMAP,
    old: HGDIOBJ,
    bits: *mut u32,
    pub w: i32,
    pub h: i32,
}

impl Renderer {
    pub fn new(w: i32, h: i32) -> Option<Renderer> {
        if w <= 0 || h <= 0 {
            return None;
        }
        unsafe {
            let screen = GetDC(null_mut());
            let hdc = CreateCompatibleDC(screen);
            let mut bmi: BITMAPINFO = std::mem::zeroed();
            bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = w;
            bmi.bmiHeader.biHeight = -h; // top-down
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            bmi.bmiHeader.biCompression = BI_RGB;
            let mut bits: *mut core::ffi::c_void = null_mut();
            let bmp = CreateDIBSection(screen, &bmi, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
            ReleaseDC(null_mut(), screen);
            if hdc.is_null() || bmp.is_null() || bits.is_null() {
                if !bmp.is_null() {
                    DeleteObject(bmp as HGDIOBJ);
                }
                if !hdc.is_null() {
                    DeleteDC(hdc);
                }
                return None;
            }
            let old = SelectObject(hdc, bmp as HGDIOBJ);
            Some(Renderer {
                hdc,
                bmp,
                old,
                bits: bits as *mut u32,
                w,
                h,
            })
        }
    }

    fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, c: u32) {
        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + w).min(self.w);
        let y1 = (y + h).min(self.h);
        for yy in y0..y1 {
            let row = (yy * self.w) as usize;
            for xx in x0..x1 {
                unsafe {
                    *self.bits.add(row + xx as usize) = c;
                }
            }
        }
    }

    fn bar(&mut self, rect: (i32, i32, i32, i32), pct: f32, track: u32, fill: u32) {
        let (x, y, w, h) = rect;
        self.fill(x, y, w, h, track);
        let fw = ((w as f32 * (pct / 100.0).clamp(0.0, 1.0)).round() as i32).min(w);
        if fw > 0 {
            self.fill(x, y, fw, h, fill);
        }
    }

    pub fn draw(&mut self, cpu_pct: f32, mem_pct: f32, pal: &Palette, dpi: u32) {
        // Fill the whole surface with a nearly invisible color (alpha=2):
        // alpha-0 pixels would let clicks fall through to the taskbar, and we
        // want the entire widget to be hit-testable.
        let hit = premul(pal.hit);
        for i in 0..(self.w * self.h) as usize {
            unsafe {
                *self.bits.add(i) = hit;
            }
        }
        let pad = scale(2, dpi);
        let bar_h = scale(6, dpi).max(3);
        let gap = scale(4, dpi).max(2);
        let total = bar_h * 2 + gap;
        let y0 = ((self.h - total) / 2).max(0);
        let track_w = (self.w - pad * 2).max(1);
        let track = premul(pal.track);
        let color_for = |pct: f32, base: Rgba| {
            if pct >= 90.0 {
                pal.warn_hi
            } else if pct >= 80.0 {
                pal.warn_mid
            } else {
                base
            }
        };
        let cpu_fill = premul(color_for(cpu_pct, pal.cpu));
        let mem_fill = premul(color_for(mem_pct, pal.mem));
        self.bar((pad, y0, track_w, bar_h), cpu_pct, track, cpu_fill);
        self.bar(
            (pad, y0 + bar_h + gap, track_w, bar_h),
            mem_pct,
            track,
            mem_fill,
        );
    }

    pub fn present(&self, widget: HWND) -> bool {
        unsafe {
            let size = SIZE {
                cx: self.w,
                cy: self.h,
            };
            let src = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            // Position is always managed via SetWindowPos; this call only
            // updates the pixels.
            UpdateLayeredWindow(
                widget,
                null_mut(),
                null(),
                &size,
                self.hdc,
                &src,
                0,
                &blend,
                ULW_ALPHA,
            ) != 0
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.hdc, self.old);
            DeleteObject(self.bmp as HGDIOBJ);
            DeleteDC(self.hdc);
        }
    }
}
