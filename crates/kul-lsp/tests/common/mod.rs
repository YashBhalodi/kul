//! Shared test helpers: tempdir layout with `kul.yml` + `.kul` files
//! so `did_open`-time project discovery finds a manifest on disk.

use std::path::PathBuf;

use tower_lsp::lsp_types::Url;

/// Set up a unique on-disk fixture directory containing `kul.yml` plus
/// one `.kul` file. Returns the file URL the tests should send via LSP.
///
/// `name` should be unique per test (a function name is fine) so parallel
/// runs don't collide. The directory is reset on each call.
#[allow(dead_code)]
pub fn fixture_url(name: &str, kul_basename: &str, kul_contents: &str) -> Url {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create fixture dir");
    std::fs::write(dir.join("kul.yml"), "kul: \"0.1\"\n").expect("write kul.yml");
    let kul_path = dir.join(kul_basename);
    std::fs::write(&kul_path, kul_contents).expect("write fixture");
    Url::from_file_path(&kul_path).expect("file URL for fixture")
}

/// Set up a unique on-disk fixture *project* containing `kul.yml` plus
/// every `(basename, contents)` pair. Returns the project root path and
/// the URL of each `.kul` file in the same order as the input slice —
/// callers index them by name.
///
/// This is the multi-file counterpart to [`fixture_url`]: every file
/// lives in the same directory, sibling to one shared manifest, so the
/// LSP's project discovery (find `kul.yml`, enumerate `*.kul` siblings)
/// sees them as one project.
#[allow(dead_code)]
pub fn fixture_project(name: &str, files: &[(&str, &str)]) -> (PathBuf, Vec<Url>) {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create fixture dir");
    std::fs::write(dir.join("kul.yml"), "kul: \"0.1\"\n").expect("write kul.yml");
    let urls = files
        .iter()
        .map(|(basename, contents)| {
            let path = dir.join(basename);
            std::fs::write(&path, contents).expect("write fixture file");
            Url::from_file_path(&path).expect("file URL for fixture")
        })
        .collect();
    (dir, urls)
}
