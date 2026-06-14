//! Shared data types used by both the unprivileged app and the privileged helper.

use serde::{Deserialize, Serialize};

/// How an interface should obtain its address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Static,
    Dhcp,
}

/// A network preset the user can apply to an interface.
///
/// Example (Coda Audio): mode=static, ip=192.168.0.245, prefix=24, no gateway.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    /// Stable id, e.g. "coda".
    pub id: String,
    /// Friendly display name, e.g. "Coda Audio".
    pub name: String,
    /// Hex color for the dot shown in the tray, e.g. "#3B82F6".
    #[serde(default)]
    pub color: String,
    pub mode: Mode,
    /// Required when mode == Static.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    /// CIDR prefix length (e.g. 24 or 8). Required when mode == Static.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefix: Option<u8>,
    /// Optional. Most flat AV LANs have no gateway, so this is usually empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gateway: Option<String>,
    /// Optional DNS servers. Usually empty for isolated LANs.
    #[serde(default)]
    pub dns: Vec<String>,
    /// When true (and `dns` is empty), revert DNS to automatic/DHCP-provided.
    /// Distinct from leaving DNS untouched.
    #[serde(default, skip_serializing_if = "is_false")]
    pub dns_clear: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// The on-disk presets file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetsFile {
    pub version: u32,
    pub presets: Vec<Preset>,
}

/// A network interface as the user sees it — friendly name only, never a MAC
/// address or cryptic device id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interface {
    /// The friendly name we pass straight back to the OS tools.
    /// macOS: a network *service* name ("Wi-Fi", "USB 10/100/1000 LAN").
    /// Windows: the connection *alias* ("Ethernet", "Wi-Fi", "Ethernet 2").
    pub name: String,
    /// Whether the interface is currently up/connected.
    pub connected: bool,
    /// Current IPv4 address if known (for display next to the name).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_ip: Option<String>,
    /// A human hint about the kind of port (Wi-Fi / Ethernet / USB), best-effort.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

/// A validated, ready-to-apply configuration. Built only via `validate`, so the
/// helper can trust every field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyRequest {
    pub interface: String,
    pub mode: Mode,
    pub ip: Option<String>,
    pub prefix: Option<u8>,
    pub gateway: Option<String>,
    pub dns: Vec<String>,
    #[serde(default)]
    pub dns_clear: bool,
}

/// What the app sends to the privileged helper. We send the whole preset plus
/// the chosen interface, and let the helper re-validate — the helper never
/// trusts a pre-built command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelperRequest {
    pub preset: Preset,
    pub interface: String,
}

/// Response sent back from the privileged helper over IPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelperResponse {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl HelperResponse {
    pub fn ok() -> Self {
        Self { ok: true, error: None }
    }
    pub fn err(msg: impl Into<String>) -> Self {
        Self { ok: false, error: Some(msg.into()) }
    }
}
