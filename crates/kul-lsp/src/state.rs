//! Project-keyed LSP cache.
//!
//! A Kul project is a directory holding a `kul.yml` manifest plus one or
//! more sibling `*.kul` files (ADR-0015). The cache holds one
//! [`ProjectEntry`] per discovered project root; every URI that belongs to
//! the project reads through the same cached [`kul_core::CheckResult`].
//! The deep module here is the project lifecycle: discover on first
//! `did_open`, refresh on every `did_open` / `did_change`, evict when the
//! last open URI closes.
//!
//! The per-URI overlay map records which URIs the editor currently holds
//! buffers for. A `Some(Arc<str>)` entry wins over the disk source for
//! that URI; `None` means the URI is part of the project but the buffer
//! has gone back to disk (after `did_close`, before eviction).
//!
//! The previous URI-keyed cache (one cached check per URI) is gone with
//! this slice — the cache key is the project root, not the URI.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use kul_core::CheckResult;
use kul_core::ast::InputFile;
use kul_core::semantic::ResolvedDocument;
use kul_core::span::FileId;
use tokio::sync::RwLock;
use tower_lsp::lsp_types::{FileChangeType, Position, Url};

use crate::convert::LineIndex;

/// Hashable handle to a project root directory.
///
/// Constructed from a `.kul` URI by taking the parent path; the LSP
/// cache stores one [`ProjectEntry`] per `ProjectRoot`. Non-`file://`
/// URIs (which can't be turned into a filesystem path) collapse to the
/// URI string so each such URI ends up its own singleton project — the
/// closest we can do without filesystem reach.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProjectRoot(PathBuf);

impl ProjectRoot {
    /// Project root for a `.kul` URI: the URI's parent directory.
    pub fn for_uri(uri: &Url) -> Self {
        let path = uri
            .to_file_path()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf));
        ProjectRoot(path.unwrap_or_else(|| PathBuf::from(uri.to_string())))
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

/// Outcome of processing a single `workspace/didChangeWatchedFiles`
/// event. The caller in `server.rs` translates this into the right
/// combination of project broadcasts and empty-publishes.
///
/// The cache mutation happens inside [`Documents::process_watcher_event`];
/// this is just the bag of side-effects the LSP layer still owes the
/// client (publish diagnostics, clear squiggles).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchAction {
    /// The event had no effect on the cache. The static `reason` is the
    /// label used in the debug log line (matches the codes the issue
    /// spec calls out: `unknown-project`, `overlaid`, …).
    Ignored { reason: &'static str },
    /// The project was reloaded. The caller should broadcast project-wide
    /// diagnostics via [`crate::server::Backend::publish_project`] and
    /// additionally empty-publish each URL in `cleared` (used for the
    /// deleted-`.kul` case so the deleted URI's squiggles go away).
    Reloaded { cleared: Vec<Url> },
    /// The project was evicted (manifest deleted). The caller should
    /// empty-publish each URL in `cleared`; there's nothing to broadcast
    /// because the project no longer exists.
    Evicted { cleared: Vec<Url> },
}

impl WatchAction {
    /// Human-readable action label for the debug log line. Pairs with
    /// the URI and event kind the LSP layer logs alongside.
    pub fn log_label(&self) -> &'static str {
        match self {
            WatchAction::Ignored { reason } => reason,
            WatchAction::Reloaded { .. } => "reloaded",
            WatchAction::Evicted { .. } => "evicted",
        }
    }
}

/// One cached project: the [`CheckResult`] plus per-file metadata
/// (line indices and URLs in `FileId(1..)` order) and the per-URI
/// overlay map.
///
/// `urls[i]` is the URL of the `.kul` file at `FileId(i + 1)`; the
/// `+1` accounts for the manifest at `FileId::MANIFEST`. The two slices
/// (`line_indices`, `urls`) stay in lock-step with
/// `check.document().kul_files`.
#[derive(Debug, Clone)]
pub struct ProjectEntry {
    pub root: ProjectRoot,
    pub check: CheckResult,
    pub line_indices: Vec<LineIndex>,
    pub urls: Vec<Url>,
    /// Per-URI overlay. `Some(_)` carries the editor-buffer source for an
    /// open URI; `None` marks a URI the editor recently closed (still
    /// known to the project, but read from disk on the next rebuild).
    pub overlay: HashMap<Url, Option<Arc<str>>>,
}

