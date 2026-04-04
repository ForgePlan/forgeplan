//! Topological sort (Kahn's algorithm) and cycle detection for artifact DAG.

use std::collections::{HashMap, HashSet, VecDeque};

/// Result of topological sort.
#[derive(Debug, Clone)]
pub struct TopologicalResult {
    /// Artifacts in dependency order (do first -> do last).
    pub order: Vec<String>,
    /// Cycle paths if any (e.g., ["A", "B", "C", "A"]).
    pub cycles: Vec<Vec<String>>,
    /// Artifacts with no dependencies (ready to work on).
    pub ready: Vec<String>,
    /// Artifacts that are blocked (have unmet dependencies).
    pub blocked: Vec<(String, Vec<String>)>,
}

/// Structural relation types that imply dependency (blocking).
/// Informational relations like "informs" and "supports" do NOT block.
const STRUCTURAL_RELATIONS: &[&str] = &["based_on", "refines", "supersedes", "contradicts"];

/// Check if a relation type is structural (blocking).
pub fn is_structural_relation(relation: &str) -> bool {
    STRUCTURAL_RELATIONS.contains(&relation.to_lowercase().as_str())
}

/// Run Kahn's algorithm on artifact relations.
///
/// `edges`: (from, to, relation_type) — "from" depends on "to"
/// `resolved_ids`: set of artifact IDs that are considered "resolved" (active, deprecated, or superseded)
///
/// Only structural relations (based_on, refines, supersedes, contradicts) are treated
/// as blocking dependencies. Informational relations (informs, supports) are excluded.
///
/// Returns topological order + cycle detection + ready/blocked classification.
pub fn kahn_sort(
    edges: &[(String, String, String)],
    resolved_ids: &HashSet<String>,
) -> TopologicalResult {
    // Filter to structural (blocking) relations only
    let structural_edges: Vec<_> = edges
        .iter()
        .filter(|(_, _, rel)| is_structural_relation(rel))
        .cloned()
        .collect();
    let edges = structural_edges.as_slice();

    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut all_nodes: HashSet<String> = HashSet::new();

    for (from, to, _rel) in edges {
        all_nodes.insert(from.clone());
        all_nodes.insert(to.clone());
        adj.entry(from.clone()).or_default().push(to.clone());
        *in_degree.entry(to.clone()).or_default() += 1;
        in_degree.entry(from.clone()).or_default();
    }

    // Kahn's: start with nodes that have in_degree 0
    let mut queue: VecDeque<String> = all_nodes
        .iter()
        .filter(|n| *in_degree.get(*n).unwrap_or(&0) == 0)
        .cloned()
        .collect();

    // Sort queue for deterministic output
    let mut initial: Vec<String> = queue.drain(..).collect();
    initial.sort();
    queue.extend(initial);

    let mut order = Vec::new();
    let mut visited = HashSet::new();

    while let Some(node) = queue.pop_front() {
        order.push(node.clone());
        visited.insert(node.clone());

        if let Some(neighbors) = adj.get(&node) {
            let mut sorted_neighbors: Vec<&String> = neighbors.iter().collect();
            sorted_neighbors.sort();
            for neighbor in sorted_neighbors {
                if let Some(deg) = in_degree.get_mut(neighbor) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(neighbor.clone());
                    }
                }
            }
        }
    }

    // Detect cycles: nodes not in order are part of cycles
    let cycle_nodes: HashSet<String> = all_nodes.difference(&visited).cloned().collect();
    let cycles = if cycle_nodes.is_empty() {
        vec![]
    } else {
        detect_cycle_paths(&adj, &cycle_nodes)
    };

    // Classify: ready (no unmet deps) vs blocked
    let mut ready = Vec::new();
    let mut blocked = Vec::new();

    for node in &order {
        let unmet: Vec<String> = edges
            .iter()
            .filter(|(from, _, _)| from == node)
            .map(|(_, to, _)| to.clone())
            .filter(|dep| !resolved_ids.contains(dep))
            .collect();

        if unmet.is_empty() {
            ready.push(node.clone());
        } else {
            blocked.push((node.clone(), unmet));
        }
    }

    TopologicalResult {
        order,
        cycles,
        ready,
        blocked,
    }
}

/// Find cycle paths using DFS.
fn detect_cycle_paths(
    adj: &HashMap<String, Vec<String>>,
    cycle_nodes: &HashSet<String>,
) -> Vec<Vec<String>> {
    let mut cycles = Vec::new();
    let mut visited = HashSet::new();

    let mut sorted_starts: Vec<&String> = cycle_nodes.iter().collect();
    sorted_starts.sort();

    for start in sorted_starts {
        if visited.contains(start) {
            continue;
        }
        let mut path = vec![start.clone()];
        let mut current = start.clone();

        loop {
            visited.insert(current.clone());
            let next = adj
                .get(&current)
                .and_then(|neighbors| neighbors.iter().find(|n| cycle_nodes.contains(*n)));

            match next {
                Some(n) if path.contains(n) => {
                    if let Some(pos) = path.iter().position(|x| x == n) {
                        let mut cycle: Vec<String> = path[pos..].to_vec();
                        cycle.push(n.clone());
                        cycles.push(cycle);
                    }
                    break;
                }
                Some(n) => {
                    path.push(n.clone());
                    current = n.clone();
                }
                None => break,
            }
        }
    }

    cycles
}

