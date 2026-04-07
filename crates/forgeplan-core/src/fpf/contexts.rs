//! Bounded Context detection — auto-detect artifact clusters from link graph.
//!
//! Uses connected-component analysis on the undirected link graph.
//! Artifacts that are densely connected form a "bounded context" (module).

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::db::store::ArtifactRecord;

/// A detected bounded context (cluster of related artifacts).
#[derive(Debug, Clone)]
pub struct BoundedContext {
    /// Auto-generated name from the most common kind prefix.
    pub name: String,
    /// Artifact IDs in this context.
    pub members: Vec<String>,
    /// Internal link count (edges within the context).
    pub internal_links: usize,
    /// External link count (edges crossing context boundary).
    pub external_links: usize,
    /// Cohesion = internal / (internal + external). Higher = more cohesive.
    pub cohesion: f64,
}

/// Detect bounded contexts using connected components on the link graph.
///
/// Each connected component = one bounded context.
/// Singletons (no links) are grouped into an "Unlinked" context.
pub fn detect(records: &[ArtifactRecord], edges: &[(String, String)]) -> Vec<BoundedContext> {
    let all_ids: BTreeSet<String> = records.iter().map(|r| r.id.clone()).collect();

    // Build adjacency list (undirected)
    let mut adj: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (src, tgt) in edges {
        if all_ids.contains(src) && all_ids.contains(tgt) {
            adj.entry(src.clone()).or_default().insert(tgt.clone());
            adj.entry(tgt.clone()).or_default().insert(src.clone());
        }
    }

    // BFS to find connected components
    let mut visited: BTreeSet<String> = BTreeSet::new();
    let mut components: Vec<Vec<String>> = Vec::new();

    for id in &all_ids {
        if visited.contains(id) {
            continue;
        }
        // Check if this node has any edges
        if !adj.contains_key(id) {
            visited.insert(id.clone());
            continue; // Skip singletons for now
        }

        let mut component = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(id.clone());
        visited.insert(id.clone());

        while let Some(current) = queue.pop_front() {
            component.push(current.clone());
            if let Some(neighbors) = adj.get(&current) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        visited.insert(neighbor.clone());
                        queue.push_back(neighbor.clone());
                    }
                }
            }
        }

        component.sort();
        components.push(component);
    }

    // Collect singletons (no adjacency = no links to other artifacts)
    let singletons: Vec<String> = all_ids
        .iter()
        .filter(|id| !adj.contains_key(*id))
        .cloned()
        .collect();

    // Build BoundedContext for each component
    let mut contexts: Vec<BoundedContext> = Vec::new();

    for (i, members) in components.iter().enumerate() {
        if members.len() < 2 {
            continue; // Skip trivial components
        }

        let member_set: BTreeSet<&String> = members.iter().collect();

        // Count internal vs external links
        let mut internal = 0;
        let mut external = 0;
        for (src, tgt) in edges {
            let src_in = member_set.contains(src);
            let tgt_in = member_set.contains(tgt);
            if src_in && tgt_in {
                internal += 1;
            } else if src_in || tgt_in {
                external += 1;
            }
        }

        let cohesion = if internal + external > 0 {
            internal as f64 / (internal + external) as f64
        } else {
            0.0
        };

        // Name from dominant kind
        let name = name_from_members(members, i);

        contexts.push(BoundedContext {
            name,
            members: members.clone(),
            internal_links: internal,
            external_links: external,
            cohesion,
        });
    }

    // Add unlinked context if there are orphans
    if !singletons.is_empty() {
        contexts.push(BoundedContext {
            name: "Unlinked".to_string(),
            members: singletons,
            internal_links: 0,
            external_links: 0,
            cohesion: 0.0,
        });
    }

    // Sort by size descending
    contexts.sort_by(|a, b| b.members.len().cmp(&a.members.len()));

    contexts
}

/// Generate a name from the most common kind in the cluster.
fn name_from_members(members: &[String], index: usize) -> String {
    let mut kind_count: BTreeMap<&str, usize> = BTreeMap::new();
    for id in members {
        let kind = if id.to_uppercase().starts_with("PRD") {
            "PRD"
        } else if id.to_uppercase().starts_with("RFC") {
            "RFC"
        } else if id.to_uppercase().starts_with("EPIC") {
            "Epic"
        } else if id.to_uppercase().starts_with("ADR") {
            "ADR"
        } else if id.to_uppercase().starts_with("PROB") {
            "Problem"
        } else {
            "Mixed"
        };
        *kind_count.entry(kind).or_default() += 1;
    }

    let dominant = kind_count
        .into_iter()
        .max_by_key(|(_, c)| *c)
        .map(|(k, _)| k)
        .unwrap_or("Mixed");

    format!("Context-{} ({})", index + 1, dominant)
}