impl ProjectEntry {
    /// The number of URIs the editor is currently holding open from this
    /// project. Eviction fires when this hits zero.
    pub fn open_count(&self) -> usize {
        self.overlay.values().filter(|s| s.is_some()).count()
    }

    /// The `FileId` of `uri` inside this project's `CheckResult`, or
    /// `None` if the URI isn't part of the project.
    pub fn file_id_for(&self, uri: &Url) -> Option<FileId> {
        self.urls
            .iter()
            .position(|u| u == uri)
            .map(|i| FileId::from_raw((i + 1) as u32))
    }

    /// The URL of the `.kul` file at `file`, or `None` if `file` is the
    /// manifest or out of range.
    pub fn url_for(&self, file: FileId) -> Option<&Url> {
        let i = file.as_u32().checked_sub(1)? as usize;
        self.urls.get(i)
    }

    /// The cached [`LineIndex`] for `file`, or `None` if `file` is the
    /// manifest or out of range.
    pub fn line_index_for(&self, file: FileId) -> Option<&LineIndex> {
        let i = file.as_u32().checked_sub(1)? as usize;
        self.line_indices.get(i)
    }

    /// Every URL currently part of this project, in `FileId` order.
    pub fn project_urls(&self) -> &[Url] {
        &self.urls
    }

    /// Build the per-URI [`View`] for cursor-less LSP requests
    /// (document-symbol, semantic-tokens). `None` when `uri` isn't part
    /// of the project.
    pub fn view_for_uri(&self, uri: &Url) -> Option<View<'_>> {
        let file = self.file_id_for(uri)?;
        let line_index = self.line_index_for(file)?;
        Some(View {
            file,
            resolved: self.check.resolved(),
            line_index,
        })
    }

    /// Build the per-URI [`Cursor`] for cursor-shaped LSP requests
    /// (hover, definition, completion, references, rename,
    /// prepare-rename). `None` when `uri` isn't part of the project or
    /// the position is past the document's end.
    pub fn cursor_for_uri(&self, uri: &Url, position: Position) -> Option<Cursor<'_>> {
        let file = self.file_id_for(uri)?;
        let line_index = self.line_index_for(file)?;
        let offset = line_index.byte_offset(position)?;
        Some(Cursor {
            file,
            resolved: self.check.resolved(),
            line_index,
            offset,
        })
    }

    /// Test-only convenience: build a `View` for the project's first
    /// (and, in tests, only) `.kul` file. Lets per-feature unit tests
    /// keep their existing `let v = doc.view();` line.
    #[cfg(test)]
    pub fn view(&self) -> View<'_> {
        let file = FileId::from_raw(1);
        View {
            file,
            resolved: self.check.resolved(),
            line_index: self
                .line_index_for(file)
                .expect("test fixture has at least one .kul file"),
        }
    }

    /// Test-only convenience matching [`Self::view`].
    #[cfg(test)]
    pub fn cursor(&self, position: Position) -> Option<Cursor<'_>> {
        let file = FileId::from_raw(1);
        let line_index = self.line_index_for(file)?;
        let offset = line_index.byte_offset(position)?;
        Some(Cursor {
            file,
            resolved: self.check.resolved(),
            line_index,
            offset,
        })
    }
}

/// Resolved-document view for a single URI without a cursor — for the
/// per-URI listing requests (`textDocument/documentSymbol`,
/// `textDocument/semanticTokens/full`). Built once per request via
/// [`ProjectEntry::view_for_uri`].
pub struct View<'a> {
    pub file: FileId,
    pub resolved: &'a ResolvedDocument,
    pub line_index: &'a LineIndex,
}

/// Resolved-document view for a single URI plus a cursor — for
/// "what's at byte offset X?" requests (hover, goto-definition,
/// completion, references, prepare-rename, rename). Built once per
/// request via [`ProjectEntry::cursor_for_uri`].
pub struct Cursor<'a> {
    pub file: FileId,
    pub resolved: &'a ResolvedDocument,
    pub line_index: &'a LineIndex,
    pub offset: usize,
}

/// Thread-safe handle to the project cache.
///
/// Cheap to clone (it's an `Arc`). The lock is held only for the duration
/// of one request's read or write — every request's actual work runs
/// under a snapshot of the entry's contents.
#[derive(Debug, Clone, Default)]
pub struct Documents {
    inner: Arc<RwLock<HashMap<ProjectRoot, ProjectEntry>>>,
}

