//! Cytoscape JSON projection of the canonical kinship-native graph.
//!
//! A pure transformer: takes an [`ExportedGraph`] and produces a
//! [`CytoscapeGraph`] that can be loaded directly into Cytoscape Desktop,
//! Cytoscape.js, Sigma.js, vis-network, or any other tool that consumes
//! the standard `{ nodes, edges }` shape with `data` records.
//!
//! ## Modeling
//!
//! Marriages are promoted to **first-class nodes** rather than being
//! flattened into spouse-to-spouse edges. The reason: marriages carry their
//! own source-level facts (`start`, `end`, `end_reason`) that need a node
//! to sit on. The resulting graph is bipartite — every edge runs from a
//! marriage node to a person node, never person-to-person.
//!
//! Ids are prefixed (`p:` for persons, `m:` for marriages) so the
//! cytoscape side can hold both kinds of node in one flat namespace
//! without collision (a person and a marriage with the same source id are
//! both legal mid-edit; rule 01 reports the conflict but the export still
//! runs through the transformer if errors are absent).
//!
//! Edges are tagged with a `type`:
//! - `"spouse"` — marriage → person, one per spouse position.
//! - `"biological_child"` — marriage → child, one per `birth` link.
//! - `"adoptive_child"` — marriage → child, one per `adoption` link.
//!   Adoptive edges carry the adoption's `start` (and `end` if present)
//!   as `data` fields so consumers can render the timeline.

use serde::Serialize;

use crate::export::{ExportedDate, ExportedGraph};

/// The Cytoscape JSON graph shape.
#[derive(Debug, Clone, Serialize)]
pub struct CytoscapeGraph {
    pub nodes: Vec<CytoscapeNode>,
    pub edges: Vec<CytoscapeEdge>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CytoscapeNode {
    pub data: NodeData,
}

/// One node's `data` payload. Untagged: the variant is chosen at
/// serialization time by which fields are present, matching the Cytoscape
/// convention of "the data object is whatever the consumer wants."
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum NodeData {
    Person(PersonNodeData),
    Marriage(MarriageNodeData),
}

#[derive(Debug, Clone, Serialize)]
pub struct PersonNodeData {
    /// `p:<person-id>`.
    pub id: String,
    /// Always `"person"`.
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub given: Option<String>,
    pub gender: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub born: Option<ExportedDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub died: Option<ExportedDate>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MarriageNodeData {
    /// `m:<marriage-id>`.
    pub id: String,
    /// Always `"marriage"`.
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub start: ExportedDate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<ExportedDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CytoscapeEdge {
    pub data: EdgeData,
}

#[derive(Debug, Clone, Serialize)]
pub struct EdgeData {
    /// `m:<marriage-id>` (always; every edge originates at a marriage).
    pub source: String,
    /// `p:<person-id>` (always; every edge ends at a person).
    pub target: String,
    /// `"spouse"`, `"biological_child"`, or `"adoptive_child"`.
    #[serde(rename = "type")]
    pub kind: &'static str,
    /// `start:` of an adoption. Always absent on spouse and bio-child edges.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<ExportedDate>,
    /// `end:` of an adoption. Always absent on spouse and bio-child edges.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<ExportedDate>,
}

/// Project the kinship-native [`ExportedGraph`] into the Cytoscape shape.
///
/// Pure mapping; the cytoscape graph contains exactly the same data as the
/// input (nothing is dropped, nothing is invented). Order: every person
/// becomes a node first, then every marriage becomes a node and emits its
/// two spouse edges, then every parenthood link becomes an edge.
pub fn to_cytoscape(graph: &ExportedGraph) -> CytoscapeGraph {
    let mut nodes = Vec::with_capacity(graph.persons.len() + graph.marriages.len());
    let mut edges = Vec::with_capacity(graph.marriages.len() * 2 + graph.parenthood_links.len());

    for p in &graph.persons {
        nodes.push(CytoscapeNode {
            data: NodeData::Person(PersonNodeData {
                id: format!("p:{}", p.id),
                kind: "person",
                name: p.name.clone(),
                family: p.family.clone(),
                given: p.given.clone(),
                gender: p.gender,
                born: p.born.clone(),
                died: p.died.clone(),
            }),
        });
    }

    for m in &graph.marriages {
        let marriage_id = format!("m:{}", m.id);
        nodes.push(CytoscapeNode {
            data: NodeData::Marriage(MarriageNodeData {
                id: marriage_id.clone(),
                kind: "marriage",
                start: m.start.clone(),
                end: m.end.clone(),
                end_reason: m.end_reason.clone(),
            }),
        });
        for spouse in &m.spouses {
            edges.push(CytoscapeEdge {
                data: EdgeData {
                    source: marriage_id.clone(),
                    target: format!("p:{spouse}"),
                    kind: "spouse",
                    start: None,
                    end: None,
                },
            });
        }
    }

    for link in &graph.parenthood_links {
        let kind = match link.kind {
            "biological" => "biological_child",
            "adoptive" => "adoptive_child",
            other => other,
        };
        edges.push(CytoscapeEdge {
            data: EdgeData {
                source: format!("m:{}", link.marriage_id),
                target: format!("p:{}", link.child_id),
                kind,
                start: link.start.clone(),
                end: link.end.clone(),
            },
        });
    }

    CytoscapeGraph { nodes, edges }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::{ExportOptions, export};

    fn cytoscape_for(source: &str) -> CytoscapeGraph {
        let check = crate::check(source);
        let envelope = export(source, &check, ExportOptions::default());
        let crate::export::ExportEnvelope::Success(success) = envelope else {
            panic!("expected success envelope");
        };
        let crate::export::GraphPayload::Native(native) = success.graph else {
            panic!("default options should produce native graph");
        };
        to_cytoscape(&native)
    }

    fn find_node<'a>(graph: &'a CytoscapeGraph, id: &str) -> Option<&'a CytoscapeNode> {
        graph.nodes.iter().find(|n| match &n.data {
            NodeData::Person(p) => p.id == id,
            NodeData::Marriage(m) => m.id == id,
        })
    }

