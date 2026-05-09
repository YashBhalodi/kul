//! Integration test: open a document and verify
//! `textDocument/definition` returns the right declaration `Location`
//! for each reference site.

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

const FIXTURE: &str = "person alice name:\"A\" gender:female\n\
                        person bob name:\"B\" gender:male\n\
                        marriage m alice bob start:2010\n\
                        person kid name:\"K\" gender:other\n  birth m\n  adoption m start:2015\n";

fn open_fixture(handle: &mut Handle, uri: &str) {
    write_message(
        &mut handle.stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#,
    );
    let init = handle
        .recv_response(1, Instant::now() + Duration::from_secs(5))
        .expect("initialize");
    let caps = &init["result"]["capabilities"];
    assert_eq!(caps["definitionProvider"].as_bool(), Some(true));

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

fn definition_at(handle: &mut Handle, uri: &str, id: i64, line: u32, character: u32) -> Value {
    let req = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "textDocument/definition",
        "params": {
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        }
    });
    write_message(&mut handle.stdin, &req.to_string());
    handle
        .recv_response(id, Instant::now() + Duration::from_secs(5))
        .expect("definition response")
}

#[test]
fn definition_on_spouse_ref_jumps_to_person_decl() {
    let mut handle = Handle::spawn();
    let kul_url = common::fixture_url(
        "definition_on_spouse_ref_jumps_to_person_decl",
        "def.kul",
        FIXTURE,
    );
    let uri = kul_url.as_str();
    open_fixture(&mut handle, uri);
    // Line 2 is `marriage m alice bob ...`. `alice` starts at column 11.
    let resp = definition_at(&mut handle, uri, 10, 2, 11);
    let result = &resp["result"];
    assert!(!result.is_null(), "expected a Location, got null");
    assert_eq!(result["uri"].as_str(), Some(uri));
    // alice's decl is on line 0 starting at column 7.
    assert_eq!(result["range"]["start"]["line"].as_u64(), Some(0));
    assert_eq!(result["range"]["start"]["character"].as_u64(), Some(7));
}

#[test]
fn definition_on_birth_marriage_ref_jumps_to_marriage_decl() {
    let mut handle = Handle::spawn();
    let kul_url = common::fixture_url(
        "definition_on_birth_marriage_ref_jumps_to_marriage_decl",
        "def.kul",
        FIXTURE,
    );
    let uri = kul_url.as_str();
    open_fixture(&mut handle, uri);
    // Line 4 is `  birth m`. The `m` is at column 8.
    let resp = definition_at(&mut handle, uri, 11, 4, 8);
    let result = &resp["result"];
    assert!(!result.is_null());
    // Marriage `m` decl on line 2 column 9.
    assert_eq!(result["range"]["start"]["line"].as_u64(), Some(2));
    assert_eq!(result["range"]["start"]["character"].as_u64(), Some(9));
}

#[test]
fn definition_on_adoption_marriage_ref_jumps_to_marriage_decl() {
    let mut handle = Handle::spawn();
    let kul_url = common::fixture_url(
        "definition_on_adoption_marriage_ref_jumps_to_marriage_decl",
        "def.kul",
        FIXTURE,
    );
    let uri = kul_url.as_str();
    open_fixture(&mut handle, uri);
    // Line 5 is `  adoption m start:2015`. `m` at column 11.
    let resp = definition_at(&mut handle, uri, 12, 5, 11);
    let result = &resp["result"];
    assert!(!result.is_null());
    assert_eq!(result["range"]["start"]["line"].as_u64(), Some(2));
}

#[test]
fn definition_on_decl_id_returns_null() {
    let mut handle = Handle::spawn();
    let kul_url = common::fixture_url("definition_on_decl_id_returns_null", "def.kul", FIXTURE);
    let uri = kul_url.as_str();
    open_fixture(&mut handle, uri);
    // Line 0 column 7 = `alice` decl id.
    let resp = definition_at(&mut handle, uri, 13, 0, 7);
    assert!(resp["result"].is_null());
}

#[test]
fn definition_on_keyword_returns_null() {
    let mut handle = Handle::spawn();
    let kul_url = common::fixture_url("definition_on_keyword_returns_null", "def.kul", FIXTURE);
    let uri = kul_url.as_str();
    open_fixture(&mut handle, uri);
    // Line 0 column 0 = `person` keyword.
    let resp = definition_at(&mut handle, uri, 14, 0, 0);
    assert!(resp["result"].is_null());
}
