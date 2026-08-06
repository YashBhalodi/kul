//! The **batched detail lookup** — one check, any number of entities, each
//! answered with its own fields *plus* its relational neighbourhood.
//!
//! The single-entity [`person`](super::person) / [`marriage`](super::marriage)
//! lookups (ADR-0024) cannot answer a detail panel: `ExportedMarriage` carries
//! no children, and no parenthood-link lookup exists at all. Composing them
//! with kin-set queries would work, but every stateless call re-pays the
//! project check — the dominant per-call cost — so a four-call composition
//! costs ~4× a one-call answer and an N-row list costs ~N×. This operation
//! pays the check **once** and answers every target off the same
//! [`ResolvedDocument`], which is what makes its cost flat in the number of
//! targets (ADR-0035, ADR-0037).
//!
//! **Absence is the answer**, exactly as for the single-entity lookups: a
//! target naming no entity yields `None` at its position, never an error. The
//! target *is* the subject of the question (ADR-0024 draws that line against
//! the anchor of a relationship question, which is a typed error).
//!
//! Every entity in the answer carries the **export shapes** —
//! [`ExportedPerson`], [`ExportedMarriage`], [`ExportedParenthoodLink`] —
//! single-sourced through the export's `build_one_*` builders. There is no
//! second, leaner person shape; a consumer that needs a display name reads
//! `person.name`, which is why this one operation also supplies a kin list's
//! row labels.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "tsify")]
use tsify::Tsify;

use crate::ast::{AdoptionSub, BirthSub, MarriageStmt, PersonStmt};
use crate::export::{
    ExportOptions, ExportedMarriage, ExportedParenthoodLink, ExportedPerson,
    build_one_adoption_link, build_one_birth_link, build_one_marriage, build_one_person,
};
use crate::semantic::{ChildLink, ParentLinkKind, ResolvedDocument};

/// What one entry of a batched detail lookup addresses.
///
/// Three entity kinds, because three things on a rendered tree are selectable:
/// a person card, a marriage edge, and an adoption edge. An **adoption link
/// has no id of its own** — it is a sub-statement of the child — so it is
/// addressed by the `(childId, marriageId)` pair, the same pair the rendered
/// adoption edge carries. Variants stay additive: a `birth` target would be a
/// new variant, never a reshape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(Tsify), tsify(from_wasm_abi, into_wasm_abi))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DetailTarget {
    /// A person, by id.
    Person {
        /// The person's id.
        id: String,
    },
    /// A marriage, by id.
    Marriage {
        /// The marriage's id.
        id: String,
    },
    /// One `adoption` sub-statement, addressed by the child it belongs to and
    /// the marriage it adopts into.
    #[serde(rename_all = "camelCase")]
    Adoption {
        /// Id of the adopted person.
        child_id: String,
        /// Id of the marriage adopted into.
        marriage_id: String,
    },
}

impl DetailTarget {
    /// A person target.
    #[must_use]
    pub fn person(id: impl Into<String>) -> Self {
        DetailTarget::Person { id: id.into() }
    }

    /// A marriage target.
    #[must_use]
    pub fn marriage(id: impl Into<String>) -> Self {
        DetailTarget::Marriage { id: id.into() }
    }

    /// An adoption target: the `adoption` sub-statement on `child_id` that
    /// names `marriage_id`.
    #[must_use]
    pub fn adoption(child_id: impl Into<String>, marriage_id: impl Into<String>) -> Self {
        DetailTarget::Adoption {
            child_id: child_id.into(),
            marriage_id: marriage_id.into(),
        }
    }
}

/// A neighbouring person plus the parenthood link that reaches them.
///
/// One row per **link**, not per person: a person reached both by birth and by
/// a later adoption appears twice, with a different `link` each time. That is
/// the same path-identity discipline the kin-set results carry (ADR-0026) —
/// the engine does not collapse two distinct ties into one row.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub struct LinkedPerson {
    /// The neighbour, in the export shape.
    pub person: ExportedPerson,
    /// The `birth` / `adoption` link connecting them to the subject, carrying
    /// its `biological` / `adoptive` kind and an adoption's own dates.
    pub link: ExportedParenthoodLink,
}

/// One marriage a person is a spouse in, with the person on the other side.
///
/// Each marriage is its own row rather than a joined spouse list, because the
/// marriage's `start` / `end` / `endReason` are what explain a person's
/// history (ADR-0035).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub struct MarriageTie {
    /// The marriage, in the export shape.
    pub marriage: ExportedMarriage,
    /// The other spouse. Absent only when the marriage names no second
    /// person — which a project that passes its checks cannot do (R02, R04) —
    /// so consumers omit the field rather than rendering a placeholder.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spouse: Option<ExportedPerson>,
}

