//! Integration test for `textDocument/documentSymbol`. Spawns the server,
//! opens a fixture, and verifies the outline tree shape and selection ranges.

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

fn binary_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_kula-lsp"))
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
        let mut child = cmd.spawn().expect("spawn kula-lsp");
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

const FIXTURE: &str = "person alice name:\"Alice\" gender:female born:1950\n\
                        person bob name:\"Bob\" gender:male born:1948\n\
                        marriage m alice bob start:1972 end:1990 end_reason:divorce\n\
                        person kid name:\"Kid\" gender:other\n  birth m\n";

fn open_fixture(handle: &mut Handle) {
    write_message(
        &mut handle.stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#,
    );
    let init = handle
        .recv_response(1, Instant::now() + Duration::from_secs(5))
        .expect("initialize");
    let caps = &init["result"]["capabilities"];
    assert_eq!(caps["documentSymbolProvider"].as_bool(), Some(true));

    write_message(
        &mut handle.stdin,
        r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#,
    );

    let escaped = serde_json::to_string(FIXTURE).unwrap();
    let did_open = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"file:///sym.kula","languageId":"kula","version":1,"text":{escaped}}}}}}}"#
    );
    write_message(&mut handle.stdin, &did_open);
}

fn document_symbol(handle: &mut Handle, id: i64) -> Value {
    let req = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "textDocument/documentSymbol",
        "params": {
            "textDocument": { "uri": "file:///sym.kula" }
        }
    });
    write_message(&mut handle.stdin, &req.to_string());
    handle
        .recv_response(id, Instant::now() + Duration::from_secs(5))
        .expect("documentSymbol response")
}

#[test]
fn outline_lists_persons_marriages_and_nests_birth() {
    let mut handle = Handle::spawn();
    open_fixture(&mut handle);

    let resp = document_symbol(&mut handle, 10);
    let result = resp["result"].as_array().expect("array of symbols");
    assert_eq!(result.len(), 4); // alice, bob, m, kid

    // Names: persons use display names; marriage uses spouse names.
    let names: Vec<&str> = result.iter().map(|s| s["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["Alice", "Bob", "Alice & Bob", "Kid"]);

    // The marriage's selection range points at its id `m` (line 2, col 9).
    let marriage = &result[2];
    assert_eq!(
        marriage["selectionRange"]["start"]["line"].as_u64(),
        Some(2)
    );
    assert_eq!(
        marriage["selectionRange"]["start"]["character"].as_u64(),
        Some(9),
    );

    // Kid has a `birth m` child symbol.
    let kid = &result[3];
    let children = kid["children"].as_array().expect("kid has children");
    assert_eq!(children.len(), 1);
    assert_eq!(children[0]["name"].as_str(), Some("birth m"));
}

#[test]
fn empty_document_returns_empty_array() {
    let mut handle = Handle::spawn();
    write_message(
        &mut handle.stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#,
    );
    handle
        .recv_response(1, Instant::now() + Duration::from_secs(5))
        .expect("initialize");
    write_message(
        &mut handle.stdin,
        r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#,
    );
    write_message(
        &mut handle.stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///empty.kula","languageId":"kula","version":1,"text":""}}}"#,
    );
    let req = json!({
        "jsonrpc": "2.0",
        "id": 11,
        "method": "textDocument/documentSymbol",
        "params": { "textDocument": { "uri": "file:///empty.kula" } }
    });
    write_message(&mut handle.stdin, &req.to_string());
    let resp = handle
        .recv_response(11, Instant::now() + Duration::from_secs(5))
        .expect("documentSymbol response");
    let result = resp["result"].as_array().expect("array");
    assert!(result.is_empty());
}
