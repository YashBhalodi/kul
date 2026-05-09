//! Per-URI document cache.
//!
//! Holds the source text, the cached `kul_core::CheckResult` (or the
//! manifest-failure diagnostic that displaces it), and a `LineIndex` for
//! byte ↔ LSP-position conversion. All access goes through the `Documents`
//! handle so the locking story stays in one place.
//!
//! The source is stored as an [`Arc<str>`] shared with the [`LineIndex`] so
//! a single heap buffer backs both fields.

use std::collections::HashMap;
use std::sync::Arc;

use kul_core::CheckResult;
use kul_core::manifest::Manifest;
use tokio::sync::RwLock;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range, Url};

use crate::convert::LineIndex;

/// Outcome of preparing a document on `did_open` / `did_change`.
///
/// On success we own the full pipeline result; on a manifest failure we
/// hold a synthetic diagnostic and skip semantic / validation. Parse-only
/// state still allows highlighting in the editor — the parser runs without
/// a manifest dependency.
#[derive(Debug, Clone)]
pub enum DocumentState {
    Ok(CheckResult),
    ManifestFailed(Diagnostic),
}

/// One open document.
#[derive(Debug, Clone)]
pub struct Document {
    pub source: Arc<str>,
    pub line_index: LineIndex,
    pub state: DocumentState,
}

impl Document {
    fn from_source(source: String, manifest: Result<Manifest, String>) -> Self {
        let source: Arc<str> = Arc::from(source);
        let line_index = LineIndex::new(Arc::clone(&source));
        let state = match manifest {
            Ok(manifest) => DocumentState::Ok(kul_core::check(&source, &manifest)),
            Err(message) => DocumentState::ManifestFailed(synthetic_manifest_diagnostic(message)),
        };
        Self {
            source,
            line_index,
            state,
        }
    }

    /// The cached `CheckResult` if the manifest loaded successfully, else
    /// `None`. Feature handlers that need semantic info should bail when
    /// this is `None`; the synthetic diagnostic explains the manifest
    /// failure to the user.
    pub fn check(&self) -> Option<&CheckResult> {
        match &self.state {
            DocumentState::Ok(c) => Some(c),
            DocumentState::ManifestFailed(_) => None,
        }
    }

    /// LSP diagnostics to publish for this document. Either the
    /// `kul-core` diagnostics translated by the caller, or the single
    /// synthetic manifest-failure diagnostic.
    pub fn manifest_diagnostic(&self) -> Option<&Diagnostic> {
        match &self.state {
            DocumentState::ManifestFailed(d) => Some(d),
            DocumentState::Ok(_) => None,
        }
    }
}

fn synthetic_manifest_diagnostic(message: String) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position::new(0, 0),
            end: Position::new(0, 1),
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: None,
        code_description: None,
        source: Some("kul".to_string()),
        message,
        related_information: None,
        tags: None,
        data: None,
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
        let manifest = crate::manifest::load_for(&uri).map_err(|e| e.message());
        let mut map = self.inner.write().await;
        map.insert(uri, Document::from_source(source, manifest));
    }

    /// Replace the cached document for `uri` with a freshly-parsed one. Used
    /// by `didChange` (full sync — the whole text is sent each time). The
    /// manifest is *not* re-loaded here; the cached one (or its failure)
    /// from `did_open` is reused.
    pub async fn update(&self, uri: Url, source: String) {
        let manifest = {
            let map = self.inner.read().await;
            match map.get(&uri).map(|d| &d.state) {
                Some(DocumentState::Ok(c)) => Ok(c.manifest.clone()),
                Some(DocumentState::ManifestFailed(d)) => Err(d.message.clone()),
                None => crate::manifest::load_for(&uri).map_err(|e| e.message()),
            }
        };
        let mut map = self.inner.write().await;
        map.insert(uri, Document::from_source(source, manifest));
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

    fn make_document(source: &str) -> Document {
        Document::from_source(source.to_string(), Ok(Manifest::default()))
    }

    #[tokio::test]
    async fn document_caches_source_and_check() {
        let docs = Documents::default();
        let doc = make_document("person alice name:\"A\" gender:female\n");
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
    async fn manifest_failure_displaces_check() {
        let doc = Document::from_source(
            "person alice name:\"A\" gender:female\n".to_string(),
            Err("missing manifest".to_string()),
        );
        assert!(doc.check().is_none());
        let diag = doc.manifest_diagnostic().expect("synthetic diagnostic");
        assert_eq!(diag.message, "missing manifest");
        assert_eq!(diag.range.start.line, 0);
        assert_eq!(diag.range.start.character, 0);
        assert_eq!(diag.range.end.character, 1);
    }

    #[tokio::test]
    async fn close_drops_document() {
        let docs = Documents::default();
        let mut map = docs.inner.write().await;
        map.insert(url("file:///a.kul"), make_document(""));
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
