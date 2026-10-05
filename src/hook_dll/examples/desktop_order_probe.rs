//! Read-only desktop ordering diagnostic: no focus, input, messages or restacking.
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, RECT},
        Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute},
        UI::WindowsAndMessaging::*,
    },
    core::{BOOL, w},
};

struct Entry {
    hwnd: HWND,
    pid: u32,
    class: String,
    desktop: bool,
    host: bool,
    visible: bool,
    topmost: bool,
    minimized: bool,
    cloaked: bool,
    owner: isize,
    previous: isize,
    rect: RECT,
}

unsafe extern "system" fn collect(hwnd: HWND, lp: LPARAM) -> BOOL {
    unsafe {
        let entries = &mut *(lp.0 as *mut Vec<Entry>);
        let mut class = [0u16; 256];
        let count = GetClassNameW(hwnd, &mut class) as usize;
        let class = String::from_utf16_lossy(&class[..count]);
        let mut title = [0u16; 256];
        let count = GetWindowTextW(hwnd, &mut title) as usize;
        let desktop = String::from_utf16_lossy(&title[..count]) == "桌面管理";
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        let mut cloaked = 0u32;
        let _ = DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &mut cloaked as *mut _ as _, 4);
        let mut rect = RECT::default();
        let _ = GetWindowRect(hwnd, &mut rect);
        entries.push(Entry {
            hwnd,
            pid,
            desktop,
            host: matches!(class.as_str(), "Progman" | "WorkerW")
                && FindWindowExW(Some(hwnd), None, w!("SHELLDLL_DefView"), None).is_ok(),
            class,
            visible: IsWindowVisible(hwnd).as_bool(),
            topmost: GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0,
            minimized: IsIconic(hwnd).as_bool(),
            cloaked: cloaked != 0,
            owner: GetWindow(hwnd, GW_OWNER).unwrap_or_default().0 as isize,
            previous: GetWindow(hwnd, GW_HWNDPREV).unwrap_or_default().0 as isize,
            rect,
        });
        BOOL(1)
    }
}

fn main() -> windows::core::Result<()> {
    let mut entries: Vec<Entry> = Vec::new();
    let mut cursor = unsafe { GetTopWindow(None) }.ok();
    let mut seen = std::collections::HashSet::new();
    while let Some(hwnd) = cursor {
        if entries.len() >= 4096 || !seen.insert(hwnd.0 as isize) {
            return Err(windows::core::Error::new(
                windows::core::HRESULT(-1),
                "Unstable window stack; retry probe",
            ));
        }
        unsafe {
            let _ = collect(hwnd, LPARAM(&mut entries as *mut _ as isize));
            cursor = GetWindow(hwnd, GW_HWNDNEXT).ok();
        }
    }
    let foreground = unsafe { GetForegroundWindow() }.0 as isize;
    println!("foreground={foreground}");
    let interactive = |e: &&Entry| {
        e.visible
            && !e.minimized
            && !e.cloaked
            && !e.topmost
            && !e.host
            && e.class != "WorkerW"
            && e.rect.right > e.rect.left
            && e.rect.bottom > e.rect.top
    };
    if let Some(own) = entries.iter().position(|e| e.desktop) {
        let above = entries[..own]
            .iter()
            .filter(|e| e.visible && !e.topmost && !e.host)
            .count();
        let below = entries[own + 1..]
            .iter()
            .filter(|e| e.visible && !e.topmost && !e.host && e.class != "WorkerW")
            .count();
        println!(
            "desktop_topmost={} normal_visible_above={above} normal_visible_below={below}",
            entries[own].topmost
        );
        println!(
            "visible_surface_above={} below={}",
            entries[..own].iter().filter(interactive).count(),
            entries[own + 1..].iter().filter(interactive).count()
        );
    }
    let special: Vec<_> = entries
        .iter()
        .enumerate()
        .filter_map(|(i, e)| (e.host || e.desktop).then_some(i))
        .collect();
    for (i, e) in entries.iter().enumerate() {
        if special.iter().any(|j| i.abs_diff(*j) <= 2) || (e.visible && !e.minimized && !e.cloaked)
        {
            println!(
                "{i}: hwnd={} pid={} class={} visible={} topmost={} desktop={} icon_host={} minimized={} cloaked={} owner={} previous={} rect={:?}",
                e.hwnd.0 as isize,
                e.pid,
                e.class,
                e.visible,
                e.topmost,
                e.desktop,
                e.host,
                e.minimized,
                e.cloaked,
                e.owner,
                e.previous,
                e.rect
            );
        }
    }
    Ok(())
}