impl Documents {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply a `did_open` for `uri` with `source` and re-run the project
    /// check. Returns the URLs of every file in the project after the
    /// update — the caller broadcasts diagnostics to each of them.
    pub async fn open(&self, uri: Url, source: String) -> Vec<Url> {
        let root = ProjectRoot::for_uri(&uri);
        let mut map = self.inner.write().await;
        let overlay = match map.remove(&root) {
            Some(entry) => entry.overlay,
            None => HashMap::new(),
        };
        let mut overlay = overlay;
        overlay.insert(uri, Some(Arc::from(source)));
        let entry = build_entry(root.clone(), overlay);
        let urls = entry.urls.clone();
        map.insert(root, entry);
        urls
    }

    /// Apply a `did_change` for `uri` with `source` and re-run the
    /// project check. Same broadcast contract as [`Self::open`].
    pub async fn update(&self, uri: Url, source: String) -> Vec<Url> {
        // `did_change` is structurally the same operation as `did_open`
        // in our model: overlay the URI's buffer, rebuild the project.
        // Existing overlay entries for other URIs in the project carry
        // through (they're in `entry.overlay`).
        self.open(uri, source).await
    }

    /// Apply a `did_close` for `uri`. Removes the URI's editor buffer
    /// (flips its overlay entry to `None`) and re-runs the project check
    /// against disk-backed source. Returns:
    ///
    /// - `(urls, false)` if other URIs in the project are still open;
    ///   the caller broadcasts an empty diagnostic publish to `uri` and
    ///   refreshed diagnostics to the rest.
    /// - `(urls, true)` if `uri` was the last open URI of its project;
    ///   the project is evicted. The caller broadcasts an empty
    ///   publish to every URL in `urls`, clearing any stale squiggles
    ///   the Problems pane was still showing for project siblings.
    pub async fn close(&self, uri: &Url) -> (Vec<Url>, bool) {
        let root = ProjectRoot::for_uri(uri);
        let mut map = self.inner.write().await;
        let Some(entry) = map.remove(&root) else {
            return (vec![uri.clone()], true);
        };
        let mut overlay = entry.overlay;
        overlay.insert(uri.clone(), None);
        let still_open = overlay.values().any(|s| s.is_some());
        if !still_open {
            let mut urls: Vec<Url> = overlay.into_keys().collect();
            urls.sort_by(|a, b| a.as_str().cmp(b.as_str()));
            return (urls, true);
        }
        let new_entry = build_entry(root.clone(), overlay);
        let urls = new_entry.urls.clone();
        map.insert(root, new_entry);
        (urls, false)
    }

    /// Run `f` against the project entry that owns `uri`, if any. Used
    /// by request handlers (hover, definition, completion, …) — every
    /// LSP feature reads through this seam.
    pub async fn with_project<R>(
        &self,
        uri: &Url,
        f: impl FnOnce(&ProjectEntry) -> R,
    ) -> Option<R> {
        let root = ProjectRoot::for_uri(uri);
        let map = self.inner.read().await;
        map.get(&root).map(f)
    }

    /// Apply one `workspace/didChangeWatchedFiles` event to the cache and
    /// return the side-effects the caller still owes the client. The
    /// rules (see issue #86 and PRD #80):
    ///
    /// - `.kul` `Created` / `Changed` / `Deleted` only act when the
    ///   parent directory is already a cached project — discovery stays
    ///   lazy. `Changed` is ignored when the URI is currently overlaid
    ///   so the editor buffer remains authoritative.
    /// - `kul.yml` `Created` / `Changed` reload the cached project's
    ///   manifest; `Deleted` evicts the project entirely (the directory
    ///   ceases to be a project).
    /// - Any other URI shape (not `.kul`, not `kul.yml`) is ignored.
    pub async fn process_watcher_event(&self, uri: &Url, kind: FileChangeType) -> WatchAction {
        let Some(classified) = classify_watched_uri(uri) else {
            return WatchAction::Ignored {
                reason: "unknown-file-type",
            };
        };

        let mut map = self.inner.write().await;
        match classified {
            WatchedUri::Manifest { root } => apply_manifest_event(&mut map, root, kind),
            WatchedUri::Kul { root, uri } => apply_kul_event(&mut map, root, uri, kind),
        }
    }

    #[cfg(test)]
    async fn project_count(&self) -> usize {
        self.inner.read().await.len()
    }
}

