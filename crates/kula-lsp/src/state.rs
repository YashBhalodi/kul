//! Per-URI document cache.
//!
//! Holds the source text, the cached `kula_core::CheckResult`, and a
//! `LineIndex` for byte ↔ LSP-position conversion. All access goes through
//! the `Documents` handle so the locking story stays in one place.

use std::collections::HashMap;
use std::sync::Arc;

use kula_core::CheckResult;
use tokio::sync::RwLock;
use tower_lsp::lsp_types::Url;

use crate::convert::LineIndex;

/// One open document.
#[derive(Debug)]
pub struct Document {
    pub source: String,
    pub line_index: LineIndex,
    pub check: CheckResult,
}

impl Document {
    fn from_source(source: String) -> Self {
        let line_index = LineIndex::new(&source);
        let check = kula_core::check(&source);
        Self {
            source,
            line_index,
            check,
        }
    }
}

/// Thread-safe handle to the open-document map.
///
/// Cheap to clone (it's an `Arc`).
#[derive(Debug, Clone, Default)]
pub struct Documents {
    inner: Arc<RwLock<HashMap<Url, Document>>>,
}

impl Documents {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn open(&self, uri: Url, source: String) {
        let mut map = self.inner.write().await;
        map.insert(uri, Document::from_source(source));
    }

    /// Replace the cached document for `uri` with a freshly-parsed one. Used
    /// by `didChange` (full sync — the whole text is sent each time).
    pub async fn update(&self, uri: Url, source: String) {
        let mut map = self.inner.write().await;
        map.insert(uri, Document::from_source(source));
    }

    pub async fn close(&self, uri: &Url) {
        let mut map = self.inner.write().await;
        map.remove(uri);
    }

    /// Run `f` against the cached document under `uri`, if any. Returns
    /// `None` if the document is not open.
    pub async fn with<R>(&self, uri: &Url, f: impl FnOnce(&Document) -> R) -> Option<R> {
        let map = self.inner.read().await;
        map.get(uri).map(f)
    }

    #[cfg(test)]
    async fn open_count(&self) -> usize {
        self.inner.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[tokio::test]
    async fn open_caches_source_and_check() {
        let docs = Documents::new();
        docs.open(url("file:///a.kula"), "kula 1\nperson alice\n".into())
            .await;
        let stored_source = docs
            .with(&url("file:///a.kula"), |d| d.source.clone())
            .await
            .unwrap();
        assert_eq!(stored_source, "kula 1\nperson alice\n");
    }

    #[tokio::test]
    async fn update_replaces_check() {
        let docs = Documents::new();
        docs.open(url("file:///a.kula"), String::new()).await;
        docs.update(
            url("file:///a.kula"),
            "kula 1\nperson alice name:\"A\" gender:female\n".into(),
        )
        .await;
        let diag_count = docs
            .with(&url("file:///a.kula"), |d| d.check.diagnostics.len())
            .await
            .unwrap();
        // Cached check matches a fresh check of the same source.
        let fresh = kula_core::check("kula 1\nperson alice name:\"A\" gender:female\n");
        assert_eq!(diag_count, fresh.diagnostics.len());
    }

    #[tokio::test]
    async fn close_drops_document() {
        let docs = Documents::new();
        docs.open(url("file:///a.kula"), "kula 1\n".into()).await;
        assert_eq!(docs.open_count().await, 1);
        docs.close(&url("file:///a.kula")).await;
        assert_eq!(docs.open_count().await, 0);
    }

    #[tokio::test]
    async fn with_returns_none_for_unknown() {
        let docs = Documents::new();
        let got = docs.with(&url("file:///nope.kula"), |_| 1).await;
        assert!(got.is_none());
    }
}
