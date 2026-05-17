//! Integration tests for the project-keyed cache and cross-file features
//! introduced in issue #85.
//!
//! These tests drive the real `kul-lsp` binary via the same stdio LSP
//! client every other integration test in this crate uses. They cover:
//!
//! - Two `did_open`s in the same project share one cached check.
//! - `did_change` on one URI only updates that URI's overlay; the
//!   sibling reads disk-backed source.
//! - `did_close` on the last URI evicts the project.
//! - Cross-file goto-definition jumps to a sibling file's declaration.
//! - Cross-file find-references surfaces uses in sibling files.
//! - Cross-file rename produces a workspace edit keyed by every
//!   affected URL.
//! - Diagnostics broadcast: an R02 in a sibling file is published
//!   under the sibling's URI even when only the other file is open.
//! - Two unrelated projects in the same workspace do not leak
//!   diagnostics across project boundaries.

use std::collections::BTreeMap;
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

    /// Drain notifications until `deadline`, collecting per-URI
    /// `publishDiagnostics` payloads. The last publish for a given URI
    /// wins (later publishes overwrite earlier ones — that's the LSP
    /// semantics).
    fn collect_publishes(&self, deadline: Instant) -> BTreeMap<String, Value> {
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
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#,
    );
    let _ = handle
        .recv_response(1, Instant::now() + Duration::from_secs(5))
        .expect("initialize response");
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

fn did_change(handle: &mut Handle, uri: &str, source: &str, version: i32) {
    let escaped = serde_json::to_string(source).unwrap();
    let msg = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"textDocument":{{"uri":"{uri}","version":{version}}},"contentChanges":[{{"text":{escaped}}}]}}}}"#
    );
    write_message(&mut handle.stdin, &msg);
}

fn did_close(handle: &mut Handle, uri: &str) {
    let msg = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didClose","params":{{"textDocument":{{"uri":"{uri}"}}}}}}"#
    );
    write_message(&mut handle.stdin, &msg);
}

/// Diagnostic codes the LSP published for `uri` in the most recent
/// publish, as a sorted vec.
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

const SMITHS: &str = "person alice name:\"Alice\" gender:female\n\
                      person bob name:\"Bob\" gender:male\n\
                      marriage m_alice_bob alice bob start:1972\n";

const JONESES: &str = "person carol name:\"Carol\" gender:female\n\
                       person dave name:\"Dave\" gender:male\n\
                       marriage m_carol_dave carol dave start:1980\n\
                       person eve name:\"Eve\" gender:female\n  birth m_alice_bob\n";

#[test]
fn cross_file_definition_jumps_to_sibling_file() {
    let (_, urls) = common::fixture_project(
        "multi_file_cross_file_definition",
        &[("smiths.kul", SMITHS), ("joneses.kul", JONESES)],
    );
    let smiths_uri = urls[0].as_str();
    let joneses_uri = urls[1].as_str();

    let mut handle = Handle::spawn();
    handshake(&mut handle);
    did_open(&mut handle, joneses_uri, JONESES);

    // The `birth m_alice_bob` line in joneses.kul is line 4 (0-indexed).
    // `m_alice_bob` starts at column 8 (after "  birth ").
    let req = json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "textDocument/definition",
        "params": {
            "textDocument": { "uri": joneses_uri },
            "position": { "line": 4, "character": 8 }
        }
    });
    write_message(&mut handle.stdin, &req.to_string());
    let resp = handle
        .recv_response(10, Instant::now() + Duration::from_secs(5))
        .expect("definition response");
    let result = &resp["result"];
    assert!(!result.is_null(), "expected a Location, got null");
    assert_eq!(
        result["uri"].as_str(),
        Some(smiths_uri),
        "definition should resolve to the sibling smiths.kul file"
    );
    // `marriage m_alice_bob` is on line 2 of smiths.kul, with the id
    // starting at column "marriage ".len() = 9.
    assert_eq!(result["range"]["start"]["line"].as_u64(), Some(2));
    assert_eq!(result["range"]["start"]["character"].as_u64(), Some(9));
}

#[test]
fn cross_file_references_surface_uses_in_sibling_file() {
    let (_, urls) = common::fixture_project(
        "multi_file_cross_file_references",
        &[("smiths.kul", SMITHS), ("joneses.kul", JONESES)],
    );
    let smiths_uri = urls[0].as_str();
    let joneses_uri = urls[1].as_str();

    let mut handle = Handle::spawn();
    handshake(&mut handle);
    did_open(&mut handle, smiths_uri, SMITHS);

    // Cursor on `m_alice_bob` decl in smiths.kul (line 2, col 9).
    let req = json!({
        "jsonrpc": "2.0",
        "id": 11,
        "method": "textDocument/references",
        "params": {
            "textDocument": { "uri": smiths_uri },
            "position": { "line": 2, "character": 9 },
            "context": { "includeDeclaration": false }
        }
    });
    write_message(&mut handle.stdin, &req.to_string());
    let resp = handle
        .recv_response(11, Instant::now() + Duration::from_secs(5))
        .expect("references response");
    let locs = resp["result"].as_array().expect("locations array");
    assert_eq!(locs.len(), 1, "expected one cross-file reference");
    assert_eq!(
        locs[0]["uri"].as_str(),
        Some(joneses_uri),
        "the reference should be the `birth m_alice_bob` in joneses.kul"
    );
    assert_eq!(locs[0]["range"]["start"]["line"].as_u64(), Some(4));
}

