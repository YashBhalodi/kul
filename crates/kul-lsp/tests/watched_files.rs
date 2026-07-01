//! Integration tests for `workspace/didChangeWatchedFiles`: external
//! Create/Change/Delete of `.kul` files and `kul.yml`, including the
//! overlay-wins rule for files currently open in the editor.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;
use tower_lsp::lsp_types::Url;

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

    /// Drain notifications until `deadline`, collecting per-URI
    /// `publishDiagnostics` (last publish wins). Answers
    /// server-initiated requests (`client/registerCapability`) with an
    /// empty success so the server's `await` resolves.
    fn collect_publishes(&mut self, deadline: Instant) -> BTreeMap<String, Value> {
        let mut out: BTreeMap<String, Value> = BTreeMap::new();
        loop {
            let now = Instant::now();
            if now >= deadline {
                return out;
            }
            let Ok(raw) = self.rx.recv_timeout(deadline - now) else {
                return out;
            };
            let Ok(v): Result<Value, _> = serde_json::from_str(&raw) else {
                continue;
            };
            if let Some(id) = v.get("id").and_then(Value::as_i64)
                && v.get("method").is_some()
            {
                let resp = format!(r#"{{"jsonrpc":"2.0","id":{id},"result":null}}"#);
                write_message(&mut self.stdin, &resp);
                continue;
            }
            if v.get("method").and_then(Value::as_str) == Some("textDocument/publishDiagnostics")
                && let Some(uri) = v["params"]["uri"].as_str()
            {
                out.insert(uri.to_owned(), v);
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

fn handshake(handle: &mut Handle) {
    write_message(
        &mut handle.stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{"workspace":{"didChangeWatchedFiles":{"dynamicRegistration":true}}}}}"#,
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let now = Instant::now();
        assert!(now < deadline, "no initialize response");
        let Ok(raw) = handle.rx.recv_timeout(deadline - now) else {
            panic!("no initialize response (timeout)")
        };
        let Ok(v): Result<Value, _> = serde_json::from_str(&raw) else {
            continue;
        };
        if v.get("id").and_then(Value::as_i64) == Some(1) {
            break;
        }
    }
    write_message(
        &mut handle.stdin,
        r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#,
    );
}

fn did_open(handle: &mut Handle, uri: &str, source: &str) {
    let escaped = serde_json::to_string(source).unwrap();
    let msg = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{uri}","languageId":"kul","version":1,"text":{escaped}}}}}}}"#
    );
    write_message(&mut handle.stdin, &msg);
}

/// `kind` matches `FileChangeType`: 1 = Created, 2 = Changed, 3 = Deleted.
fn did_change_watched_file(handle: &mut Handle, uri: &str, kind: i32) {
    let msg = format!(
        r#"{{"jsonrpc":"2.0","method":"workspace/didChangeWatchedFiles","params":{{"changes":[{{"uri":"{uri}","type":{kind}}}]}}}}"#
    );
    write_message(&mut handle.stdin, &msg);
}

/// Diagnostic codes in the most recent publish for `uri`, sorted.
fn codes_in(publishes: &BTreeMap<String, Value>, uri: &str) -> Vec<String> {
    let Some(v) = publishes.get(uri) else {
        return Vec::new();
    };
    let mut codes: Vec<String> = v["params"]["diagnostics"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|d| d["code"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    codes.sort();
    codes
}

/// `true` when the most recent publish for `uri` carried zero diagnostics.
fn empty_publish_for(publishes: &BTreeMap<String, Value>, uri: &str) -> bool {
    let Some(v) = publishes.get(uri) else {
        return false;
    };
    let arr = v["params"]["diagnostics"].as_array();
    arr.map(|a| a.is_empty()).unwrap_or(false)
}

const SMITHS: &str = "person alice name:\"Alice\" gender:female\n\
                      person bob name:\"Bob\" gender:male\n\
                      marriage m_alice_bob alice bob start:1972\n";

const JONESES: &str = "person carol name:\"Carol\" gender:female\n\
                       person dave name:\"Dave\" gender:male\n\
                       marriage m_carol_dave carol dave start:1980\n";

/// JONESES plus a cross-file reference to `m_alice_bob` in smiths.kul.
const JONESES_WITH_XREF: &str = "person carol name:\"Carol\" gender:female\n\
                                 person dave name:\"Dave\" gender:male\n\
                                 marriage m_carol_dave carol dave start:1980\n\
                                 person eve name:\"Eve\" gender:female\n  birth m_alice_bob\n";

#[test]
fn created_event_picks_up_new_kul_file() {
    let (dir, urls) = common::fixture_project(
        "watched_created_picks_up_new_file",
        &[("smiths.kul", SMITHS)],
    );
    let smiths_uri = urls[0].as_str().to_owned();

    let mut handle = Handle::spawn();
    handshake(&mut handle);
    did_open(&mut handle, &smiths_uri, SMITHS);
    // Drain the initial broadcast so the next collect only sees what
    // the watcher event triggered.
    let _ = handle.collect_publishes(Instant::now() + Duration::from_millis(400));

    let joneses_path = dir.join("joneses.kul");
    std::fs::write(&joneses_path, JONESES_WITH_XREF).expect("write joneses.kul");
    let joneses_uri = Url::from_file_path(&joneses_path)
        .expect("file URL")
        .to_string();

    did_change_watched_file(&mut handle, &joneses_uri, 1);

    let publishes = handle.collect_publishes(Instant::now() + Duration::from_millis(800));
    let joneses_codes = codes_in(&publishes, &joneses_uri);
    let smiths_codes = codes_in(&publishes, &smiths_uri);
    assert!(
        publishes.contains_key(&joneses_uri),
        "joneses.kul should receive a publish after Created; got: {:?}",
        publishes.keys().collect::<Vec<_>>()
    );
    assert!(
        joneses_codes.is_empty(),
        "joneses.kul references m_alice_bob (in smiths.kul) — project-wide check should be clean, got: {joneses_codes:?}",
    );
    assert!(
        smiths_codes.is_empty(),
        "smiths.kul stays clean, got: {smiths_codes:?}",
    );
}

#[test]
fn changed_event_reloads_closed_in_editor_file() {
    // External edit of a sibling without an overlay must be picked up.
    let (dir, urls) = common::fixture_project(
        "watched_changed_reloads_closed_file",
        &[("smiths.kul", SMITHS), ("joneses.kul", JONESES)],
    );
    let smiths_uri = urls[0].as_str().to_owned();
    let joneses_uri = urls[1].as_str().to_owned();

    let mut handle = Handle::spawn();
    handshake(&mut handle);
    did_open(&mut handle, &smiths_uri, SMITHS);
    let _ = handle.collect_publishes(Instant::now() + Duration::from_millis(400));

    let joneses_with_error = "person eve name:\"Eve\" gender:female\n  birth m_missing\n";
    write_file(dir.join("joneses.kul"), joneses_with_error);
    did_change_watched_file(&mut handle, &joneses_uri, 2);

    let publishes = handle.collect_publishes(Instant::now() + Duration::from_millis(800));
    let joneses_codes = codes_in(&publishes, &joneses_uri);
    assert!(
        joneses_codes.iter().any(|c| c == "KUL-R02"),
        "joneses.kul should report KUL-R02 after external edit; got: {joneses_codes:?}",
    );
}

#[test]
fn changed_event_on_overlaid_file_is_ignored() {
    // Overlay buffer is authoritative: external `Changed` against a
    // file with an open buffer must be ignored.
    let (dir, urls) = common::fixture_project(
        "watched_changed_overlay_wins",
        &[("smiths.kul", SMITHS), ("joneses.kul", JONESES)],
    );
    let smiths_uri = urls[0].as_str().to_owned();
    let joneses_uri = urls[1].as_str().to_owned();

    let mut handle = Handle::spawn();
    handshake(&mut handle);
    did_open(&mut handle, &smiths_uri, SMITHS);
    did_open(&mut handle, &joneses_uri, JONESES);
    let _ = handle.collect_publishes(Instant::now() + Duration::from_millis(400));

    let garbage = "person eve name:\"Eve\" gender:female\n  birth m_does_not_exist\n";
    write_file(dir.join("smiths.kul"), garbage);
    did_change_watched_file(&mut handle, &smiths_uri, 2);

    let publishes = handle.collect_publishes(Instant::now() + Duration::from_millis(800));
    let smiths_codes = codes_in(&publishes, &smiths_uri);
    // Failure mode to rule out: an R02 from the corrupt disk source.
    assert!(
        smiths_codes.iter().all(|c| c != "KUL-R02"),
        "overlay buffer should win — disk corruption must not surface; got: {smiths_codes:?}",
    );
}

#[test]
fn deleted_event_drops_kul_file_and_empty_publishes_uri() {
    let (dir, urls) = common::fixture_project(
        "watched_deleted_kul_file",
        &[("smiths.kul", SMITHS), ("joneses.kul", JONESES_WITH_XREF)],
    );
    let smiths_uri = urls[0].as_str().to_owned();
    let joneses_uri = urls[1].as_str().to_owned();

    let mut handle = Handle::spawn();
    handshake(&mut handle);
    did_open(&mut handle, &joneses_uri, JONESES_WITH_XREF);
    let _ = handle.collect_publishes(Instant::now() + Duration::from_millis(400));

    std::fs::remove_file(dir.join("smiths.kul")).expect("remove smiths.kul");
    did_change_watched_file(&mut handle, &smiths_uri, 3);

    let publishes = handle.collect_publishes(Instant::now() + Duration::from_millis(800));
    assert!(
        empty_publish_for(&publishes, &smiths_uri),
        "smiths.kul URI should get a clearing publish after deletion; got: {:?}",
        publishes.get(&smiths_uri),
    );
    let joneses_codes = codes_in(&publishes, &joneses_uri);
    assert!(
        joneses_codes.iter().any(|c| c == "KUL-R02"),
        "joneses.kul should now report KUL-R02 — m_alice_bob's declaration was deleted; got: {joneses_codes:?}",
    );
}

#[test]
fn deleted_event_keeps_open_buffer_and_its_dependents() {
    // Issue #245: an on-disk DELETE of a file that is open with an editor
    // buffer must not evict it. Atomic-save editors and git checkout/stash
    // delete-then-recreate under an open buffer; the buffer stays
    // authoritative and its cross-file dependents keep resolving.
    let (dir, urls) = common::fixture_project(
        "watched_deleted_open_buffer_survives",
        &[("smiths.kul", SMITHS), ("joneses.kul", JONESES_WITH_XREF)],
    );
    let smiths_uri = urls[0].as_str().to_owned();
    let joneses_uri = urls[1].as_str().to_owned();

    let mut handle = Handle::spawn();
    handshake(&mut handle);
    // smiths.kul is the open buffer; it declares m_alice_bob, which the
    // disk-only joneses.kul references cross-file.
    did_open(&mut handle, &smiths_uri, SMITHS);
    let _ = handle.collect_publishes(Instant::now() + Duration::from_millis(400));

    // Delete smiths.kul on disk while its buffer is open, then recreate it
    // on disk with a *broken* body — mimicking an atomic save that lands
    // corrupt content the editor buffer has already superseded.
    std::fs::remove_file(dir.join("smiths.kul")).expect("remove smiths.kul");
    did_change_watched_file(&mut handle, &smiths_uri, 3);
    let garbage = "person eve name:\"Eve\" gender:female\n  birth m_does_not_exist\n";
    write_file(dir.join("smiths.kul"), garbage);
    did_change_watched_file(&mut handle, &smiths_uri, 1);

    let publishes = handle.collect_publishes(Instant::now() + Duration::from_millis(800));
    let smiths_codes = codes_in(&publishes, &smiths_uri);
    let joneses_codes = codes_in(&publishes, &joneses_uri);
    // Buffer wins over the corrupt disk body: smiths stays clean.
    assert!(
        smiths_codes.iter().all(|c| c != "KUL-R02"),
        "open buffer must survive the delete and win over corrupt disk; got: {smiths_codes:?}",
    );
    // The cross-file dependent still resolves m_alice_bob from the buffer.
    assert!(
        joneses_codes.iter().all(|c| c != "KUL-R02"),
        "joneses.kul must keep resolving m_alice_bob from the surviving buffer; got: {joneses_codes:?}",
    );
}

#[test]
fn deleted_event_for_kul_yml_evicts_project_and_clears_all_uris() {
    let (dir, urls) = common::fixture_project(
        "watched_deleted_kul_yml",
        &[("smiths.kul", SMITHS), ("joneses.kul", JONESES_WITH_XREF)],
    );
    let smiths_uri = urls[0].as_str().to_owned();
    let joneses_uri = urls[1].as_str().to_owned();

    let mut handle = Handle::spawn();
    handshake(&mut handle);
    did_open(&mut handle, &joneses_uri, JONESES_WITH_XREF);
    let _ = handle.collect_publishes(Instant::now() + Duration::from_millis(400));

    let manifest_path = dir.join("kul.yml");
    std::fs::remove_file(&manifest_path).expect("remove kul.yml");
    let manifest_uri = Url::from_file_path(&manifest_path)
        .expect("file URL")
        .to_string();
    did_change_watched_file(&mut handle, &manifest_uri, 3);

    let publishes = handle.collect_publishes(Instant::now() + Duration::from_millis(800));
    assert!(
        empty_publish_for(&publishes, &smiths_uri),
        "smiths.kul should get a clearing publish on manifest deletion; got: {:?}",
        publishes.get(&smiths_uri),
    );
    assert!(
        empty_publish_for(&publishes, &joneses_uri),
        "joneses.kul should get a clearing publish on manifest deletion; got: {:?}",
        publishes.get(&joneses_uri),
    );
}

fn write_file(path: impl AsRef<Path>, contents: &str) {
    std::fs::write(path, contents).expect("write file");
}
