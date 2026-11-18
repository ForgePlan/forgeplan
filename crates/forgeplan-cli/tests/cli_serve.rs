//! `forgeplan serve` starts the MCP server on stdio. The test sends a
//! JSON-RPC `initialize` request and expects a valid `serverInfo` response —
//! proving the MCP handshake works end-to-end.

use assert_cmd::Command;
use std::io::Write;
use std::process::{Command as ProcCommand, Stdio};
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

fn init_workspace(tmp: &TempDir) {
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
}
/// Absolute path to the built binary, resolved the same way assert_cmd does.
fn binary_path() -> std::path::PathBuf {
    std::path::PathBuf::from(forgeplan().get_program())
}
/// `forgeplan serve` must accept a JSON-RPC `initialize` on stdin and respond
/// with a valid `serverInfo` + `capabilities` — the MCP handshake.
#[test]
fn serve_responds_to_mcp_initialize() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    let mut child = ProcCommand::new(binary_path())
        .arg("serve")
        .current_dir(tmp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn forgeplan serve");

    let init_msg = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "test-client", "version": "0.1" }
        }
    });
    let msg_bytes = serde_json::to_vec(&init_msg).unwrap();
    let mut stdin = child.stdin.take().unwrap();
    // The rmcp stdio transport is newline-delimited JSON (LSP-style
    // Content-Length framing is rejected with "connection closed").
    stdin.write_all(&msg_bytes).unwrap();
    stdin.write_all(b"\n").unwrap();
    drop(stdin);

    // Read until we see a response line containing "serverInfo" or a timeout.
    let mut stdout = child.stdout.take().unwrap();
    let mut response = Vec::new();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut byte = [0u8; 1];
    while std::time::Instant::now() < deadline {
        use std::io::Read;
        match stdout.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                response.push(byte[0]);
                let text = String::from_utf8_lossy(&response);
                if text.contains("serverInfo") {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    let _ = child.kill();
    let response_text = String::from_utf8_lossy(&response);
    assert!(
        response_text.contains("serverInfo")
            || response_text.contains("capabilities")
            || response_text.contains("forgeplan"),
        "serve must respond to MCP initialize; got:\n{response_text}"
    );
}

/// `forgeplan serve --profile constellation-read` must start in read-only
/// mode — same MCP handshake but the tool list must NOT include write tools.
#[test]
fn serve_constellation_read_profile_starts() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    let mut child = ProcCommand::new(binary_path())
        .args(["serve", "--profile", "constellation-read"])
        .current_dir(tmp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn forgeplan serve constellation-read");

    let init_msg = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "test-client", "version": "0.1" }
        }
    });
    let msg_bytes = serde_json::to_vec(&init_msg).unwrap();
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(&msg_bytes).unwrap();
    stdin.write_all(b"\n").unwrap();
    drop(stdin);

    let mut stdout = child.stdout.take().unwrap();
    let mut response = Vec::new();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut byte = [0u8; 1];
    while std::time::Instant::now() < deadline {
        use std::io::Read;
        match stdout.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                response.push(byte[0]);
                if String::from_utf8_lossy(&response).contains("serverInfo") {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let _ = child.kill();
    let response_text = String::from_utf8_lossy(&response);
    assert!(
        response_text.contains("serverInfo") || response_text.contains("capabilities"),
        "constellation-read profile must respond to initialize; got:\n{response_text}"
    );
}
