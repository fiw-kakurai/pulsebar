use windows_sys::Win32::Foundation::FILETIME;
use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows_sys::Win32::System::Threading::GetSystemTimes;

fn ft(f: &FILETIME) -> u64 {
    ((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64
}

/// CPU / memory usage. Uses only the cheapest system calls —
/// no WMI and no performance counters.
pub struct Metrics {
    prev: Option<(u64, u64, u64)>, // cumulative (idle, kernel, user)
    pub cpu_pct: f32,
    pub mem_pct: f32,
    pub mem_used_gb: f32,
    pub mem_total_gb: f32,
}

impl Metrics {
    pub fn new() -> Metrics {
        Metrics {
            prev: None,
            cpu_pct: 0.0,
            mem_pct: 0.0,
            mem_used_gb: 0.0,
            mem_total_gb: 0.0,
        }
    }

    pub fn sample(&mut self) {
        unsafe {
            let mut idle: FILETIME = std::mem::zeroed();
            let mut kernel: FILETIME = std::mem::zeroed();
            let mut user: FILETIME = std::mem::zeroed();
            if GetSystemTimes(&mut idle, &mut kernel, &mut user) != 0 {
                let (i, k, u) = (ft(&idle), ft(&kernel), ft(&user));
                if let Some((pi, pk, pu)) = self.prev {
                    // Kernel time includes idle time.
                    let total = k.saturating_sub(pk) + u.saturating_sub(pu);
                    let idle_d = i.saturating_sub(pi);
                    if total > 0 {
                        self.cpu_pct = (total.saturating_sub(idle_d) as f32 / total as f32 * 100.0)
                            .clamp(0.0, 100.0);
                    }
                }
                self.prev = Some((i, k, u));
            }

            let mut ms: MEMORYSTATUSEX = std::mem::zeroed();
            ms.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
            if GlobalMemoryStatusEx(&mut ms) != 0 {
                const GB: f32 = 1024.0 * 1024.0 * 1024.0;
                self.mem_pct = ms.dwMemoryLoad as f32;
                self.mem_total_gb = ms.ullTotalPhys as f32 / GB;
                self.mem_used_gb = ms.ullTotalPhys.saturating_sub(ms.ullAvailPhys) as f32 / GB;
            }
        }
    }

    pub fn tooltip_text(&self) -> String {
        format!(
            "CPU: {:.1} %\nMemory: {:.1} / {:.1} GB ({:.0} %)",
            self.cpu_pct, self.mem_used_gb, self.mem_total_gb, self.mem_pct
        )
    }
}
