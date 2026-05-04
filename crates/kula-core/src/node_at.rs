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

use crate::ast::{
    AdoptionField, AdoptionSub, BirthSub, Ident, MarriageField, MarriageStmt, PersonField,
    PersonStmt, Statement, VersionDecl,
};
use crate::semantic::ResolvedDocument;
use crate::span::ByteSpan;

/// A keyword token: one of the fixed words in Kula's grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeywordKind {
    /// `kula` (in the version declaration).
    Kula,
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
    /// The version literal (e.g. `1`) in a `kula <v>` declaration.
    VersionLiteral(&'a VersionDecl),
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

impl<'a> ResolvedDocument<'a> {
    /// What's at `byte_offset`?
    ///
    /// Spans are half-open: a span `[s, e)` contains `offset` iff
    /// `s <= offset < e`. Returns `None` for whitespace, comments, and
    /// out-of-range offsets. Smallest enclosing span wins.
    ///
    /// # Example
    ///
    /// ```
    /// use kula_core::lexer::tokenize;
    /// use kula_core::parser::parse;
    /// use kula_core::semantic::resolve;
    /// use kula_core::node_at::{KeywordKind, Node};
    ///
    /// let source = "person alice name:\"Alice\" gender:female\n";
    /// let tokens = tokenize(source);
    /// let (document, _) = parse(&tokens);
    /// let (resolved, _) = resolve(&document);
    ///
    /// // Cursor on the `person` keyword.
    /// let node = resolved.node_at(0).expect("a node");
    /// assert!(matches!(node, Node::Keyword(KeywordKind::Person, _)));
    ///
    /// // Cursor inside the id.
    /// let id_offset = source.find("alice").unwrap();
    /// let node = resolved.node_at(id_offset).expect("a node");
    /// assert!(matches!(node, Node::PersonDeclId(_)));
    /// ```
    pub fn node_at(&self, byte_offset: usize) -> Option<Node<'a>> {
        if let Some(version) = &self.document.version
            && contains(version.span, byte_offset)
        {
            if contains(version.version_span, byte_offset) {
                return Some(Node::VersionLiteral(version));
            }
            if contains(version.keyword_span, byte_offset) {
                return Some(Node::Keyword(KeywordKind::Kula, version.keyword_span));
            }
            return None;
        }

        for stmt in &self.document.statements {
            match stmt {
                Statement::Person(p) if contains(p.span, byte_offset) => {
                    return self.node_in_person(p, byte_offset);
                }
                Statement::Marriage(m) if contains(m.span, byte_offset) => {
                    return self.node_in_marriage(m, byte_offset);
                }
                _ => continue,
            }
        }
        None
    }

    fn node_in_person(&self, p: &'a PersonStmt, offset: usize) -> Option<Node<'a>> {
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
            return self.node_in_birth(b, offset);
        }
        for adopt in &p.adoptions {
            if contains(adopt.span, offset) {
                return self.node_in_adoption(adopt, offset);
            }
        }
        None
    }

    fn node_in_birth(&self, b: &'a BirthSub, offset: usize) -> Option<Node<'a>> {
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

    fn node_in_adoption(&self, a: &'a AdoptionSub, offset: usize) -> Option<Node<'a>> {
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

    fn node_in_marriage(&self, m: &'a MarriageStmt, offset: usize) -> Option<Node<'a>> {
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

fn contains(span: ByteSpan, offset: usize) -> bool {
    offset >= span.start && offset < span.end
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{AdoptionFieldKind, MarriageFieldKind, PersonFieldKind};
    use crate::semantic::resolve;

    /// Owned mirror of `Node` for assertion ergonomics — capture the
    /// variant and a short identifier without holding any borrow.
    #[derive(Debug, PartialEq, Eq)]
    enum Probe {
        None,
        Keyword(KeywordKind),
        VersionLiteral,
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
                Some(Node::VersionLiteral(_)) => Probe::VersionLiteral,
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
        let tokens = crate::lexer::tokenize(source);
        let (document, _) = crate::parser::parse(&tokens);
        let (resolved, _) = resolve(&document);
        Probe::from(resolved.node_at(offset))
    }

    fn idx(source: &str, pat: &str) -> usize {
        source.find(pat).expect("pattern in source")
    }

    #[test]
    fn version_keyword_and_literal() {
        let src = "kula 1\n";
        assert_eq!(at(src, 0), Probe::Keyword(KeywordKind::Kula));
        assert_eq!(at(src, 3), Probe::Keyword(KeywordKind::Kula));
        // Whitespace between `kula` and `1` is in neither span.
        assert_eq!(at(src, 4), Probe::None);
        assert_eq!(at(src, 5), Probe::VersionLiteral);
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
    fn person_field_value_covers_dates() {
        let src = "person alice name:\"A\" gender:female born:1900-01-01 died:~1980\n";
        assert_eq!(
            at(src, idx(src, "1900-01-01")),
            Probe::PersonFieldValue("born".into()),
        );
        assert_eq!(
            at(src, idx(src, "~1980")),
            Probe::PersonFieldValue("died".into()),
        );
    }

    #[test]
    fn marriage_keyword_id_spouses_and_fields() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage mab a b start:2010 end:2020 end_reason:divorce\n";
        let marriage_line = idx(src, "marriage ");

        assert_eq!(
            at(src, marriage_line),
            Probe::Keyword(KeywordKind::Marriage),
        );
        assert_eq!(
            at(src, idx(src, "mab")),
            Probe::MarriageDeclId("mab".into()),
        );

        // First " a " *after* the marriage keyword is the spouse_a ref.
        let spouse_a = src[marriage_line..]
            .find(" a ")
            .map(|i| marriage_line + i + 1)
            .unwrap();
        let spouse_b = src[marriage_line..]
            .find(" b ")
            .map(|i| marriage_line + i + 1)
            .unwrap();
        assert_eq!(
            at(src, spouse_a),
            Probe::PersonRef {
                name: "a".into(),
                resolved: true,
            },
        );
        assert_eq!(
            at(src, spouse_b),
            Probe::PersonRef {
                name: "b".into(),
                resolved: true,
            },
        );

        assert_eq!(
            at(src, idx(src, "start:")),
            Probe::MarriageFieldName("start".into()),
        );
        assert_eq!(
            at(src, idx(src, "end:")),
            Probe::MarriageFieldName("end".into()),
        );
        assert_eq!(
            at(src, idx(src, "end_reason:")),
            Probe::MarriageFieldName("end_reason".into()),
        );
        assert_eq!(
            at(src, idx(src, "divorce")),
            Probe::MarriageFieldValue("end_reason".into()),
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
    fn birth_keyword_and_marriage_ref() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1980\n\
                   person kid name:\"K\" gender:other\n  birth m\n";
        assert_eq!(
            at(src, idx(src, "birth")),
            Probe::Keyword(KeywordKind::Birth),
        );
        let m_ref = idx(src, "birth m") + "birth ".len();
        assert_eq!(
            at(src, m_ref),
            Probe::MarriageRef {
                name: "m".into(),
                resolved: true,
            },
        );
    }

    #[test]
    fn adoption_keyword_marriage_ref_and_fields() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1980\n\
                   person kid name:\"K\" gender:other\n  adoption m start:2000 end:2010\n";
        let adoption_line = idx(src, "adoption");
        assert_eq!(
            at(src, adoption_line),
            Probe::Keyword(KeywordKind::Adoption),
        );
        let m_ref = idx(src, "adoption m") + "adoption ".len();
        assert_eq!(
            at(src, m_ref),
            Probe::MarriageRef {
                name: "m".into(),
                resolved: true,
            },
        );
        let start_field = src[adoption_line..]
            .find("start:")
            .map(|i| adoption_line + i)
            .unwrap();
        let end_field = src[adoption_line..]
            .find("end:")
            .map(|i| adoption_line + i)
            .unwrap();
        assert_eq!(
            at(src, start_field),
            Probe::AdoptionFieldName("start".into()),
        );
        assert_eq!(at(src, end_field), Probe::AdoptionFieldName("end".into()),);
        let start_value = src[adoption_line..]
            .find("2000")
            .map(|i| adoption_line + i)
            .unwrap();
        assert_eq!(
            at(src, start_value),
            Probe::AdoptionFieldValue("start".into()),
        );
    }

    #[test]
    fn span_boundary_start_inclusive_end_exclusive() {
        let src = "kula 1\n";
        // `kula` span = [0, 4); byte 0 hits, byte 4 (the space) doesn't.
        assert_eq!(at(src, 0), Probe::Keyword(KeywordKind::Kula));
        assert_eq!(at(src, 3), Probe::Keyword(KeywordKind::Kula));
        assert_eq!(at(src, 4), Probe::None);
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
        let src = "kula 1\n";
        assert_eq!(at(src, src.len()), Probe::None);
        assert_eq!(at(src, src.len() + 999), Probe::None);
    }

    #[test]
    fn smallest_enclosing_wins() {
        // Cursor on the person id should yield PersonDeclId, not Keyword.
        let src = "person alice name:\"A\" gender:female\n";
        assert!(matches!(at(src, idx(src, "alice")), Probe::PersonDeclId(_)));
    }
}
