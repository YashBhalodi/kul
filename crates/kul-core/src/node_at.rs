//! "What's at this byte offset?" — the foundation for hover, goto-def,
//! and completion. Lives on [`ResolvedDocument`] (ADR-0001) so reference
//! variants carry the resolved target.
//!
//! Smallest enclosing span wins. Whitespace and comments yield `None`.
//! Reference targets resolve project-wide (ADR-0015).

use crate::ast::{
    AdoptionField, AdoptionSub, BirthSub, Ident, MarriageField, MarriageStmt, PersonField,
    PersonStmt, Statement,
};
use crate::lexer::FieldName;
use crate::semantic::{EntityKind, ResolvedDocument};
use crate::span::{ByteSpan, FileId, FileSpan};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeywordKind {
    Person,
    Marriage,
    Birth,
    Adoption,
}

/// AST element under a byte offset. Reference variants carry the resolved
/// target, or `None` for unresolved (R02).
#[derive(Debug, Clone)]
pub enum Node<'a> {
    Keyword(KeywordKind, ByteSpan),
    PersonDeclId(&'a PersonStmt),
    MarriageDeclId(&'a MarriageStmt),
    /// Person reference (e.g. spouse position). `target` carries the
    /// declaring file; may differ from the reference's file (ADR-0015).
    PersonRef {
        ident: &'a Ident,
        target: Option<(FileId, &'a PersonStmt)>,
    },
    /// Marriage reference (in `birth`/`adoption`).
    MarriageRef {
        ident: &'a Ident,
        target: Option<(FileId, &'a MarriageStmt)>,
    },
    PersonFieldName(&'a PersonField),
    PersonFieldValue(&'a PersonField),
    MarriageFieldName(&'a MarriageField),
    MarriageFieldValue(&'a MarriageField),
    AdoptionFieldName(&'a AdoptionField),
    AdoptionFieldValue(&'a AdoptionField),
}

/// Resolved entity (person or marriage) referred to from a [`Node`].
/// Carries the declaring [`FileId`] alongside the AST node so consumers
/// don't re-query [`ResolvedDocument::entity`].
#[derive(Debug, Clone, Copy)]
pub enum EntityTarget<'a> {
    Person {
        file: FileId,
        stmt: &'a PersonStmt,
    },
    Marriage {
        file: FileId,
        stmt: &'a MarriageStmt,
    },
}

impl<'a> EntityTarget<'a> {
    /// File the declaration lives in.
    #[must_use]
    pub fn file(self) -> FileId {
        match self {
            EntityTarget::Person { file, .. } | EntityTarget::Marriage { file, .. } => file,
        }
    }

    /// The declaration id's [`FileSpan`] — the goto-definition anchor.
    #[must_use]
    pub fn decl_span(self) -> FileSpan {
        match self {
            EntityTarget::Person { file, stmt } => FileSpan::new(file, stmt.id.span),
            EntityTarget::Marriage { file, stmt } => FileSpan::new(file, stmt.id.span),
        }
    }
}

/// "Entity at the cursor" summary returned by [`Node::entity_reference`].
/// `None` for keywords, fields, whitespace.
#[derive(Debug, Clone, Copy)]
pub struct EntityNode<'a> {
    pub kind: EntityKind,
    /// The id text under the cursor; for unresolved references this may
    /// not match any declaration.
    pub name: &'a str,
    /// Cursor's id span (the rename popover highlight).
    pub ident_span: FileSpan,
    /// True if the cursor is on a declaration; false if on a reference.
    pub is_decl: bool,
    /// Resolved entity; `None` only for unresolved references.
    pub target: Option<EntityTarget<'a>>,
}

impl<'a> EntityNode<'a> {
    /// Declaration id [`FileSpan`], or `None` for unresolved references.
    /// For a decl, the cursor's own span.
    #[must_use]
    pub fn decl_span(&self) -> Option<FileSpan> {
        if self.is_decl {
            return Some(self.ident_span);
        }
        self.target.map(|t| t.decl_span())
    }
}

/// "Field at the cursor" summary returned by [`Node::field_node`].
/// Mirrors [`EntityNode`] so callers don't pattern-match the six
/// `*FieldName`/`*FieldValue` variants by hand.
#[derive(Debug, Clone, Copy)]
pub struct FieldNode {
    pub name: FieldName,
    /// `<name>:` lhs span.
    pub name_span: ByteSpan,
    /// Value rhs span.
    pub value_span: ByteSpan,
    /// True on the name side; false on the value side.
    pub is_name: bool,
}

impl ResolvedDocument {
    /// What's at `byte_offset` inside `file`?
    ///
    /// Half-open spans. Returns `None` for whitespace, comments, or
    /// out-of-range positions. Smallest enclosing span wins.
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
    /// let kul_file = FileId::from_raw(1);
    /// let tokens = tokenize(source);
    /// let (statements, _) = parse(&tokens, kul_file);
    /// let kf = Arc::new(KulFile::new("test.kul", source, statements));
    /// let document = Arc::new(Document::new("kul.yml", vec![kf]));
    /// let (resolved, _) = resolve(document);
    ///
    /// let node = resolved.node_at(kul_file, 0).expect("a node");
    /// assert!(matches!(node, Node::Keyword(KeywordKind::Person, _)));
    /// ```
    #[must_use]
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
                target: self.marriage_with_file(&b.marriage_ref.name),
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
                target: self.marriage_with_file(&a.marriage_ref.name),
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
                target: self.person_with_file(&m.spouse_a.name),
            });
        }
        if contains(m.spouse_b.span, offset) {
            return Some(Node::PersonRef {
                ident: &m.spouse_b,
                target: self.person_with_file(&m.spouse_b.name),
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
    /// [`EntityNode`] summary if the cursor sits on a person/marriage id
    /// (decl or reference); `None` otherwise.
    #[must_use]
    pub fn entity_reference(&self, file: FileId) -> Option<EntityNode<'a>> {
        match *self {
            Node::PersonDeclId(p) => Some(EntityNode {
                kind: EntityKind::Person,
                name: p.id.name.as_str(),
                ident_span: FileSpan::new(file, p.id.span),
                is_decl: true,
                target: Some(EntityTarget::Person { file, stmt: p }),
            }),
            Node::MarriageDeclId(m) => Some(EntityNode {
                kind: EntityKind::Marriage,
                name: m.id.name.as_str(),
                ident_span: FileSpan::new(file, m.id.span),
                is_decl: true,
                target: Some(EntityTarget::Marriage { file, stmt: m }),
            }),
            Node::PersonRef { ident, target } => Some(EntityNode {
                kind: EntityKind::Person,
                name: ident.name.as_str(),
                ident_span: FileSpan::new(file, ident.span),
                is_decl: false,
                target: target.map(|(file, stmt)| EntityTarget::Person { file, stmt }),
            }),
            Node::MarriageRef { ident, target } => Some(EntityNode {
                kind: EntityKind::Marriage,
                name: ident.name.as_str(),
                ident_span: FileSpan::new(file, ident.span),
                is_decl: false,
                target: target.map(|(file, stmt)| EntityTarget::Marriage { file, stmt }),
            }),
            _ => None,
        }
    }

    /// [`FieldNode`] summary if the cursor sits on any field side; `None`
    /// otherwise.
    #[must_use]
    pub fn field_node(&self) -> Option<FieldNode> {
        match *self {
            Node::PersonFieldName(f) => Some(field_node_of(
                f.kind.field_name(),
                f.name_span,
                f.kind.value_span(),
                true,
            )),
            Node::PersonFieldValue(f) => Some(field_node_of(
                f.kind.field_name(),
                f.name_span,
                f.kind.value_span(),
                false,
            )),
            Node::MarriageFieldName(f) => Some(field_node_of(
                f.kind.field_name(),
                f.name_span,
                f.kind.value_span(),
                true,
            )),
            Node::MarriageFieldValue(f) => Some(field_node_of(
                f.kind.field_name(),
                f.name_span,
                f.kind.value_span(),
                false,
            )),
            Node::AdoptionFieldName(f) => Some(field_node_of(
                f.kind.field_name(),
                f.name_span,
                f.kind.value_span(),
                true,
            )),
            Node::AdoptionFieldValue(f) => Some(field_node_of(
                f.kind.field_name(),
                f.name_span,
                f.kind.value_span(),
                false,
            )),
            _ => None,
        }
    }
}

fn field_node_of(
    name: FieldName,
    name_span: ByteSpan,
    value_span: ByteSpan,
    is_name: bool,
) -> FieldNode {
    FieldNode {
        name,
        name_span,
        value_span,
        is_name,
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
        let kf = Arc::new(KulFile::new("test.kul", source, statements));
        let document = Arc::new(Document::new("kul.yml", vec![kf]));
        let (resolved, _) = resolve(document);
        (resolved, file)
    }

    #[derive(Debug, PartialEq, Eq)]
    enum Probe {
        None,
        Keyword(KeywordKind),
        PersonDeclId(String),
        MarriageDeclId(String),
        PersonRef {
            name: String,
            resolved: bool,
            target_file: Option<FileId>,
        },
        MarriageRef {
            name: String,
            resolved: bool,
            target_file: Option<FileId>,
        },
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
                    target_file: target.map(|(file, _)| file),
                },
                Some(Node::MarriageRef { ident, target }) => Probe::MarriageRef {
                    name: ident.name.clone(),
                    resolved: target.is_some(),
                    target_file: target.map(|(file, _)| file),
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
                target_file: None,
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
                target_file: None,
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

    fn field_at(source: &str, offset: usize) -> Option<(FieldName, bool)> {
        let (resolved, file) = build(source);
        resolved
            .node_at(file, offset)
            .and_then(|n| n.field_node())
            .map(|f| (f.name, f.is_name))
    }

    #[test]
    fn field_node_distinguishes_name_from_value_across_kinds() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   marriage m alice bob start:2010 end:2020 end_reason:divorce\n\
                   person kid name:\"K\" gender:other\n  adoption m start:2005 end:2008\n";
        assert_eq!(
            field_at(src, idx(src, "name:")),
            Some((FieldName::Name, true))
        );
        assert_eq!(
            field_at(src, idx(src, "\"A\"")),
            Some((FieldName::Name, false)),
        );
        assert_eq!(
            field_at(src, idx(src, "gender:")),
            Some((FieldName::Gender, true)),
        );
        assert_eq!(
            field_at(src, idx(src, "start:")),
            Some((FieldName::Start, true)),
        );
        assert_eq!(
            field_at(src, idx(src, "2010")),
            Some((FieldName::Start, false)),
        );
        assert_eq!(
            field_at(src, idx(src, "end_reason:")),
            Some((FieldName::EndReason, true)),
        );
        let adoption_line = idx(src, "adoption");
        let adopt_start = src[adoption_line..].find("start:").unwrap() + adoption_line;
        assert_eq!(field_at(src, adopt_start), Some((FieldName::Start, true)),);
    }

    #[test]
    fn field_node_returns_none_for_keywords_ids_and_whitespace() {
        let src = "person alice name:\"A\" gender:female\n";
        assert!(field_at(src, 0).is_none());
        assert!(field_at(src, idx(src, "alice")).is_none());
    }
}