/// Classification of a URI carried in a `workspace/didChangeWatchedFiles`
/// event. The watcher is registered for `**/*.kul` and `**/kul.yml`, so
/// every well-formed event lands in one of the two variants — but the
/// classifier is tolerant of malformed input (a non-`file://` URI, or a
/// URI whose extension is neither) and reports `None` so the caller can
/// log and skip.
enum WatchedUri<'a> {
    /// `.kul` file event. `root` is the file's parent directory.
    Kul { root: ProjectRoot, uri: &'a Url },
    /// `kul.yml` manifest event. `root` is the manifest's parent
    /// directory (i.e. the project root the manifest belongs to).
    Manifest { root: ProjectRoot },
}

fn classify_watched_uri(uri: &Url) -> Option<WatchedUri<'_>> {
    let path = uri.to_file_path().ok()?;
    let file_name = path.file_name()?.to_str()?;
    if file_name == "kul.yml" {
        let root = path.parent().map(Path::to_path_buf)?;
        return Some(WatchedUri::Manifest {
            root: ProjectRoot(root),
        });
    }
    if path.extension().and_then(|s| s.to_str()) == Some("kul") {
        let root = path.parent().map(Path::to_path_buf)?;
        return Some(WatchedUri::Kul {
            root: ProjectRoot(root),
            uri,
        });
    }
    None
}

fn apply_kul_event(
    map: &mut HashMap<ProjectRoot, ProjectEntry>,
    root: ProjectRoot,
    uri: &Url,
    kind: FileChangeType,
) -> WatchAction {
    // The project must already be cached — discovery stays lazy
    // (issue #86 spec). A `Created` event in a directory that has a
    // `kul.yml` but isn't yet open lands here and is ignored.
    let Some(entry) = map.get(&root) else {
        return WatchAction::Ignored {
            reason: "unknown-project",
        };
    };

    match kind {
        FileChangeType::CREATED => {
            let overlay = entry.overlay.clone();
            let new_entry = build_entry(root.clone(), overlay);
            map.insert(root, new_entry);
            WatchAction::Reloaded {
                cleared: Vec::new(),
            }
        }
        FileChangeType::CHANGED => {
            // Overlay wins: if the editor holds a buffer for this URI,
            // the disk change is stale by definition — drop it.
            if matches!(entry.overlay.get(uri), Some(Some(_))) {
                return WatchAction::Ignored { reason: "overlaid" };
            }
            let overlay = entry.overlay.clone();
            let new_entry = build_entry(root.clone(), overlay);
            map.insert(root, new_entry);
            WatchAction::Reloaded {
                cleared: Vec::new(),
            }
        }
        FileChangeType::DELETED => {
            // Drop the URI from the overlay so build_entry doesn't
            // resurrect it. The file vanished from disk, so the
            // re-read on rebuild won't pick it up either.
            let mut overlay = entry.overlay.clone();
            overlay.remove(uri);
            let new_entry = build_entry(root.clone(), overlay);
            map.insert(root, new_entry);
            // Empty-publish to the deleted URI so its squiggles clear.
            WatchAction::Reloaded {
                cleared: vec![uri.clone()],
            }
        }
        _ => WatchAction::Ignored {
            reason: "unknown-change-type",
        },
    }
}

fn apply_manifest_event(
    map: &mut HashMap<ProjectRoot, ProjectEntry>,
    root: ProjectRoot,
    kind: FileChangeType,
) -> WatchAction {
    let Some(entry) = map.get(&root) else {
        return WatchAction::Ignored {
            reason: "unknown-project",
        };
    };
    match kind {
        FileChangeType::DELETED => {
            // Directory ceases to be a project. Surface clearing
            // publishes for every URI the project ever covered —
            // both disk-discovered files and editor-overlay URIs.
            let mut cleared: Vec<Url> = entry.urls.clone();
            for url in entry.overlay.keys() {
                if !cleared.contains(url) {
                    cleared.push(url.clone());
                }
            }
            cleared.sort_by(|a, b| a.as_str().cmp(b.as_str()));
            map.remove(&root);
            WatchAction::Evicted { cleared }
        }
        FileChangeType::CREATED | FileChangeType::CHANGED => {
            let overlay = entry.overlay.clone();
            let new_entry = build_entry(root.clone(), overlay);
            map.insert(root, new_entry);
            WatchAction::Reloaded {
                cleared: Vec::new(),
            }
        }
        _ => WatchAction::Ignored {
            reason: "unknown-change-type",
        },
    }
}

