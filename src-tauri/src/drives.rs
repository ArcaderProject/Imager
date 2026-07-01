use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
pub struct Drive {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub removable: bool,
}

pub fn list_drives() -> Result<Vec<Drive>, String> {
    #[cfg(target_os = "linux")]
    {
        linux::list()
    }
    #[cfg(target_os = "macos")]
    {
        macos::list()
    }
    #[cfg(target_os = "windows")]
    {
        windows::list()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err("Unsupported platform".into())
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use serde_json::Value;
    use std::process::Command;

    pub fn list() -> Result<Vec<Drive>, String> {
        let out = Command::new("lsblk")
            .args([
                "-d",
                "-b",
                "-J",
                "-o",
                "NAME,MODEL,SIZE,RM,TYPE,TRAN,MOUNTPOINT,HOTPLUG",
            ])
            .output()
            .map_err(|e| format!("failed to run lsblk: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "lsblk failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        let json: Value =
            serde_json::from_slice(&out.stdout).map_err(|e| format!("bad lsblk json: {e}"))?;
        let mut drives = Vec::new();
        if let Some(devs) = json.get("blockdevices").and_then(|v| v.as_array()) {
            for d in devs {
                let dtype = d.get("type").and_then(|v| v.as_str()).unwrap_or("");
                if dtype != "disk" {
                    continue;
                }
                let name = d.get("name").and_then(|v| v.as_str()).unwrap_or("");
                if name.starts_with("loop")
                    || name.starts_with("dm-")
                    || name.starts_with("ram")
                    || name.starts_with("zram")
                {
                    continue;
                }
                let tran = d.get("tran").and_then(|v| v.as_str()).unwrap_or("");
                let rm = d.get("rm").and_then(|v| v.as_bool()).unwrap_or(false);
                let hotplug = d.get("hotplug").and_then(|v| v.as_bool()).unwrap_or(false);
                let removable = rm || hotplug || tran == "usb";
                let size = d.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
                let model = d
                    .get("model")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let label = if model.is_empty() {
                    format!("/dev/{name}")
                } else {
                    model.clone()
                };
                let path = format!("/dev/{name}");
                drives.push(Drive {
                    path,
                    name: label,
                    size,
                    removable,
                });
            }
        }
        Ok(drives.into_iter().filter(|d| d.removable).collect())
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use std::process::Command;

    pub fn list() -> Result<Vec<Drive>, String> {
        let out = Command::new("diskutil")
            .args(["list", "-plist", "external", "physical"])
            .output()
            .map_err(|e| format!("failed to run diskutil: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "diskutil failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        let plist = String::from_utf8_lossy(&out.stdout);
        let mut drives = Vec::new();
        for id in extract_disk_ids(&plist) {
            if let Some(d) = describe_disk(&id) {
                drives.push(d);
            }
        }
        Ok(drives)
    }

    fn extract_disk_ids(plist: &str) -> Vec<String> {
        let mut ids = Vec::new();
        let mut in_whole = false;
        for line in plist.lines() {
            let l = line.trim();
            if l.contains("<key>WholeDisks</key>") {
                in_whole = true;
                continue;
            }
            if in_whole {
                if l.starts_with("<string>") {
                    let s = l
                        .trim_start_matches("<string>")
                        .trim_end_matches("</string>");
                    if s.starts_with("disk") {
                        ids.push(s.to_string());
                    }
                } else if l.starts_with("</array>") {
                    break;
                }
            }
        }
        ids
    }

    fn describe_disk(id: &str) -> Option<Drive> {
        let out = Command::new("diskutil")
            .args(["info", "-plist", id])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let plist = String::from_utf8_lossy(&out.stdout);
        let media = plist_string(&plist, "MediaName").unwrap_or_else(|| id.to_string());
        let size = plist_integer(&plist, "TotalSize")
            .or_else(|| plist_integer(&plist, "Size"))
            .unwrap_or(0);
        let removable = plist_bool(&plist, "Removable").unwrap_or(true)
            || plist_bool(&plist, "Internal").map(|i| !i).unwrap_or(true);
        let path = format!("/dev/r{id}");
        let label = if media.trim().is_empty() {
            format!("/dev/{id}")
        } else {
            media.trim().to_string()
        };
        Some(Drive {
            path,
            name: label,
            size,
            removable,
        })
    }

    fn plist_string(plist: &str, key: &str) -> Option<String> {
        value_after_key(plist, key).and_then(|l| {
            l.trim()
                .strip_prefix("<string>")
                .and_then(|s| s.strip_suffix("</string>"))
                .map(|s| s.to_string())
        })
    }

    fn plist_integer(plist: &str, key: &str) -> Option<u64> {
        value_after_key(plist, key).and_then(|l| {
            l.trim()
                .strip_prefix("<integer>")
                .and_then(|s| s.strip_suffix("</integer>"))
                .and_then(|s| s.parse().ok())
        })
    }

    fn plist_bool(plist: &str, key: &str) -> Option<bool> {
        value_after_key(plist, key).map(|l| l.trim().contains("<true"))
    }

    fn value_after_key(plist: &str, key: &str) -> Option<String> {
        let needle = format!("<key>{key}</key>");
        let mut lines = plist.lines();
        while let Some(line) = lines.next() {
            if line.contains(&needle) {
                return lines.next().map(|s| s.to_string());
            }
        }
        None
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use super::*;
    use std::process::Command;

    pub fn list() -> Result<Vec<Drive>, String> {
        let script = r#"
$disks = Get-CimInstance -ClassName Win32_DiskDrive
$out = foreach ($d in $disks) {
  [PSCustomObject]@{
    path      = $d.DeviceID
    name      = ($d.Model).Trim()
    size      = [uint64]($d.Size)
    removable = ($d.MediaType -match 'Removable') -or ($d.InterfaceType -eq 'USB')
    interface = $d.InterfaceType
    index     = $d.Index
  }
}
$out | ConvertTo-Json -Compress -Depth 3
"#;
        let out = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .output()
            .map_err(|e| format!("failed to run powershell: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "powershell failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        let stdout = String::from_utf8_lossy(&out.stdout);
        let trimmed = stdout.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }
        let json: serde_json::Value =
            serde_json::from_str(trimmed).map_err(|e| format!("bad powershell json: {e}"))?;
        let arr = match json {
            serde_json::Value::Array(a) => a,
            other => vec![other],
        };
        let mut drives = Vec::new();
        for d in arr {
            let path = d
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if path.is_empty() {
                continue;
            }
            let size = d.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
            let iface = d.get("interface").and_then(|v| v.as_str()).unwrap_or("");
            let removable = d.get("removable").and_then(|v| v.as_bool()).unwrap_or(false)
                || iface.eq_ignore_ascii_case("usb");
            let model = d
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let label = if model.is_empty() {
                path.clone()
            } else {
                model
            };
            drives.push(Drive {
                path,
                name: label,
                size,
                removable,
            });
        }
        Ok(drives.into_iter().filter(|d| d.removable).collect())
    }
}
