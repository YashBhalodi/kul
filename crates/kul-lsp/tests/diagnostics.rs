//! Integration test: open a multi-error document and verify the
//! `textDocument/publishDiagnostics` notification matches what
//! `kul_core::check` produces directly.
//!
//! This is the proof that the LSP layer is a faithful adapter — same
//! diagnostics, same codes, same byte ranges (round-tripped through the
//! UTF-16 ↔ byte conversion).

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use kul_lsp::convert::LineIndex;
use serde_json::Value;
use tower_lsp::lsp_types::{Position, Url};

/// Set up an on-disk fixture directory with a `kul.yml` manifest. Returns
/// `(dir, kul_path, kul_url)`. `dir` is unique per test name to keep
/// concurrent runs isolated.
fn fixture_layout(
    name: &str,
    kul_basename: &str,
    kul_contents: &str,
) -> (std::path::PathBuf, std::path::PathBuf, Url) {
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create fixture dir");
    std::fs::write(dir.join("kul.yml"), "kul: \"0.1\"\n").expect("write kul.yml");
    let kul_path = dir.join(kul_basename);
    std::fs::write(&kul_path, kul_contents).expect("write fixture");
    let url = Url::from_file_path(&kul_path).expect("file URL for fixture");
    (dir, kul_path, url)
}

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

    fn recv_until<F: Fn(&Value) -> bool>(&self, deadline: Instant, predicate: F) -> Option<Value> {
        loop {
            let now = Instant::now();
            if now >= deadline {
                return None;
            }
            let raw = self.rx.recv_timeout(deadline - now).ok()?;
            let parsed: Value = serde_json::from_str(&raw).ok()?;
            if predicate(&parsed) {
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

const FIXTURE: &str = "person dup_a name:\"A\" gender:female
person dup_a name:\"A2\" gender:female
person bad_dates name:\"B\" gender:female born:2000 died:1950
person noname
marriage bad_self bad_dates bad_dates start:2010
marriage missing_start dup_a bad_dates
marriage end_no_reason dup_a bad_dates start:2000 end:2010
marriage reason_no_end dup_a bad_dates start:2000 end_reason:divorce
marriage bad_reason dup_a bad_dates start:2000 end:2010 end_reason:foo
marriage bad_order dup_a bad_dates start:2010 end:2000 end_reason:divorce
person ref_unknown name:\"R\" gender:male
  birth m_does_not_exist
";

#[test]
fn publish_diagnostics_match_kul_core() {
    let (_dir, _kul_path, kul_url) =
        fixture_layout("publish_diagnostics_match_kul_core", "fixture.kul", FIXTURE);

    let mut handle = Handle::spawn();

    write_message(
        &mut handle.stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#,
    );
    let _init = handle
        .recv(Duration::from_secs(5))
        .expect("initialize response");

    write_message(
        &mut handle.stdin,
        r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#,
    );

    let escaped = serde_json::to_string(FIXTURE).unwrap();
    let uri_str = kul_url.as_str();
    let did_open = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{uri_str}","languageId":"kul","version":1,"text":{escaped}}}}}}}"#
    );
    write_message(&mut handle.stdin, &did_open);

    let deadline = Instant::now() + Duration::from_secs(5);
    let publish = handle
        .recv_until(deadline, |v| {
            v.get("method")
                .and_then(Value::as_str)
                .is_some_and(|m| m == "textDocument/publishDiagnostics")
        })
        .expect("publishDiagnostics notification");

    let params = &publish["params"];
    assert_eq!(params["uri"].as_str().expect("uri"), uri_str);

    let lsp_diags = params["diagnostics"].as_array().expect("diagnostics array");
    let inputs = vec![kul_core::ast::InputFile::new("test.kul", FIXTURE)];
    let core_diags = kul_core::check_with_manifest(
        "kul.yml",
        "kul: \"0.1\"\n",
        &kul_core::manifest::Manifest::default(),
        &inputs,
    )
    .diagnostics;
    let line_index = LineIndex::new(FIXTURE);

    assert_eq!(
        lsp_diags.len(),
        core_diags.len(),
        "lsp diagnostic count diverged from kul_core::check"
    );

    for (lsp, core) in lsp_diags.iter().zip(core_diags.iter()) {
        let code = lsp["code"].as_str().expect("code");
        let message = lsp["message"].as_str().expect("message");
        let source = lsp["source"].as_str().expect("source");
        assert_eq!(code, core.code);
        assert_eq!(message, core.message.as_str());
        assert_eq!(source, "kul");

        let range = &lsp["range"];
        let start_line = range["start"]["line"].as_u64().expect("start.line") as u32;
        let start_char = range["start"]["character"]
            .as_u64()
            .expect("start.character") as u32;
        let end_line = range["end"]["line"].as_u64().expect("end.line") as u32;
        let end_char = range["end"]["character"].as_u64().expect("end.character") as u32;

        let start_byte = line_index
            .byte_offset(Position {
                line: start_line,
                character: start_char,
            })
            .expect("start byte");
        let end_byte = line_index
            .byte_offset(Position {
                line: end_line,
                character: end_char,
            })
            .expect("end byte");

        let primary = core.primary.expect("anchored diagnostic");
        assert_eq!(
            start_byte, primary.span.start,
            "primary start mismatch for {}",
            core.code
        );
        assert_eq!(
            end_byte, primary.span.end,
            "primary end mismatch for {}",
            core.code
        );
    }

    // Sanity: at least one rule should fire on this fixture, and we should
    // see codes from across the spec range — not just R03 over and over.
    let codes: std::collections::BTreeSet<&str> = lsp_diags
        .iter()
        .map(|d| d["code"].as_str().expect("code"))
        .collect();
    assert!(
        codes.len() >= 5,
        "fixture should fire at least 5 distinct rule codes, got: {codes:?}"
    );
}

#[test]
fn close_clears_diagnostics() {
    let (_dir, _kul_path, kul_url) =
        fixture_layout("close_clears_diagnostics", "c.kul", "person a\n");
    let uri_str = kul_url.as_str();

    let mut handle = Handle::spawn();

    write_message(
        &mut handle.stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#,
    );
    let _init = handle
        .recv(Duration::from_secs(5))
        .expect("initialize response");
    write_message(
        &mut handle.stdin,
        r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#,
    );

    let did_open = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{uri_str}","languageId":"kul","version":1,"text":"person a\n"}}}}}}"#
    );
    write_message(&mut handle.stdin, &did_open);

    // First publish should carry diagnostics (person `a` is missing name + gender).
    let publish = handle
        .recv_until(Instant::now() + Duration::from_secs(5), |v| {
            v.get("method")
                .and_then(Value::as_str)
                .is_some_and(|m| m == "textDocument/publishDiagnostics")
        })
        .expect("publishDiagnostics notification");
    let count = publish["params"]["diagnostics"]
        .as_array()
        .expect("diags")
        .len();
    assert!(count > 0, "expected diagnostics on a malformed person");

    let did_close = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didClose","params":{{"textDocument":{{"uri":"{uri_str}"}}}}}}"#
    );
    write_message(&mut handle.stdin, &did_close);

    let cleared = handle
        .recv_until(Instant::now() + Duration::from_secs(5), |v| {
            v.get("method")
                .and_then(Value::as_str)
                .is_some_and(|m| m == "textDocument/publishDiagnostics")
        })
        .expect("expected a clearing publishDiagnostics on close");
    let cleared_diags = cleared["params"]["diagnostics"]
        .as_array()
        .expect("cleared diags array");
    assert!(
        cleared_diags.is_empty(),
        "didClose should publish an empty diagnostic list"
    );
}
