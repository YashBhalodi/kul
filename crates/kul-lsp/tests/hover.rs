//! Integration test for `textDocument/hover`.

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

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

    fn recv_response(&self, expected_id: i64, deadline: Instant) -> Option<Value> {
        loop {
            let now = Instant::now();
            if now >= deadline {
                return None;
            }
            let raw = self.rx.recv_timeout(deadline - now).ok()?;
            let parsed: Value = serde_json::from_str(&raw).ok()?;
            if parsed.get("id").and_then(Value::as_i64) == Some(expected_id) {
                return Some(parsed);
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

const FIXTURE: &str = "person alice name:\"Alice Doe\" gender:female born:1900-01-01\n\
                        person bob name:\"Bob Smith\" gender:male\n\
                        marriage m alice bob start:2010 end:2020 end_reason:divorce\n";

fn open_fixture(handle: &mut Handle, uri: &str) {
    write_message(
        &mut handle.stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#,
    );
    let init = handle
        .recv_response(1, Instant::now() + Duration::from_secs(5))
        .expect("initialize");
    let caps = &init["result"]["capabilities"];
    assert_eq!(caps["hoverProvider"].as_bool(), Some(true));

    write_message(
        &mut handle.stdin,
        r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#,
    );

    let escaped = serde_json::to_string(FIXTURE).unwrap();
    let did_open = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{uri}","languageId":"kul","version":1,"text":{escaped}}}}}}}"#
    );
    write_message(&mut handle.stdin, &did_open);
}

fn hover_at(handle: &mut Handle, uri: &str, id: i64, line: u32, character: u32) -> Value {
    let req = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "textDocument/hover",
        "params": {
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        }
    });
    write_message(&mut handle.stdin, &req.to_string());
    handle
        .recv_response(id, Instant::now() + Duration::from_secs(5))
        .expect("hover response")
}

#[test]
fn hover_on_person_decl_id() {
    let mut handle = Handle::spawn();
    let kul_url = common::fixture_url("hover_on_person_decl_id", "hov.kul", FIXTURE);
    let uri = kul_url.as_str();
    open_fixture(&mut handle, uri);
    let resp = hover_at(&mut handle, uri, 10, 0, 7);
    let body = resp["result"]["contents"]["value"]
        .as_str()
        .expect("hover markdown");
    assert!(body.contains("person alice"));
    assert!(body.contains("Alice Doe"));
    assert!(body.contains("female"));
    assert!(body.contains("1900-01-01"));
}

#[test]
fn hover_on_marriage_decl_id() {
    let mut handle = Handle::spawn();
    let kul_url = common::fixture_url("hover_on_marriage_decl_id", "hov.kul", FIXTURE);
    let uri = kul_url.as_str();
    open_fixture(&mut handle, uri);
    let resp = hover_at(&mut handle, uri, 11, 2, 9);
    let body = resp["result"]["contents"]["value"]
        .as_str()
        .expect("hover markdown");
    assert!(body.contains("marriage m"));
    assert!(body.contains("Alice Doe"));
    assert!(body.contains("Bob Smith"));
    assert!(body.contains("divorce"));
}

#[test]
fn hover_on_keyword() {
    let mut handle = Handle::spawn();
    let kul_url = common::fixture_url("hover_on_keyword", "hov.kul", FIXTURE);
    let uri = kul_url.as_str();
    open_fixture(&mut handle, uri);
    let resp = hover_at(&mut handle, uri, 12, 0, 0);
    let body = resp["result"]["contents"]["value"]
        .as_str()
        .expect("hover markdown");
    assert!(body.contains("`person`"));
    assert!(body.contains("Top-level statements") || body.contains("top-level"));
}

#[test]
fn hover_on_whitespace_returns_null() {
    let mut handle = Handle::spawn();
    let kul_url = common::fixture_url("hover_on_whitespace_returns_null", "hov.kul", FIXTURE);
    let uri = kul_url.as_str();
    open_fixture(&mut handle, uri);
    let resp = hover_at(&mut handle, uri, 13, 0, 200);
    let result = &resp["result"];
    assert!(
        result.is_null() || result.get("contents").is_none(),
        "expected null hover, got: {result}"
    );
}
