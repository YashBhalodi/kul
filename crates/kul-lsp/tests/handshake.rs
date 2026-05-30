//! End-to-end handshake + `didOpen` against the real `kul-lsp` binary.
//! Hand-rolled client so the test exercises the binary, not a library
//! impersonation of it.

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

fn binary_path() -> std::path::PathBuf {
    let path = env!("CARGO_BIN_EXE_kul-lsp");
    std::path::PathBuf::from(path)
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
        cmd.env("RUST_LOG", "kul_lsp=debug")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
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

    fn recv_until<F: Fn(&str) -> bool>(&self, deadline: Instant, predicate: F) -> Option<String> {
        loop {
            let now = Instant::now();
            if now >= deadline {
                return None;
            }
            match self.rx.recv_timeout(deadline - now) {
                Ok(msg) if predicate(&msg) => return Some(msg),
                Ok(_) => continue,
                Err(_) => return None,
            }
        }
    }

    fn drain_stderr(mut self) -> String {
        drop(self.stdin);
        let _ = self.child.wait();
        let mut out = String::new();
        if let Some(mut err) = self.child.stderr.take() {
            err.read_to_string(&mut out).expect("read stderr");
        }
        out
    }
}

fn initialize_request() -> String {
    r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#.to_owned()
}

fn initialized_notification() -> String {
    r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#.to_owned()
}

fn did_open_notification(uri: &str, text: &str) -> String {
    let escaped_text = serde_json::to_string(text).expect("escape text");
    format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{uri}","languageId":"kul","version":1,"text":{escaped_text}}}}}}}"#
    )
}

fn shutdown_request() -> String {
    r#"{"jsonrpc":"2.0","id":2,"method":"shutdown"}"#.to_owned()
}

fn exit_notification() -> String {
    r#"{"jsonrpc":"2.0","method":"exit"}"#.to_owned()
}

#[test]
fn handshake_and_did_open() {
    let mut handle = Handle::spawn();

    write_message(&mut handle.stdin, &initialize_request());
    let init_response = handle
        .recv(Duration::from_secs(5))
        .expect("initialize response");
    assert!(
        init_response.contains("\"id\":1"),
        "initialize response missing id: {init_response}"
    );
    assert!(
        init_response.contains("\"capabilities\""),
        "initialize response missing capabilities: {init_response}"
    );
    assert!(
        init_response.contains("kul-lsp"),
        "initialize response missing server info: {init_response}"
    );

    write_message(&mut handle.stdin, &initialized_notification());
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("handshake");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create fixture dir");
    std::fs::write(dir.join("kul.yml"), "kul: \"0.1\"\n").expect("write kul.yml");
    let kul_path = dir.join("alice.kul");
    let kul_url = tower_lsp::lsp_types::Url::from_file_path(&kul_path).expect("file URL");
    write_message(
        &mut handle.stdin,
        &did_open_notification(kul_url.as_str(), "person alice\n"),
    );

    // Drain the `window/logMessage` from `initialized` so the shutdown
    // response is unambiguous.
    let deadline = Instant::now() + Duration::from_secs(2);
    let _ = handle.recv_until(deadline, |msg| msg.contains("kul-lsp initialized"));

    write_message(&mut handle.stdin, &shutdown_request());
    // Drain through the `publishDiagnostics` notification triggered by
    // `did_open` to find the shutdown *response*.
    let shutdown_response = handle
        .recv_until(Instant::now() + Duration::from_secs(5), |msg| {
            msg.contains("\"id\":2")
        })
        .expect("shutdown response with id:2");
    assert!(
        shutdown_response.contains("\"result\""),
        "shutdown response missing result: {shutdown_response}"
    );

    write_message(&mut handle.stdin, &exit_notification());

    let stderr = handle.drain_stderr();
    assert!(
        stderr.contains("document opened"),
        "stderr missing 'document opened' log line:\n{stderr}"
    );
}
