//! Project loader for the Kul toolchain.
//!
//! A Kul *project* is a directory holding a `kul.yml` manifest plus one
//! or more sibling `*.kul` files (ADR-0013, ADR-0015). The `kul-core`
//! crate operates on already-loaded bytes — it cannot, by policy, touch
//! the filesystem. The CLI's `kul validate` / `format` / `export`
//! subcommands and the LSP's project-discovery layer both need the same
//! shape: a project-root directory in, manifest YAML plus the `.kul`
//! sources out. They differ only in posture toward failure: the CLI
//! wants typed errors for the filesystem-level failure modes; the LSP
//! tolerates the same failures silently because an editor session keeps
//! going even when the on-disk state is broken.
//!
//! This crate is the shared seam. It exposes two functions over one
//! discovery rule:
//!
//! - [`load`] — strict. Returns a typed [`ProjectLoadError`] on any
//!   filesystem failure (missing manifest, unreadable file, etc.).
//!   Used by the CLI.
//! - [`discover`] — lenient. Never fails: a missing manifest collapses
//!   to an empty YAML string, an unreadable `.kul` file is silently
//!   skipped, an unreadable directory yields an empty file list. Used
//!   by the LSP, which surfaces the same conditions through its own
//!   editor-shaped channels.
//!
//! Both share one internal directory-walk helper so the discovery rule
//! (flat directory, `*.kul` only, subdirectories ignored, lexicographic
//! order) lives in exactly one place.
//!
//! The crate lives outside `kul-core` because `kul-core` forbids
//! filesystem IO, and outside `kul-cli` because `kul-cli` already
//! depends on `kul-lsp` (the `kul lsp` subcommand) — placing the loader
//! inside `kul-cli` would put it out of `kul-lsp`'s reach. A small
//! dedicated crate sits below both consumers without any cycle.
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

/// Bare file name for a `.kul` path — what `InputFile::name` and
/// [`DiscoveredFile::name`] both carry. Bare names keep diagnostic
/// output stable regardless of where the project lives on disk (a
/// snapshot test taken in `/tmp/abc-XYZ/` would otherwise embed the
/// temp-dir randomness).
fn bare_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

/// Enumerate the `.kul` files in `root`, in lexicographic order, with
/// subdirectories and non-`.kul` files filtered out. Returns the bare
/// `io::Error` from `read_dir` so callers can project it to whichever
/// error variant (or to silence) fits their posture. Does not read file
/// contents.
fn enumerate_kul_paths(root: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        // Subdirectories ignored (ADR-0015 flat-directory rule). We
        // explicitly check the file type rather than `path.is_file()`
        // because the latter conflates "directory" with "broken
        // symlink / permission denied" — we want directories silently
        // skipped but unreadable `.kul` entries surfaced (either as a
        // typed error in `load` or as a silent skip in `discover`).
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

/// A loaded Kul project: the manifest bytes and every `.kul` file in
/// the project root. The strict-loader output ([`load`]).
///
/// `manifest_name` is the canonical label `kul_core::check` will store
/// for the manifest `FileId` (shows up in JSON `file:` fields,
/// `miette` source-block headings, etc.). `inputs` holds one
/// [`InputFile`] per `.kul` file in lexicographic order.
#[derive(Debug, Clone)]
pub struct LoadedProject {
    /// Path the loader read this project from — the directory holding
    /// `kul.yml` plus the `.kul` files. Surfaced so callers that need
    /// to write back to disk (e.g. `kul format`) don't re-derive it
    /// from CWD, since the loader already established it.
    pub root: PathBuf,
    /// Path-string label for the manifest, e.g. `"./kul.yml"`. This is
    /// what `kul_core::check` will surface as the manifest's filename
    /// in diagnostics.
    pub manifest_name: String,
    /// Raw bytes of the manifest file.
    pub manifest_yaml: String,
    /// The project's `.kul` inputs, lexicographically ordered.
    pub inputs: Vec<InputFile>,
}

/// One `.kul` file the lenient discovery rule found. The lenient-loader
/// output ([`discover`]) carries these instead of `InputFile`s because
/// the LSP-shaped consumer needs the on-disk `path` (to build editor
/// URLs) in addition to the bare `name` and `source` `kul_core::check`
/// wants.
#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    /// On-disk path the file was read from.
    pub path: PathBuf,
    /// Bare file name — same value `load` would put in
    /// [`InputFile::name`] for this file.
    pub name: String,
    /// File contents.
    pub source: String,
}

/// A leniently-discovered Kul project. The lenient-loader output
/// ([`discover`]).
///
/// Mirrors [`LoadedProject`] but absorbs filesystem failures instead of
/// erroring: a missing or unreadable manifest collapses `manifest_yaml`
/// to an empty string (the LSP layers its own "missing/unreadable
/// manifest" notice on top); an unreadable directory yields an empty
/// `files` vec; individual unreadable `.kul` files are silently
/// skipped. `manifest_name` is always populated — it's the *expected*
/// path label, useful for diagnostic headings even when the bytes
/// couldn't be read.
#[derive(Debug, Clone)]
pub struct DiscoveredProject {
    /// Path the loader read this project from.
    pub root: PathBuf,
    /// Path-string label for the manifest. Populated even when the
    /// manifest file is missing — it carries the *expected* location so
    /// downstream renderers can surface "we looked here" without
    /// re-deriving it.
    pub manifest_name: String,
    /// Raw bytes of the manifest file, or an empty string when the
    /// manifest is missing or unreadable.
    pub manifest_yaml: String,
    /// The project's `.kul` files, lexicographically ordered. Excludes
    /// any entry the loader couldn't read.
    pub files: Vec<DiscoveredFile>,
}

/// Filesystem-level failures the strict loader surfaces to callers.
/// Each variant carries the path it tripped on so renderers can produce
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

/// Strict load. Returns either a fully-populated [`LoadedProject`] or a
/// typed [`ProjectLoadError`] on the first filesystem failure
/// encountered.
///
/// The loader does not validate manifest YAML or `.kul` syntax; it only
/// reads bytes. Use [`kul_core::check`] on the result to run the
/// pipeline.
///
/// The empty-`inputs` case is **not** an error here: per ADR-0015 the
/// "empty project" diagnostic is `KUL-M06`, which `kul_core::check`
/// emits when it sees a non-empty manifest with zero inputs. Returning
/// `Ok(LoadedProject { inputs: vec![], .. })` lets that diagnostic
/// flow through the standard renderer.
///
/// Lenient editor-shaped consumers should call [`discover`] instead.
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

/// Lenient load. Never fails: failures collapse to empty contents or
/// skipped entries so an editor session can keep displaying *something*
/// while the on-disk state is incomplete.
///
/// Specifically:
///
/// - Missing or unreadable manifest → `manifest_yaml` is empty, but
///   `manifest_name` still points at the expected `<root>/kul.yml`
///   path. `kul_core::check` will treat the empty YAML as a default
///   [`kul_core::manifest::Manifest`] and emit no `KUL-Mxx` diagnostic;
///   the LSP layers its own missing-manifest notice on top.
/// - Unreadable project directory → `files` is empty.
/// - Unreadable individual `.kul` file → that file is silently omitted
///   from `files`.
///
/// Strict CLI-shaped consumers should call [`load`] instead.
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
