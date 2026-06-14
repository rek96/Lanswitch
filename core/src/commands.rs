//! Turns a *validated* `ApplyRequest` into a sequence of explicit argv vectors
//! for the native OS tools. We return `Vec<Vec<String>>` (never a shell string)
//! so the helper spawns each step with
//! `Command::new(argv[0]).args(&argv[1..])` — the OS never re-parses our values,
//! so there's no command-injection surface.
//!
//! A static config can need more than one command: set the address, then set
//! DNS servers if any were provided.

use crate::types::{ApplyRequest, Mode};
use crate::validate::prefix_to_mask;

/// macOS: `networksetup`, which speaks friendly *service* names directly.
#[cfg(target_os = "macos")]
pub fn build_commands(req: &ApplyRequest) -> Vec<Vec<String>> {
    let mut cmds: Vec<Vec<String>> = Vec::new();
    let iface = req.interface.clone();

    match req.mode {
        Mode::Dhcp => cmds.push(vec!["networksetup".into(), "-setdhcp".into(), iface.clone()]),
        Mode::Static => {
            let ip = req.ip.clone().unwrap_or_default();
            let mask = prefix_to_mask(req.prefix.unwrap_or(24));
            // -setmanual requires a router token. Use the gateway if given,
            // otherwise the host's own IP (no usable default route — correct
            // for isolated AV LANs).
            let router = req.gateway.clone().unwrap_or_else(|| ip.clone());
            cmds.push(vec![
                "networksetup".into(),
                "-setmanual".into(),
                iface.clone(),
                ip,
                mask,
                router,
            ]);
        }
    }

    if !req.dns.is_empty() {
        let mut c = vec!["networksetup".into(), "-setdnsservers".into(), iface];
        c.extend(req.dns.iter().cloned());
        cmds.push(c);
    } else if req.dns_clear {
        // "Empty" reverts the service to automatic/DHCP-provided DNS.
        cmds.push(vec!["networksetup".into(), "-setdnsservers".into(), iface, "Empty".into()]);
    }

    cmds
}

/// Windows: `netsh`, which overwrites cleanly. `name=` takes the friendly alias.
#[cfg(target_os = "windows")]
pub fn build_commands(req: &ApplyRequest) -> Vec<Vec<String>> {
    let mut cmds: Vec<Vec<String>> = Vec::new();
    let name = format!("name={}", req.interface);

    let mut addr = vec![
        "netsh".into(),
        "interface".into(),
        "ip".into(),
        "set".into(),
        "address".into(),
        name.clone(),
    ];
    match req.mode {
        Mode::Dhcp => addr.push("dhcp".into()),
        Mode::Static => {
            addr.push("static".into());
            addr.push(req.ip.clone().unwrap_or_default());
            addr.push(prefix_to_mask(req.prefix.unwrap_or(24)));
            if let Some(gw) = &req.gateway {
                addr.push(gw.clone());
            }
        }
    }
    cmds.push(addr);

    // DNS: first server replaces the list, the rest are appended by index.
    if let Some((first, rest)) = req.dns.split_first() {
        cmds.push(vec![
            "netsh".into(), "interface".into(), "ip".into(), "set".into(), "dns".into(),
            name.clone(), "static".into(), first.clone(),
        ]);
        for (i, d) in rest.iter().enumerate() {
            cmds.push(vec![
                "netsh".into(), "interface".into(), "ip".into(), "add".into(), "dns".into(),
                name.clone(), d.clone(), format!("index={}", i + 2),
            ]);
        }
    } else if req.dns_clear {
        // Revert to DHCP-provided DNS.
        cmds.push(vec![
            "netsh".into(), "interface".into(), "ip".into(), "set".into(), "dns".into(),
            name, "dhcp".into(),
        ]);
    }

    cmds
}

/// Fallback so the crate type-checks on other targets (e.g. CI on Linux).
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn build_commands(_req: &ApplyRequest) -> Vec<Vec<String>> {
    vec![vec!["true".into()]]
}
