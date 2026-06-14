//! Input validation. Everything the privileged helper acts on must pass through
//! `validate_apply` first. We never build shell strings — callers always spawn
//! with an explicit argv (see `commands.rs`), so the main defense here is
//! rejecting anything malformed before it reaches the OS tools.

use std::net::Ipv4Addr;

use crate::types::{ApplyRequest, Mode, Preset};

#[derive(Debug)]
pub struct ValidationError(pub String);

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for ValidationError {}

fn err<T>(msg: impl Into<String>) -> Result<T, ValidationError> {
    Err(ValidationError(msg.into()))
}

/// Interface names we accept. Covers macOS service names ("USB 10/100/1000 LAN",
/// "Wi-Fi", "Thunderbolt Bridge") and Windows aliases ("Ethernet 2"). We allow
/// letters, digits, space, and a small punctuation set. No shell metacharacters.
pub fn validate_interface_name(name: &str) -> Result<(), ValidationError> {
    if name.is_empty() || name.len() > 128 {
        return err("interface name has an invalid length");
    }
    let ok = name.chars().all(|c| {
        c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_' | '/' | '(' | ')' | '.')
    });
    if !ok {
        return err("interface name contains unsupported characters");
    }
    Ok(())
}

fn validate_ipv4(label: &str, value: &str) -> Result<Ipv4Addr, ValidationError> {
    value
        .parse::<Ipv4Addr>()
        .map_err(|_| ValidationError(format!("{label} is not a valid IPv4 address: {value}")))
}

/// Build a validated ApplyRequest from a preset + a chosen interface.
///
/// `live_interfaces` is the set of friendly names currently present on the
/// machine. We require the chosen interface to be one of them — the strongest
/// possible check against a forged/injected interface argument.
pub fn validate_apply(
    preset: &Preset,
    interface: &str,
    live_interfaces: &[String],
) -> Result<ApplyRequest, ValidationError> {
    validate_interface_name(interface)?;
    if !live_interfaces.iter().any(|i| i == interface) {
        return err(format!("interface \"{interface}\" is not currently present"));
    }

    match preset.mode {
        Mode::Dhcp => Ok(ApplyRequest {
            interface: interface.to_string(),
            mode: Mode::Dhcp,
            ip: None,
            prefix: None,
            gateway: None,
            dns: vec![],
            dns_clear: false,
        }),
        Mode::Static => {
            let ip = preset.ip.as_deref().ok_or_else(|| ValidationError(
                "static preset is missing an IP address".into(),
            ))?;
            validate_ipv4("IP address", ip)?;

            let prefix = preset.prefix.ok_or_else(|| ValidationError(
                "static preset is missing a prefix length".into(),
            ))?;
            if prefix > 32 {
                return err("prefix length must be between 0 and 32");
            }

            if let Some(gw) = preset.gateway.as_deref().filter(|s| !s.is_empty()) {
                validate_ipv4("gateway", gw)?;
            }
            for d in &preset.dns {
                if !d.is_empty() {
                    validate_ipv4("DNS server", d)?;
                }
            }

            Ok(ApplyRequest {
                interface: interface.to_string(),
                mode: Mode::Static,
                ip: Some(ip.to_string()),
                prefix: Some(prefix),
                gateway: preset.gateway.clone().filter(|s| !s.is_empty()),
                dns: preset.dns.iter().filter(|s| !s.is_empty()).cloned().collect(),
                dns_clear: preset.dns_clear,
            })
        }
    }
}

/// Convert a CIDR prefix (e.g. 24) to a dotted subnet mask (e.g. 255.255.255.0).
/// macOS `networksetup` and `netsh ... static` both want a dotted mask.
pub fn prefix_to_mask(prefix: u8) -> String {
    let bits: u32 = if prefix == 0 { 0 } else { u32::MAX << (32 - prefix as u32) };
    Ipv4Addr::from(bits).to_string()
}

/// Convert a dotted subnet mask (e.g. 255.255.255.0) to a CIDR prefix (24).
/// Returns None if the mask isn't a valid contiguous mask.
pub fn mask_to_prefix(mask: &str) -> Option<u8> {
    let bits = u32::from(mask.parse::<Ipv4Addr>().ok()?);
    let ones = bits.leading_ones();
    let expected = if ones == 0 { 0 } else { u32::MAX << (32 - ones) };
    if expected == bits {
        Some(ones as u8)
    } else {
        None
    }
}

/// Accept whatever a user types for "subnet": a prefix ("24" or "/24") or a
/// dotted mask ("255.255.255.0"). Returns the CIDR prefix.
pub fn parse_subnet(input: &str) -> Result<u8, ValidationError> {
    let s = input.trim().trim_start_matches('/');
    if s.contains('.') {
        mask_to_prefix(s)
            .ok_or_else(|| ValidationError(format!("not a valid subnet mask: {input}")))
    } else {
        let p: u8 = s
            .parse()
            .map_err(|_| ValidationError(format!("not a valid prefix length: {input}")))?;
        if p > 32 {
            return err("prefix length must be between 0 and 32");
        }
        Ok(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks() {
        assert_eq!(prefix_to_mask(24), "255.255.255.0");
        assert_eq!(prefix_to_mask(8), "255.0.0.0");
        assert_eq!(prefix_to_mask(16), "255.255.0.0");
        assert_eq!(prefix_to_mask(0), "0.0.0.0");
    }

    #[test]
    fn subnet_parsing() {
        assert_eq!(parse_subnet("24").unwrap(), 24);
        assert_eq!(parse_subnet("/24").unwrap(), 24);
        assert_eq!(parse_subnet("255.255.255.0").unwrap(), 24);
        assert_eq!(parse_subnet("255.0.0.0").unwrap(), 8);
        assert!(parse_subnet("255.255.0.255").is_err()); // non-contiguous
        assert!(parse_subnet("40").is_err());
    }

    #[test]
    fn rejects_bad_interface() {
        assert!(validate_interface_name("Wi-Fi; rm -rf /").is_err());
        assert!(validate_interface_name("USB 10/100/1000 LAN").is_ok());
        assert!(validate_interface_name("Ethernet 2").is_ok());
    }
}
