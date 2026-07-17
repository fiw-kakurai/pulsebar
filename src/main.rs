#![windows_subsystem = "windows"]

mod menu;
mod metrics;
mod render;
mod taskbar;
mod theme;
mod tooltip;
mod util;
mod window;

use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::Controls::{
    InitCommonControlsEx, ICC_WIN95_CLASSES, INITCOMMONCONTROLSEX,
};
use windows_sys::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, RegisterWindowMessageW, SetTimer, TranslateMessage, MSG,
};

use crate::util::wide;
use crate::window::{App, TIMER_ID};

fn main() {
    unsafe {
        // Single-instance guard (the mutex is released automatically on exit).
        let name = wide("Local\\pulsebar-singleton");
        let _mutex = CreateMutexW(null(), 0, name.as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return;
        }

        // Also declared in the manifest; this is a safety net for builds
        // produced without the resource compiler.
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

        let icc = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_WIN95_CLASSES,
        };
        InitCommonControlsEx(&icc);

        let hinst = GetModuleHandleW(null());
        let taskbar_created = RegisterWindowMessageW(wide("TaskbarCreated").as_ptr());

        window::register_classes(hinst);
        let manager = window::create_manager(hinst);
        if manager.is_null() {
            return;
        }
        window::init_app(App::new(hinst, manager, taskbar_created));

        let app = window::app();
        app.metrics.sample();
        app.ensure_widget();

        SetTimer(manager, TIMER_ID, 1000, None);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
