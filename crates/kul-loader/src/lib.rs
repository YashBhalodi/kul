//! Project loader for the Kul toolchain.
//!
//! A Kul *project* is a directory holding a `kul.yml` manifest plus one
//! or more sibling `*.kul` files (ADR-0013, ADR-0015). The `kul-core`
//! crate operates on already-loaded bytes — it cannot, by policy, touch
//! the filesystem. The CLI's `kul validate` / `format` / `export`
//! subcommands and (later) the LSP's project-discovery layer both need
//! the same shape: a project-root directory in, a `(manifest_yaml,
//! Vec<InputFile>)` tuple out, with typed errors for the
//! filesystem-level failure modes.
//!
//! This crate is that shared seam. It lives outside `kul-core` because
//! `kul-core` forbids filesystem IO, and outside `kul-cli` because
//! `kul-cli` already depends on `kul-lsp` (the `kul lsp` subcommand) —
//! placing the loader inside `kul-cli` would put it out of `kul-lsp`'s
//! reach. A small dedicated crate sits below both consumers without any
//! cycle.
//!
//! # Behavior
//!
//! - **Manifest discovery.** The project root is the directory passed
//!   in; the manifest is `<root>/kul.yml`. There is no walk-up — a
//!   directory without a sibling `kul.yml` is not a Kul project.
//! - **`.kul` enumeration.** Every entry in the root directory whose
//!   extension is `kul` is loaded. Order is lexicographic by file name
//!   so callers get deterministic input order across runs.
//! - **Subdirectories ignored.** Nested directories are not walked
//!   (ADR-0015's flat-directory rule). Per-spec, multi-file projects are
//!   one flat directory.
//! - **Non-`.kul` files ignored.** READMEs, `.gitignore`, editor
//!   backups, dotfiles — silently skipped.
//! - **IO errors surface as typed variants** so call sites can render
//!   them without parsing `io::Error` messages.
//!
//! # What the loader does *not* do
//!
//! The loader is filesystem-only. It does not parse the manifest YAML
//! (that's `kul_core::manifest::validate`), does not lex / parse `.kul`
//! sources (that's `kul_core::check`), and does not emit `KUL-Mxx`
//! diagnostics — those flow from `kul_core::check` once the loader has
//! handed it the bytes. `KUL-M06` (empty project) in particular is
//! emitted by `kul_core::check` itself when `inputs.is_empty()` and the
//! manifest is non-empty; the loader returns the empty `inputs` vector
//! straight through.

use std::path::{Path, PathBuf};

use kul_core::ast::InputFile;
use thiserror::Error;

/// Resolve the manifest path for a project rooted at `root`. The Kul
/// manifest sits at `<root>/kul.yml` per `spec/14-project-manifest.md`.
/// This is the project-root counterpart to
/// [`kul_core::manifest::sibling_path`] (which solves the per-input-file
/// case the LSP needs when handed a `.kul` URI).
fn manifest_path(root: &Path) -> PathBuf {
    root.join("kul.yml")
}

/// A loaded Kul project: the manifest bytes and every `.kul` file in
/// the project root.
///
/// `manifest_name` is the canonical label `kul_core::check` will store
/// for the manifest `FileId` (shows up in JSON `file:` fields,
/// `miette` source-block headings, etc.). `inputs` holds one
/// [`InputFile`] per `.kul` file in lexicographic order.
#[derive(Debug, Clone)]
pub struct LoadedProject {
    /// Path-string label for the manifest, e.g. `"./kul.yml"`. This is
    /// what `kul_core::check` will surface as the manifest's filename
    /// in diagnostics.
    pub manifest_name: String,
    /// Raw bytes of the manifest file.
    pub manifest_yaml: String,
    /// The project's `.kul` inputs, lexicographically ordered.
    pub inputs: Vec<InputFile>,
}

/// Filesystem-level failures the loader surfaces to callers. Each
/// variant carries the path it tripped on so renderers can produce
/// path-aware error messages without re-deriving them.
#[derive(Debug, Error)]
pub enum ProjectLoadError {
    /// The project root is missing a sibling `kul.yml`. The
    /// `expected` path is the location the loader looked for the
    /// manifest at.
    #[error(
        "not a Kul project root: no kul.yml in {root}\n  (a Kul project is a directory containing a sibling kul.yml; the loader checked {expected})"
    )]
    ManifestNotFound { root: PathBuf, expected: PathBuf },
    /// The manifest exists but could not be read off disk.
    #[error("failed to read project manifest {path}: {source}")]
    ManifestReadFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// The project root could not be listed (permissions, broken
    /// directory handle, etc.).
    #[error("failed to enumerate project directory {path}: {source}")]
    DirectoryReadFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// A `.kul` file in the project root could not be read.
    #[error("failed to read project file {path}: {source}")]
    InputReadFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Load the Kul project rooted at `root`.
///
/// Returns either a fully-populated [`LoadedProject`] or a typed
/// [`ProjectLoadError`]. The loader does not validate manifest YAML or
/// `.kul` syntax; it only reads bytes. Use [`kul_core::check`] on the
/// result to run the pipeline.
///
/// The empty-`inputs` case is **not** an error here: per ADR-0015 the
/// "empty project" diagnostic is `KUL-M06`, which `kul_core::check`
/// emits when it sees a non-empty manifest with zero inputs. Returning
/// `Ok(LoadedProject { inputs: vec![], .. })` lets that diagnostic
/// flow through the standard renderer.
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

    let mut kul_files: Vec<PathBuf> = Vec::new();
    let entries =
        std::fs::read_dir(root).map_err(|source| ProjectLoadError::DirectoryReadFailed {
            path: root.to_path_buf(),
            source,
        })?;
    for entry in entries {
        let entry = entry.map_err(|source| ProjectLoadError::DirectoryReadFailed {
            path: root.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        // Subdirectories ignored (ADR-0015 flat-directory rule). We
        // explicitly check the file type rather than `path.is_file()`
        // because the latter conflates "directory" with "broken
        // symlink / permission denied" — we want directories silently
        // skipped but unreadable `.kul` entries surfaced as a typed
        // error in the read pass below.
        if let Ok(ft) = entry.file_type()
            && ft.is_dir()
        {
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("kul") {
            continue;
        }
        kul_files.push(path);
    }
    kul_files.sort();

    let mut inputs = Vec::with_capacity(kul_files.len());
    for path in &kul_files {
        let source =
            std::fs::read_to_string(path).map_err(|source| ProjectLoadError::InputReadFailed {
                path: path.clone(),
                source,
            })?;
        // The InputFile name is the bare file name. `kul_core::check`
        // labels diagnostics with this string; bare names keep
        // diagnostic output stable regardless of where the project
        // lives on disk (a snapshot test taken in `/tmp/abc-XYZ/`
        // would otherwise embed the temp-dir randomness).
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        inputs.push(InputFile::new(name, source));
    }

    let manifest_name = manifest.to_string_lossy().into_owned();
    Ok(LoadedProject {
        manifest_name,
        manifest_yaml,
        inputs,
    })
}
