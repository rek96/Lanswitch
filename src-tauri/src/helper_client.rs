//! Unprivileged client that talks to the privileged helper over the local
//! socket. Used only for *applying* a config; everything read-only is done
//! directly in-process via `lanswitch_core::discover`.

use std::io::{BufRead, BufReader, Write};

use interprocess::local_socket::{prelude::*, GenericNamespaced, Stream};

use lanswitch_core::types::{HelperRequest, HelperResponse, Preset};

const SOCKET_NAME: &str = "lanswitch-helper.sock";

/// Send an apply request to the helper and return its result.
/// Returns a human-readable error string on failure (so the UI can show it).
pub fn apply_via_helper(preset: &Preset, interface: &str) -> Result<(), String> {
    let name = SOCKET_NAME
        .to_ns_name::<GenericNamespaced>()
        .map_err(|e| format!("bad socket name: {e}"))?;

    let conn = Stream::connect(name).map_err(|e| {
        format!(
            "Couldn't reach the LANSwitch helper ({e}). Is the helper installed and running? \
             See docs/PRIVILEGED-HELPER.md."
        )
    })?;

    let req = HelperRequest { preset: preset.clone(), interface: interface.to_string() };
    let line = serde_json::to_string(&req).map_err(|e| e.to_string())?;

    let mut reader = BufReader::new(conn);
    {
        let writer = reader.get_mut();
        writer.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
        writer.write_all(b"\n").map_err(|e| e.to_string())?;
        writer.flush().map_err(|e| e.to_string())?;
    }

    let mut resp_line = String::new();
    reader.read_line(&mut resp_line).map_err(|e| e.to_string())?;

    let resp: HelperResponse =
        serde_json::from_str(resp_line.trim()).map_err(|e| e.to_string())?;

    if resp.ok {
        Ok(())
    } else {
        Err(resp.error.unwrap_or_else(|| "unknown helper error".into()))
    }
}
