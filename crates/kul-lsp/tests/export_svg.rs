//! Integration test for the `kul/exportSvg` custom request — end-to-end
//! via Content-Length framing, JSON-RPC, custom-method routing. Mirrors
//! the `kul/render` integration test; the only behavioural difference is
//! the baked self-contained SVG (ADR-0022).

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

mod common;

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
    let experimental = &init["result"]["capabilities"]["experimental"];
    // Both SVG-producing requests are advertised independently so clients
    // can feature-detect each one without coupling them.
    assert_eq!(experimental["kulRender"]["format"], "svg");
    assert_eq!(experimental["kulExportSvg"]["format"], "svg");
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

fn send_export_svg(handle: &mut Handle, id: i64, uri: &str) -> Value {
    let req = format!(
        r#"{{"jsonrpc":"2.0","id":{id},"method":"kul/exportSvg","params":{{"uri":"{uri}"}}}}"#
    );
    write_message(&mut handle.stdin, &req);
    handle.recv_response(id, Duration::from_secs(5))
}

#[test]
fn export_svg_clean_document_returns_self_contained_svg() {
    let mut handle = Handle::spawn();
    let source = "person alice name:\"Alice\" gender:female\nperson bob name:\"Bob\" gender:male\nmarriage m alice bob start:1972\n";
    let kul_url = common::fixture_url(
        "export_svg_clean_document_returns_self_contained_svg",
        "clean.kul",
        source,
    );
    let uri = kul_url.as_str();
    handshake(&mut handle);
    open(&mut handle, uri, source);
    let response = send_export_svg(&mut handle, 100, uri);
    let envelope = &response["result"];
    assert_eq!(envelope["ok"], true);
    let svg = envelope["svg"].as_str().expect("svg string");
    assert!(svg.starts_with("<svg"), "expected SVG document: {svg}");
    // Self-contained marker — distinguishes `kul/exportSvg` from the
    // theme-agnostic `kul/render` output (ADR-0016 vs ADR-0022).
    assert!(
        svg.contains("<style>"),
        "expected inline <style> in self-contained SVG: {svg}"
    );
}

#[test]
fn export_svg_dirty_document_returns_failure_with_diagnostics() {
    let mut handle = Handle::spawn();
    let source = "person alice gender:female\n";
    let kul_url = common::fixture_url(
        "export_svg_dirty_document_returns_failure_with_diagnostics",
        "dirty.kul",
        source,
    );
    let uri = kul_url.as_str();
    handshake(&mut handle);
    open(&mut handle, uri, source);
    let response = send_export_svg(&mut handle, 200, uri);
    let envelope = &response["result"];
    assert_eq!(envelope["ok"], false);
    let diags = envelope["diagnostics"]
        .as_array()
        .expect("diagnostics array");
    let anchored = diags
        .iter()
        .find(|d| d.get("uri").is_some() && d.get("range").is_some())
        .unwrap_or_else(|| panic!("expected at least one anchored diagnostic: {envelope}"));
    assert_eq!(
        anchored["severity"], "error",
        "errors-only filter must hold: {anchored}"
    );
    assert!(
        envelope.get("svg").is_none(),
        "failure envelope must not carry an svg field: {envelope}"
    );
}

#[test]
fn export_svg_unknown_document_returns_invalid_params_error() {
    let mut handle = Handle::spawn();
    let kul_url = common::fixture_url(
        "export_svg_unknown_document_returns_invalid_params_error",
        "never-opened.kul",
        "",
    );
    let uri = kul_url.as_str();
    handshake(&mut handle);
    let response = send_export_svg(&mut handle, 300, uri);
    let error = &response["error"];
    assert!(!error.is_null(), "expected error response, got {response}");
    assert_eq!(error["code"], -32602);
    assert!(
        error["message"].as_str().unwrap().contains("not open"),
        "error message should mention the document is not open: {error}"
    );
}
