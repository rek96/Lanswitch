//! LANSwitch privileged helper.
//!
//! Runs elevated (root via launchd on macOS, LocalSystem service on Windows).
//! It listens on a local socket and, for each request:
//!   1. parses a `HelperRequest` (preset + chosen interface),
//!   2. re-enumerates the *live* interfaces itself,
//!   3. re-runs full validation against that live list,
//!   4. builds an explicit argv (never a shell string) and executes it.
//!
//! Steps 2–4 mean a forged or malformed request cannot make the helper run
//! anything but a well-formed `networksetup` / `netsh` invocation against a
//! real interface.

use std::io::{BufRead, BufReader};

use interprocess::local_socket::{
    prelude::*, GenericNamespaced, ListenerOptions,
};

use lanswitch_core::{
    commands::build_commands,
    discover::live_interface_names,
    types::{HelperRequest, HelperResponse},
    validate::validate_apply,
};

/// Identifier for the local socket. On Windows this becomes a named pipe
/// (\\.\pipe\lanswitch-helper); on Unix a namespaced socket. The app uses the
/// same name to connect.
const SOCKET_NAME: &str = "lanswitch-helper.sock";

fn main() {
    if let Err(e) = run() {
        eprintln!("lanswitch-helper fatal: {e}");
        std::process::exit(1);
    }
}

fn run() -> std::io::Result<()> {
    let name = SOCKET_NAME.to_ns_name::<GenericNamespaced>()?;
    let listener = ListenerOptions::new().name(name).create_sync()?;
    eprintln!("lanswitch-helper listening on {SOCKET_NAME}");

    for conn in listener.incoming() {
        match conn {
            Ok(stream) => {
                // One request / one response per connection. Keep it simple and
                // synchronous; network changes are infrequent.
                if let Err(e) = handle(stream) {
                    eprintln!("connection error: {e}");
                }
            }
            Err(e) => eprintln!("accept error: {e}"),
        }
    }
    Ok(())
}

fn handle(stream: impl std::io::Read + std::io::Write) -> std::io::Result<()> {
    // We need both read and write halves; wrap in a buffered reader for the
    // request line and keep the writer for the response.
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;

    let resp = process(&line);
    let json = serde_json::to_string(&resp)
        .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"serialize failed\"}".into());

    let mut writer = reader.into_inner();
    writer.write_all(json.as_bytes())?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}

fn process(line: &str) -> HelperResponse {
    let req: HelperRequest = match serde_json::from_str(line.trim()) {
        Ok(r) => r,
        Err(e) => return HelperResponse::err(format!("bad request: {e}")),
    };

    // Re-validate against the interfaces that actually exist *right now*.
    let live = live_interface_names();
    let apply = match validate_apply(&req.preset, &req.interface, &live) {
        Ok(a) => a,
        Err(e) => return HelperResponse::err(e.to_string()),
    };

    let cmds = build_commands(&apply);
    if cmds.is_empty() {
        return HelperResponse::err("no command for this platform");
    }

    // Run each step in order; stop at the first failure so we don't leave a
    // half-applied config silently.
    for argv in &cmds {
        if argv.is_empty() {
            continue;
        }
        match std::process::Command::new(&argv[0]).args(&argv[1..]).output() {
            Ok(out) if out.status.success() => {}
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                let stdout = String::from_utf8_lossy(&out.stdout);
                let msg = if !stderr.trim().is_empty() { stderr } else { stdout };
                return HelperResponse::err(format!(
                    "step `{}` failed: {}",
                    argv.join(" "),
                    msg.trim()
                ));
            }
            Err(e) => {
                return HelperResponse::err(format!("could not run `{}`: {e}", argv.join(" ")))
            }
        }
    }

    HelperResponse::ok()
}
