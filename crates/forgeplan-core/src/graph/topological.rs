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

/// Run Kahn's algorithm on artifact relations.
///
/// `edges`: (from, to, relation_type) — "from" depends on "to"
/// `active_ids`: set of artifact IDs that are considered "done" (active status)
///
/// Returns topological order + cycle detection + ready/blocked classification.
pub fn kahn_sort(
    edges: &[(String, String, String)],
    active_ids: &HashSet<String>,
) -> TopologicalResult {
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
            .filter(|dep| !active_ids.contains(dep))
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
pub fn get_blocked_by(
    artifact_id: &str,
    edges: &[(String, String, String)],
    active_ids: &HashSet<String>,
) -> Vec<String> {
    edges
        .iter()
        .filter(|(from, _, _)| from == artifact_id)
        .map(|(_, to, _)| to.clone())
        .filter(|dep| !active_ids.contains(dep))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_chain_sorted_correctly() {
        let edges = vec![
            ("A".into(), "B".into(), "depends_on".into()),
            ("B".into(), "C".into(), "depends_on".into()),
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
            ("D".into(), "B".into(), "depends_on".into()),
            ("D".into(), "C".into(), "depends_on".into()),
            ("B".into(), "A".into(), "depends_on".into()),
            ("C".into(), "A".into(), "depends_on".into()),
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
            ("A".into(), "B".into(), "depends_on".into()),
            ("B".into(), "A".into(), "depends_on".into()),
        ];
        let result = kahn_sort(&edges, &HashSet::new());
        assert!(!result.cycles.is_empty());
        assert!(result.order.is_empty());
    }

    #[test]
    fn ready_vs_blocked() {
        let edges = vec![(
            "RFC-001".into(),
            "PRD-001".into(),
            "based_on".into(),
        )];
        let mut active = HashSet::new();
        active.insert("PRD-001".to_string());

        let result = kahn_sort(&edges, &active);
        assert!(result.ready.contains(&"RFC-001".to_string()));
        assert!(result.ready.contains(&"PRD-001".to_string()));
        assert!(result.blocked.is_empty());
    }

    #[test]
    fn blocked_when_dep_not_active() {
        let edges = vec![(
            "RFC-001".into(),
            "PRD-001".into(),
            "based_on".into(),
        )];
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
    fn three_node_cycle() {
        let edges = vec![
            ("A".into(), "B".into(), "depends_on".into()),
            ("B".into(), "C".into(), "depends_on".into()),
            ("C".into(), "A".into(), "depends_on".into()),
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
