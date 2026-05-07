//! Integration test for `textDocument/references`. Spawns the server,
//! opens a fixture, and verifies reference locations come back for each
//! supported cursor position.

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

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
                        marriage m alice bob start:1972\n\
                        person kid name:\"K\" gender:other\n  birth m\n  adoption m start:2015\n";

fn open_fixture(handle: &mut Handle) {
    write_message(
        &mut handle.stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#,
    );
    let init = handle
        .recv_response(1, Instant::now() + Duration::from_secs(5))
        .expect("initialize");
    let caps = &init["result"]["capabilities"];
    assert_eq!(caps["referencesProvider"].as_bool(), Some(true));

    write_message(
        &mut handle.stdin,
        r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#,
    );

    let escaped = serde_json::to_string(FIXTURE).unwrap();
    let did_open = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"file:///r.kul","languageId":"kul","version":1,"text":{escaped}}}}}}}"#
    );
    write_message(&mut handle.stdin, &did_open);
}

fn references_at(
    handle: &mut Handle,
    id: i64,
    line: u32,
    character: u32,
    include_declaration: bool,
) -> Value {
    let req = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "textDocument/references",
        "params": {
            "textDocument": { "uri": "file:///r.kul" },
            "position": { "line": line, "character": character },
            "context": { "includeDeclaration": include_declaration }
        }
    });
    write_message(&mut handle.stdin, &req.to_string());
    handle
        .recv_response(id, Instant::now() + Duration::from_secs(5))
        .expect("references response")
}

#[test]
fn references_on_person_decl_returns_spouse_position() {
    let mut handle = Handle::spawn();
    open_fixture(&mut handle);
    // Line 0 col 7 = `alice` decl id.
    let resp = references_at(&mut handle, 10, 0, 7, false);
    let result = resp["result"].as_array().expect("array");
    assert_eq!(result.len(), 1);
    // Spouse position is on the marriage line (line 2).
    assert_eq!(result[0]["range"]["start"]["line"].as_u64(), Some(2));
}

#[test]
fn references_on_marriage_decl_returns_birth_and_adoption() {
    let mut handle = Handle::spawn();
    open_fixture(&mut handle);
    // Line 2 col 9 = `m` decl id ("marriage " is 9 chars).
    let resp = references_at(&mut handle, 11, 2, 9, false);
    let result = resp["result"].as_array().expect("array");
    assert_eq!(result.len(), 2);
}

#[test]
fn references_with_include_declaration_returns_decl_first() {
    let mut handle = Handle::spawn();
    open_fixture(&mut handle);
    let resp = references_at(&mut handle, 12, 0, 7, true);
    let result = resp["result"].as_array().expect("array");
    // `alice` has 1 ref + 1 decl = 2.
    assert_eq!(result.len(), 2);
    // Sorted by position: decl on line 0 comes first.
    assert_eq!(result[0]["range"]["start"]["line"].as_u64(), Some(0));
}

#[test]
fn references_on_keyword_returns_null() {
    let mut handle = Handle::spawn();
    open_fixture(&mut handle);
    // Line 0 col 0 = `person` keyword.
    let resp = references_at(&mut handle, 13, 0, 0, true);
    assert!(resp["result"].is_null());
}
