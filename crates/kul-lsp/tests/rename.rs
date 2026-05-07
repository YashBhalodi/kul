//! Integration test for `textDocument/prepareRename` and
//! `textDocument/rename`. Spawns the server and verifies the workspace
//! edit shape, plus the error-response paths.

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
                        marriage m alice bob start:1972\n";

fn open_fixture(handle: &mut Handle) {
    write_message(
        &mut handle.stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#,
    );
    let init = handle
        .recv_response(1, Instant::now() + Duration::from_secs(5))
        .expect("initialize");
    let caps = &init["result"]["capabilities"];
    let rename_cap = &caps["renameProvider"];
    assert!(
        rename_cap.is_object() || rename_cap.as_bool() == Some(true),
        "missing renameProvider capability: {caps}",
    );
    if let Some(prep) = rename_cap.get("prepareProvider") {
        assert_eq!(prep.as_bool(), Some(true));
    }

    write_message(
        &mut handle.stdin,
        r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#,
    );

    let escaped = serde_json::to_string(FIXTURE).unwrap();
    let did_open = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"file:///rn.kul","languageId":"kul","version":1,"text":{escaped}}}}}}}"#
    );
    write_message(&mut handle.stdin, &did_open);
}

fn prepare_rename_at(handle: &mut Handle, id: i64, line: u32, character: u32) -> Value {
    let req = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "textDocument/prepareRename",
        "params": {
            "textDocument": { "uri": "file:///rn.kul" },
            "position": { "line": line, "character": character }
        }
    });
    write_message(&mut handle.stdin, &req.to_string());
    handle
        .recv_response(id, Instant::now() + Duration::from_secs(5))
        .expect("prepareRename response")
}

fn rename_at(handle: &mut Handle, id: i64, line: u32, character: u32, new_name: &str) -> Value {
    let req = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "textDocument/rename",
        "params": {
            "textDocument": { "uri": "file:///rn.kul" },
            "position": { "line": line, "character": character },
            "newName": new_name
        }
    });
    write_message(&mut handle.stdin, &req.to_string());
    handle
        .recv_response(id, Instant::now() + Duration::from_secs(5))
        .expect("rename response")
}

#[test]
fn prepare_rename_on_decl_returns_range() {
    let mut handle = Handle::spawn();
    open_fixture(&mut handle);
    // Line 0 col 7 = `alice` decl id.
    let resp = prepare_rename_at(&mut handle, 10, 0, 7);
    let result = &resp["result"];
    assert!(!result.is_null());
    // Range covering "alice".
    assert_eq!(result["start"]["character"].as_u64(), Some(7));
    assert_eq!(result["end"]["character"].as_u64(), Some(12));
}

#[test]
fn prepare_rename_on_keyword_returns_null() {
    let mut handle = Handle::spawn();
    open_fixture(&mut handle);
    let resp = prepare_rename_at(&mut handle, 11, 0, 0);
    assert!(resp["result"].is_null());
}

#[test]
fn rename_returns_workspace_edit_covering_decl_and_refs() {
    let mut handle = Handle::spawn();
    open_fixture(&mut handle);
    let resp = rename_at(&mut handle, 12, 0, 7, "alicia");
    let we = &resp["result"];
    assert!(!we.is_null());
    let edits = we["changes"]["file:///rn.kul"]
        .as_array()
        .expect("array of edits");
    assert_eq!(edits.len(), 2);
    assert!(
        edits
            .iter()
            .all(|e| e["newText"].as_str() == Some("alicia"))
    );
}

#[test]
fn rename_to_reserved_keyword_returns_error() {
    let mut handle = Handle::spawn();
    open_fixture(&mut handle);
    let resp = rename_at(&mut handle, 13, 0, 7, "person");
    assert!(resp["result"].is_null());
    assert!(!resp["error"].is_null());
    let msg = resp["error"]["message"].as_str().unwrap();
    assert!(msg.contains("reserved keyword"), "msg was: {msg}");
}

#[test]
fn rename_to_collision_returns_error() {
    let mut handle = Handle::spawn();
    open_fixture(&mut handle);
    // alice → bob would collide with the existing person `bob`.
    let resp = rename_at(&mut handle, 14, 0, 7, "bob");
    assert!(resp["result"].is_null());
    assert!(!resp["error"].is_null());
}

#[test]
fn rename_to_invalid_id_returns_error() {
    let mut handle = Handle::spawn();
    open_fixture(&mut handle);
    let resp = rename_at(&mut handle, 15, 0, 7, "1bad");
    assert!(resp["result"].is_null());
    assert!(!resp["error"].is_null());
}
