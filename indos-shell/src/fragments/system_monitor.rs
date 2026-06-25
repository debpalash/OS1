//! System Monitor fragment — displays CPU, memory, disk usage
//!
//! Props:
//!   "show_cpu": bool (default true)
//!   "show_memory": bool (default true)
//!   "show_disk": bool (default true)
//!
//! Reads from /proc/stat, /proc/meminfo, /proc/mounts at render time.
//! For M1 this is a static snapshot — live updating is M2 work.

use iced::widget::{column, container, row, text};
use iced::{Element, Length};

/// Read memory info from /proc/meminfo
fn read_meminfo() -> (u64, u64) {
    let content = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut total = 0u64;
    let mut available = 0u64;

    for line in content.lines() {
        if line.starts_with("MemTotal:") {
            total = parse_kb_line(line);
        } else if line.starts_with("MemAvailable:") {
            available = parse_kb_line(line);
        }
    }
    (total, total.saturating_sub(available))
}

fn parse_kb_line(line: &str) -> u64 {
    line.split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

/// Read load average from /proc/loadavg
fn read_loadavg() -> String {
    std::fs::read_to_string("/proc/loadavg")
        .unwrap_or_default()
        .split_whitespace()
        .take(3)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Read disk usage for root partition
fn read_disk_usage() -> (u64, u64) {
    // Use statvfs for root
    unsafe {
        let mut stat: libc::statvfs = std::mem::zeroed();
        let path = std::ffi::CString::new("/").unwrap();
        if libc::statvfs(path.as_ptr(), &mut stat) == 0 {
            let total = stat.f_blocks * stat.f_frsize;
            let free = stat.f_bfree * stat.f_frsize;
            (
                total / (1024 * 1024 * 1024),
                (total - free) / (1024 * 1024 * 1024),
            )
        } else {
            (0, 0)
        }
    }
}

/// Read uptime
fn read_uptime() -> String {
    let secs: f64 = std::fs::read_to_string("/proc/uptime")
        .unwrap_or_default()
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);

    let hours = (secs / 3600.0) as u64;
    let mins = ((secs % 3600.0) / 60.0) as u64;
    format!("{}h {}m", hours, mins)
}

fn bar_text(label: &str, used: u64, total: u64, unit: &str) -> String {
    let pct = if total > 0 {
        (used as f64 / total as f64 * 100.0) as u32
    } else {
        0
    };
    let bar_width = 20;
    let filled = (pct as usize * bar_width) / 100;
    let bar: String = "█".repeat(filled) + &"░".repeat(bar_width - filled);
    format!(
        "{}: [{}] {}/{} {} ({}%)",
        label, bar, used, total, unit, pct
    )
}

pub fn render<'a, M: 'a + Clone>(props: &serde_json::Value) -> Element<'a, M> {
    let show_cpu = props
        .get("show_cpu")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let show_memory = props
        .get("show_memory")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let show_disk = props
        .get("show_disk")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let mut col = column![text("◆ System Monitor")
        .size(15)
        .color(iced::Color::from_rgb(0.5, 0.8, 1.0)),]
    .spacing(6)
    .padding(12)
    .width(Length::Fill);

    // Uptime + load
    let load = read_loadavg();
    let uptime = read_uptime();
    col = col.push(
        text(format!("Uptime: {}  ·  Load: {}", uptime, load))
            .size(13)
            .color(iced::Color::from_rgb(0.6, 0.6, 0.65)),
    );

    if show_memory {
        let (total_kb, used_kb) = read_meminfo();
        let total_mb = total_kb / 1024;
        let used_mb = used_kb / 1024;
        col = col.push(
            text(bar_text("RAM", used_mb, total_mb, "MB"))
                .size(13)
                .color(iced::Color::from_rgb(0.6, 0.9, 0.6)),
        );
    }

    if show_disk {
        let (total_gb, used_gb) = read_disk_usage();
        col = col.push(
            text(bar_text("Disk", used_gb, total_gb, "GB"))
                .size(13)
                .color(iced::Color::from_rgb(0.9, 0.7, 0.5)),
        );
    }

    container(col)
        .width(Length::Fill)
        .style(|_theme: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb(
                0.06, 0.08, 0.1,
            ))),
            border: iced::Border {
                color: iced::Color::from_rgb(0.15, 0.25, 0.35),
                width: 1.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        })
        .into()
}