#[test]
fn cross_file_rename_produces_workspace_edit_for_every_affected_file() {
    let (_, urls) = common::fixture_project(
        "multi_file_cross_file_rename",
        &[("smiths.kul", SMITHS), ("joneses.kul", JONESES)],
    );
    let smiths_uri = urls[0].as_str();
    let joneses_uri = urls[1].as_str();

    let mut handle = Handle::spawn();
    handshake(&mut handle);
    did_open(&mut handle, smiths_uri, SMITHS);

    // Rename `m_alice_bob` (line 2, col 9 in smiths.kul) → `m_smiths`.
    let req = json!({
        "jsonrpc": "2.0",
        "id": 12,
        "method": "textDocument/rename",
        "params": {
            "textDocument": { "uri": smiths_uri },
            "position": { "line": 2, "character": 9 },
            "newName": "m_smiths"
        }
    });
    write_message(&mut handle.stdin, &req.to_string());
    let resp = handle
        .recv_response(12, Instant::now() + Duration::from_secs(5))
        .expect("rename response");
    let changes = &resp["result"]["changes"];
    assert!(changes.is_object(), "expected `changes` map");
    assert!(
        changes.get(smiths_uri).is_some(),
        "smiths.kul should appear in the workspace edit"
    );
    assert!(
        changes.get(joneses_uri).is_some(),
        "joneses.kul should appear in the workspace edit (cross-file rename)"
    );
    let smiths_edits = changes[smiths_uri].as_array().unwrap();
    let joneses_edits = changes[joneses_uri].as_array().unwrap();
    assert_eq!(smiths_edits.len(), 1, "decl edit in smiths.kul");
    assert_eq!(joneses_edits.len(), 1, "ref edit in joneses.kul");
    assert_eq!(smiths_edits[0]["newText"].as_str(), Some("m_smiths"));
    assert_eq!(joneses_edits[0]["newText"].as_str(), Some("m_smiths"));
}

#[test]
fn did_open_publishes_diagnostics_for_every_project_file() {
    // joneses.kul references `m_alice_bob` (defined in smiths.kul), so
    // the project-wide check should report no R02. Both files publish
    // empty diagnostic lists. The point: when the user opens only
    // smiths.kul, joneses.kul still gets a publish so the Problems pane
    // shows project-wide health.
    let (_, urls) = common::fixture_project(
        "multi_file_broadcasts_diagnostics",
        &[("smiths.kul", SMITHS), ("joneses.kul", JONESES)],
    );
    let smiths_uri = urls[0].as_str();
    let joneses_uri = urls[1].as_str();

    let mut handle = Handle::spawn();
    handshake(&mut handle);
    did_open(&mut handle, smiths_uri, SMITHS);

    // Collect publishes for a moment.
    let publishes = handle.collect_publishes(Instant::now() + Duration::from_millis(800));
    assert!(
        publishes.contains_key(smiths_uri),
        "smiths.kul publish missing; got publishes for {:?}",
        publishes.keys().collect::<Vec<_>>()
    );
    assert!(
        publishes.contains_key(joneses_uri),
        "joneses.kul publish missing — broadcast should reach sibling files. got: {:?}",
        publishes.keys().collect::<Vec<_>>()
    );
}

#[test]
fn diagnostics_in_sibling_file_surface_under_sibling_uri() {
    // Variant of the broadcast test that *introduces* an error in the
    // unopened file. smiths.kul is unchanged; we redefine joneses.kul
    // with a `birth m_missing` referencing an id no file declares — an
    // R02. The user only opens smiths.kul; the publish for joneses.kul
    // should still carry the R02 diagnostic.
    let joneses_with_error = "person eve name:\"Eve\" gender:female\n  birth m_missing\n";
    let (_, urls) = common::fixture_project(
        "multi_file_diag_in_unopened_file",
        &[("smiths.kul", SMITHS), ("joneses.kul", joneses_with_error)],
    );
    let smiths_uri = urls[0].as_str();
    let joneses_uri = urls[1].as_str();

    let mut handle = Handle::spawn();
    handshake(&mut handle);
    did_open(&mut handle, smiths_uri, SMITHS);

    let publishes = handle.collect_publishes(Instant::now() + Duration::from_millis(800));
    let smiths_codes = codes_in(&publishes, smiths_uri);
    let joneses_codes = codes_in(&publishes, joneses_uri);
    assert!(
        smiths_codes.is_empty(),
        "smiths.kul has no errors, but got: {smiths_codes:?}",
    );
    assert!(
        joneses_codes.iter().any(|c| c == "KUL-R02"),
        "joneses.kul should report KUL-R02 for `m_missing`, got: {joneses_codes:?}",
    );
}

