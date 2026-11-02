//! Kul in the browser: a stateful [`Workspace`] over one project's files.
//!
//! The workspace holds the project's `.kul` files, checks them on demand and
//! caches the [`CheckResult`] (and with it the `ResolvedDocument`, ADR-0007),
//! so a hover or a completion after an edit pays one check, not one per
//! request. Every editor feature is a thin call into `kul-language-service`;
//! rendering goes through `kul_visual::render_from_check` (ADR-0031). See
//! ADR-0011 for the surface and ADR-0012 for the derived TypeScript types.
//!
//! **Offsets.** Rust counts UTF-8 bytes; JavaScript counts UTF-16 code units.
//! Every offset crossing this boundary, in either direction, is a UTF-16
//! code-unit offset into that file's text, converted here and nowhere else.
//!
//! `console_error_panic_hook` is installed when a workspace is created, so a
//! bug surfaces as a readable console error.

mod utf16;

use std::cell::OnceCell;
use std::collections::BTreeMap;
use std::sync::Arc;

use kul_core::CheckResult;
use kul_core::ast::InputFile;
use kul_core::export::ExportedDiagnostic;
use kul_core::query::EntityDetail;
use kul_language_service::{
    self as service, CompletionItem, Diagnostic, Entity, FileEdit, Hover, Location, OutlineSymbol,
    QuickFix, Related, Severity, Span, TextEdit, Token,
};
use kul_visual::{ThemeConfig, render_from_check};
use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use utf16::Utf16Index;

/// The version of the Kul toolchain this module was built from.
#[wasm_bindgen(js_name = "KUL_CORE_VERSION")]
pub fn kul_core_version() -> String {
    kul_core::VERSION.into()
}

/// The version of the Kul language this module implements.
#[wasm_bindgen(js_name = "KUL_LANGUAGE_VERSION")]
pub fn kul_language_version() -> String {
    kul_core::export::LANGUAGE_VERSION.into()
}

/// One analysis of the whole project: what was analyzed, what is wrong with
/// it, and the rendered tree.
#[derive(Debug, Clone, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct Analysis {
    /// Every file, in file order, with the version of the text analyzed —
    /// compare it with the latest version sent to discard a stale answer.
    pub files: Vec<FileVersion>,
    /// Every diagnostic of the project. Filter by `location.file` for one
    /// file's; a diagnostic without a location concerns the whole project.
    pub diagnostics: Vec<Diagnostic>,
    /// The theme-agnostic SVG of the family tree (semantic CSS classes, no
    /// colours). Absent when the project has errors, or cannot be drawn.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub svg: Option<String>,
}

/// A file name and the version of its text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct FileVersion {
    /// The file's name.
    pub name: String,
    /// The version passed with its text to `setFile`.
    pub version: u32,
}

/// The answer to `rename`: the edits to apply, or why the rename was
/// refused.
#[derive(Debug, Clone, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
#[serde(untagged)]
pub enum RenameResult {
    /// The rename is valid.
    Renamed {
        /// Always `true`.
        #[tsify(type = "true")]
        ok: bool,
        /// One entry per affected file, in file order. Apply them all
        /// together.
        edits: Vec<FileEdit>,
    },
    /// The rename was refused.
    Refused {
        /// Always `false`.
        #[tsify(type = "false")]
        ok: bool,
        /// Why, written for the author.
        message: String,
    },
}

/// How `renderSvg` themes the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Tsify)]
#[tsify(from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub enum SvgTheme {
    /// Semantic CSS classes and no colours, for a page whose stylesheet themes
    /// it (the same SVG `analyze` returns).
    Agnostic,
    /// Self-contained, for download: the light palette baked in an inline
    /// style, plus a legend.
    FileExport,
}

struct SourceFile {
    text: Arc<str>,
    version: u32,
    offsets: OnceCell<Utf16Index>,
}

impl SourceFile {
    fn offsets(&self) -> &Utf16Index {
        self.offsets.get_or_init(|| Utf16Index::new(&self.text))
    }
}

