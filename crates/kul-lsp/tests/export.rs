//! Integration test: spawn `kul-lsp`, complete the handshake, open a
//! document, send a `kul/export` custom request, and verify the response
//! envelope.
//!
//! Mirrors the hand-rolled minimal LSP client used by the other
//! integration tests so the cross-process behaviour is exercised end-to-
//! end (Content-Length framing, JSON-RPC, custom-method routing).

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

fn binary_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_kul-lsp"))
}

fn write_message(stdin: &mut ChildStdin, msg: &str) {
    write!(stdin, "Content-Length: {}\r\n\r\n{}", msg.len(), msg).expect("write message");
    stdin.flush().expect("flush stdin");
}

fn read_message(stdout: &mut BufReader<ChildStdout>) -> Option<String> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        let n = stdout.read_line(&mut line).ok()?;
        if n == 0 {
            return None;
        }
        if line == "\r\n" {
            break;
        }
        if let Some(rest) = line.strip_prefix("Content-Length:") {
            content_length = Some(rest.trim().parse().ok()?);
        }
    }
    let len = content_length?;
    let mut body = vec![0u8; len];
    stdout.read_exact(&mut body).ok()?;
    String::from_utf8(body).ok()
}

struct Handle {
    child: Child,
    stdin: ChildStdin,
    rx: mpsc::Receiver<String>,
    _reader: thread::JoinHandle<()>,
}

impl Handle {
    fn spawn() -> Self {
        let mut cmd = Command::new(binary_path());
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = cmd.spawn().expect("spawn kul-lsp");
        let stdin = child.stdin.take().expect("stdin");
        let stdout = BufReader::new(child.stdout.take().expect("stdout"));
        let (tx, rx) = mpsc::channel();
        let reader = thread::spawn(move || {
            let mut stdout = stdout;
            while let Some(msg) = read_message(&mut stdout) {
                if tx.send(msg).is_err() {
                    return;
                }
            }
        });
        Self {
            child,
            stdin,
            rx,
            _reader: reader,
        }
    }

    fn recv(&self, timeout: Duration) -> Option<String> {
        self.rx.recv_timeout(timeout).ok()
    }

    fn recv_response(&self, id: i64, timeout: Duration) -> Value {
        let deadline = Instant::now() + timeout;
        loop {
            let now = Instant::now();
            if now >= deadline {
                panic!("timed out waiting for response id {id}");
            }
            let raw = self
                .rx
                .recv_timeout(deadline - now)
                .expect("response message");
            let parsed: Value = serde_json::from_str(&raw).expect("valid json");
            if parsed.get("id").and_then(Value::as_i64) == Some(id) {
                return parsed;
            }
        }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn handshake(handle: &mut Handle) {
    write_message(
        &mut handle.stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#,
    );
    let init = handle.recv(Duration::from_secs(5)).expect("initialize");
    let init: Value = serde_json::from_str(&init).expect("valid json");
    // Verify the experimental capability advertises kulExport.
    let experimental = &init["result"]["capabilities"]["experimental"];
    assert_eq!(experimental["kulExport"]["formats"][0], "json");
    assert_eq!(experimental["kulExport"]["formats"][1], "cytoscape");
    write_message(
        &mut handle.stdin,
        r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#,
    );
}

fn open(handle: &mut Handle, uri: &str, source: &str) {
    let escaped = serde_json::to_string(source).unwrap();
    let did_open = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{uri}","languageId":"kul","version":1,"text":{escaped}}}}}}}"#
    );
    write_message(&mut handle.stdin, &did_open);
}

fn send_export(
    handle: &mut Handle,
    id: i64,
    uri: &str,
    format: &str,
    with_positions: bool,
) -> Value {
    let req = format!(
        r#"{{"jsonrpc":"2.0","id":{id},"method":"kul/export","params":{{"uri":"{uri}","format":"{format}","withPositions":{with_positions}}}}}"#
    );
    write_message(&mut handle.stdin, &req);
    handle.recv_response(id, Duration::from_secs(5))
}

#[test]
fn export_clean_document_returns_success_envelope() {
    let mut handle = Handle::spawn();
    handshake(&mut handle);
    open(
        &mut handle,
        "file:///clean.kul",
        "person alice name:\"Alice\" gender:female\nperson bob name:\"Bob\" gender:male\nmarriage m alice bob start:1972\n",
    );
    let response = send_export(&mut handle, 100, "file:///clean.kul", "json", false);
    let envelope = &response["result"];
    assert_eq!(envelope["ok"], true);
    assert_eq!(envelope["schema"], 1);
    assert!(envelope["graph"]["persons"].is_array());
    assert!(envelope["graph"]["marriages"].is_array());
    assert!(envelope["graph"]["parenthoodLinks"].is_array());
}

#[test]
fn export_dirty_document_returns_failure_envelope() {
    let mut handle = Handle::spawn();
    handshake(&mut handle);
    open(
        &mut handle,
        "file:///dirty.kul",
        "person alice gender:female\n",
    );
    let response = send_export(&mut handle, 200, "file:///dirty.kul", "json", false);
    let envelope = &response["result"];
    assert_eq!(envelope["ok"], false);
    assert!(
        envelope["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "KUL-R03"),
        "expected R03 in failure envelope: {envelope}"
    );
}

#[test]
fn export_cytoscape_format_returns_nodes_and_edges() {
    let mut handle = Handle::spawn();
    handshake(&mut handle);
    open(
        &mut handle,
        "file:///cy.kul",
        "person alice name:\"A\" gender:female\nperson bob name:\"B\" gender:male\nmarriage m alice bob start:1972\n",
    );
    let response = send_export(&mut handle, 300, "file:///cy.kul", "cytoscape", false);
    let envelope = &response["result"];
    assert_eq!(envelope["ok"], true);
    let nodes = envelope["graph"]["nodes"].as_array().expect("nodes");
    let edges = envelope["graph"]["edges"].as_array().expect("edges");
    assert!(nodes.iter().any(|n| n["data"]["id"] == "p:alice"));
    assert!(nodes.iter().any(|n| n["data"]["id"] == "m:m"));
    assert!(edges.iter().any(|e| e["data"]["type"] == "spouse"));
}

#[test]
fn export_unknown_document_returns_invalid_params_error() {
    let mut handle = Handle::spawn();
    handshake(&mut handle);
    let response = send_export(&mut handle, 400, "file:///never-opened.kul", "json", false);
    let error = &response["error"];
    assert!(!error.is_null(), "expected error response, got {response}");
    // -32602 is JSON-RPC's `Invalid Params`.
    assert_eq!(error["code"], -32602);
    assert!(
        error["message"].as_str().unwrap().contains("not open"),
        "error message should mention the document is not open: {error}"
    );
}