/// One entity's own fields plus its relational neighbourhood — the answer to
/// a single [`DetailTarget`].
///
/// A tagged union over the three selectable entity kinds, so a consumer's
/// panel variant maps 1:1 onto a variant here. Absent fields are **omitted**
/// from the wire (the export shapes' `skip_serializing_if`), never sent as an
/// empty placeholder.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum EntityDetail {
    /// A person: their own fields, every parent they are linked to, every
    /// marriage they are a spouse in, and every child of those marriages.
    Person {
        /// The person's own recorded fields.
        person: ExportedPerson,
        /// Every parent, in declaration order: the birth family's spouses
        /// first, then each adoption's. One row per parent per link — a
        /// link naming a marriage yields a row for each of its spouses,
        /// so three links can produce six rows.
        parents: Vec<LinkedPerson>,
        /// Every marriage this person is a spouse in, in declaration order.
        marriages: Vec<MarriageTie>,
        /// Every child of those marriages, in marriage-then-declaration
        /// order, one row per link.
        children: Vec<LinkedPerson>,
    },
    /// A marriage: its own fields, its spouses, and its children.
    Marriage {
        /// The marriage's own recorded fields.
        marriage: ExportedMarriage,
        /// The declared spouses, in declaration order.
        spouses: Vec<ExportedPerson>,
        /// Every child born or adopted into this marriage, in declaration
        /// order, one row per link.
        children: Vec<LinkedPerson>,
    },
    /// An adoption link: its own fields, the adopted child, and the adopting
    /// couple.
    Adoption {
        /// The adoption link's own recorded fields (`start:` / `end:`).
        adoption: ExportedParenthoodLink,
        /// The adopted person.
        child: ExportedPerson,
        /// The adopting couple — the spouses of the marriage adopted into,
        /// in declaration order.
        parents: Vec<ExportedPerson>,
    },
}

/// Answer every `target` off one [`ResolvedDocument`], in the order asked.
///
/// Position `i` of the answer is the detail for `targets[i]`, or `None` when
/// that target names no entity. The per-invocation spouse index is built
/// **once** and shared by every target; children come from
/// [`ResolvedDocument::children_of_marriage`] (the resolver-owned inverse
/// index). The marginal cost of a target is its own neighbourhood — the batch
/// is flat in the number of targets relative to the check that precedes it
/// (ADR-0037).
#[must_use]
pub fn details(resolved: &ResolvedDocument, targets: &[DetailTarget]) -> Vec<Option<EntityDetail>> {
    let index = Adjacency::build(resolved);
    targets
        .iter()
        .map(|target| index.detail(resolved, target))
        .collect()
}

/// One parenthood link, borrowed from the resolved document. Held by
/// reference so building the adjacency never allocates a serialized link that
/// no target asks for.
enum LinkRef<'a> {
    Birth(&'a PersonStmt, &'a BirthSub),
    Adoption(&'a PersonStmt, &'a AdoptionSub),
}

impl LinkRef<'_> {
    /// The child on this link.
    fn child(&self) -> &PersonStmt {
        match self {
            LinkRef::Birth(child, _) | LinkRef::Adoption(child, _) => child,
        }
    }

    /// Project to the serialized export shape.
    fn exported(&self, options: &ExportOptions) -> ExportedParenthoodLink {
        match self {
            LinkRef::Birth(child, birth) => build_one_birth_link(child, birth, options),
            LinkRef::Adoption(child, adoption) => build_one_adoption_link(child, adoption, options),
        }
    }

    /// Project to a neighbour row reached *through* this link — used for a
    /// parent row, where the far end is one of the marriage's spouses.
    fn parent_row(&self, parent: &PersonStmt, options: &ExportOptions) -> LinkedPerson {
        LinkedPerson {
            person: build_one_person(parent, options),
            link: self.exported(options),
        }
    }

    /// Project to a child row: the far end is the link's own child.
    fn child_row(&self, options: &ExportOptions) -> LinkedPerson {
        LinkedPerson {
            person: build_one_person(self.child(), options),
            link: self.exported(options),
        }
    }
}

/// The spouses of `marriage_id` as parent rows on `link`. Empty when the
/// reference does not resolve — R02 reports that, and a project that passes
/// its checks has none.
fn parent_rows(
    resolved: &ResolvedDocument,
    marriage_id: &str,
    link: &LinkRef<'_>,
    options: &ExportOptions,
) -> Vec<LinkedPerson> {
    resolved
        .marriage(marriage_id)
        .into_iter()
        .flat_map(|m| resolved.spouses_of(m))
        .map(|parent| link.parent_row(parent, options))
        .collect()
}

