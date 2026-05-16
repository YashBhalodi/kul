//! "What's at this byte offset?" query — the foundation that hover,
//! go-to-definition, and completion all build on.
//!
//! Returning a typed enum (rather than the raw AST plus a "what kind" tag)
//! lets each LSP feature pattern-match cleanly without re-walking the tree.
//! The function lives on [`ResolvedDocument`] (per ADR-0001) so reference
//! variants can carry the resolved target alongside the source ident.
//!
//! Resolution rule: smallest enclosing span wins. The walk descends into
//! the most specific child whose span contains the offset; whitespace and
//! comments inside a statement still yield `None`.
//!
//! `node_at` and `statement_at` keep their [`FileId`] parameter because
//! byte offsets are inherently per-file. Reference targets resolve
//! project-wide (per ADR-0015): a spouse spelled in one file's marriage
//! can point at a person declared in a sibling file.

use crate::ast::{
    AdoptionField, AdoptionSub, BirthSub, Ident, MarriageField, MarriageStmt, PersonField,
    PersonStmt, Statement,
};
use crate::semantic::{EntityKind, ResolvedDocument};
use crate::span::{ByteSpan, FileId, FileSpan};

/// A keyword token: one of the fixed words in Kul's grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeywordKind {
    /// `person` (top-level statement).
    Person,
    /// `marriage` (top-level statement).
    Marriage,
    /// `birth` (sub-statement of `person`).
    Birth,
    /// `adoption` (sub-statement of `person`).
    Adoption,
}