    #[test]
    fn marriage_is_promoted_to_a_node_with_m_prefix() {
        let cy = cytoscape_for(
            "person a name:\"A\" gender:female\nperson b name:\"B\" gender:male\nmarriage m a b start:1972\n",
        );
        assert!(find_node(&cy, "m:m").is_some(), "marriage node missing");
        match &find_node(&cy, "m:m").unwrap().data {
            NodeData::Marriage(md) => assert_eq!(md.kind, "marriage"),
            _ => panic!("expected marriage data"),
        }
    }

    #[test]
    fn person_nodes_use_p_prefix_and_carry_type_person() {
        let cy = cytoscape_for(
            "person alice name:\"A\" gender:female\nperson bob name:\"B\" gender:male\nmarriage m alice bob start:1972\n",
        );
        let node = find_node(&cy, "p:alice").expect("alice node");
        match &node.data {
            NodeData::Person(pd) => assert_eq!(pd.kind, "person"),
            _ => panic!("expected person data"),
        }
    }

    #[test]
    fn marriage_emits_two_spouse_edges_one_per_position() {
        let cy = cytoscape_for(
            "person alice name:\"A\" gender:female\nperson bob name:\"B\" gender:male\nmarriage m alice bob start:1972\n",
        );
        let spouse_edges: Vec<&CytoscapeEdge> = cy
            .edges
            .iter()
            .filter(|e| e.data.kind == "spouse" && e.data.source == "m:m")
            .collect();
        assert_eq!(spouse_edges.len(), 2);
        let targets: std::collections::HashSet<&str> = spouse_edges
            .iter()
            .map(|e| e.data.target.as_str())
            .collect();
        assert!(targets.contains("p:alice"));
        assert!(targets.contains("p:bob"));
    }

    #[test]
    fn biological_birth_emits_one_biological_child_edge() {
        let cy = cytoscape_for(
            "\
person a name:\"A\" gender:female
person b name:\"B\" gender:male
person kid name:\"K\" gender:other
  birth m
marriage m a b start:1972
",
        );
        let bio_edges: Vec<&CytoscapeEdge> = cy
            .edges
            .iter()
            .filter(|e| e.data.kind == "biological_child")
            .collect();
        assert_eq!(bio_edges.len(), 1);
        assert_eq!(bio_edges[0].data.source, "m:m");
        assert_eq!(bio_edges[0].data.target, "p:kid");
        assert!(
            bio_edges[0].data.start.is_none(),
            "biological edges have no start"
        );
    }

    #[test]
    fn adoption_emits_one_adoptive_child_edge_with_start_date() {
        let cy = cytoscape_for(
            "\
person a name:\"A\" gender:female
person b name:\"B\" gender:male
person kid name:\"K\" gender:other
  adoption m start:2000-06-01
marriage m a b start:1972
",
        );
        let adoptive_edges: Vec<&CytoscapeEdge> = cy
            .edges
            .iter()
            .filter(|e| e.data.kind == "adoptive_child")
            .collect();
        assert_eq!(adoptive_edges.len(), 1);
        let start = adoptive_edges[0]
            .data
            .start
            .as_ref()
            .expect("adoptive edges carry start date");
        assert_eq!(start.value, "2000-06-01");
        assert_eq!(start.precision, "day");
    }
}
