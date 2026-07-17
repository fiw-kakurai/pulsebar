use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{HINSTANCE, HWND, LPARAM};
use windows_sys::Win32::UI::Controls::{
    TTF_IDISHWND, TTF_SUBCLASS, TTM_ADDTOOLW, TTM_SETMAXTIPWIDTH, TTM_UPDATETIPTEXTW,
    TTS_ALWAYSTIP, TTS_NOPREFIX, TTTOOLINFOW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, SendMessageW, CW_USEDEFAULT, WS_EX_TOPMOST, WS_POPUP,
};

use crate::util::wide;

fn tool_info(widget: HWND, text: *mut u16) -> TTTOOLINFOW {
    let mut ti: TTTOOLINFOW = unsafe { std::mem::zeroed() };
    ti.cbSize = std::mem::size_of::<TTTOOLINFOW>() as u32;
    // TTF_SUBCLASS: let the tooltip control track the mouse itself
    // (no TrackMouseEvent bookkeeping on our side).
    ti.uFlags = TTF_IDISHWND | TTF_SUBCLASS;
    ti.hwnd = widget;
    ti.uId = widget as usize;
    ti.lpszText = text;
    ti
}

pub unsafe fn create(widget: HWND, hinst: HINSTANCE) -> HWND {
    let tt = CreateWindowExW(
        WS_EX_TOPMOST,
        wide("tooltips_class32").as_ptr(),
        null(),
        WS_POPUP | TTS_ALWAYSTIP | TTS_NOPREFIX,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        widget,
        null_mut(),
        hinst,
        null(),
    );
    if tt.is_null() {
        return null_mut();
    }
    let mut text = wide("CPU / Memory");
    let ti = tool_info(widget, text.as_mut_ptr());
    SendMessageW(tt, TTM_ADDTOOLW, 0, &ti as *const TTTOOLINFOW as LPARAM);
    // A maximum width is required for multi-line tooltip text.
    SendMessageW(tt, TTM_SETMAXTIPWIDTH, 0, 600);
    tt
}

pub unsafe fn update(tt: HWND, widget: HWND, text: &str) {
    let mut buf = wide(text);
    let ti = tool_info(widget, buf.as_mut_ptr());
    // The tooltip control copies the string, so a temporary buffer is fine.
    SendMessageW(
        tt,
        TTM_UPDATETIPTEXTW,
        0,
        &ti as *const TTTOOLINFOW as LPARAM,
    );
}