/// Discover the project at `root` from disk, overlay any editor buffers,
/// and run [`kul_core::check`]. Always reads the manifest and every
/// sibling `.kul` file fresh; external filesystem changes reach the
/// cache via [`Documents::process_watcher_event`] (issue #86), which
/// also reuses this builder.
fn build_entry(root: ProjectRoot, overlay: HashMap<Url, Option<Arc<str>>>) -> ProjectEntry {
    let (manifest_label, manifest_yaml, mut disk_files) = discover_disk(root.as_path());

    // Apply overlay overrides. Editor-open URIs (`Some`) take precedence
    // over the disk source. URIs in overlay but absent from disk
    // (untitled documents, files the editor created but hasn't saved)
    // are included when their overlay entry is `Some` and dropped when
    // it's `None`.
    for (url, slot) in &overlay {
        match slot {
            Some(buf) => {
                if let Some(found) = disk_files.iter_mut().find(|(u, _)| u == url) {
                    found.1 = Arc::clone(buf);
                } else {
                    disk_files.push((url.clone(), Arc::clone(buf)));
                }
            }
            None => {
                // Closed URI: keep the disk version if it's there; drop
                // otherwise (file vanished from disk while still open in
                // the editor, then closed).
                if !disk_files.iter().any(|(u, _)| u == url) {
                    // already absent from disk; nothing to do
                }
            }
        }
    }

    disk_files.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));

    let urls: Vec<Url> = disk_files.iter().map(|(u, _)| u.clone()).collect();
    let inputs: Vec<InputFile> = disk_files
        .iter()
        .map(|(u, src)| InputFile::new(url_label(u), src.as_ref()))
        .collect();
    let line_indices: Vec<LineIndex> = disk_files
        .iter()
        .map(|(_, src)| LineIndex::new(Arc::clone(src)))
        .collect();

    let check = kul_core::check(manifest_label, &manifest_yaml, &inputs);

    ProjectEntry {
        root,
        check,
        line_indices,
        urls,
        overlay,
    }
}

/// Read the manifest and every `.kul` file in `root` off disk. Returns
/// the manifest label (path string), the manifest YAML bytes (empty
/// when the manifest is missing or unreadable), and the sibling `.kul`
/// files as `(Url, Arc<str>)` pairs in directory-iteration order.
///
/// Reusing [`kul_loader::load`] directly isn't quite the right shape
/// here: the loader returns bare-basename `InputFile`s and errors out
/// when `kul.yml` is absent, but the LSP wants editor-shaped `Url`s and
/// tolerates a missing manifest (existing behaviour — the file is still
/// useful to edit; `KUL-M01` is the CLI's job to surface). The
/// discovery rule itself — `kul.yml` in the same dir, flat
/// enumeration, `*.kul` extension, subdirectories ignored — is the same
/// rule [`kul_loader::load`] encodes, so this mirrors its logic.
fn discover_disk(root: &Path) -> (String, String, Vec<(Url, Arc<str>)>) {
    let manifest_path = root.join("kul.yml");
    let manifest_label = manifest_path.to_string_lossy().into_owned();
    let manifest_yaml = std::fs::read_to_string(&manifest_path).unwrap_or_default();

    let mut out: Vec<(Url, Arc<str>)> = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return (manifest_label, manifest_yaml, out);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("kul") {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(url) = Url::from_file_path(&path) else {
            continue;
        };
        out.push((url, Arc::from(source)));
    }
    (manifest_label, manifest_yaml, out)
}

/// Build the [`InputFile::name`] label for a project URL. The bare file
/// name is what `kul-core` puts in `KulFile.name`; the LSP keeps its
/// own URL ↔ `FileId` mapping in [`ProjectEntry::urls`] so the label
/// here is for diagnostic rendering only.
fn url_label(url: &Url) -> String {
    if let Ok(p) = url.to_file_path()
        && let Some(name) = p.file_name().map(|n| n.to_string_lossy().into_owned())
    {
        return name;
    }
    url.to_string()
}