/// Get blocked status for a specific artifact.
/// Only structural relations are considered blocking.
pub fn get_blocked_by(
    artifact_id: &str,
    edges: &[(String, String, String)],
    resolved_ids: &HashSet<String>,
) -> Vec<String> {
    edges
        .iter()
        .filter(|(from, _, rel)| from == artifact_id && is_structural_relation(rel))
        .map(|(_, to, _)| to.clone())
        .filter(|dep| !resolved_ids.contains(dep))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_chain_sorted_correctly() {
        let edges = vec![
            ("A".into(), "B".into(), "based_on".into()),
            ("B".into(), "C".into(), "based_on".into()),
        ];
        let result = kahn_sort(&edges, &HashSet::new());
        // A depends on B, B depends on C → order: A, B, C (roots first in Kahn's)
        // But "from depends on to" means to has lower in-degree from from's perspective
        // In our adjacency: A->B, B->C. in_degree: A=0, B=1, C=1
        // Wait — Kahn's processes in_degree=0 first. A has in_degree 0.
        // After removing A: B in_degree drops to 0. After B: C.
        // So order = [A, B, C]
        assert_eq!(result.order, vec!["A", "B", "C"]);
        assert!(result.cycles.is_empty());
    }

    #[test]
    fn diamond_dependency() {
        let edges = vec![
            ("D".into(), "B".into(), "based_on".into()),
            ("D".into(), "C".into(), "based_on".into()),
            ("B".into(), "A".into(), "based_on".into()),
            ("C".into(), "A".into(), "based_on".into()),
        ];
        let result = kahn_sort(&edges, &HashSet::new());
        // in_degree: D=0, B=1, C=1, A=2
        // D first, then B and C (both in_degree 0), then A
        assert_eq!(*result.order.first().unwrap(), "D");
        assert_eq!(*result.order.last().unwrap(), "A");
        assert!(result.cycles.is_empty());
    }

    #[test]
    fn cycle_detected() {
        let edges = vec![
            ("A".into(), "B".into(), "based_on".into()),
            ("B".into(), "A".into(), "based_on".into()),
        ];
        let result = kahn_sort(&edges, &HashSet::new());
        assert!(!result.cycles.is_empty());
        assert!(result.order.is_empty());
    }

    #[test]
    fn ready_vs_blocked() {
        let edges = vec![("RFC-001".into(), "PRD-001".into(), "based_on".into())];
        let mut active = HashSet::new();
        active.insert("PRD-001".to_string());

        let result = kahn_sort(&edges, &active);
        assert!(result.ready.contains(&"RFC-001".to_string()));
        assert!(result.ready.contains(&"PRD-001".to_string()));
        assert!(result.blocked.is_empty());
    }

    #[test]
    fn blocked_when_dep_not_active() {
        let edges = vec![("RFC-001".into(), "PRD-001".into(), "based_on".into())];
        let result = kahn_sort(&edges, &HashSet::new());
        assert!(result.blocked.iter().any(|(id, _)| id == "RFC-001"));
    }

    #[test]
    fn no_edges_all_ready() {
        let result = kahn_sort(&[], &HashSet::new());
        assert!(result.order.is_empty());
        assert!(result.cycles.is_empty());
        assert!(result.ready.is_empty());
    }

    #[test]
    fn get_blocked_by_returns_unmet_deps() {
        let edges = vec![
            ("RFC-001".into(), "PRD-001".into(), "based_on".into()),
            ("RFC-001".into(), "PRD-002".into(), "based_on".into()),
        ];
        let mut active = HashSet::new();
        active.insert("PRD-001".to_string());

        let blocked = get_blocked_by("RFC-001", &edges, &active);
        assert_eq!(blocked, vec!["PRD-002".to_string()]);
    }

    #[test]
    fn informational_relations_do_not_block() {
        // EVID-001 --informs--> PRD-001: should NOT make PRD-001 blocked
        let edges = vec![
            ("EVID-001".into(), "PRD-001".into(), "informs".into()),
            ("EVID-002".into(), "PRD-001".into(), "supports".into()),
        ];
        let result = kahn_sort(&edges, &HashSet::new());
        // Informational relations are excluded, so no edges => no nodes in graph
        assert!(
            result.blocked.is_empty(),
            "informational relations should not block"
        );
        assert!(
            result.order.is_empty(),
            "no structural edges => empty graph"
        );

        // get_blocked_by should also ignore informational relations
        let blocked = get_blocked_by("EVID-001", &edges, &HashSet::new());
        assert!(blocked.is_empty(), "informs should not count as blocking");
    }

    #[test]
    fn structural_relations_do_block() {
        let edges = vec![
            ("RFC-001".into(), "PRD-001".into(), "based_on".into()),
            ("EVID-001".into(), "PRD-001".into(), "informs".into()),
        ];
        let result = kahn_sort(&edges, &HashSet::new());
        // Only based_on edge should be in the graph
        assert_eq!(result.order.len(), 2, "should have RFC-001 and PRD-001");
        assert!(
            result.blocked.iter().any(|(id, _)| id == "RFC-001"),
            "RFC-001 should be blocked by PRD-001 via based_on"
        );
        // EVID-001 should NOT appear at all (its only relation is informational)
        assert!(
            !result.order.contains(&"EVID-001".to_string()),
            "EVID-001 should not be in the graph (only has informational relation)"
        );
    }

    #[test]
    fn deprecated_does_not_block() {
        // PROB-010 (deprecated) should NOT block PRD-014
        let edges = vec![("PRD-014".into(), "PROB-010".into(), "based_on".into())];
        // resolved_ids includes deprecated artifacts
        let mut resolved = HashSet::new();
        resolved.insert("PROB-010".to_string()); // deprecated but resolved

        let result = kahn_sort(&edges, &resolved);
        // PRD-014 should NOT be blocked
        assert!(
            result.blocked.is_empty(),
            "deprecated artifact should not block: {:?}",
            result.blocked
        );
        assert!(result.ready.contains(&"PRD-014".to_string()));
    }

    #[test]
    fn superseded_does_not_block() {
        // PRD-002 (superseded) should NOT block RFC-001
        let edges = vec![("RFC-001".into(), "PRD-002".into(), "based_on".into())];
        let mut resolved = HashSet::new();
        resolved.insert("PRD-002".to_string()); // superseded but resolved

        let result = kahn_sort(&edges, &resolved);
        assert!(
            result.blocked.is_empty(),
            "superseded artifact should not block: {:?}",
            result.blocked
        );
    }

    #[test]
    fn draft_still_blocks() {
        // Draft artifacts should still block
        let edges = vec![("RFC-001".into(), "NOTE-015".into(), "based_on".into())];
        // NOTE-015 is draft, NOT in resolved set
        let resolved = HashSet::new();
        let result = kahn_sort(&edges, &resolved);
        assert!(
            result.blocked.iter().any(|(id, _)| id == "RFC-001"),
            "draft artifact should block"
        );
    }

    #[test]
    fn mixed_draft_and_deprecated_deps() {
        // RFC-001 depends on: PRD-001 (draft) + ADR-001 (deprecated=resolved)
        // Only PRD-001 should block
        let edges = vec![
            ("RFC-001".into(), "PRD-001".into(), "based_on".into()),
            ("RFC-001".into(), "ADR-001".into(), "based_on".into()),
        ];
        let mut resolved = HashSet::new();
        resolved.insert("ADR-001".to_string()); // deprecated = resolved
        let result = kahn_sort(&edges, &resolved);
        let blocked_entry = result.blocked.iter().find(|(id, _)| id == "RFC-001");
        assert!(
            blocked_entry.is_some(),
            "RFC-001 should be blocked by draft PRD-001"
        );
        let blockers = &blocked_entry.unwrap().1;
        assert_eq!(blockers, &vec!["PRD-001".to_string()]);
        assert!(
            !blockers.contains(&"ADR-001".to_string()),
            "deprecated ADR-001 should NOT block"
        );
    }

    #[test]
    fn stale_blocks_by_design() {
        // Stale artifacts are NOT in resolved_ids — they should block.
        // This is intentional: stale = expired valid_until, needs renewal before it can unblock.
        let edges = vec![("RFC-001".into(), "ADR-001".into(), "based_on".into())];
        // ADR-001 is stale — NOT in resolved set
        let resolved = HashSet::new();
        let result = kahn_sort(&edges, &resolved);
        assert!(
            result.blocked.iter().any(|(id, _)| id == "RFC-001"),
            "stale artifact should block (by design — needs renew first)"
        );
    }

    #[test]
    fn three_node_cycle() {
        let edges = vec![
            ("A".into(), "B".into(), "based_on".into()),
            ("B".into(), "C".into(), "based_on".into()),
            ("C".into(), "A".into(), "based_on".into()),
        ];
        let result = kahn_sort(&edges, &HashSet::new());
        assert!(result.order.is_empty());
        assert!(!result.cycles.is_empty());
        // The cycle should contain all three nodes
        let cycle = &result.cycles[0];
        assert!(cycle.contains(&"A".to_string()));
        assert!(cycle.contains(&"B".to_string()));
        assert!(cycle.contains(&"C".to_string()));
    }
}
