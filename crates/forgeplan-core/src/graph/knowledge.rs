//! In-memory knowledge graph built from LanceDB artifact store using petgraph.
//!
//! Provides fast neighbor lookups, evidence discovery, and impact analysis
//! without repeated LanceDB traversal.

use std::collections::HashMap;

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;

use crate::db::store::LanceStore;

/// A node in the knowledge graph representing an artifact.
#[derive(Debug, Clone)]
pub struct ArtifactNode {
    pub id: String,
    pub kind: String,
    pub status: String,
}

/// A directed edge in the knowledge graph representing a relation.
#[derive(Debug, Clone)]
pub struct RelationEdge {
    pub relation: String,
}

/// In-memory directed graph of artifacts and their relations.
///
/// Built from [`LanceStore`] data, this provides O(1) node lookup by ID
/// and efficient neighbor traversal via petgraph's adjacency list.
pub struct KnowledgeGraph {
    graph: DiGraph<ArtifactNode, RelationEdge>,
    index: HashMap<String, NodeIndex>,
}

impl KnowledgeGraph {
    /// Build a knowledge graph by loading all artifacts and relations from the store.
    pub async fn from_store(store: &LanceStore) -> anyhow::Result<Self> {
        // Use list_artifacts (summary, no body) to avoid loading full bodies into memory
        let artifacts = store.list_artifacts(None).await?;
        let relations = store.get_all_relations().await?;

        let mut graph = DiGraph::new();
        let mut index = HashMap::new();

        // Add all artifacts as nodes.
        for artifact in &artifacts {
            let node = ArtifactNode {
                id: artifact.id.clone(),
                kind: artifact.kind.clone(),
                status: artifact.status.clone(),
            };
            let idx = graph.add_node(node);
            index.insert(artifact.id.clone(), idx);
        }

        // Add all relations as directed edges (source -> target).
        for (source_id, target_id, relation_type) in &relations {
            if let (Some(&src_idx), Some(&tgt_idx)) = (index.get(source_id), index.get(target_id))
            {
                graph.add_edge(
                    src_idx,
                    tgt_idx,
                    RelationEdge {
                        relation: relation_type.clone(),
                    },
                );
            }
        }

        Ok(Self { graph, index })
    }

    /// Build a knowledge graph from pre-built nodes and edges (useful for testing).
    pub fn from_parts(
        nodes: Vec<ArtifactNode>,
        edges: Vec<(String, String, String)>,
    ) -> Self {
        let mut graph = DiGraph::new();
        let mut index = HashMap::new();

        for node in nodes {
            let id = node.id.clone();
            let idx = graph.add_node(node);
            index.insert(id, idx);
        }

        for (source_id, target_id, relation_type) in &edges {
            if let (Some(&src_idx), Some(&tgt_idx)) = (index.get(source_id), index.get(target_id))
            {
                graph.add_edge(
                    src_idx,
                    tgt_idx,
                    RelationEdge {
                        relation: relation_type.clone(),
                    },
                );
            }
        }

        Self { graph, index }
    }

    /// Get all neighbors of an artifact (both outgoing and incoming directions).
    pub fn neighbors(&self, id: &str) -> Vec<&ArtifactNode> {
        let Some(&idx) = self.index.get(id) else {
            return Vec::new();
        };

        let mut seen = HashMap::new();

        for direction in [Direction::Outgoing, Direction::Incoming] {
            for neighbor_idx in self.graph.neighbors_directed(idx, direction) {
                seen.entry(neighbor_idx).or_insert(&self.graph[neighbor_idx]);
            }
        }

        seen.into_values().collect()
    }

    /// Get evidence nodes linked to an artifact (both directions).
    ///
    /// Returns neighbors whose `kind` (case-insensitive) is `"evidence"`.
    pub fn evidence_for(&self, id: &str) -> Vec<&ArtifactNode> {
        self.neighbors(id)
            .into_iter()
            .filter(|n| n.kind.eq_ignore_ascii_case("evidence"))
            .collect()
    }

    /// Get artifacts that depend on this one (incoming edges = "who points at me").
    ///
    /// Useful for impact analysis: if this artifact changes, these are affected.
    pub fn impact_of(&self, id: &str) -> Vec<&ArtifactNode> {
        let Some(&idx) = self.index.get(id) else {
            return Vec::new();
        };

        self.graph
            .neighbors_directed(idx, Direction::Incoming)
            .map(|ni| &self.graph[ni])
            .collect()
    }

    /// Look up an artifact node by its ID.
    pub fn get(&self, id: &str) -> Option<&ArtifactNode> {
        self.index.get(id).map(|&idx| &self.graph[idx])
    }

