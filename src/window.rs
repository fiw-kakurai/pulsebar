use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowLongPtrW, IsWindow, KillTimer,
    LoadCursorW, PostMessageW, PostQuitMessage, RegisterClassW, SetParent, SetWindowLongPtrW,
    SetWindowPos, ShowWindow, GWL_STYLE, HWND_TOP, HWND_TOPMOST, IDC_ARROW, MA_NOACTIVATE,
    SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SW_SHOWNOACTIVATE,
    WM_APP, WM_DESTROY, WM_DISPLAYCHANGE, WM_LBUTTONUP, WM_MOUSEACTIVATE, WM_NCDESTROY,
    WM_RBUTTONUP, WM_SETTINGCHANGE, WM_TIMER, WNDCLASSW, WS_CHILD, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_OVERLAPPED, WS_POPUP,
};

use crate::menu;
use crate::metrics::Metrics;
use crate::render::Renderer;
use crate::taskbar::{self, Placement};
use crate::theme;
use crate::tooltip;
use crate::util::wide;

pub const TIMER_ID: usize = 1;
/// Tells the manager window that the widget was destroyed
/// (e.g. by an Explorer restart).
pub const WM_APP_WIDGET_DEAD: u32 = WM_APP + 1;

const MANAGER_CLASS: &str = "PulsebarManager";
const WIDGET_CLASS: &str = "PulsebarWidget";

/// Consecutive taskbar-attach failures before giving up.
const MAX_ATTACH_FAILS: u32 = 3;
/// Interval (ticks = seconds) between reattach attempts while in fallback mode.
const REATTACH_INTERVAL: u32 = 30;

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    Taskbar,
    Fallback,
}

pub struct App {
    pub hinst: HINSTANCE,
    pub manager: HWND,
    pub widget: HWND,
    pub tooltip: HWND,
    pub taskbar_created_msg: u32,
    mode: Mode,
    attach_fails: u32,
    present_fails: u32,
    reattach_countdown: u32,
    placement: Option<Placement>,
    renderer: Option<Renderer>,
    pub metrics: Metrics,
    light_theme: bool,
}

// The UI is single-threaded and the WndProcs need access, so the state is
// kept behind a raw pointer. Convention: borrow briefly inside each handler
// and never hold a borrow across a modal loop (TrackPopupMenuEx etc.).
static mut APP: *mut App = null_mut();

pub fn init_app(a: App) {
    unsafe {
        APP = Box::into_raw(Box::new(a));
    }
}

#[allow(static_mut_refs)]
pub fn app() -> &'static mut App {
    unsafe { &mut *APP }
}

fn app_ready() -> bool {
    unsafe { !APP.is_null() }
}

impl App {
    pub fn new(hinst: HINSTANCE, manager: HWND, taskbar_created_msg: u32) -> App {
        App {
            hinst,
            manager,
            widget: null_mut(),
            tooltip: null_mut(),
            taskbar_created_msg,
            mode: Mode::Taskbar,
            attach_fails: 0,
            present_fails: 0,
            reattach_countdown: 0,
            placement: None,
            renderer: None,
            metrics: Metrics::new(),
            light_theme: theme::system_uses_light_theme(),
        }
    }

    pub fn ensure_widget(&mut self) {
        if !self.widget.is_null() && unsafe { IsWindow(self.widget) } != 0 {
            return;
        }
        self.drop_widget_state();
        match self.mode {
            Mode::Taskbar => {
                if !self.try_attach() {
                    self.attach_fails += 1;
                    if self.attach_fails >= MAX_ATTACH_FAILS {
                        self.mode = Mode::Fallback;
                        self.create_fallback();
                    }
                }
            }
            Mode::Fallback => self.create_fallback(),
        }
    }

    fn drop_widget_state(&mut self) {
        unsafe {
            if !self.tooltip.is_null() && IsWindow(self.tooltip) != 0 {
                DestroyWindow(self.tooltip);
            }
            if !self.widget.is_null() && IsWindow(self.widget) != 0 {
                DestroyWindow(self.widget);
            }
        }
        self.tooltip = null_mut();
        self.widget = null_mut();
        self.renderer = None;
        self.placement = None;
    }