/// One project's `.kul` files, with the language features over them.
///
/// Files are keyed by name; the project's file order is the byte-wise order
/// of those names. Edits replace a file's whole text. Analysis runs on
/// demand and is cached until the next edit.
#[wasm_bindgen]
pub struct Workspace {
    files: BTreeMap<String, SourceFile>,
    check: OnceCell<CheckResult>,
}

impl Default for Workspace {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl Workspace {
    /// An empty project.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new() -> Workspace {
        console_error_panic_hook::set_once();
        Workspace {
            files: BTreeMap::new(),
            check: OnceCell::new(),
        }
    }

    /// Add the file `name`, or replace its text. `version` tags the text: an
    /// analysis reports the version it saw for each file.
    #[wasm_bindgen(js_name = "setFile")]
    pub fn set_file(&mut self, name: String, text: String, version: u32) {
        self.files.insert(
            name,
            SourceFile {
                text: text.into(),
                version,
                offsets: OnceCell::new(),
            },
        );
        self.check = OnceCell::new();
    }

    /// Rename the file `from` to `to`, keeping its text and version. Returns
    /// `false`, changing nothing, when `from` does not exist or `to` does.
    #[wasm_bindgen(js_name = "renameFile")]
    pub fn rename_file(&mut self, from: &str, to: String) -> bool {
        if self.files.contains_key(&to) {
            return false;
        }
        let Some(file) = self.files.remove(from) else {
            return false;
        };
        self.files.insert(to, file);
        self.check = OnceCell::new();
        true
    }

    /// Remove the file `name`. Returns `false` when there is no such file.
    #[wasm_bindgen(js_name = "deleteFile")]
    pub fn delete_file(&mut self, name: &str) -> bool {
        let removed = self.files.remove(name).is_some();
        if removed {
            self.check = OnceCell::new();
        }
        removed
    }

    /// Check the project: its diagnostics, and its tree when it has none
    /// that are errors.
    pub fn analyze(&self) -> Analysis {
        let check = self.check();
        let mut diagnostics: Vec<Diagnostic> = service::diagnostics(check)
            .into_iter()
            .map(|d| self.diagnostic(d))
            .collect();
        let svg = match render_from_check(check, &ThemeConfig::default()) {
            Ok(svg) => Some(svg),
            Err(failure) => {
                // A project that passes its checks can still be too deep to
                // draw (KUL-V01); only those render failures are new.
                if !check.has_errors() {
                    diagnostics.extend(failure.into_iter().map(|d| self.render_diagnostic(d)));
                }
                None
            }
        };
        Analysis {
            files: self
                .files
                .iter()
                .map(|(name, file)| FileVersion {
                    name: name.clone(),
                    version: file.version,
                })
                .collect(),
            diagnostics,
            svg,
        }
    }

    /// The SVG of the family tree in `theme`, or `undefined` when the project
    /// has errors or cannot be drawn.
    #[wasm_bindgen(js_name = "renderSvg")]
    pub fn render_svg(&self, theme: SvgTheme) -> Option<String> {
        let theme = match theme {
            SvgTheme::Agnostic => ThemeConfig::default(),
            SvgTheme::FileExport => ThemeConfig::for_file_export(),
        };
        render_from_check(self.check(), &theme).ok()
    }

    /// Hover content for `offset` in `file`.
    pub fn hover(&self, file: &str, offset: u32) -> Option<Hover> {
        let at = self.byte(file, offset)?;
        let Hover { markdown, span } = service::hover(self.check(), file, at)?;
        Some(Hover {
            markdown,
            span: self.span(file, span),
        })
    }

    /// The completions at `offset` in `file`: the full set for the position,
    /// to be filtered by the word already typed.
    pub fn complete(&self, file: &str, offset: u32) -> Vec<CompletionItem> {
        let Some(at) = self.byte(file, offset) else {
            return Vec::new();
        };
        service::complete(self.check(), file, at)
            .into_iter()
            .map(completion_item)
            .collect()
    }

    /// The declaration the reference at `offset` in `file` names.
    pub fn definition(&self, file: &str, offset: u32) -> Option<Location> {
        let at = self.byte(file, offset)?;
        service::definition(self.check(), file, at).map(|l| self.location(l))
    }

