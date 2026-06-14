//! Read-only discovery of interfaces and their current IPs. This needs **no**
//! elevation, so it runs in the normal app process. Only *applying* a change
//! goes through the privileged helper.

use crate::types::Interface;

#[cfg(target_os = "macos")]
pub fn list_interfaces() -> Vec<Interface> {
    use std::process::Command;

    // Friendly service names, one per line. A leading "*" marks a disabled
    // service; the first line is a human-readable header we skip.
    let out = match Command::new("networksetup")
        .arg("-listallnetworkservices")
        .output()
    {
        Ok(o) => o,
        Err(_) => return vec![],
    };
    let text = String::from_utf8_lossy(&out.stdout);

    let mut result = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if i == 0 {
            continue; // header line
        }
        let disabled = line.starts_with('*');
        let name = line.trim_start_matches('*').trim().to_string();
        if name.is_empty() {
            continue;
        }

        // `networksetup -getinfo "<service>"` gives "IP address: x" when active.
        let (current_ip, connected) = getinfo_macos(&name);
        result.push(Interface {
            name: name.clone(),
            connected: connected && !disabled,
            current_ip,
            kind: Some(guess_kind(&name)),
        });
    }
    result
}

#[cfg(target_os = "macos")]
fn getinfo_macos(service: &str) -> (Option<String>, bool) {
    use std::process::Command;
    let out = match Command::new("networksetup").args(["-getinfo", service]).output() {
        Ok(o) => o,
        Err(_) => return (None, false),
    };
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("IP address: ") {
            let ip = rest.trim().to_string();
            if ip.is_empty() || ip == "none" {
                return (None, false);
            }
            return (Some(ip), true);
        }
    }
    (None, false)
}

#[cfg(target_os = "macos")]
fn guess_kind(name: &str) -> String {
    let n = name.to_lowercase();
    if n.contains("wi-fi") || n.contains("wifi") || n.contains("airport") {
        "Wi-Fi".into()
    } else if n.contains("usb") {
        "USB Ethernet".into()
    } else if n.contains("thunderbolt") {
        "Thunderbolt".into()
    } else {
        "Ethernet".into()
    }
}

#[cfg(target_os = "windows")]
pub fn list_interfaces() -> Vec<Interface> {
    use serde_json::Value;
    use std::process::Command;

    // Pull adapters + their IPv4 addresses as JSON in one PowerShell call.
    // `-Compress` keeps it on one line; we tolerate a single object or an array.
    let script = r#"
$ads = Get-NetAdapter | Select-Object Name, Status, InterfaceDescription
$ips = Get-NetIPAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue |
       Select-Object InterfaceAlias, IPAddress
[PSCustomObject]@{ adapters = $ads; ips = $ips } | ConvertTo-Json -Depth 4 -Compress
"#;

    let out = match Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
    {
        Ok(o) => o,
        Err(_) => return vec![],
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let root: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    let as_array = |v: &Value| -> Vec<Value> {
        match v {
            Value::Array(a) => a.clone(),
            Value::Null => vec![],
            other => vec![other.clone()],
        }
    };

    let ips = as_array(&root["ips"]);
    let ip_for = |alias: &str| -> Option<String> {
        ips.iter()
            .find(|e| e["InterfaceAlias"].as_str() == Some(alias))
            .and_then(|e| e["IPAddress"].as_str().map(|s| s.to_string()))
    };

    as_array(&root["adapters"])
        .iter()
        .filter_map(|a| {
            let name = a["Name"].as_str()?.to_string();
            let status = a["Status"].as_str().unwrap_or("");
            let desc = a["InterfaceDescription"].as_str().unwrap_or("").to_lowercase();
            let kind = if desc.contains("wi-fi") || desc.contains("wireless") {
                "Wi-Fi"
            } else if desc.contains("usb") {
                "USB Ethernet"
            } else {
                "Ethernet"
            };
            Some(Interface {
                current_ip: ip_for(&name),
                connected: status.eq_ignore_ascii_case("Up"),
                kind: Some(kind.to_string()),
                name,
            })
        })
        .collect()
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn list_interfaces() -> Vec<Interface> {
    vec![]
}

/// Just the friendly names — handed to `validate_apply` as the allow-list.
pub fn live_interface_names() -> Vec<String> {
    list_interfaces().into_iter().map(|i| i.name).collect()
}
