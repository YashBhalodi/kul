//! Shared test helpers for LSP integration tests.
//!
//! Each LSP integration test now needs an on-disk `kul.yml` next to the
//! `.kul` file under test, because the language server discovers the
//! project manifest via the file system at `did_open` time. This module
//! owns the tempdir layout so the test bodies stay focused.

use tower_lsp::lsp_types::Url;

/// Set up a unique on-disk fixture directory containing `kul.yml` plus
/// one `.kul` file. Returns the file URL the tests should send via LSP.
///
/// `name` should be unique per test (a function name is fine) so parallel
/// runs don't collide. The directory is reset on each call.
#[allow(dead_code)]
pub fn fixture_url(name: &str, kul_basename: &str, kul_contents: &str) -> Url {
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create fixture dir");
    std::fs::write(dir.join("kul.yml"), "kul: \"0.1\"\n").expect("write kul.yml");
    let kul_path = dir.join(kul_basename);
    std::fs::write(&kul_path, kul_contents).expect("write fixture");
    Url::from_file_path(&kul_path).expect("file URL for fixture")
}
