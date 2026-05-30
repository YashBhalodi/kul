//! Project-keyed LSP cache (ADR-0015).
//!
//! Holds one [`ProjectEntry`] per discovered project root. Project
//! lifecycle: discover on first `did_open`, refresh on every change,
//! evict when the last open URI closes.
//!
//! Per-URI overlay: `Some(Arc<str>)` is an open editor buffer (wins over
//! disk), `None` is a URI known to the project but read from disk.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use kul_core::CheckResult;
use kul_core::ast::InputFile;
use kul_core::semantic::ResolvedDocument;
use kul_core::span::{FileId, FileSpan};
use tokio::sync::RwLock;
use tower_lsp::lsp_types::{FileChangeType, Location, Position, Url};

use crate::convert::LineIndex;

/// Hashable handle to a project root directory.
///
/// Built from a `.kul` URI's parent path. Non-`file://` URIs collapse to
/// the URI string so each ends up its own singleton project.
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

    /// Construct from a filesystem path. Used by integration-test fixtures.
    pub fn from_path(path: PathBuf) -> Self {
        ProjectRoot(path)
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

/// Outcome of a `workspace/didChangeWatchedFiles` event — the
/// side-effects [`Documents::process_watcher_event`] still owes the
/// client after mutating the cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchAction {
    /// No effect. `reason` is the debug-log label.
    Ignored { reason: &'static str },
    /// Project reloaded. Caller should broadcast and empty-publish each
    /// URL in `cleared` (used for the deleted-`.kul` case).
    Reloaded { cleared: Vec<Url> },
    /// Project evicted (manifest deleted). Empty-publish each URL.
    Evicted { cleared: Vec<Url> },
}

impl WatchAction {
    pub fn log_label(&self) -> &'static str {
        match self {
            WatchAction::Ignored { reason } => reason,
            WatchAction::Reloaded { .. } => "reloaded",
            WatchAction::Evicted { .. } => "evicted",
        }
    }
}

/// One cached project: [`CheckResult`] + per-file metadata + the overlay.
///
/// `urls[i]` is the URL of the `.kul` file at `FileId(i + 1)`; `+1` skips
/// the manifest at `FileId::MANIFEST`. `line_indices`, `urls`, and
/// `check.document().kul_files` stay in lock-step.
#[derive(Debug, Clone)]
pub struct ProjectEntry {
    pub root: ProjectRoot,
    pub check: CheckResult,
    pub line_indices: Vec<LineIndex>,
    pub urls: Vec<Url>,
    /// `Some(_)` is an open editor buffer; `None` is a URI known to the
    /// project but read from disk.
    pub overlay: HashMap<Url, Option<Arc<str>>>,
}

impl ProjectEntry {
    /// Number of URIs the editor is currently holding open. Eviction fires at zero.
    pub fn open_count(&self) -> usize {
        self.overlay.values().filter(|s| s.is_some()).count()
    }

    pub fn file_id_for(&self, uri: &Url) -> Option<FileId> {
        self.urls
            .iter()
            .position(|u| u == uri)
            .map(|i| FileId::from_raw((i + 1) as u32))
    }

    pub fn url_for(&self, file: FileId) -> Option<&Url> {
        let i = file.as_u32().checked_sub(1)? as usize;
        self.urls.get(i)
    }

    pub fn line_index_for(&self, file: FileId) -> Option<&LineIndex> {
        let i = file.as_u32().checked_sub(1)? as usize;
        self.line_indices.get(i)
    }

    /// Map a project-wide [`FileSpan`] to an LSP [`Location`].
    pub fn location_for(&self, fs: FileSpan) -> Option<Location> {
        let url = self.url_for(fs.file)?;
        let line_index = self.line_index_for(fs.file)?;
        Some(Location {
            uri: url.clone(),
            range: line_index.range(fs.span),
        })
    }

    /// Every URL in this project, in `FileId` order.
    pub fn project_urls(&self) -> &[Url] {
        &self.urls
    }

    /// Per-URI [`View`] for cursor-less requests (document-symbol, semantic-tokens).
    pub fn view_for_uri(&self, uri: &Url) -> Option<View<'_>> {
        let file = self.file_id_for(uri)?;
        let line_index = self.line_index_for(file)?;
        Some(View {
            file,
            resolved: self.check.resolved(),
            line_index,
        })
    }

    /// Per-URI [`Cursor`] for cursor-shaped requests (hover, definition,
    /// completion, references, rename).
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

    /// Test-only: `View` for the project's first `.kul` file.
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

    /// Test-only counterpart to [`Self::view`].
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

/// Per-URI resolved-document view without a cursor.
pub struct View<'a> {
    pub file: FileId,
    pub resolved: &'a ResolvedDocument,
    pub line_index: &'a LineIndex,
}

/// Per-URI resolved-document view with a cursor — for "what's at byte
/// offset X?" requests.
pub struct Cursor<'a> {
    pub file: FileId,
    pub resolved: &'a ResolvedDocument,
    pub line_index: &'a LineIndex,
    pub offset: usize,
}

impl<'a> Cursor<'a> {
    /// The entity under the cursor, if any. Shared by goto-definition,
    /// find-references, and rename.
    pub fn entity(&self) -> Option<kul_core::node_at::EntityNode<'a>> {
        self.resolved
            .node_at(self.file, self.offset)?
            .entity_reference(self.file)
    }
}