#[test]
fn two_open_uris_in_one_project_share_one_check() {
    // Opening two files of the same project should produce diagnostics
    // that reflect the project's joint state — not two independent
    // single-file checks. If the cache treated each URI separately,
    // joneses.kul would fire R02 for `m_alice_bob` (no decl in its own
    // file). With one project-wide check, R02 is silent.
    let (_, urls) = common::fixture_project(
        "multi_file_shared_check",
        &[("smiths.kul", SMITHS), ("joneses.kul", JONESES)],
    );
    let smiths_uri = urls[0].as_str();
    let joneses_uri = urls[1].as_str();

    let mut handle = Handle::spawn();
    handshake(&mut handle);
    did_open(&mut handle, joneses_uri, JONESES);
    did_open(&mut handle, smiths_uri, SMITHS);

    let publishes = handle.collect_publishes(Instant::now() + Duration::from_millis(800));
    let joneses_codes = codes_in(&publishes, joneses_uri);
    assert!(
        !joneses_codes.iter().any(|c| c == "KUL-R02"),
        "the cross-file reference `m_alice_bob` should resolve once smiths.kul is open in the same project; got: {joneses_codes:?}",
    );
}

#[test]
fn did_change_on_one_file_does_not_corrupt_sibling_source() {
    // joneses.kul references `m_alice_bob`. We open it (project loads
    // smiths.kul from disk), then send a `did_change` for joneses.kul
    // that drops the reference. The expectation: smiths.kul's
    // declarations stay visible (we read its source from disk, not
    // from the joneses.kul overlay).
    let (_, urls) = common::fixture_project(
        "multi_file_overlay_isolation",
        &[("smiths.kul", SMITHS), ("joneses.kul", JONESES)],
    );
    let smiths_uri = urls[0].as_str();
    let joneses_uri = urls[1].as_str();

    let mut handle = Handle::spawn();
    handshake(&mut handle);
    did_open(&mut handle, joneses_uri, JONESES);
    did_change(
        &mut handle,
        joneses_uri,
        "person eve name:\"Eve\" gender:female\n",
        2,
    );

    let publishes = handle.collect_publishes(Instant::now() + Duration::from_millis(800));
    // smiths.kul never opened in editor but its diagnostics should
    // publish — and it should be clean.
    let smiths_codes = codes_in(&publishes, smiths_uri);
    assert!(
        smiths_codes.is_empty(),
        "smiths.kul should stay clean after a did_change on joneses.kul: got {smiths_codes:?}",
    );
}

#[test]
fn close_of_last_uri_evicts_project_and_clears_diagnostics() {
    let (_, urls) = common::fixture_project(
        "multi_file_close_evicts",
        &[("smiths.kul", SMITHS), ("joneses.kul", JONESES)],
    );
    let smiths_uri = urls[0].as_str();
    let joneses_uri = urls[1].as_str();

    let mut handle = Handle::spawn();
    handshake(&mut handle);
    did_open(&mut handle, smiths_uri, SMITHS);
    // Wait for the initial broadcast.
    let _ = handle.collect_publishes(Instant::now() + Duration::from_millis(400));

    did_close(&mut handle, smiths_uri);
    let publishes = handle.collect_publishes(Instant::now() + Duration::from_millis(800));

    let smiths_codes = codes_in(&publishes, smiths_uri);
    let joneses_codes = codes_in(&publishes, joneses_uri);
    assert!(
        smiths_codes.is_empty(),
        "smiths.kul should receive a clearing publish on close, got: {smiths_codes:?}",
    );
    assert!(
        joneses_codes.is_empty(),
        "joneses.kul (sibling project file) should also receive a clearing publish so its squiggles don't go stale: got {joneses_codes:?}",
    );
}

#[test]
fn two_projects_in_one_workspace_do_not_leak_diagnostics() {
    let (_, urls_a) = common::fixture_project(
        "multi_file_two_projects_a",
        &[("only.kul", "person alice name:\"A\" gender:female\n")],
    );
    let (_, urls_b) = common::fixture_project(
        "multi_file_two_projects_b",
        &[(
            "only.kul",
            "person bob name:\"B\" gender:male\n  birth m_does_not_exist\n",
        )],
    );
    let uri_a = urls_a[0].as_str();
    let uri_b = urls_b[0].as_str();

    let mut handle = Handle::spawn();
    handshake(&mut handle);
    did_open(
        &mut handle,
        uri_a,
        "person alice name:\"A\" gender:female\n",
    );
    did_open(
        &mut handle,
        uri_b,
        "person bob name:\"B\" gender:male\n  birth m_does_not_exist\n",
    );

    let publishes = handle.collect_publishes(Instant::now() + Duration::from_millis(800));
    let codes_a = codes_in(&publishes, uri_a);
    let codes_b = codes_in(&publishes, uri_b);
    assert!(
        codes_a.is_empty(),
        "project A is clean; should not see any diagnostics, got: {codes_a:?}",
    );
    assert!(
        codes_b.iter().any(|c| c == "KUL-R02"),
        "project B should still see its own R02 (the broken birth ref), got: {codes_b:?}",
    );
}