    /// Every use of the id at `offset` in `file`, optionally with its
    /// declaration; `undefined` when the cursor is not on an id.
    pub fn references(
        &self,
        file: &str,
        offset: u32,
        #[wasm_bindgen(js_name = "includeDeclaration")] include_declaration: bool,
    ) -> Option<Vec<Location>> {
        let at = self.byte(file, offset)?;
        let found = service::references(self.check(), file, at, include_declaration)?;
        Some(found.into_iter().map(|l| self.location(l)).collect())
    }

    /// The range a rename at `offset` in `file` would change, or `undefined`
    /// when nothing there can be renamed.
    #[wasm_bindgen(js_name = "prepareRename")]
    pub fn prepare_rename(&self, file: &str, offset: u32) -> Option<Span> {
        let at = self.byte(file, offset)?;
        service::prepare_rename(self.check(), file, at).map(|s| self.span(file, s))
    }

    /// Rename the id at `offset` in `file` to `newName`, across the project.
    pub fn rename(
        &self,
        file: &str,
        offset: u32,
        #[wasm_bindgen(js_name = "newName")] new_name: &str,
    ) -> RenameResult {
        let renamed = self
            .byte(file, offset)
            .ok_or(service::RenameError::NotRenameable)
            .and_then(|at| service::rename(self.check(), file, at, new_name));
        match renamed {
            Ok(edits) => RenameResult::Renamed {
                ok: true,
                edits: edits.into_iter().map(|e| self.file_edit(e)).collect(),
            },
            Err(e) => RenameResult::Refused {
                ok: false,
                message: e.message(),
            },
        }
    }

    /// The outline of `file`.
    pub fn symbols(&self, file: &str) -> Vec<OutlineSymbol> {
        service::symbols(self.check(), file)
            .into_iter()
            .map(|s| self.symbol(file, s))
            .collect()
    }

    /// The quick fixes for diagnostics in `file` touching `start..end`.
    #[wasm_bindgen(js_name = "quickFixes")]
    pub fn quick_fixes(&self, file: &str, start: u32, end: u32) -> Vec<QuickFix> {
        let (Some(start), Some(end)) = (self.byte(file, start), self.byte(file, end)) else {
            return Vec::new();
        };
        service::quick_fixes(self.check(), file, Span { start, end })
            .into_iter()
            .map(
                |QuickFix {
                     title,
                     diagnostic,
                     edits,
                 }| QuickFix {
                    title,
                    diagnostic: self.diagnostic(diagnostic),
                    edits: edits.into_iter().map(|e| self.edit(file, e)).collect(),
                },
            )
            .collect()
    }

    /// Highlighting for `file` from its syntax tree. Paint these over
    /// `lexicalTokens` where both cover the same text.
    #[wasm_bindgen(js_name = "semanticTokens")]
    pub fn semantic_tokens(&self, file: &str) -> Vec<Token> {
        service::semantic_tokens(self.check(), file)
            .into_iter()
            .map(|t| self.token(file, t))
            .collect()
    }

    /// Highlighting for `file` from the lexer alone. Needs no analysis, so it
    /// is cheap enough for every keystroke.
    #[wasm_bindgen(js_name = "lexicalTokens")]
    pub fn lexical_tokens(&self, file: &str) -> Vec<Token> {
        let Some(source) = self.files.get(file) else {
            return Vec::new();
        };
        service::lexical_tokens(&source.text)
            .into_iter()
            .map(|t| self.token(file, t))
            .collect()
    }

    /// The edits that format `file` canonically: none when it already is,
    /// `undefined` when it has syntax errors.
    pub fn format(&self, file: &str) -> Option<Vec<TextEdit>> {
        let edits = service::format(self.check(), file)?;
        Some(edits.into_iter().map(|e| self.edit(file, e)).collect())
    }

    /// The entity whose id sits at `offset` in `file`.
    #[wasm_bindgen(js_name = "entityAt")]
    pub fn entity_at(&self, file: &str, offset: u32) -> Option<Entity> {
        let at = self.byte(file, offset)?;
        service::entity_at(self.check(), file, at)
    }