    /// Attach as a child window of Shell_TrayWnd (the TrafficMonitor technique).
    fn try_attach(&mut self) -> bool {
        let Some((tray, notify)) = taskbar::find_tray() else {
            return false;
        };
        let Some(pl) = taskbar::placement_in_tray(tray, notify) else {
            return false;
        };
        unsafe {
            let widget = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                wide(WIDGET_CLASS).as_ptr(),
                null(),
                WS_POPUP,
                0,
                0,
                pl.w,
                pl.h,
                null_mut(),
                null_mut(),
                self.hinst,
                null(),
            );
            if widget.is_null() {
                return false;
            }
            if SetParent(widget, tray).is_null() {
                DestroyWindow(widget);
                return false;
            }
            // SetParent does not rewrite styles; swap WS_POPUP for WS_CHILD by hand.
            let style = GetWindowLongPtrW(widget, GWL_STYLE);
            SetWindowLongPtrW(
                widget,
                GWL_STYLE,
                (style & !(WS_POPUP as isize)) | WS_CHILD as isize,
            );
            SetWindowPos(
                widget,
                HWND_TOP,
                pl.x,
                pl.y,
                pl.w,
                pl.h,
                SWP_FRAMECHANGED | SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            self.widget = widget;
            self.tooltip = tooltip::create(widget, self.hinst);
        }
        self.placement = Some(pl);
        self.renderer = Renderer::new(pl.w, pl.h);
        self.attach_fails = 0;
        self.present_fails = 0;
        self.render_now();
        true
    }

    /// Compatibility mode: a plain always-on-top borderless window.
    fn create_fallback(&mut self) {
        let pl = taskbar::placement_fallback();
        unsafe {
            let widget = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                wide(WIDGET_CLASS).as_ptr(),
                null(),
                WS_POPUP,
                pl.x,
                pl.y,
                pl.w,
                pl.h,
                null_mut(),
                null_mut(),
                self.hinst,
                null(),
            );
            if widget.is_null() {
                return;
            }
            ShowWindow(widget, SW_SHOWNOACTIVATE);
            self.widget = widget;
            self.tooltip = tooltip::create(widget, self.hinst);
        }
        self.placement = Some(pl);
        self.renderer = Renderer::new(pl.w, pl.h);
        self.reattach_countdown = REATTACH_INTERVAL;
        self.render_now();
    }

    pub fn tick(&mut self) {
        self.metrics.sample();
        self.ensure_widget();
        if self.widget.is_null() {
            return;
        }
        match self.mode {
            Mode::Taskbar => self.sync_taskbar_placement(),
            Mode::Fallback => self.tick_fallback(),
        }
        self.render_now();
        self.update_tooltip();
    }

    /// Detect taskbar size/position/DPI changes by comparing RECTs every tick
    /// (GetWindowRect is cheap enough, and this is more robust than keeping a
    /// registered appbar just for the notifications).
    fn sync_taskbar_placement(&mut self) {
        let Some((tray, notify)) = taskbar::find_tray() else {
            return;
        };
        let Some(pl) = taskbar::placement_in_tray(tray, notify) else {
            return;
        };
        if Some(pl) != self.placement {
            let size_changed = self.placement.is_none_or(|p| p.w != pl.w || p.h != pl.h);
            unsafe {
                SetWindowPos(
                    self.widget,
                    HWND_TOP,
                    pl.x,
                    pl.y,
                    pl.w,
                    pl.h,
                    SWP_NOACTIVATE,
                );
            }
            if size_changed {
                self.renderer = Renderer::new(pl.w, pl.h);
            }
            self.placement = Some(pl);
        }
    }

    fn tick_fallback(&mut self) {
        // Periodically try to get back onto the taskbar (Explorer may have
        // recovered, or the environment may have changed).
        if self.reattach_countdown > 0 {
            self.reattach_countdown -= 1;
        } else if taskbar::find_tray()
            .and_then(|(t, n)| taskbar::placement_in_tray(t, n))
            .is_some()
        {
            self.mode = Mode::Taskbar;
            self.attach_fails = 0;
            self.drop_widget_state();
            self.ensure_widget();
            return;
        } else {
            self.reattach_countdown = REATTACH_INTERVAL;
        }
        unsafe {
            // Reassert topmost every tick so other topmost windows don't bury us.
            SetWindowPos(
                self.widget,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }

    pub fn render_now(&mut self) {
        if self.widget.is_null() {
            return;
        }
        let pal = theme::palette(self.light_theme);
        let (cpu, mem) = (self.metrics.cpu_pct, self.metrics.mem_pct);
        let dpi = self.placement.map_or(96, |p| p.dpi);
        let ok = match self.renderer.as_mut() {
            Some(r) => {
                r.draw(cpu, mem, &pal, dpi);
                r.present(self.widget)
            }
            None => false,
        };
        if ok {
            self.present_fails = 0;
        } else {
            self.present_fails += 1;
            // UpdateLayeredWindow on a child window is unavailable in this
            // environment; switch to compatibility mode.
            if self.present_fails >= MAX_ATTACH_FAILS && self.mode == Mode::Taskbar {
                self.mode = Mode::Fallback;
                self.drop_widget_state();
            }
        }
    }

    fn update_tooltip(&mut self) {
        if self.tooltip.is_null() || self.widget.is_null() {
            return;
        }
        let text = self.metrics.tooltip_text();
        unsafe {
            tooltip::update(self.tooltip, self.widget, &text);
        }
    }

    pub fn reload_theme(&mut self) {
        let light = theme::system_uses_light_theme();
        if light != self.light_theme {
            self.light_theme = light;
            self.render_now();
        }
    }

    pub fn on_taskbar_created(&mut self) {
        self.mode = Mode::Taskbar;
        self.attach_fails = 0;
        self.drop_widget_state();
        // TrayNotifyWnd may not exist yet right after TaskbarCreated.
        // If this attempt fails, the next tick's ensure_widget retries.
        self.ensure_widget();
    }

    fn on_widget_dead(&mut self) {
        // The window itself is already gone; clean up handles and resources.
        self.drop_widget_state();
    }

    fn shutdown(&mut self) {
        self.drop_widget_state();
    }
}

pub fn register_classes(hinst: HINSTANCE) {
    unsafe {
        let cursor = LoadCursorW(null_mut(), IDC_ARROW);
        let mgr_name = wide(MANAGER_CLASS);
        let mgr = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(manager_wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: null_mut(),
            hCursor: cursor,
            hbrBackground: null_mut(),
            lpszMenuName: null(),
            lpszClassName: mgr_name.as_ptr(),
        };
        RegisterClassW(&mgr);

        let wdg_name = wide(WIDGET_CLASS);
        let wdg = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(widget_wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: null_mut(),
            hCursor: cursor,
            hbrBackground: null_mut(),
            lpszMenuName: null(),
            lpszClassName: wdg_name.as_ptr(),
        };
        RegisterClassW(&wdg);
    }
}

/// Broadcasts such as TaskbarCreated and WM_SETTINGCHANGE are only delivered
/// to top-level windows, so a hidden top-level manager window receives them.
pub fn create_manager(hinst: HINSTANCE) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(MANAGER_CLASS).as_ptr(),
            wide("Pulsebar").as_ptr(),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            null_mut(),
            null_mut(),
            hinst,
            null(),
        )
    }
}

