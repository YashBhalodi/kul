//! Integration test: spawn `kul-lsp`, complete the handshake, open a
//! document, send a `kul/render` custom request, and verify the
//! response envelope.
//!
//! Mirrors `tests/export.rs` so the cross-process behaviour
//! (Content-Length framing, JSON-RPC, custom-method routing) is
//! exercised end-to-end.

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
    // The render capability advertises the SVG format under
    // `experimental.kulRender` so clients can detect support.
    let experimental = &init["result"]["capabilities"]["experimental"];
    assert_eq!(experimental["kulRender"]["format"], "svg");
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

fn send_render(handle: &mut Handle, id: i64, uri: &str) -> Value {
    let req = format!(
        r#"{{"jsonrpc":"2.0","id":{id},"method":"kul/render","params":{{"uri":"{uri}"}}}}"#
    );
    write_message(&mut handle.stdin, &req);
    handle.recv_response(id, Duration::from_secs(5))
}

#[test]
fn render_clean_document_returns_success_with_svg() {
    let mut handle = Handle::spawn();
    let source = "person alice name:\"Alice\" gender:female\nperson bob name:\"Bob\" gender:male\nmarriage m alice bob start:1972\n";
    let kul_url = common::fixture_url(
        "render_clean_document_returns_success_with_svg",
        "clean.kul",
        source,
    );
    let uri = kul_url.as_str();
    handshake(&mut handle);
    open(&mut handle, uri, source);
    let response = send_render(&mut handle, 100, uri);
    let envelope = &response["result"];
    assert_eq!(envelope["ok"], true);
    let svg = envelope["svg"].as_str().expect("svg string");
    assert!(svg.starts_with("<svg"), "expected SVG document: {svg}");
    assert!(
        svg.contains(r#"data-kind="canonical""#),
        "expected canonical card in SVG: {svg}"
    );
    assert!(
        svg.contains(r#"data-link-kind="marriage""#),
        "expected the marriage edge in SVG: {svg}"
    );
}

#[test]
fn render_dirty_document_returns_failure_with_diagnostics() {
    let mut handle = Handle::spawn();
    // Missing required `name:` triggers R03.
    let source = "person alice gender:female\n";
    let kul_url = common::fixture_url(
        "render_dirty_document_returns_failure_with_diagnostics",
        "dirty.kul",
        source,
    );
    let uri = kul_url.as_str();
    handshake(&mut handle);
    open(&mut handle, uri, source);
    let response = send_render(&mut handle, 200, uri);
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
    assert!(
        envelope.get("svg").is_none(),
        "failure envelope must not carry an svg field: {envelope}"
    );
}

#[test]
fn render_is_uri_invariant_for_a_multi_file_project() {
    // One project = one render (ADR-0015): the LSP keys its cache per
    // project, so every URI in the same project must produce the same
    // SVG. Pin that here by opening all three files of a multi-file
    // project and asserting the responses are byte-equal.
    let mut handle = Handle::spawn();
    let files: &[(&str, &str)] = &[
        (
            "01-founders.kul",
            "person ramesh name:\"Ramesh\" gender:male\n\
             person sita   name:\"Sita\"   gender:female\n\
             marriage m_ramesh_sita ramesh sita start:1952\n",
        ),
        (
            "02-parents.kul",
            "person alice name:\"Alice\" gender:female\n  birth m_ramesh_sita\n\
             person bob   name:\"Bob\"   gender:male\n\
             marriage m_alice_bob alice bob start:1978\n",
        ),
        (
            "03-grandchildren.kul",
            "person carol name:\"Carol\" gender:female\n  birth m_alice_bob\n",
        ),
    ];
    let (_dir, urls) =
        common::fixture_project("render_is_uri_invariant_for_a_multi_file_project", files);
    handshake(&mut handle);
    for (url, (_, source)) in urls.iter().zip(files.iter()) {
        open(&mut handle, url.as_str(), source);
    }

    let svgs: Vec<String> = urls
        .iter()
        .enumerate()
        .map(|(i, url)| {
            let response = send_render(&mut handle, 400 + i as i64, url.as_str());
            let envelope = &response["result"];
            assert_eq!(
                envelope["ok"], true,
                "render for {url} should succeed: {envelope}"
            );
            envelope["svg"]
                .as_str()
                .unwrap_or_else(|| panic!("svg string for {url}: {envelope}"))
                .to_owned()
        })
        .collect();

    // Byte-identical SVGs prove the render is keyed off the project,
    // not the URI — opening `Kul: Show Preview` from any sibling file
    // shows the same unified diagram.
    for (i, svg) in svgs.iter().enumerate().skip(1) {
        assert_eq!(
            svg, &svgs[0],
            "SVG for {} diverged from SVG for {}",
            urls[i], urls[0]
        );
    }
    // Sanity-check the unified render really did pull in every file:
    // each person from each of the three files should appear by name.
    let unified = &svgs[0];
    for name in ["Ramesh", "Sita", "Alice", "Bob", "Carol"] {
        assert!(
            unified.contains(name),
            "unified SVG should contain {name}: {unified}"
        );
    }
}

#[test]
fn render_unknown_document_returns_invalid_params_error() {
    let mut handle = Handle::spawn();
    let kul_url = common::fixture_url(
        "render_unknown_document_returns_invalid_params_error",
        "never-opened.kul",
        "",
    );
    let uri = kul_url.as_str();
    handshake(&mut handle);
    let response = send_render(&mut handle, 300, uri);
    let error = &response["error"];
    assert!(!error.is_null(), "expected error response, got {response}");
    // -32602 is JSON-RPC's `Invalid Params`.
    assert_eq!(error["code"], -32602);
    assert!(
        error["message"].as_str().unwrap().contains("not open"),
        "error message should mention the document is not open: {error}"
    );
}