    /// The declaration of the person or marriage `id`.
    pub fn locate(&self, id: &str) -> Option<Location> {
        service::locate(self.check(), id).map(|l| self.location(l))
    }

    /// The facts about the person or marriage `id`, for a details panel;
    /// `undefined` when there is no such entity or the project has errors.
    pub fn details(&self, id: &str) -> Option<EntityDetail> {
        service::details(self.check(), id)
    }
}

impl Workspace {
    /// The cached check of the current files.
    fn check(&self) -> &CheckResult {
        self.check.get_or_init(|| {
            let inputs: Vec<InputFile> = self
                .files
                .iter()
                .map(|(name, file)| InputFile::new(name.clone(), Arc::clone(&file.text)))
                .collect();
            kul_core::check(&inputs)
        })
    }

    fn offsets(&self, file: &str) -> &Utf16Index {
        self.files
            .get(file)
            .expect("every result names a workspace file")
            .offsets()
    }

    /// A JavaScript offset into `file` as a byte offset.
    fn byte(&self, file: &str, offset: u32) -> Option<usize> {
        let source = self.files.get(file)?;
        Some(source.offsets().to_byte(offset as usize))
    }

    fn span(&self, file: &str, Span { start, end }: Span) -> Span {
        let offsets = self.offsets(file);
        Span {
            start: offsets.to_utf16(start),
            end: offsets.to_utf16(end),
        }
    }

    fn location(&self, Location { file, span }: Location) -> Location {
        let span = self.span(&file, span);
        Location { file, span }
    }

    fn edit(&self, file: &str, TextEdit { span, text }: TextEdit) -> TextEdit {
        TextEdit {
            span: self.span(file, span),
            text,
        }
    }

    fn file_edit(&self, FileEdit { file, edits }: FileEdit) -> FileEdit {
        let edits = edits.into_iter().map(|e| self.edit(&file, e)).collect();
        FileEdit { file, edits }
    }

    fn token(&self, file: &str, Token { span, kind }: Token) -> Token {
        Token {
            span: self.span(file, span),
            kind,
        }
    }

    fn symbol(&self, file: &str, s: OutlineSymbol) -> OutlineSymbol {
        let OutlineSymbol {
            name,
            detail,
            kind,
            span,
            selection,
            children,
        } = s;
        OutlineSymbol {
            name,
            detail,
            kind,
            span: self.span(file, span),
            selection: self.span(file, selection),
            children: children.into_iter().map(|c| self.symbol(file, c)).collect(),
        }
    }

    fn diagnostic(&self, d: Diagnostic) -> Diagnostic {
        let Diagnostic {
            code,
            severity,
            message,
            location,
            related,
        } = d;
        Diagnostic {
            code,
            severity,
            message,
            location: location.map(|l| self.location(l)),
            related: related
                .into_iter()
                .map(|Related { message, location }| Related {
                    message,
                    location: self.location(location),
                })
                .collect(),
        }
    }

    /// A render-shape diagnostic (`KUL-V*`) in the language service's shape.
    fn render_diagnostic(&self, d: ExportedDiagnostic) -> Diagnostic {
        let severity = match d.severity {
            "warning" => Severity::Warning,
            "note" => Severity::Note,
            _ => Severity::Error,
        };
        let location = |file: String, start, end| {
            self.location(Location {
                file,
                span: Span { start, end },
            })
        };
        Diagnostic {
            code: d.code,
            severity,
            message: d.message,
            location: d
                .primary
                .map(|p| location(p.file, p.byte_start, p.byte_end)),
            related: d
                .related
                .into_iter()
                .map(|r| Related {
                    message: r.label,
                    location: location(r.span.file, r.span.byte_start, r.span.byte_end),
                })
                .collect(),
        }
    }
}

/// Completion items carry no file offsets; an insertion's cursor is an
/// offset into its own (ASCII) text, converted all the same.
fn completion_item(item: CompletionItem) -> CompletionItem {
    let insert = item.insert.map(|mut insertion| {
        insertion.cursor = insertion.text[..insertion.cursor].encode_utf16().count();
        insertion
    });
    CompletionItem { insert, ..item }
}