/// The AST element under a byte offset.
///
/// Variants carry references to the underlying AST nodes so callers don't
/// have to re-walk; reference variants additionally carry the resolved
/// target (or `None` if the reference is unresolved — rule 2 reports it).
#[derive(Debug, Clone)]
pub enum Node<'a> {
    /// A keyword token (e.g. `person`, `birth`). The span identifies the
    /// keyword's source range so callers can highlight the right token.
    Keyword(KeywordKind, ByteSpan),
    /// The id token of a `person` declaration.
    PersonDeclId(&'a PersonStmt),
    /// The id token of a `marriage` declaration.
    MarriageDeclId(&'a MarriageStmt),
    /// A reference to a person (currently: spouse positions in a marriage).
    PersonRef {
        ident: &'a Ident,
        target: Option<&'a PersonStmt>,
    },
    /// A reference to a marriage (in `birth` or `adoption` sub-statements).
    MarriageRef {
        ident: &'a Ident,
        target: Option<&'a MarriageStmt>,
    },
    /// The lhs (`name:`, `gender:`, …) of a person field.
    PersonFieldName(&'a PersonField),
    /// The value side of a person field — anything in the field's span
    /// that isn't part of the lhs name.
    PersonFieldValue(&'a PersonField),
    /// The lhs (`start:`, `end:`, `end_reason:`) of a marriage field.
    MarriageFieldName(&'a MarriageField),
    /// The value side of a marriage field.
    MarriageFieldValue(&'a MarriageField),
    /// The lhs (`start:`, `end:`) of an adoption field.
    AdoptionFieldName(&'a AdoptionField),
    /// The value side of an adoption field.
    AdoptionFieldValue(&'a AdoptionField),
}

/// The resolved entity — a person or a marriage — referred to from a
/// [`Node`]. Sibling type to [`EntityNode`].
#[derive(Debug, Clone, Copy)]
pub enum EntityTarget<'a> {
    Person(&'a PersonStmt),
    Marriage(&'a MarriageStmt),
}

impl<'a> EntityTarget<'a> {
    /// Span of the entity's declaration id (the id token after `person` /
    /// `marriage`). The anchor LSP features (definition, rename) point at.
    pub fn decl_span(self) -> ByteSpan {
        match self {
            EntityTarget::Person(p) => p.id.span,
            EntityTarget::Marriage(m) => m.id.span,
        }
    }
}

/// The "entity reference at the cursor" summary: `Some` when the cursor is
/// on a person/marriage id (decl or reference), `None` otherwise (keyword,
/// field, version literal, …).
///
/// Returned by [`Node::entity_reference`]. The LSP features that key on
/// "what entity is at the cursor?" (goto-definition, find-references,
/// rename) all phrase themselves as a query for this summary.
#[derive(Debug, Clone, Copy)]
pub struct EntityNode<'a> {
    /// Kind of entity the cursor is on.
    pub kind: EntityKind,
    /// The id text under the cursor — the decl id for a decl, or the
    /// reference's spelling for a reference (which may not match any
    /// declaration if unresolved).
    pub name: &'a str,
    /// Source span of the id under the cursor as a project-wide
    /// [`FileSpan`] (the span the LSP rename popover should highlight).
    pub ident_span: FileSpan,
    /// `true` if the cursor is on the declaration site itself; `false` if
    /// on a reference. A decl always has a target; an unresolved reference
    /// has none.
    pub is_decl: bool,
    /// The resolved entity, if any. `None` only for unresolved references.
    /// The target may live in a sibling file under project-wide
    /// resolution (per ADR-0015).
    pub target: Option<EntityTarget<'a>>,
}

impl<'a> EntityNode<'a> {
    /// Span of the declaration id of the referenced entity, or `None` for
    /// an unresolved reference. For a decl this is the span the cursor is
    /// already on; for a resolved reference, the corresponding declaration.
    /// Returned as a [`FileSpan`] anchored on the containing file.
    pub fn decl_span(&self) -> Option<FileSpan> {
        if self.is_decl {
            return Some(self.ident_span);
        }
        let target = self.target?;
        Some(FileSpan::new(self.ident_span.file, target.decl_span()))
    }
}

impl ResolvedDocument {
    /// What's at `byte_offset` inside `file`?
    ///
    /// Spans are half-open: a span `[s, e)` contains `offset` iff
    /// `s <= offset < e`. Returns `None` for whitespace, comments,
    /// out-of-range offsets, or out-of-range files. Smallest enclosing
    /// span wins.
    ///
    /// # Example
    ///
    /// ```
    /// use std::sync::Arc;
    /// use kul_core::ast::{Document, KulFile};
    /// use kul_core::lexer::tokenize;
    /// use kul_core::parser::parse;
    /// use kul_core::semantic::resolve;
    /// use kul_core::node_at::{KeywordKind, Node};
    /// use kul_core::span::FileId;
    ///
    /// let source = "person alice name:\"Alice\" gender:female\n";
    /// let file = FileId::MANIFEST; // placeholder for doctest
    /// let kul_file = FileId(1);
    /// let tokens = tokenize(source);
    /// let (statements, _) = parse(&tokens, kul_file);
    /// let kf = Arc::new(KulFile {
    ///     name: "test.kul".to_string(),
    ///     source: source.to_string(),
    ///     statements,
    /// });
    /// let document = Arc::new(Document {
    ///     manifest_name: "kul.yml".to_string(),
    ///     manifest_source: String::new(),
    ///     kul_files: vec![kf],
    /// });
    /// let (resolved, _) = resolve(document);
    ///
    /// let node = resolved.node_at(kul_file, 0).expect("a node");
    /// assert!(matches!(node, Node::Keyword(KeywordKind::Person, _)));
    /// let _ = file; // silence unused
    /// ```
    pub fn node_at(&self, file: FileId, byte_offset: usize) -> Option<Node<'_>> {
        let kf = self.document().kul_file(file)?;
        for stmt in &kf.statements {
            match stmt {
                Statement::Person(p) if contains(p.span, byte_offset) => {
                    return self.node_in_person(file, p, byte_offset);
                }
                Statement::Marriage(m) if contains(m.span, byte_offset) => {
                    return self.node_in_marriage(file, m, byte_offset);
                }
                _ => continue,
            }
        }
        None
    }

    fn node_in_person<'a>(
        &'a self,
        file: FileId,
        p: &'a PersonStmt,
        offset: usize,
    ) -> Option<Node<'a>> {
        if contains(p.keyword_span, offset) {
            return Some(Node::Keyword(KeywordKind::Person, p.keyword_span));
        }
        if contains(p.id.span, offset) {
            return Some(Node::PersonDeclId(p));
        }
        for f in &p.fields {
            if contains(f.span, offset) {
                return Some(if contains(f.name_span, offset) {
                    Node::PersonFieldName(f)
                } else {
                    Node::PersonFieldValue(f)
                });
            }
        }
        if let Some(b) = &p.birth
            && contains(b.span, offset)
        {
            return self.node_in_birth(file, b, offset);
        }
        for adopt in &p.adoptions {
            if contains(adopt.span, offset) {
                return self.node_in_adoption(file, adopt, offset);
            }
        }
        None
    }

    fn node_in_birth<'a>(
        &'a self,
        _file: FileId,
        b: &'a BirthSub,
        offset: usize,
    ) -> Option<Node<'a>> {
        if contains(b.keyword_span, offset) {
            return Some(Node::Keyword(KeywordKind::Birth, b.keyword_span));
        }
        if contains(b.marriage_ref.span, offset) {
            return Some(Node::MarriageRef {
                ident: &b.marriage_ref,
                target: self.marriage(&b.marriage_ref.name),
            });
        }
        None
    }

    fn node_in_adoption<'a>(
        &'a self,
        _file: FileId,
        a: &'a AdoptionSub,
        offset: usize,
    ) -> Option<Node<'a>> {
        if contains(a.keyword_span, offset) {
            return Some(Node::Keyword(KeywordKind::Adoption, a.keyword_span));
        }
        if contains(a.marriage_ref.span, offset) {
            return Some(Node::MarriageRef {
                ident: &a.marriage_ref,
                target: self.marriage(&a.marriage_ref.name),
            });
        }
        for f in &a.fields {
            if contains(f.span, offset) {
                return Some(if contains(f.name_span, offset) {
                    Node::AdoptionFieldName(f)
                } else {
                    Node::AdoptionFieldValue(f)
                });
            }
        }
        None
    }

    fn node_in_marriage<'a>(
        &'a self,
        _file: FileId,
        m: &'a MarriageStmt,
        offset: usize,
    ) -> Option<Node<'a>> {
        if contains(m.keyword_span, offset) {
            return Some(Node::Keyword(KeywordKind::Marriage, m.keyword_span));
        }
        if contains(m.id.span, offset) {
            return Some(Node::MarriageDeclId(m));
        }
        if contains(m.spouse_a.span, offset) {
            return Some(Node::PersonRef {
                ident: &m.spouse_a,
                target: self.person(&m.spouse_a.name),
            });
        }
        if contains(m.spouse_b.span, offset) {
            return Some(Node::PersonRef {
                ident: &m.spouse_b,
                target: self.person(&m.spouse_b.name),
            });
        }
        for f in &m.fields {
            if contains(f.span, offset) {
                return Some(if contains(f.name_span, offset) {
                    Node::MarriageFieldName(f)
                } else {
                    Node::MarriageFieldValue(f)
                });
            }
        }
        None
    }
}