    /// Total number of artifact nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Total number of relation edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: &str, kind: &str, status: &str) -> ArtifactNode {
        ArtifactNode {
            id: id.to_string(),
            kind: kind.to_string(),
            status: status.to_string(),
        }
    }

    #[test]
    fn empty_graph() {
        let kg = KnowledgeGraph::from_parts(vec![], vec![]);
        assert_eq!(kg.node_count(), 0);
        assert_eq!(kg.edge_count(), 0);
        assert!(kg.get("PRD-001").is_none());
        assert!(kg.neighbors("PRD-001").is_empty());
        assert!(kg.evidence_for("PRD-001").is_empty());
        assert!(kg.impact_of("PRD-001").is_empty());
    }

    #[test]
    fn nodes_without_edges() {
        let nodes = vec![
            make_node("PRD-001", "prd", "active"),
            make_node("RFC-001", "rfc", "draft"),
        ];
        let kg = KnowledgeGraph::from_parts(nodes, vec![]);

        assert_eq!(kg.node_count(), 2);
        assert_eq!(kg.edge_count(), 0);
        assert!(kg.get("PRD-001").is_some());
        assert_eq!(kg.get("PRD-001").unwrap().kind, "prd");
        assert!(kg.neighbors("PRD-001").is_empty());
    }

    #[test]
    fn nodes_with_edges_and_neighbors() {
        let nodes = vec![
            make_node("PRD-001", "prd", "active"),
            make_node("RFC-001", "rfc", "active"),
            make_node("ADR-001", "adr", "draft"),
        ];
        let edges = vec![
            ("RFC-001".into(), "PRD-001".into(), "based_on".into()),
            ("ADR-001".into(), "RFC-001".into(), "decides".into()),
        ];
        let kg = KnowledgeGraph::from_parts(nodes, edges);

        assert_eq!(kg.node_count(), 3);
        assert_eq!(kg.edge_count(), 2);

        // PRD-001 has one neighbor: RFC-001 (incoming edge)
        let prd_neighbors = kg.neighbors("PRD-001");
        assert_eq!(prd_neighbors.len(), 1);
        assert_eq!(prd_neighbors[0].id, "RFC-001");

        // RFC-001 has two neighbors: PRD-001 (outgoing) and ADR-001 (incoming)
        let rfc_neighbors = kg.neighbors("RFC-001");
        assert_eq!(rfc_neighbors.len(), 2);
        let rfc_ids: Vec<&str> = rfc_neighbors.iter().map(|n| n.id.as_str()).collect();
        assert!(rfc_ids.contains(&"PRD-001"));
        assert!(rfc_ids.contains(&"ADR-001"));
    }

    #[test]
    fn evidence_for_filters_by_kind() {
        let nodes = vec![
            make_node("PRD-001", "prd", "active"),
            make_node("EVID-001", "evidence", "active"),
            make_node("EVID-002", "evidence", "draft"),
            make_node("RFC-001", "rfc", "active"),
        ];
        let edges = vec![
            ("EVID-001".into(), "PRD-001".into(), "informs".into()),
            ("EVID-002".into(), "PRD-001".into(), "informs".into()),
            ("RFC-001".into(), "PRD-001".into(), "based_on".into()),
        ];
        let kg = KnowledgeGraph::from_parts(nodes, edges);

        let evidence = kg.evidence_for("PRD-001");
        assert_eq!(evidence.len(), 2);
        assert!(evidence.iter().all(|n| n.kind == "evidence"));
    }

    #[test]
    fn evidence_for_checks_both_directions() {
        let nodes = vec![
            make_node("PRD-001", "prd", "active"),
            make_node("EVID-001", "evidence", "active"),
        ];
        // Edge goes FROM PRD TO evidence (reverse direction)
        let edges = vec![
            ("PRD-001".into(), "EVID-001".into(), "supported_by".into()),
        ];
        let kg = KnowledgeGraph::from_parts(nodes, edges);

        let evidence = kg.evidence_for("PRD-001");
        assert_eq!(evidence.len(), 1);
        assert_eq!(evidence[0].id, "EVID-001");
    }

    #[test]
    fn impact_of_returns_incoming_dependents() {
        let nodes = vec![
            make_node("PRD-001", "prd", "active"),
            make_node("RFC-001", "rfc", "active"),
            make_node("RFC-002", "rfc", "draft"),
            make_node("ADR-001", "adr", "active"),
        ];
        let edges = vec![
            ("RFC-001".into(), "PRD-001".into(), "based_on".into()),
            ("RFC-002".into(), "PRD-001".into(), "based_on".into()),
            ("ADR-001".into(), "RFC-001".into(), "decides".into()),
        ];
        let kg = KnowledgeGraph::from_parts(nodes, edges);

        // Two RFCs depend on PRD-001
        let impact = kg.impact_of("PRD-001");
        assert_eq!(impact.len(), 2);
        let ids: Vec<&str> = impact.iter().map(|n| n.id.as_str()).collect();
        assert!(ids.contains(&"RFC-001"));
        assert!(ids.contains(&"RFC-002"));

        // Nothing depends on ADR-001
        assert!(kg.impact_of("ADR-001").is_empty());
    }

    #[test]
    fn get_returns_none_for_missing_id() {
        let nodes = vec![make_node("PRD-001", "prd", "active")];
        let kg = KnowledgeGraph::from_parts(nodes, vec![]);
        assert!(kg.get("NONEXISTENT").is_none());
    }

    #[test]
    fn edges_with_unknown_nodes_are_skipped() {
        let nodes = vec![make_node("PRD-001", "prd", "active")];
        // Edge references RFC-001 which doesn't exist as a node
        let edges = vec![
            ("RFC-001".into(), "PRD-001".into(), "based_on".into()),
        ];
        let kg = KnowledgeGraph::from_parts(nodes, edges);

        assert_eq!(kg.node_count(), 1);
        assert_eq!(kg.edge_count(), 0); // Edge skipped since RFC-001 not in graph
    }
}