/// Per-invocation spouse index over the resolved project. Built once per
/// batch; no cross-call cache (ADR-0029). Children come from
/// [`ResolvedDocument::children_of_marriage`].
struct Adjacency<'a> {
    /// Person id → every marriage they are a spouse in, in declaration order.
    marriages_of_spouse: HashMap<&'a str, Vec<&'a MarriageStmt>>,
}

impl<'a> Adjacency<'a> {
    fn build(resolved: &'a ResolvedDocument) -> Self {
        let mut marriages_of_spouse: HashMap<&str, Vec<&MarriageStmt>> = HashMap::new();
        for marriage in resolved.marriages() {
            for spouse in resolved.spouses_of(marriage) {
                marriages_of_spouse
                    .entry(spouse.id.name.as_str())
                    .or_default()
                    .push(marriage);
            }
        }

        Adjacency {
            marriages_of_spouse,
        }
    }

    fn detail(
        &self,
        resolved: &'a ResolvedDocument,
        target: &DetailTarget,
    ) -> Option<EntityDetail> {
        match target {
            DetailTarget::Person { id } => self.person_detail(resolved, id),
            DetailTarget::Marriage { id } => self.marriage_detail(resolved, id),
            DetailTarget::Adoption {
                child_id,
                marriage_id,
            } => self.adoption_detail(resolved, child_id, marriage_id),
        }
    }

    fn person_detail(&self, resolved: &'a ResolvedDocument, id: &str) -> Option<EntityDetail> {
        let options = ExportOptions::default();
        let person = resolved.person(id)?;

        let mut parents = Vec::new();
        if let Some(birth) = &person.birth {
            let link = LinkRef::Birth(person, birth);
            parents.extend(parent_rows(
                resolved,
                &birth.marriage_ref.name,
                &link,
                &options,
            ));
        }
        for adoption in &person.adoptions {
            let link = LinkRef::Adoption(person, adoption);
            parents.extend(parent_rows(
                resolved,
                &adoption.marriage_ref.name,
                &link,
                &options,
            ));
        }

        let own_marriages = self
            .marriages_of_spouse
            .get(person.id.name.as_str())
            .map_or(&[][..], Vec::as_slice);

        let marriages = own_marriages
            .iter()
            .map(|marriage| MarriageTie {
                marriage: build_one_marriage(marriage, &options),
                spouse: resolved
                    .spouses_of(marriage)
                    .find(|s| s.id.name != person.id.name)
                    .map(|s| build_one_person(s, &options)),
            })
            .collect();

        let mut children = Vec::new();
        for marriage in own_marriages {
            for link in resolved.children_of_marriage(marriage) {
                if let Some(link_ref) = link_ref_from_child_link(&link) {
                    children.push(link_ref.child_row(&options));
                }
            }
        }

        Some(EntityDetail::Person {
            person: build_one_person(person, &options),
            parents,
            marriages,
            children,
        })
    }

    fn marriage_detail(&self, resolved: &'a ResolvedDocument, id: &str) -> Option<EntityDetail> {
        let options = ExportOptions::default();
        let marriage = resolved.marriage(id)?;
        let mut children = Vec::new();
        for link in resolved.children_of_marriage(marriage) {
            if let Some(link_ref) = link_ref_from_child_link(&link) {
                children.push(link_ref.child_row(&options));
            }
        }
        Some(EntityDetail::Marriage {
            marriage: build_one_marriage(marriage, &options),
            spouses: resolved
                .spouses_of(marriage)
                .map(|s| build_one_person(s, &options))
                .collect(),
            children,
        })
    }

    fn adoption_detail(
        &self,
        resolved: &'a ResolvedDocument,
        child_id: &str,
        marriage_id: &str,
    ) -> Option<EntityDetail> {
        let options = ExportOptions::default();
        let child = resolved.person(child_id)?;
        let adoption = child
            .adoptions
            .iter()
            .find(|a| a.marriage_ref.name == marriage_id)?;
        let parents = resolved
            .marriage(marriage_id)
            .into_iter()
            .flat_map(|m| resolved.spouses_of(m))
            .map(|s| build_one_person(s, &options))
            .collect();
        Some(EntityDetail::Adoption {
            adoption: build_one_adoption_link(child, adoption, &options),
            child: build_one_person(child, &options),
            parents,
        })
    }
}

/// Project a resolver [`ChildLink`] to the export-facing [`LinkRef`].
fn link_ref_from_child_link<'a>(link: &ChildLink<'a>) -> Option<LinkRef<'a>> {
    match link.kind {
        ParentLinkKind::Bio => link.birth().map(|birth| LinkRef::Birth(link.child, birth)),
        ParentLinkKind::Adoption => link
            .adoption()
            .map(|adoption| LinkRef::Adoption(link.child, adoption)),
    }
}