impl<'a> Node<'a> {
    /// If the cursor sits on a person/marriage id (a declaration site or a
    /// reference site), returns the [`EntityNode`] summary anchored at
    /// `file`. Returns `None` for keywords, field names/values, the
    /// version literal, and other non-id positions.
    pub fn entity_reference(&self, file: FileId) -> Option<EntityNode<'a>> {
        match *self {
            Node::PersonDeclId(p) => Some(EntityNode {
                kind: EntityKind::Person,
                name: p.id.name.as_str(),
                ident_span: FileSpan::new(file, p.id.span),
                is_decl: true,
                target: Some(EntityTarget::Person(p)),
            }),
            Node::MarriageDeclId(m) => Some(EntityNode {
                kind: EntityKind::Marriage,
                name: m.id.name.as_str(),
                ident_span: FileSpan::new(file, m.id.span),
                is_decl: true,
                target: Some(EntityTarget::Marriage(m)),
            }),
            Node::PersonRef { ident, target } => Some(EntityNode {
                kind: EntityKind::Person,
                name: ident.name.as_str(),
                ident_span: FileSpan::new(file, ident.span),
                is_decl: false,
                target: target.map(EntityTarget::Person),
            }),
            Node::MarriageRef { ident, target } => Some(EntityNode {
                kind: EntityKind::Marriage,
                name: ident.name.as_str(),
                ident_span: FileSpan::new(file, ident.span),
                is_decl: false,
                target: target.map(EntityTarget::Marriage),
            }),
            _ => None,
        }
    }
}