/// Detect which bounded context a specific artifact belongs to.
///
/// Returns (cluster_name, member_count, cohesion) or None if the artifact
/// is a singleton (no links). Runs full graph detection internally.
pub async fn detect_for_artifact(
    store: &crate::db::store::LanceStore,
    artifact_id: &str,
) -> anyhow::Result<Option<(String, usize, f64)>> {
    let all_records = store.list_records(None).await?;
    let all_relations = store.get_all_relations().await?;
    let edges: Vec<(String, String)> = all_relations
        .iter()
        .map(|(s, t, _)| (s.clone(), t.clone()))
        .collect();
    let ctxs = detect(&all_records, &edges);
    Ok(ctxs
        .into_iter()
        .find(|c| c.members.iter().any(|m| m == artifact_id))
        .map(|c| (c.name, c.members.len(), c.cohesion)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: &str) -> ArtifactRecord {
        ArtifactRecord {
            id: id.into(),
            kind: "prd".into(),
            status: "draft".into(),
            title: id.into(),
            body: String::new(),
            depth: "standard".into(),
            author: None,
            parent_epic: None,
            r_eff_score: 0.0,
            valid_until: None,
            created_at: "2026-01-01T00:00:00".into(),
            updated_at: "2026-01-01T00:00:00".into(),
            tags: Vec::new(),
        }
    }

    #[test]
    fn no_edges_no_contexts() {
        let records = vec![record("PRD-001"), record("PRD-002")];
        let contexts = detect(&records, &[]);
        // Singletons go to "Unlinked"
        assert!(contexts.iter().any(|c| c.name == "Unlinked"));
    }

    #[test]
    fn connected_artifacts_form_cluster() {
        let records = vec![
            record("PRD-001"),
            record("RFC-001"),
            record("ADR-001"),
            record("NOTE-001"),
        ];
        let edges = vec![
            ("PRD-001".into(), "RFC-001".into()),
            ("RFC-001".into(), "ADR-001".into()),
        ];
        let contexts = detect(&records, &edges);

        // PRD-001, RFC-001, ADR-001 should be in one context
        let main_ctx = contexts.iter().find(|c| c.members.len() == 3);
        assert!(main_ctx.is_some(), "Expected a 3-member cluster");
        let ctx = main_ctx.unwrap();
        assert!(ctx.members.contains(&"PRD-001".to_string()));
        assert!(ctx.members.contains(&"RFC-001".to_string()));
        assert!(ctx.members.contains(&"ADR-001".to_string()));

        // NOTE-001 should be in Unlinked
        let unlinked = contexts.iter().find(|c| c.name == "Unlinked");
        assert!(unlinked.is_some());
    }

    #[test]
    fn two_separate_clusters() {
        let records = vec![
            record("PRD-001"),
            record("RFC-001"),
            record("PRD-002"),
            record("RFC-002"),
        ];
        let edges = vec![
            ("PRD-001".into(), "RFC-001".into()),
            ("PRD-002".into(), "RFC-002".into()),
        ];
        let contexts = detect(&records, &edges);

        let real_contexts: Vec<_> = contexts.iter().filter(|c| c.name != "Unlinked").collect();
        assert_eq!(real_contexts.len(), 2);
    }

    #[test]
    fn cohesion_calculated() {
        let records = vec![record("PRD-001"), record("RFC-001"), record("ADR-001")];
        let edges = vec![
            ("PRD-001".into(), "RFC-001".into()),
            ("RFC-001".into(), "ADR-001".into()),
            ("PRD-001".into(), "ADR-001".into()),
        ];
        let contexts = detect(&records, &edges);

        let ctx = contexts.iter().find(|c| c.members.len() == 3).unwrap();
        assert_eq!(ctx.internal_links, 3);
        assert_eq!(ctx.external_links, 0);
        assert!((ctx.cohesion - 1.0).abs() < f64::EPSILON);
    }
}
