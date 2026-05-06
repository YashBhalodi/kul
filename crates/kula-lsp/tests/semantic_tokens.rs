//! Integration test for `textDocument/semanticTokens/full`. Spawns the
//! server, opens a fixture, and verifies the legend in `initializeResult`
//! plus the encoded token stream returned by the request.

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

const FIXTURE: &str = "person alice name:\"Alice\" gender:female\n\
                        person bob name:\"Bob\" gender:male\n\
                        marriage m alice bob start:1972\n";

fn initialize(handle: &mut Handle) -> Value {
    write_message(
        &mut handle.stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#,
    );
    let init = handle
        .recv_response(1, Instant::now() + Duration::from_secs(5))
        .expect("initialize");
    write_message(
        &mut handle.stdin,
        r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#,
    );
    init
}

fn open(handle: &mut Handle, uri: &str, source: &str) {
    let escaped = serde_json::to_string(source).unwrap();
    let did_open = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{uri}","languageId":"kula","version":1,"text":{escaped}}}}}}}"#
    );
    write_message(&mut handle.stdin, &did_open);
}

fn semantic_tokens_full(handle: &mut Handle, id: i64, uri: &str) -> Value {
    let req = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "textDocument/semanticTokens/full",
        "params": { "textDocument": { "uri": uri } }
    });
    write_message(&mut handle.stdin, &req.to_string());
    handle
        .recv_response(id, Instant::now() + Duration::from_secs(5))
        .expect("semanticTokens/full response")
}

#[test]
fn initialize_advertises_semantic_tokens_legend() {
    let mut handle = Handle::spawn();
    let init = initialize(&mut handle);
    let provider = &init["result"]["capabilities"]["semanticTokensProvider"];
    let legend = &provider["legend"];
    let token_types: Vec<&str> = legend["tokenTypes"]
        .as_array()
        .expect("legend.tokenTypes is an array")
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    // Order is the protocol contract — the encoded `tokenType` indexes
    // straight into this list.
    assert_eq!(
        token_types,
        vec![
            "keyword",
            "property",
            "function",
            "parameter",
            "enum",
            "number",
            "string",
        ]
    );
    assert_eq!(provider["full"], json!(true));
}

#[test]
fn full_request_returns_encoded_token_stream() {
    let mut handle = Handle::spawn();
    initialize(&mut handle);
    open(&mut handle, "file:///tokens.kula", FIXTURE);

    let resp = semantic_tokens_full(&mut handle, 10, "file:///tokens.kula");
    let data = resp["result"]["data"]
        .as_array()
        .expect("data is an array of u32 5-tuples");
    // Three statements × at least 5 tokens each = >= 15 entries × 5 = 75 u32s.
    // The actual count is fixed but asserting a floor keeps the test robust
    // to additive token-stream changes (legend additions etc.).
    assert!(data.len() >= 75, "got {} u32s", data.len());
    assert_eq!(data.len() % 5, 0, "data length must be a multiple of 5");
    // The very first token is `person` on line 0, column 0 with type
    // `keyword` (index 0 in the legend).
    assert_eq!(data[0].as_u64(), Some(0), "delta_line");
    assert_eq!(data[1].as_u64(), Some(0), "delta_start");
    assert_eq!(data[2].as_u64(), Some(6), "length"); // "person"
    assert_eq!(data[3].as_u64(), Some(0), "tokenType=keyword");
    assert_eq!(data[4].as_u64(), Some(0), "modifiers");
}

#[test]
fn full_request_on_unopened_document_returns_null() {
    let mut handle = Handle::spawn();
    initialize(&mut handle);

    let resp = semantic_tokens_full(&mut handle, 11, "file:///nope.kula");
    assert!(
        resp["result"].is_null(),
        "expected null result for unopened document, got {:?}",
        resp["result"]
    );
}