fn contains(span: ByteSpan, offset: usize) -> bool {
    offset >= span.start && offset < span.end
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{AdoptionFieldKind, Document, KulFile, MarriageFieldKind, PersonFieldKind};
    use crate::semantic::resolve;
    use std::sync::Arc;

    fn build(source: &str) -> (ResolvedDocument, FileId) {
        let file = FileId(1);
        let tokens = crate::lexer::tokenize(source);
        let (statements, _) = crate::parser::parse(&tokens, file);
        let kf = Arc::new(KulFile {
            name: "test.kul".to_string(),
            source: source.to_string(),
            statements,
        });
        let document = Arc::new(Document {
            manifest_name: "kul.yml".to_string(),
            manifest_source: String::new(),
            kul_files: vec![kf],
        });
        let (resolved, _) = resolve(document);
        (resolved, file)
    }

    /// Owned mirror of `Node` for assertion ergonomics.
    #[derive(Debug, PartialEq, Eq)]
    enum Probe {
        None,
        Keyword(KeywordKind),
        PersonDeclId(String),
        MarriageDeclId(String),
        PersonRef { name: String, resolved: bool },
        MarriageRef { name: String, resolved: bool },
        PersonFieldName(String),
        PersonFieldValue(String),
        MarriageFieldName(String),
        MarriageFieldValue(String),
        AdoptionFieldName(String),
        AdoptionFieldValue(String),
    }

    impl From<Option<Node<'_>>> for Probe {
        fn from(node: Option<Node<'_>>) -> Self {
            match node {
                None => Probe::None,
                Some(Node::Keyword(k, _)) => Probe::Keyword(k),
                Some(Node::PersonDeclId(p)) => Probe::PersonDeclId(p.id.name.clone()),
                Some(Node::MarriageDeclId(m)) => Probe::MarriageDeclId(m.id.name.clone()),
                Some(Node::PersonRef { ident, target }) => Probe::PersonRef {
                    name: ident.name.clone(),
                    resolved: target.is_some(),
                },
                Some(Node::MarriageRef { ident, target }) => Probe::MarriageRef {
                    name: ident.name.clone(),
                    resolved: target.is_some(),
                },
                Some(Node::PersonFieldName(f)) => {
                    Probe::PersonFieldName(person_field_label(&f.kind).into())
                }
                Some(Node::PersonFieldValue(f)) => {
                    Probe::PersonFieldValue(person_field_label(&f.kind).into())
                }
                Some(Node::MarriageFieldName(f)) => {
                    Probe::MarriageFieldName(marriage_field_label(&f.kind).into())
                }
                Some(Node::MarriageFieldValue(f)) => {
                    Probe::MarriageFieldValue(marriage_field_label(&f.kind).into())
                }
                Some(Node::AdoptionFieldName(f)) => {
                    Probe::AdoptionFieldName(adoption_field_label(&f.kind).into())
                }
                Some(Node::AdoptionFieldValue(f)) => {
                    Probe::AdoptionFieldValue(adoption_field_label(&f.kind).into())
                }
            }
        }
    }

    fn person_field_label(k: &PersonFieldKind) -> &'static str {
        match k {
            PersonFieldKind::Name(_) => "name",
            PersonFieldKind::Family(_) => "family",
            PersonFieldKind::Given(_) => "given",
            PersonFieldKind::Born(_) => "born",
            PersonFieldKind::Died(_) => "died",
            PersonFieldKind::Gender(_) => "gender",
        }
    }

    fn marriage_field_label(k: &MarriageFieldKind) -> &'static str {
        match k {
            MarriageFieldKind::Start(_) => "start",
            MarriageFieldKind::End(_) => "end",
            MarriageFieldKind::EndReason(_) => "end_reason",
        }
    }

    fn adoption_field_label(k: &AdoptionFieldKind) -> &'static str {
        match k {
            AdoptionFieldKind::Start(_) => "start",
            AdoptionFieldKind::End(_) => "end",
        }
    }

    fn at(source: &str, offset: usize) -> Probe {
        let (resolved, file) = build(source);
        Probe::from(resolved.node_at(file, offset))
    }

    fn idx(source: &str, pat: &str) -> usize {
        source.find(pat).expect("pattern in source")
    }

    #[test]
    fn person_keyword_id_and_field_sides() {
        let src = "person alice name:\"Alice\" gender:female\n";
        assert_eq!(
            at(src, idx(src, "person")),
            Probe::Keyword(KeywordKind::Person),
        );
        assert_eq!(
            at(src, idx(src, "alice")),
            Probe::PersonDeclId("alice".into()),
        );
        assert_eq!(
            at(src, idx(src, "name:")),
            Probe::PersonFieldName("name".into()),
        );
        assert_eq!(
            at(src, idx(src, "\"Alice\"")),
            Probe::PersonFieldValue("name".into()),
        );
        assert_eq!(
            at(src, idx(src, "gender:")),
            Probe::PersonFieldName("gender".into()),
        );
        assert_eq!(
            at(src, idx(src, "female")),
            Probe::PersonFieldValue("gender".into()),
        );
    }

    #[test]
    fn unresolved_person_ref_target_is_none() {
        let src = "marriage m ghost b start:2000\nperson b name:\"B\" gender:male\n";
        let marriage_line = idx(src, "marriage ");
        let ghost = src[marriage_line..]
            .find("ghost")
            .map(|i| marriage_line + i)
            .unwrap();
        assert_eq!(
            at(src, ghost),
            Probe::PersonRef {
                name: "ghost".into(),
                resolved: false,
            },
        );
    }

    #[test]
    fn unresolved_marriage_ref_target_is_none() {
        let src = "person a name:\"A\" gender:female\n  birth m_nope\n";
        assert_eq!(
            at(src, idx(src, "m_nope")),
            Probe::MarriageRef {
                name: "m_nope".into(),
                resolved: false,
            },
        );
    }

    #[test]
    fn span_boundary_start_inclusive_end_exclusive() {
        let src = "person alice name:\"Alice\" gender:female\n";
        assert_eq!(at(src, 0), Probe::Keyword(KeywordKind::Person));
        assert_eq!(at(src, 5), Probe::Keyword(KeywordKind::Person));
        assert_eq!(at(src, 6), Probe::None);
    }

    #[test]
    fn whitespace_between_top_level_statements_is_none() {
        let src = "person a name:\"A\" gender:female\n\
                   \n\
                   person b name:\"B\" gender:male\n";
        let blank_line = src.find("\n\n").unwrap() + 1;
        assert_eq!(at(src, blank_line), Probe::None);
    }

    #[test]
    fn past_eof_is_none() {
        let src = "person alice name:\"A\" gender:female\n";
        assert_eq!(at(src, src.len()), Probe::None);
        assert_eq!(at(src, src.len() + 999), Probe::None);
    }

    fn entity_at(source: &str, offset: usize) -> Option<(EntityKind, String, bool, bool)> {
        let (resolved, file) = build(source);
        resolved
            .node_at(file, offset)
            .and_then(|n| n.entity_reference(file))
            .map(|e| (e.kind, e.name.to_owned(), e.is_decl, e.target.is_some()))
    }

    #[test]
    fn entity_reference_on_person_decl_returns_resolved_decl() {
        let src = "person alice name:\"A\" gender:female\n";
        let got = entity_at(src, idx(src, "alice")).unwrap();
        assert_eq!(got, (EntityKind::Person, "alice".into(), true, true));
    }

    #[test]
    fn entity_reference_on_unresolved_ref_has_no_target() {
        let src = "marriage m ghost b start:2000\nperson b name:\"B\" gender:male\n";
        let marriage_line = idx(src, "marriage ");
        let ghost = src[marriage_line..]
            .find("ghost")
            .map(|i| marriage_line + i)
            .unwrap();
        let got = entity_at(src, ghost).unwrap();
        assert_eq!(got, (EntityKind::Person, "ghost".into(), false, false));
    }

    #[test]
    fn entity_reference_returns_none_for_keywords_and_fields() {
        let src = "person alice name:\"A\" gender:female\n";
        assert!(entity_at(src, 0).is_none());
        assert!(entity_at(src, idx(src, "name:")).is_none());
        assert!(entity_at(src, idx(src, "\"A\"")).is_none());
    }
}
