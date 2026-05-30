//! Integration test for `textDocument/completion`.

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
    assert!(
        caps["completionProvider"].is_object(),
        "missing completionProvider capability: {caps}"
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

fn complete_at(handle: &mut Handle, uri: &str, id: i64, line: u32, character: u32) -> Vec<String> {
    let req = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "textDocument/completion",
        "params": {
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        }
    });
    write_message(&mut handle.stdin, &req.to_string());
    let resp = handle
        .recv_response(id, Instant::now() + Duration::from_secs(5))
        .expect("completion response");
    resp["result"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v["label"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn top_level_keywords_at_start() {
    let mut handle = Handle::spawn();
    let kul_url = common::fixture_url("top_level_keywords_at_start", "c.kul", "");
    let uri = kul_url.as_str();
    open_doc(&mut handle, uri, "");
    let labels = complete_at(&mut handle, uri, 10, 0, 0);
    assert!(labels.contains(&"person".to_owned()));
    assert!(labels.contains(&"marriage".to_owned()));
}

#[test]
fn person_field_list_filters_present() {
    let mut handle = Handle::spawn();
    let source = "person a name:\"A\" gender:female \n";
    let kul_url = common::fixture_url("person_field_list_filters_present", "c.kul", source);
    let uri = kul_url.as_str();
    open_doc(&mut handle, uri, source);
    let labels = complete_at(&mut handle, uri, 11, 0, 32);
    assert!(!labels.contains(&"name:".to_owned()));
    assert!(!labels.contains(&"gender:".to_owned()));
    assert!(labels.contains(&"family:".to_owned()));
    assert!(labels.contains(&"born:".to_owned()));
}

#[test]
fn after_gender_colon_returns_enum_values() {
    let mut handle = Handle::spawn();
    let source = "person a name:\"A\" gender:";
    let kul_url = common::fixture_url("after_gender_colon_returns_enum_values", "c.kul", source);
    let uri = kul_url.as_str();
    open_doc(&mut handle, uri, source);
    let labels = complete_at(&mut handle, uri, 12, 0, 25);
    assert_eq!(
        labels,
        vec!["male".to_owned(), "female".into(), "other".into()]
    );
}

#[test]
fn after_end_reason_colon_returns_divorce() {
    let mut handle = Handle::spawn();
    let source = "marriage m a b start:2010 end:2020 end_reason:";
    let kul_url = common::fixture_url("after_end_reason_colon_returns_divorce", "c.kul", source);
    let uri = kul_url.as_str();
    open_doc(&mut handle, uri, source);
    let labels = complete_at(&mut handle, uri, 13, 0, 46);
    assert_eq!(labels, vec!["divorce".to_owned()]);
}

#[test]
fn indented_under_person_returns_sub_keywords() {
    let mut handle = Handle::spawn();
    let source = "person a name:\"A\" gender:female\n  ";
    let kul_url = common::fixture_url(
        "indented_under_person_returns_sub_keywords",
        "c.kul",
        source,
    );
    let uri = kul_url.as_str();
    open_doc(&mut handle, uri, source);
    let labels = complete_at(&mut handle, uri, 14, 1, 2);
    assert!(labels.contains(&"birth".to_owned()));
    assert!(labels.contains(&"adoption".to_owned()));
}

#[test]
fn after_birth_keyword_returns_marriage_ids() {
    let mut handle = Handle::spawn();
    let source = "person alice name:\"Alice\" gender:female\n\
         person bob name:\"Bob\" gender:male\n\
         marriage m_alice_bob alice bob start:1972\n\
         person kid name:\"K\" gender:other\n  birth ";
    let kul_url = common::fixture_url("after_birth_keyword_returns_marriage_ids", "c.kul", source);
    let uri = kul_url.as_str();
    open_doc(&mut handle, uri, source);
    let labels = complete_at(&mut handle, uri, 15, 4, 8);
    assert_eq!(labels, vec!["m_alice_bob".to_owned()]);
}

#[test]
fn after_marriage_id_returns_persons_for_spouse_a() {
    let mut handle = Handle::spawn();
    let source = "person alice name:\"Alice\" gender:female\n\
         person bob name:\"Bob\" gender:male\n\
         marriage m ";
    let kul_url = common::fixture_url(
        "after_marriage_id_returns_persons_for_spouse_a",
        "c.kul",
        source,
    );
    let uri = kul_url.as_str();
    open_doc(&mut handle, uri, source);
    let labels = complete_at(&mut handle, uri, 16, 2, 11);
    assert!(labels.contains(&"alice".to_owned()));
    assert!(labels.contains(&"bob".to_owned()));
}

#[test]
fn after_spouse_a_excludes_self_marriage() {
    let mut handle = Handle::spawn();
    let source = "person alice name:\"Alice\" gender:female\n\
         person bob name:\"Bob\" gender:male\n\
         marriage m alice ";
    let kul_url = common::fixture_url("after_spouse_a_excludes_self_marriage", "c.kul", source);
    let uri = kul_url.as_str();
    open_doc(&mut handle, uri, source);
    let labels = complete_at(&mut handle, uri, 17, 2, 17);
    assert!(!labels.contains(&"alice".to_owned()));
    assert!(labels.contains(&"bob".to_owned()));
}
