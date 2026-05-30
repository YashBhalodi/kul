//! Project loader for the Kul toolchain.
//!
//! A Kul *project* is a directory holding a `kul.yml` manifest plus
//! sibling `*.kul` files (ADR-0013, ADR-0015). `kul-core` forbids
//! filesystem IO, so this crate is the shared seam between the CLI and
//! the LSP. It exposes two postures over one discovery rule (flat
//! directory, `*.kul` only, subdirectories ignored, lexicographic
//! order):
//!
//! - [`load`] — strict. Returns a typed [`ProjectLoadError`] on any
//!   filesystem failure. Used by the CLI.
//! - [`discover`] — lenient. Never fails: missing/unreadable manifest
//!   collapses to empty YAML, unreadable entries are skipped. Used by
//!   the LSP so an editor session survives broken on-disk state.
//!
//! The loader only reads bytes; manifest YAML parsing, `.kul` syntax
//! checking, and `KUL-Mxx` diagnostics (including `KUL-M06` for empty
//! projects) all live in `kul_core::check`.

use std::path::{Path, PathBuf};

use kul_core::ast::InputFile;
use thiserror::Error;

fn manifest_path(root: &Path) -> PathBuf {
    root.join("kul.yml")
}

/// Bare names keep diagnostic output stable regardless of where the
/// project lives on disk.
fn bare_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

/// Enumerate the `.kul` files in `root`, in lexicographic order, with
/// subdirectories and non-`.kul` files filtered out.
fn enumerate_kul_paths(root: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        // Check file type explicitly rather than `path.is_file()`: the
        // latter conflates directory with broken-symlink / permission
        // denied, but we want directories silently skipped while
        // unreadable `.kul` entries surface to the caller.
        if let Ok(ft) = entry.file_type()
            && ft.is_dir()
        {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("kul") {
            continue;
        }
        paths.push(path);
    }
    paths.sort();
    Ok(paths)
}

/// Strict-loader output ([`load`]): the manifest bytes and every
/// `.kul` file in the project root.
#[derive(Debug, Clone)]
pub struct LoadedProject {
    pub root: PathBuf,
    /// Path-string label `kul_core::check` will surface for the
    /// manifest in diagnostics (e.g. `"./kul.yml"`).
    pub manifest_name: String,
    pub manifest_yaml: String,
    /// Lexicographically ordered.
    pub inputs: Vec<InputFile>,
}

/// One file from [`discover`]. Carries the on-disk `path` (which the
/// LSP turns into a `file://` URL) alongside the bare `name` and
/// `source` that `kul_core::check` wants.
#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    pub path: PathBuf,
    pub name: String,
    pub source: String,
}

/// Lenient-loader output ([`discover`]). Mirrors [`LoadedProject`] but
/// absorbs filesystem failures: missing/unreadable manifest leaves
/// `manifest_yaml` empty, unreadable directory leaves `files` empty,
/// unreadable individual files are silently skipped. `manifest_name`
/// is always populated with the *expected* path so renderers can say
/// "we looked here".
#[derive(Debug, Clone)]
pub struct DiscoveredProject {
    pub root: PathBuf,
    pub manifest_name: String,
    pub manifest_yaml: String,
    pub files: Vec<DiscoveredFile>,
}

/// Filesystem-level failures from [`load`]. Each variant carries the
/// path it tripped on so renderers can produce path-aware messages.
#[derive(Debug, Error)]
pub enum ProjectLoadError {
    #[error(
        "not a Kul project root: no kul.yml in {root}\n  (a Kul project is a directory containing a sibling kul.yml; the loader checked {expected})"
    )]
    ManifestNotFound { root: PathBuf, expected: PathBuf },
    #[error("failed to read project manifest {path}: {source}")]
    ManifestReadFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to enumerate project directory {path}: {source}")]
    DirectoryReadFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read project file {path}: {source}")]
    InputReadFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Strict load. Returns the first filesystem failure as a typed
/// [`ProjectLoadError`]; lenient consumers should call [`discover`].
///
/// Empty `inputs` is **not** an error: ADR-0015's `KUL-M06`
/// (empty-project diagnostic) is emitted by `kul_core::check`.
pub fn load(root: &Path) -> Result<LoadedProject, ProjectLoadError> {
    let manifest = manifest_path(root);
    if !manifest.exists() {
        return Err(ProjectLoadError::ManifestNotFound {
            root: root.to_path_buf(),
            expected: manifest,
        });
    }
    let manifest_yaml = std::fs::read_to_string(&manifest).map_err(|source| {
        ProjectLoadError::ManifestReadFailed {
            path: manifest.clone(),
            source,
        }
    })?;

    let kul_paths =
        enumerate_kul_paths(root).map_err(|source| ProjectLoadError::DirectoryReadFailed {
            path: root.to_path_buf(),
            source,
        })?;

    let mut inputs = Vec::with_capacity(kul_paths.len());
    for path in &kul_paths {
        let source =
            std::fs::read_to_string(path).map_err(|source| ProjectLoadError::InputReadFailed {
                path: path.clone(),
                source,
            })?;
        inputs.push(InputFile::new(bare_name(path), source));
    }

    let manifest_name = manifest.to_string_lossy().into_owned();
    Ok(LoadedProject {
        root: root.to_path_buf(),
        manifest_name,
        manifest_yaml,
        inputs,
    })
}

/// Lenient load. Never fails: missing/unreadable manifest collapses
/// `manifest_yaml` to empty (the LSP layers its own missing-manifest
/// notice on top), unreadable directory yields empty `files`, and
/// individual unreadable `.kul` files are silently omitted. Strict
/// consumers should call [`load`] instead.
pub fn discover(root: &Path) -> DiscoveredProject {
    let manifest = manifest_path(root);
    let manifest_name = manifest.to_string_lossy().into_owned();
    let manifest_yaml = std::fs::read_to_string(&manifest).unwrap_or_default();

    let files = enumerate_kul_paths(root)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|path| {
            let source = std::fs::read_to_string(&path).ok()?;
            let name = bare_name(&path);
            Some(DiscoveredFile { path, name, source })
        })
        .collect();

    DiscoveredProject {
        root: root.to_path_buf(),
        manifest_name,
        manifest_yaml,
        files,
    }
}