/// Build a [`ProjectEntry`] from in-memory project files for per-feature
/// unit tests. Mirrors the fixture shape every `features/*.rs` test
/// module needs — wraps one or more `.kul` sources in a project against
/// a default-typed [`Manifest`].
///
/// The first file is at `FileId(1)` with URL `file:///<basename>`.
#[cfg(test)]
pub(crate) fn test_project_entry(files: &[(&str, &str)]) -> ProjectEntry {
    use kul_core::manifest::Manifest;
    assert!(!files.is_empty(), "test project needs at least one file");
    let urls: Vec<Url> = files
        .iter()
        .map(|(name, _)| Url::parse(&format!("file:///{name}")).expect("test url"))
        .collect();
    let inputs: Vec<InputFile> = files
        .iter()
        .map(|(name, src)| InputFile::new(*name, *src))
        .collect();
    let check = kul_core::check_with_manifest("kul.yml", "", &Manifest::default(), &inputs);
    let line_indices: Vec<LineIndex> = files.iter().map(|(_, src)| LineIndex::new(*src)).collect();
    let mut overlay: HashMap<Url, Option<Arc<str>>> = HashMap::new();
    for (url, (_, src)) in urls.iter().zip(files.iter()) {
        overlay.insert(url.clone(), Some(Arc::from(*src)));
    }
    let root = ProjectRoot(PathBuf::from("/test"));
    ProjectEntry {
        root,
        check,
        line_indices,
        urls,
        overlay,
    }
}

/// Single-file [`ProjectEntry`] fixture for tests that don't care about
/// the multi-file shape. Equivalent to `test_project_entry(&[("t.kul", source)])`.
/// The basename is `t.kul` so the resulting URL is `file:///t.kul` —
/// the historic constant every per-feature test hardcodes for its
/// `url()` helper, which keeps existing snapshots stable.
#[cfg(test)]
pub(crate) fn test_open_file(source: &str) -> ProjectEntry {
    test_project_entry(&[("t.kul", source)])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[tokio::test]
    async fn open_caches_under_project_root() {
        let docs = Documents::default();
        let uri = url("file:///tmp/kul-test-open-cache/foo.kul");
        let _ = std::fs::create_dir_all("/tmp/kul-test-open-cache");
        // No manifest on disk; build_entry tolerates and uses the open
        // URI as the only project file.
        docs.open(
            uri.clone(),
            "person a name:\"A\" gender:female\n".to_owned(),
        )
        .await;
        assert_eq!(docs.project_count().await, 1);
    }

    #[tokio::test]
    async fn close_evicts_when_last_open_uri_closes() {
        let docs = Documents::default();
        let uri = url("file:///tmp/kul-test-close-evict/foo.kul");
        let _ = std::fs::create_dir_all("/tmp/kul-test-close-evict");
        docs.open(
            uri.clone(),
            "person a name:\"A\" gender:female\n".to_owned(),
        )
        .await;
        assert_eq!(docs.project_count().await, 1);
        let (_, evicted) = docs.close(&uri).await;
        assert!(evicted);
        assert_eq!(docs.project_count().await, 0);
    }

    #[tokio::test]
    async fn with_project_returns_none_for_unknown_uri() {
        let docs = Documents::default();
        let got = docs.with_project(&url("file:///nope.kul"), |_| 1).await;
        assert!(got.is_none());
    }

    /// Build a `file://` URL for an OS-temp-directory child. Going
    /// through [`Url::from_file_path`] keeps the URI shape valid on
    /// Windows (which requires a drive letter) as well as Unix.
    fn temp_file_url(child: &str) -> Url {
        let path = std::env::temp_dir().join(child);
        let _ = std::fs::create_dir_all(path.parent().expect("temp child has parent"));
        Url::from_file_path(&path).expect("file URL for temp child")
    }

    #[tokio::test]
    async fn watcher_event_for_unknown_project_is_ignored() {
        let docs = Documents::default();
        let action = docs
            .process_watcher_event(
                &temp_file_url("kul-test-never-cached/foo.kul"),
                FileChangeType::CHANGED,
            )
            .await;
        assert_eq!(
            action,
            WatchAction::Ignored {
                reason: "unknown-project"
            }
        );
    }

    #[tokio::test]
    async fn watcher_event_for_unknown_file_type_is_ignored() {
        let docs = Documents::default();
        let action = docs
            .process_watcher_event(
                &temp_file_url("kul-test-unknown-ext/README.md"),
                FileChangeType::CHANGED,
            )
            .await;
        assert_eq!(
            action,
            WatchAction::Ignored {
                reason: "unknown-file-type"
            }
        );
    }

    #[tokio::test]
    async fn watcher_change_on_overlaid_file_is_ignored() {
        let docs = Documents::default();
        let uri = temp_file_url("kul-test-overlay-ignore/foo.kul");
        docs.open(
            uri.clone(),
            "person a name:\"A\" gender:female\n".to_owned(),
        )
        .await;
        let action = docs
            .process_watcher_event(&uri, FileChangeType::CHANGED)
            .await;
        assert_eq!(action, WatchAction::Ignored { reason: "overlaid" });
    }
}
