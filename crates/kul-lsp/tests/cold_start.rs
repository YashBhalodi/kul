//! Sanity bench: how long does spawn → `initialize` response take?
//!
//! Treats this as a test (not a `cargo bench` benchmark) so it runs as part
//! of `just check`. Prints the measurement so a human can eyeball the
//! number; asserts a generous upper bound so it doesn't flake on slow CI.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn binary_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_kul-lsp"))
}

fn read_message<R: BufRead>(r: &mut R) -> Option<String> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        let n = r.read_line(&mut line).ok()?;
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
    r.read_exact(&mut body).ok()?;
    String::from_utf8(body).ok()
}

#[test]
fn cold_start_under_budget() {
    let request = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#;
    let payload = format!("Content-Length: {}\r\n\r\n{}", request.len(), request);

    let start = Instant::now();
    let mut child = Command::new(binary_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn kul-lsp");
    {
        let mut stdin = child.stdin.take().expect("stdin");
        stdin.write_all(payload.as_bytes()).expect("write");
        stdin.flush().expect("flush");
    }
    let mut stdout = BufReader::new(child.stdout.take().expect("stdout"));
    let response = read_message(&mut stdout).expect("initialize response");
    let elapsed = start.elapsed();

    assert!(
        response.contains("\"id\":1"),
        "expected initialize response, got: {response}"
    );

    eprintln!("cold-start (spawn → initialize response): {:?}", elapsed);

    // PRD target is 100ms on a developer machine; CI runners can be slower
    // and process spawn alone has wide variance. Asserting a 1s ceiling
    // catches catastrophic regressions without flaking on cold caches.
    assert!(
        elapsed < Duration::from_secs(1),
        "cold start exceeded 1s budget: {:?}",
        elapsed
    );

    // Cleanup. Don't care about the exit code.
    let _ = child.kill();
    let _ = child.wait();
}
