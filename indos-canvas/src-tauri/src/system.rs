use serde_json::{json, Value};
use sysinfo::System;
use std::path::Path;

pub fn get_system_info() -> Value {
    let mut sys = System::new_all();
    sys.refresh_all();

    json!({
        "hostname": System::host_name().unwrap_or_else(|| "unknown".into()),
        "os": System::name().unwrap_or_else(|| "unknown".into()),
        "osVersion": System::os_version().unwrap_or_else(|| "unknown".into()),
        "kernel": System::kernel_version().unwrap_or_else(|| "unknown".into()),
        "cpus": sys.cpus().len(),
        "cpuBrand": sys.cpus().first().map(|c| c.brand().to_string()).unwrap_or_default(),
        "totalMemory": sys.total_memory(),
        "usedMemory": sys.used_memory(),
        "totalSwap": sys.total_swap(),
        "usedSwap": sys.used_swap(),
        "uptime": System::uptime(),
        "processes": sys.processes().len(),
    })
}

pub fn list_directory(path: &str) -> Result<Vec<Value>, String> {
    let dir = Path::new(path);
    if !dir.exists() {
        return Err(format!("Path does not exist: {}", path));
    }
    if !dir.is_dir() {
        return Err(format!("Not a directory: {}", path));
    }

    let mut entries = Vec::new();
    let read_dir = std::fs::read_dir(dir).map_err(|e| format!("Cannot read directory: {}", e))?;

    for entry in read_dir {
        if let Ok(entry) = entry {
            let metadata = entry.metadata().ok();
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
            let is_hidden = name.starts_with('.');

            entries.push(json!({
                "name": name,
                "path": entry.path().to_string_lossy(),
                "isDir": is_dir,
                "size": size,
                "isHidden": is_hidden,
                "extension": entry.path().extension().map(|e| e.to_string_lossy().to_string()),
            }));
        }
    }

    // Sort: directories first, then by name
    entries.sort_by(|a, b| {
        let a_dir = a["isDir"].as_bool().unwrap_or(false);
        let b_dir = b["isDir"].as_bool().unwrap_or(false);
        match (a_dir, b_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => {
                let a_name = a["name"].as_str().unwrap_or("");
                let b_name = b["name"].as_str().unwrap_or("");
                a_name.to_lowercase().cmp(&b_name.to_lowercase())
            }
        }
    });

    Ok(entries)
}
