//! Integration test for `textDocument/codeAction` quick-fixes.

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

fn open_doc(handle: &mut Handle, uri: &str, source: &str) {
    write_message(
        &mut handle.stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#,
    );
    let init = handle
        .recv_response(1, Instant::now() + Duration::from_secs(5))
        .expect("initialize");
    let caps = &init["result"]["capabilities"];
    let cap = &caps["codeActionProvider"];
    assert!(
        cap.is_object() || cap.as_bool() == Some(true),
        "missing codeActionProvider: {caps}",
    );

    write_message(
        &mut handle.stdin,
        r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#,
    );

    let escaped = serde_json::to_string(source).unwrap();
    let did_open = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{uri}","languageId":"kul","version":1,"text":{escaped}}}}}}}"#
    );
    write_message(&mut handle.stdin, &did_open);
}

fn code_action(
    handle: &mut Handle,
    uri: &str,
    id: i64,
    line_start: u32,
    char_start: u32,
    line_end: u32,
    char_end: u32,
) -> Value {
    let req = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "textDocument/codeAction",
        "params": {
            "textDocument": { "uri": uri },
            "range": {
                "start": { "line": line_start, "character": char_start },
                "end": { "line": line_end, "character": char_end }
            },
            "context": { "diagnostics": [] }
        }
    });
    write_message(&mut handle.stdin, &req.to_string());
    handle
        .recv_response(id, Instant::now() + Duration::from_secs(5))
        .expect("codeAction response")
}

#[test]
fn missing_gender_returns_three_quick_fixes() {
    let mut handle = Handle::spawn();
    let source = "person alice name:\"Alice\"\n";
    let kul_url = common::fixture_url("missing_gender_returns_three_quick_fixes", "ca.kul", source);
    let uri = kul_url.as_str();
    open_doc(&mut handle, uri, source);
    let resp = code_action(&mut handle, uri, 10, 0, 0, 0, 12);
    let actions = resp["result"].as_array().expect("array of actions");
    let titles: Vec<&str> = actions.iter().filter_map(|a| a["title"].as_str()).collect();
    assert!(titles.contains(&"Add `gender:male`"));
    assert!(titles.contains(&"Add `gender:female`"));
    assert!(titles.contains(&"Add `gender:other`"));
}

#[test]
fn end_without_end_reason_returns_add_divorce_fix() {
    let mut handle = Handle::spawn();
    let source = "person a name:\"A\" gender:female\n\
         person b name:\"B\" gender:male\n\
         marriage m a b start:1972 end:1990\n";
    let kul_url = common::fixture_url(
        "end_without_end_reason_returns_add_divorce_fix",
        "ca.kul",
        source,
    );
    let uri = kul_url.as_str();
    open_doc(&mut handle, uri, source);
    let resp = code_action(&mut handle, uri, 11, 2, 0, 2, 50);
    let actions = resp["result"].as_array().expect("array of actions");
    assert!(
        actions
            .iter()
            .any(|a| a["title"].as_str() == Some("Add `end_reason:divorce`")),
    );
}

#[test]
fn no_diagnostics_returns_null_or_empty() {
    let mut handle = Handle::spawn();
    let source = "person alice name:\"A\" gender:female\n\
         person bob name:\"B\" gender:male\n\
         marriage m alice bob start:1972\n";
    let kul_url = common::fixture_url("no_diagnostics_returns_null_or_empty", "ca.kul", source);
    let uri = kul_url.as_str();
    open_doc(&mut handle, uri, source);
    let resp = code_action(&mut handle, uri, 12, 0, 0, 2, 80);
    let result = &resp["result"];
    let empty = result.is_null() || result.as_array().is_some_and(|a| a.is_empty());
    assert!(empty, "expected no actions, got {result}");
}