/// Whether lParam points to "ImmersiveColorSet" (the theme-change notification).
unsafe fn is_immersive_color_set(lparam: LPARAM) -> bool {
    if lparam == 0 {
        return false;
    }
    let p = lparam as *const u16;
    let target: Vec<u16> = "ImmersiveColorSet".encode_utf16().collect();
    for (i, &t) in target.iter().enumerate() {
        if *p.add(i) != t {
            return false;
        }
    }
    *p.add(target.len()) == 0
}

unsafe extern "system" fn manager_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if !app_ready() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    if msg == app().taskbar_created_msg {
        app().on_taskbar_created();
        return 0;
    }
    match msg {
        WM_TIMER if wparam == TIMER_ID => {
            app().tick();
            0
        }
        WM_APP_WIDGET_DEAD => {
            app().on_widget_dead();
            0
        }
        WM_SETTINGCHANGE => {
            if is_immersive_color_set(lparam) {
                app().reload_theme();
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_DISPLAYCHANGE => {
            app().tick();
            0
        }
        WM_DESTROY => {
            KillTimer(hwnd, TIMER_ID);
            app().shutdown();
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe extern "system" fn widget_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if !app_ready() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    match msg {
        // Never steal focus from the taskbar.
        WM_MOUSEACTIVATE => MA_NOACTIVATE as LRESULT,
        WM_LBUTTONUP => {
            menu::open_task_manager();
            0
        }
        WM_RBUTTONUP => {
            // WM_TIMER re-enters during the modal menu loop,
            // so no borrow of app() is held here.
            menu::show_context_menu(hwnd);
            0
        }
        WM_NCDESTROY => {
            // The parent took us down (Explorer restart).
            // Ask the manager to clean up.
            PostMessageW(app().manager, WM_APP_WIDGET_DEAD, 0, 0);
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
