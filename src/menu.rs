use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{HWND, POINT};
use windows_sys::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegGetValueW, RegOpenKeyExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ, RRF_RT_REG_SZ,
};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, DestroyWindow, GetCursorPos, PostMessageW,
    SetForegroundWindow, TrackPopupMenuEx, MF_CHECKED, MF_SEPARATOR, MF_STRING, SW_SHOWNORMAL,
    TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_NULL,
};

use crate::util::wide;
use crate::window;

const ID_TASKMGR: usize = 1;
const ID_STARTUP: usize = 2;
const ID_EXIT: usize = 3;

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const RUN_VALUE: &str = "Pulsebar";

pub fn open_task_manager() {
    unsafe {
        // taskmgr self-elevates through its manifest, so ShellExecuteW is enough.
        ShellExecuteW(
            null_mut(),
            wide("open").as_ptr(),
            wide("taskmgr.exe").as_ptr(),
            null(),
            null(),
            SW_SHOWNORMAL,
        );
    }
}

fn startup_enabled() -> bool {
    unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            wide(RUN_KEY).as_ptr(),
            wide(RUN_VALUE).as_ptr(),
            RRF_RT_REG_SZ,
            null_mut(),
            null_mut(),
            null_mut(),
        ) == 0
    }
}

fn toggle_startup() {
    unsafe {
        let mut key: HKEY = null_mut();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            wide(RUN_KEY).as_ptr(),
            0,
            KEY_SET_VALUE,
            &mut key,
        ) != 0
        {
            return;
        }
        if startup_enabled() {
            RegDeleteValueW(key, wide(RUN_VALUE).as_ptr());
        } else {
            let mut path = vec![0u16; 32768];
            let len = GetModuleFileNameW(null_mut(), path.as_mut_ptr(), path.len() as u32) as usize;
            if len > 0 {
                // Quote the path so it survives spaces.
                let mut value: Vec<u16> = Vec::with_capacity(len + 3);
                value.push('"' as u16);
                value.extend_from_slice(&path[..len]);
                value.push('"' as u16);
                value.push(0);
                RegSetValueExW(
                    key,
                    wide(RUN_VALUE).as_ptr(),
                    0,
                    REG_SZ,
                    value.as_ptr() as *const u8,
                    (value.len() * 2) as u32,
                );
            }
        }
        RegCloseKey(key);
    }
}

pub fn show_context_menu(widget: HWND) {
    unsafe {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return;
        }
        let check = if startup_enabled() { MF_CHECKED } else { 0 };
        AppendMenuW(
            menu,
            MF_STRING,
            ID_TASKMGR,
            wide("Open &Task Manager").as_ptr(),
        );
        AppendMenuW(
            menu,
            MF_STRING | check,
            ID_STARTUP,
            wide("Run at &startup").as_ptr(),
        );
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
        AppendMenuW(menu, MF_STRING, ID_EXIT, wide("E&xit").as_ptr());

        let mut pt = POINT { x: 0, y: 0 };
        GetCursorPos(&mut pt);
        SetForegroundWindow(widget);
        let cmd = TrackPopupMenuEx(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_NONOTIFY,
            pt.x,
            pt.y,
            widget,
            null(),
        );
        // The usual idiom to make the menu dismiss cleanly.
        PostMessageW(widget, WM_NULL, 0, 0);
        DestroyMenu(menu);

        match cmd as usize {
            ID_TASKMGR => open_task_manager(),
            ID_STARTUP => toggle_startup(),
            ID_EXIT => {
                DestroyWindow(window::app().manager);
            }
            _ => {}
        }
    }
}
