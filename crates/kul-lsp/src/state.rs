//! Per-URI document cache.
//!
//! The cache holds one [`OpenFile`] per open `.kul` URI. Each `OpenFile`
//! carries the source text, a [`LineIndex`] for byte ↔ LSP-position
//! conversion, and the cached `kul_core::CheckResult` produced by running
//! the full pipeline (manifest validation included) over the URI's
//! content. All access goes through the [`Documents`] handle so the
//! locking story stays in one place.
//!
//! The `OpenFile` rename (from `Document`) lands alongside the multi-file
//! refactor: `kul_core::ast::Document` now means the multi-file project
//! container, so the LSP's per-URI cache entry needs a name that doesn't
//! collide with it.
//!
//! The source is stored as an [`Arc<str>`] shared with the [`LineIndex`]
//! so a single heap buffer backs both fields.

use std::collections::HashMap;
use std::sync::Arc;

use kul_core::CheckResult;
use kul_core::span::FileId;
use tokio::sync::RwLock;
use tower_lsp::lsp_types::Url;

use crate::convert::LineIndex;

/// One open `.kul` document plus the cached check result.
///
/// The check pipeline is run at every `did_open` / `did_change`. Manifest
/// failures are first-class diagnostics inside `check.diagnostics` (per
/// the file-identity refactor); the LSP no longer carries a separate
/// "manifest failed" displacement.
#[derive(Debug, Clone)]
pub struct OpenFile {
    pub source: Arc<str>,
    pub line_index: LineIndex,
    pub check: CheckResult,
}

impl OpenFile {
    /// The [`FileId`] of the `.kul` file inside the cached
    /// `CheckResult.document()`. The toolchain feeds one input per URI,
    /// so the file always sits at `FileId(1)` (the manifest occupies
    /// `FileId::MANIFEST`).
    pub fn kul_file_id(&self) -> FileId {
        // First (and only) `.kul` input lives at `FileId(1)`.
        self.check
            .document()
            .kul_file_ids()
            .next()
            .unwrap_or(FileId::MANIFEST)
    }
}

/// Thread-safe handle to the open-document map.
///
/// Cheap to clone (it's an `Arc`).
#[derive(Debug, Clone, Default)]
pub struct Documents {
    inner: Arc<RwLock<HashMap<Url, OpenFile>>>,
}

impl Documents {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn open(&self, uri: Url, source: String) {
        let entry = build_open_file(&uri, source);
        let mut map = self.inner.write().await;
        map.insert(uri, entry);
    }

    /// Replace the cached document for `uri` with a freshly-checked one.
    /// Used by `didChange` (full sync — the whole text is sent each time).
    /// The manifest is re-loaded from disk on every update so the
    /// `KUL-Mxx` diagnostics reflect the manifest's current state.
    pub async fn update(&self, uri: Url, source: String) {
        let entry = build_open_file(&uri, source);
        let mut map = self.inner.write().await;
        map.insert(uri, entry);
    }

    pub async fn close(&self, uri: &Url) {
        let mut map = self.inner.write().await;
        map.remove(uri);
    }

    /// Run `f` against the cached document under `uri`, if any. Returns
    /// `None` if the document is not open.
    pub async fn with<R>(&self, uri: &Url, f: impl FnOnce(&OpenFile) -> R) -> Option<R> {
        let map = self.inner.read().await;
        map.get(uri).map(f)
    }

    #[cfg(test)]
    async fn open_count(&self) -> usize {
        self.inner.read().await.len()
    }
}

fn build_open_file(uri: &Url, source: String) -> OpenFile {
    use kul_core::ast::InputFile;
    let source: Arc<str> = Arc::from(source);
    let line_index = LineIndex::new(Arc::clone(&source));
    let label = uri.to_string();
    let inputs = vec![InputFile::new(label, source.as_ref())];
    let (manifest_name, manifest_yaml) = crate::manifest::manifest_yaml_for(uri);
    let check = kul_core::check(manifest_name, &manifest_yaml, &inputs);
    OpenFile {
        source,
        line_index,
        check,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kul_core::ast::InputFile;
    use kul_core::manifest::Manifest;

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    fn make_open_file(source: &str) -> OpenFile {
        let source_arc: Arc<str> = Arc::from(source);
        let line_index = LineIndex::new(Arc::clone(&source_arc));
        let inputs = vec![InputFile::new("test.kul", source)];
        let check = kul_core::check_with_manifest("kul.yml", "", &Manifest::default(), &inputs);
        OpenFile {
            source: source_arc,
            line_index,
            check,
        }
    }

    #[tokio::test]
    async fn document_caches_source_and_check() {
        let docs = Documents::default();
        let doc = make_open_file("person alice name:\"A\" gender:female\n");
        let mut map = docs.inner.write().await;
        map.insert(url("file:///a.kul"), doc);
        drop(map);
        let stored_source = docs
            .with(&url("file:///a.kul"), |d| d.source.clone())
            .await
            .unwrap();
        assert_eq!(&*stored_source, "person alice name:\"A\" gender:female\n");
    }

    #[tokio::test]
    async fn close_drops_document() {
        let docs = Documents::default();
        let mut map = docs.inner.write().await;
        map.insert(url("file:///a.kul"), make_open_file(""));
        drop(map);
        assert_eq!(docs.open_count().await, 1);
        docs.close(&url("file:///a.kul")).await;
        assert_eq!(docs.open_count().await, 0);
    }

    #[tokio::test]
    async fn with_returns_none_for_unknown() {
        let docs = Documents::default();
        let got = docs.with(&url("file:///nope.kul"), |_| 1).await;
        assert!(got.is_none());
    }
}
