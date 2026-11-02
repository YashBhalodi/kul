//! The **detail lookup** — one entity's own fields *plus* its relational
//! neighbourhood, for a details panel.
//!
//! The [`person`](super::person) / [`marriage`](super::marriage) lookups
//! (ADR-0024) cannot answer a details panel: `ExportedMarriage` carries no
//! children, and no parenthood-link lookup exists. [`detail`] answers a person
//! or a marriage by id (they share one namespace) with its parents, marriages,
//! spouses and children, read through [`ResolvedDocument`]'s one-hop seams
//! (ADR-0001, ADR-0037).
//!
//! **Absence is the answer**, exactly as for the other lookups: an id naming
//! no entity yields `None`, never an error.
//!
//! Every entity in the answer carries the **export shapes** —
//! [`ExportedPerson`], [`ExportedMarriage`], [`ExportedParenthoodLink`] —
//! single-sourced through `projection::build_one_*`, so the panel and the
//! export can never disagree about a person's shape.

use serde::Serialize;
#[cfg(feature = "tsify")]
use tsify::Tsify;

use crate::ast::{AdoptionSub, BirthSub, MarriageStmt, PersonStmt};
use crate::export::{ExportOptions, ExportedMarriage, ExportedParenthoodLink, ExportedPerson};
use crate::projection::{
    build_one_adoption_link, build_one_birth_link, build_one_marriage, build_one_person,
};
use crate::semantic::{ChildLink, EntityKind, ParentLinkKind, ResolvedDocument};

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
/// history.
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

/// One entity's own fields plus its relational neighbourhood, as a details
/// panel shows it.
///
/// A tagged union over the two entity kinds an id can name, so a consumer's
/// panel variant maps 1:1 onto a variant here. Absent fields are **omitted**
/// from the wire (the export shapes' `skip_serializing_if`), never sent as an
/// empty placeholder.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify), tsify(into_wasm_abi))]
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
}

/// The details of the person or marriage called `id`, or `None` when no
/// entity has that id.
///
/// Children come from [`ResolvedDocument::children_of_marriage`] (the
/// resolver-owned inverse index); a person's marriages are found by one pass
/// over the project's marriages.
#[must_use]
pub fn detail(resolved: &ResolvedDocument, id: &str) -> Option<EntityDetail> {
    match resolved.entity(id)?.kind {
        EntityKind::Person => person_detail(resolved, id),
        EntityKind::Marriage => marriage_detail(resolved, id),
    }
}

/// One parenthood link, borrowed from the resolved document, projected only
/// when a row needs it.
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

fn person_detail(resolved: &ResolvedDocument, id: &str) -> Option<EntityDetail> {
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

    let own_marriages: Vec<&MarriageStmt> = resolved
        .marriages()
        .filter(|marriage| {
            resolved
                .spouses_of(marriage)
                .any(|spouse| spouse.id.name == person.id.name)
        })
        .collect();

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

    let children = own_marriages
        .iter()
        .flat_map(|marriage| child_rows(resolved, marriage, &options))
        .collect();

    Some(EntityDetail::Person {
        person: build_one_person(person, &options),
        parents,
        marriages,
        children,
    })
}

fn marriage_detail(resolved: &ResolvedDocument, id: &str) -> Option<EntityDetail> {
    let options = ExportOptions::default();
    let marriage = resolved.marriage(id)?;
    Some(EntityDetail::Marriage {
        marriage: build_one_marriage(marriage, &options),
        spouses: resolved
            .spouses_of(marriage)
            .map(|s| build_one_person(s, &options))
            .collect(),
        children: child_rows(resolved, marriage, &options),
    })
}

/// Every child born or adopted into `marriage`, one row per link.
fn child_rows(
    resolved: &ResolvedDocument,
    marriage: &MarriageStmt,
    options: &ExportOptions,
) -> Vec<LinkedPerson> {
    resolved
        .children_of_marriage(marriage)
        .iter()
        .filter_map(link_ref_from_child_link)
        .map(|link| link.child_row(options))
        .collect()
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
