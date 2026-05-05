//! Integration test for `textDocument/formatting`. Spawns the server,
//! opens a document, requests formatting, and verifies the response shape.

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

fn open_doc(handle: &mut Handle, source: &str) {
    write_message(
        &mut handle.stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#,
    );
    let init = handle
        .recv_response(1, Instant::now() + Duration::from_secs(5))
        .expect("initialize");
    let caps = &init["result"]["capabilities"];
    let cap = &caps["documentFormattingProvider"];
    assert!(
        cap.is_object() || cap.as_bool() == Some(true),
        "missing documentFormattingProvider: {caps}",
    );

    write_message(
        &mut handle.stdin,
        r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#,
    );

    let escaped = serde_json::to_string(source).unwrap();
    let did_open = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"file:///fmt.kula","languageId":"kula","version":1,"text":{escaped}}}}}}}"#
    );
    write_message(&mut handle.stdin, &did_open);
}

fn formatting(handle: &mut Handle, id: i64) -> Value {
    let req = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "textDocument/formatting",
        "params": {
            "textDocument": { "uri": "file:///fmt.kula" },
            "options": { "tabSize": 2, "insertSpaces": true }
        }
    });
    write_message(&mut handle.stdin, &req.to_string());
    handle
        .recv_response(id, Instant::now() + Duration::from_secs(5))
        .expect("formatting response")
}

#[test]
fn formatting_returns_full_doc_replacement_on_dirty_input() {
    let mut handle = Handle::spawn();
    open_doc(
        &mut handle,
        "person alice born:1950 name:\"Alice\" gender:female\n",
    );
    let resp = formatting(&mut handle, 10);
    let edits = resp["result"].as_array().expect("array of edits");
    assert_eq!(edits.len(), 1);
    let edit = &edits[0];
    assert_eq!(edit["range"]["start"]["line"], 0);
    assert_eq!(edit["range"]["start"]["character"], 0);
    assert_eq!(
        edit["newText"].as_str().unwrap(),
        "person alice  name:\"Alice\"  gender:female  born:1950\n"
    );
}

#[test]
fn formatting_returns_empty_edit_list_when_canonical() {
    let mut handle = Handle::spawn();
    open_doc(
        &mut handle,
        "person alice  name:\"Alice\"  gender:female  born:1950\n",
    );
    let resp = formatting(&mut handle, 11);
    let edits = resp["result"].as_array().expect("array of edits");
    assert!(edits.is_empty());
}

#[test]
fn formatting_returns_null_when_input_has_parse_errors() {
    let mut handle = Handle::spawn();
    open_doc(&mut handle, "person\n");
    let resp = formatting(&mut handle, 12);
    assert!(
        resp["result"].is_null(),
        "expected null, got {}",
        resp["result"]
    );
}