/// Thread-safe handle to the project cache. Cheap to clone.
#[derive(Debug, Clone, Default)]
pub struct Documents {
    inner: Arc<RwLock<HashMap<ProjectRoot, ProjectEntry>>>,
}

impl Documents {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply a `did_open` and re-run the project check. Returns every URL
    /// in the project; the caller broadcasts diagnostics.
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

    /// Apply a `did_change`. Same shape as [`Self::open`].
    pub async fn update(&self, uri: Url, source: String) -> Vec<Url> {
        self.open(uri, source).await
    }

    /// Apply a `did_close`. Returns `(urls, evicted)`:
    /// - `evicted=false`: other URIs still open; caller clears `uri` and refreshes the rest.
    /// - `evicted=true`: project removed; caller clears every URL.
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

    /// Run `f` against the project entry that owns `uri` — the seam
    /// every LSP feature reads through.
    pub async fn with_project<R>(
        &self,
        uri: &Url,
        f: impl FnOnce(&ProjectEntry) -> R,
    ) -> Option<R> {
        let root = ProjectRoot::for_uri(uri);
        let map = self.inner.read().await;
        map.get(&root).map(f)
    }

    /// Apply one `workspace/didChangeWatchedFiles` event. Rules:
    /// - `.kul` events only act on already-cached projects (discovery stays lazy);
    ///   `Changed` is ignored for overlaid URIs (editor buffer wins).
    /// - `kul.yml` `Created`/`Changed` reload; `Deleted` evicts the project.
    /// - Other URI shapes are ignored.
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

/// Classification of a watched-files URI. Malformed input returns `None`
/// so the caller can log and skip.
enum WatchedUri<'a> {
    Kul { root: ProjectRoot, uri: &'a Url },
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
    // Project must already be cached — discovery stays lazy.
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
            // Overlay wins: editor buffer is authoritative.
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
            // Drop from overlay so build_entry doesn't resurrect it.
            let mut overlay = entry.overlay.clone();
            overlay.remove(uri);
            let new_entry = build_entry(root.clone(), overlay);
            map.insert(root, new_entry);
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
            // Directory ceases to be a project. Clear every URI it ever covered.
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

/// Discover the project at `root`, overlay editor buffers, run
/// [`kul_core::check`]. Reads disk fresh every call.
fn build_entry(root: ProjectRoot, overlay: HashMap<Url, Option<Arc<str>>>) -> ProjectEntry {
    let (manifest_label, manifest_yaml, mut disk_files) = discover_disk(root.as_path());

    // Editor-open URIs (`Some`) override disk source; closed URIs (`None`)
    // fall back to disk if present, otherwise dropped.
    for (url, slot) in &overlay {
        if let Some(buf) = slot {
            if let Some(found) = disk_files.iter_mut().find(|(u, _)| u == url) {
                found.1 = Arc::clone(buf);
            } else {
                disk_files.push((url.clone(), Arc::clone(buf)));
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

/// Read manifest + every `.kul` file at `root` off disk. Thin adapter
/// over [`kul_loader::discover`] (the lenient sibling of `load`). Overlay
/// handling is the caller's job (see [`build_entry`]).
fn discover_disk(root: &Path) -> (String, String, Vec<(Url, Arc<str>)>) {
    let project = kul_loader::discover(root);
    let files: Vec<(Url, Arc<str>)> = project
        .files
        .into_iter()
        .filter_map(|f| {
            let url = Url::from_file_path(&f.path).ok()?;
            Some((url, Arc::from(f.source)))
        })
        .collect();
    (project.manifest_name, project.manifest_yaml, files)
}

/// Build the [`InputFile::name`] label for diagnostic rendering. URL ↔
/// `FileId` mapping is kept separately in [`ProjectEntry::urls`].
fn url_label(url: &Url) -> String {
    if let Ok(p) = url.to_file_path()
        && let Some(name) = p.file_name().map(|n| n.to_string_lossy().into_owned())
    {
        return name;
    }
    url.to_string()
}

/// Test-only: build a [`ProjectEntry`] from in-memory sources. First file
/// is at `FileId(1)` with URL `file:///<basename>`.
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

/// Single-file [`ProjectEntry`] fixture. URL is `file:///t.kul`.
#[cfg(test)]
pub(crate) fn test_open_file(source: &str) -> ProjectEntry {
    test_project_entry(&[("t.kul", source)])
}

#[cfg(test)]
pub(crate) fn test_url() -> Url {
    Url::parse("file:///t.kul").unwrap()
}

#[cfg(test)]
pub(crate) fn idx(source: &str, pat: &str) -> usize {
    source.find(pat).expect("pattern in source")
}

/// LSP [`Position`] via a naive byte scan — deliberately not routed
/// through [`LineIndex`] so cursor tests get the raw byte→line/col mapping.
#[cfg(test)]
pub(crate) fn position_for(source: &str, offset: usize) -> Position {
    let mut line = 0u32;
    let mut character = 0u32;
    for (i, b) in source.bytes().enumerate() {
        if i == offset {
            break;
        }
        if b == b'\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }
    }
    Position { line, character }
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

    /// `file://` URL for an OS-temp child — Windows-safe via `from_file_path`.
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
