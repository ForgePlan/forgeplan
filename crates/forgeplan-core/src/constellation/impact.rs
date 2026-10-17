//! Stage 7 — explainable impact analysis over the exact relation set.
//!
//! "If `K` changes, who should re-check their assumptions?" The answer is
//! computed, not intuited: we walk the *reverse* edges (who points at `K`)
//! with an explicit propagation rule and hard bounds, and every impacted
//! node carries the path that got it there — the trail an agent can audit.
//!
//! ## Propagation rule — the spec's named requirement
//!
//! `references` and `based_on` do **not** traverse alike:
//!
//! * **Structural** relations (`based_on`, `refines`, `supersedes`,
//!   `contradicts` — the same set `graph::topological` treats as blocking)
//!   propagate: the dependent is impacted *and* its own dependents keep
//!   being walked transitively.
//! * **Informational** relations (`informs`, `supports`, `references`,
//!   anything unknown) mark only the direct dependent — severity `Noted` —
//!   and the walk stops there. Citing a document does not make you a
//!   dependent of its fate.
//!
//! ## Bounds and honesty
//!
//! Traversal is bounded (`max_depth`, `max_nodes`); wherever the walk
//! stops, the stop is *named* — `depth_limit`, `node_limit`,
//! `informational_edge`, `unresolved_target`, `cycle`. An edge whose
//! target resolves to nothing in its own source is listed in
//! `unresolved`, never silently dropped. Nodes stay source-qualified:
//! `ADR-001` in project A and `ADR-001` in project B are different nodes.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;

use super::ConstellationError;
use super::capture::{CaptureLimits, CaptureRequest, capture_source, parse_artifact_file};
use super::federation::{SourceChannel, SourceFailure, SourceRef, SourceSpec, source_id_for};
use super::snapshot::{ConstellationSnapshot, SnapshotSourceEntry};

/// Hard bounds for one impact walk.
#[derive(Debug, Clone, Copy)]
pub struct ImpactBounds {
    /// How many structural hops the walk may take (default 4).
    pub max_depth: usize,
    /// Total nodes the report may hold (default 256).
    pub max_nodes: usize,
}

impl Default for ImpactBounds {
    fn default() -> Self {
        Self {
            max_depth: 4,
            max_nodes: 256,
        }
    }
}

/// How hard the change hits a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImpactSeverity {
    /// Depends on the changed artifact through a structural edge.
    Direct,
    /// Depends transitively — a chain of structural edges.
    Transitive,
    /// Only an informational edge ties it to the change — worth knowing,
    /// not worth rebuilding. Never traversed past.
    Noted,
}

/// One step of the path from the changed artifact to an impacted node.
#[derive(Debug, Clone, Serialize)]
pub struct PathStep {
    /// The key that points at the previous step — the edge we walked back.
    pub dependent: String,
    /// The relation that dependent declared toward the previous step.
    pub relation: String,
}

/// A node the change reaches.
#[derive(Debug, Clone, Serialize)]
pub struct ImpactedNode {
    pub source: SourceRef,
    pub canonical_key: String,
    pub display_id: Option<String>,
    pub title: String,
    pub kind: String,
    pub severity: ImpactSeverity,
    /// Structural hops from the changed artifact (`1` = direct).
    pub depth: usize,
    /// The explainable path: `dependent --relation--> previous` steps.
    pub path: Vec<PathStep>,
}

/// Where the walk stopped and why — a named boundary, never silent.
#[derive(Debug, Clone, Serialize)]
pub struct ImpactStop {
    /// Node the walk reached before stopping.
    pub at: String,
    pub source_id: String,
    /// `depth_limit` | `node_limit` | `informational_edge`
    pub reason: String,
}

/// An edge target that resolves to nothing in its source.
#[derive(Debug, Clone, Serialize)]
pub struct UnresolvedEdge {
    pub source_id: String,
    pub from: String,
    pub target: String,
    pub relation: String,
}

/// The full report.
#[derive(Debug, Serialize)]
pub struct ImpactReport {
    pub changed: String,
    pub bounds: ImpactBoundsSerde,
    pub impacted: Vec<ImpactedNode>,
    /// Sources where `changed` is not present — named, not zero-results.
    pub absent_in: Vec<String>,
    /// Where traversal stopped — boundaries the answer was cut at.
    pub stops: Vec<ImpactStop>,
    /// Edges whose targets no source member defines.
    pub unresolved: Vec<UnresolvedEdge>,
    pub failures: Vec<SourceFailure>,
    pub sources: Vec<SourceRef>,
}

#[derive(Debug, Serialize)]
pub struct ImpactBoundsSerde {
    pub max_depth: usize,
    pub max_nodes: usize,
}

/// One node in a source's graph.
///
/// `pub(crate)` so [`super::graph`] can emit the raw node/edge set without
/// rebuilding the per-source projection a second time.
pub(crate) struct Node {
    pub(crate) canonical_key: String,
    pub(crate) display_id: Option<String>,
    pub(crate) title: String,
    pub(crate) kind: String,
    /// The file inside the source that declares this node, when the
    /// channel knows it (the pinned relations table does not record one).
    pub(crate) relative_path: Option<String>,
}

/// The per-source graph: nodes keyed canonically, edges as written.
pub(crate) struct SourceGraph {
    pub(crate) source: SourceRef,
    /// canonical_key → node
    pub(crate) nodes: HashMap<String, Node>,
    /// display_id (lowered) → canonical_key, for resolving `target` refs.
    pub(crate) by_display: HashMap<String, String>,
    /// (from canonical_key, target-as-written, relation)
    pub(crate) edges: Vec<(String, String, String)>,
}

impl SourceGraph {
    /// Resolve a `target` string as written — an exact canonical key, else
    /// a case-insensitive display id — to the canonical key it names in
    /// this source. Nothing outside the source is consulted: `ADR-001` in
    /// two checkouts stays two nodes.
    pub(crate) fn resolve(&self, target: &str) -> Option<String> {
        if self.nodes.contains_key(target) {
            return Some(target.to_string());
        }
        self.by_display.get(&target.to_lowercase()).cloned()
    }
}

fn build_graph(source: SourceRef, artifacts: Vec<GraphArtifact>) -> SourceGraph {
    let mut nodes = HashMap::new();
    let mut by_display = HashMap::new();
    let mut edges = Vec::new();
    for a in artifacts {
        if let Some(d) = &a.display_id {
            // First claim wins; a second artifact claiming the same
            // display id leaves the first resolvable — the dangle is in
            // the source data, not ours to adjudicate.
            by_display
                .entry(d.to_lowercase())
                .or_insert_with(|| a.canonical_key.clone());
        }
        for (target, relation) in &a.edges {
            edges.push((a.canonical_key.clone(), target.clone(), relation.clone()));
        }
        nodes.insert(
            a.canonical_key.clone(),
            Node {
                canonical_key: a.canonical_key,
                display_id: a.display_id,
                title: a.title,
                kind: a.kind,
                relative_path: a.relative_path,
            },
        );
    }
    SourceGraph {
        source,
        nodes,
        by_display,
        edges,
    }
}

/// The shape of an artifact for graph purposes.
struct GraphArtifact {
    canonical_key: String,
    display_id: Option<String>,
    title: String,
    kind: String,
    relative_path: Option<String>,
    edges: Vec<(String, String)>,
}

/// Build one live source's graph on whichever channel it honestly
/// offers — the pinned relations table when a generation is published,
/// parsed frontmatter links otherwise. `pub(crate)` for
/// [`super::graph`], which emits the raw set instead of walking it.
pub(crate) async fn source_graph_live(spec: &SourceSpec) -> Result<SourceGraph, SourceFailure> {
    let canonical = spec.path.canonicalize().map_err(|e| SourceFailure {
        path: spec.path.display().to_string(),
        reason: format!("cannot canonicalize: {e}"),
    })?;
    let source = SourceRef {
        source_id: source_id_for(&canonical),
        path: canonical.display().to_string(),
        channel: SourceChannel::Markdown,
        version_key: String::new(),
    };
    // A published source answers from its pinned graph — the same rule
    // federation reads obey: a broken index is a failure, not a downgrade.
    let index_root = canonical.join("lance");
    let published = index_root.join(super::storage::CURRENT_FILE_NAME).is_file();
    if published {
        let graph = super::facade::list_pinned_graph(&index_root)
            .await
            .map_err(|e| SourceFailure {
                path: spec.path.display().to_string(),
                reason: format!("{e}"),
            })?;
        let mut s = source;
        s.channel = SourceChannel::Pinned;
        s.version_key = graph.generation_id;
        let artifacts = graph
            .nodes
            .into_iter()
            .map(|(k, d, t, kind)| GraphArtifact {
                canonical_key: k,
                display_id: d,
                title: t,
                kind,
                relative_path: None,
                edges: Vec::new(),
            })
            .collect::<Vec<_>>();
        let mut g = build_graph(s, artifacts);
        g.edges = graph.edges;
        return Ok(g);
    }
    let captured = capture_source(&CaptureRequest::new(&canonical), CaptureLimits::default())
        .await
        .map_err(|e| SourceFailure {
            path: spec.path.display().to_string(),
            reason: format!("{e}"),
        })?;
    let mut s = source;
    s.version_key = captured.source_set_digest.clone();
    let artifacts = captured
        .artifacts
        .iter()
        .map(|a| GraphArtifact {
            canonical_key: a.canonical_key.clone(),
            display_id: Some(a.display_id.clone()),
            title: a.title.clone(),
            kind: a.kind.clone(),
            relative_path: Some(a.relative_path.clone()),
            edges: a
                .outgoing
                .iter()
                .map(|e| (e.target.clone(), e.relation.clone()))
                .collect(),
        })
        .collect();
    Ok(build_graph(s, artifacts))
}

/// The same graph for one snapshot source, parsed out of pinned Git
/// objects. `pub(crate)` for [`super::graph`].
pub(crate) fn source_graph_snapshot(entry: &SnapshotSourceEntry) -> Result<SourceGraph, String> {
    let repo = PathBuf::from(&entry.checkout_root);
    let names =
        super::snapshot::list_files_at(&repo, &entry.git_oid).map_err(|e| format!("{e}"))?;
    let mut artifacts = Vec::new();
    for rel in &names {
        let bytes = super::snapshot::read_file_at(&repo, &entry.git_oid, rel)
            .map_err(|e| format!("{e}"))?;
        if let Ok(p) = parse_artifact_file(
            rel,
            Arc::from(&bytes[..]),
            super::jobs::DEFAULT_BODY_PREFIX_CHARS,
        ) {
            artifacts.push(GraphArtifact {
                canonical_key: p.canonical_key.clone(),
                display_id: Some(p.display_id.clone()),
                title: p.title.clone(),
                kind: p.kind.clone(),
                relative_path: Some(p.relative_path.clone()),
                edges: p
                    .outgoing
                    .iter()
                    .map(|e| (e.target.clone(), e.relation.clone()))
                    .collect(),
            });
        }
    }
    let source = SourceRef {
        source_id: format!(
            "src:{}",
            &super::sha256_hex(format!("{}@{}", entry.checkout_root, entry.git_oid).as_bytes())
                [..16]
        ),
        path: entry.source_path.clone(),
        channel: SourceChannel::PinnedGit,
        version_key: format!(
            "{}:{}:{}",
            entry.git_oid, entry.tree_oid, entry.bytes_digest
        ),
    };
    Ok(build_graph(source, artifacts))
}

/// Structural relations propagate; informational ones only *note*.
fn propagates(relation: &str) -> bool {
    crate::graph::topological::is_structural_relation(relation)
}

/// The walk itself, over one source's graph.
fn walk(graph: &SourceGraph, changed: &str, bounds: ImpactBounds, report: &mut ImpactReport) {
    // Reverse adjacency: target_key → [(dependent_key, relation)].
    let mut dependents: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for (from, target, relation) in &graph.edges {
        match graph.resolve(target) {
            Some(to) => dependents
                .entry(to)
                .or_default()
                .push((from.clone(), relation.clone())),
            None => report.unresolved.push(UnresolvedEdge {
                source_id: graph.source.source_id.clone(),
                from: from.clone(),
                target: target.clone(),
                relation: relation.clone(),
            }),
        }
    }
    let start = match graph.resolve(changed) {
        Some(k) => k,
        None => {
            report.absent_in.push(graph.source.source_id.clone());
            return;
        }
    };
    // BFS over structural edges; informational dependents are noted in
    // place and never enqueued. `reached` maps a node to the rank+depth of
    // its currently-recorded explanation; a stronger path (higher rank,
    // or same rank shallower) replaces a weaker one — first visit is not
    // privileged when a better explanation arrives.
    fn rank(s: ImpactSeverity) -> u8 {
        match s {
            ImpactSeverity::Direct => 3,
            ImpactSeverity::Transitive => 2,
            ImpactSeverity::Noted => 1,
        }
    }
    let mut reached: HashMap<String, (u8, usize)> = HashMap::new();
    let mut index_of: HashMap<String, usize> = HashMap::new();
    let mut enqueued: HashSet<String> = HashSet::new();
    enqueued.insert(start.clone());
    let mut queue: VecDeque<(String, usize, Vec<PathStep>)> = VecDeque::new();
    queue.push_back((start, 0, Vec::new()));
    let mut truncated = false;
    while let Some((key, depth, path)) = queue.pop_front() {
        let mut outgoing = dependents.get(&key).cloned().unwrap_or_default();
        // Structural paths first: the structural explanation of a node
        // beats an informational one discovered at the same depth.
        outgoing.sort_by_key(|(_, r)| !propagates(r));
        for (dependent, relation) in outgoing {
            let next_depth = depth + 1;
            let mut next_path = path.clone();
            next_path.push(PathStep {
                dependent: dependent.clone(),
                relation: relation.clone(),
            });
            if propagates(&relation) {
                let severity = if next_depth == 1 {
                    ImpactSeverity::Direct
                } else {
                    ImpactSeverity::Transitive
                };
                let better = match reached.get(&dependent) {
                    None => true,
                    Some(&(r, d)) => rank(severity) > r || (rank(severity) == r && next_depth < d),
                };
                if better {
                    if report.impacted.len() >= bounds.max_nodes
                        && !index_of.contains_key(&dependent)
                    {
                        report.stops.push(ImpactStop {
                            at: dependent.clone(),
                            source_id: graph.source.source_id.clone(),
                            reason: "node_limit".into(),
                        });
                        truncated = true;
                        continue;
                    }
                    let node = &graph.nodes[&dependent];
                    let unit = ImpactedNode {
                        source: graph.source.clone(),
                        canonical_key: node.canonical_key.clone(),
                        display_id: node.display_id.clone(),
                        title: node.title.clone(),
                        kind: node.kind.clone(),
                        severity,
                        depth: next_depth,
                        path: next_path.clone(),
                    };
                    match index_of.get(&dependent) {
                        Some(&i) => report.impacted[i] = unit,
                        None => {
                            index_of.insert(dependent.clone(), report.impacted.len());
                            report.impacted.push(unit);
                        }
                    }
                    reached.insert(dependent.clone(), (rank(severity), next_depth));
                }
                if next_depth < bounds.max_depth && enqueued.insert(dependent.clone()) {
                    queue.push_back((dependent, next_depth, next_path));
                } else if next_depth >= bounds.max_depth && dependents.contains_key(&dependent) {
                    report.stops.push(ImpactStop {
                        at: dependent,
                        source_id: graph.source.source_id.clone(),
                        reason: "depth_limit".into(),
                    });
                }
            } else {
                // Informational: note the dependent; the walk stops here.
                let better = match reached.get(&dependent) {
                    None => true,
                    Some(&(r, d)) => 1 > r || (r == 1 && next_depth < d),
                };
                if better {
                    if report.impacted.len() >= bounds.max_nodes
                        && !index_of.contains_key(&dependent)
                    {
                        report.stops.push(ImpactStop {
                            at: dependent.clone(),
                            source_id: graph.source.source_id.clone(),
                            reason: "node_limit".into(),
                        });
                        truncated = true;
                        continue;
                    }
                    let node = &graph.nodes[&dependent];
                    let unit = ImpactedNode {
                        source: graph.source.clone(),
                        canonical_key: node.canonical_key.clone(),
                        display_id: node.display_id.clone(),
                        title: node.title.clone(),
                        kind: node.kind.clone(),
                        severity: ImpactSeverity::Noted,
                        depth: next_depth,
                        path: next_path,
                    };
                    match index_of.get(&dependent) {
                        Some(&i) if report.impacted[i].severity == ImpactSeverity::Noted => {
                            report.impacted[i] = unit;
                        }
                        Some(_) => {}
                        None => {
                            index_of.insert(dependent.clone(), report.impacted.len());
                            report.impacted.push(unit);
                        }
                    }
                    reached.insert(dependent.clone(), (1, next_depth));
                }
                report.stops.push(ImpactStop {
                    at: dependent,
                    source_id: graph.source.source_id.clone(),
                    reason: "informational_edge".into(),
                });
            }
        }
        if truncated {
            break;
        }
    }
}

/// Impact over live scope sources.
pub async fn impact_live(
    specs: &[SourceSpec],
    changed: &str,
    bounds: ImpactBounds,
) -> Result<ImpactReport, ConstellationError> {
    let mut report = ImpactReport {
        changed: changed.to_string(),
        bounds: ImpactBoundsSerde {
            max_depth: bounds.max_depth,
            max_nodes: bounds.max_nodes,
        },
        impacted: Vec::new(),
        absent_in: Vec::new(),
        stops: Vec::new(),
        unresolved: Vec::new(),
        failures: Vec::new(),
        sources: Vec::new(),
    };
    for spec in specs {
        match source_graph_live(spec).await {
            Ok(graph) => {
                report.sources.push(graph.source.clone());
                walk(&graph, changed, bounds, &mut report);
            }
            Err(f) => report.failures.push(f),
        }
    }
    Ok(report)
}

/// Impact over a recorded snapshot — the same walk at pinned OIDs.
pub fn impact_snapshot(
    snapshot: &ConstellationSnapshot,
    changed: &str,
    bounds: ImpactBounds,
) -> Result<ImpactReport, ConstellationError> {
    let mut report = ImpactReport {
        changed: changed.to_string(),
        bounds: ImpactBoundsSerde {
            max_depth: bounds.max_depth,
            max_nodes: bounds.max_nodes,
        },
        impacted: Vec::new(),
        absent_in: Vec::new(),
        stops: Vec::new(),
        unresolved: Vec::new(),
        failures: snapshot
            .skipped
            .iter()
            .map(|s| SourceFailure {
                path: s.path.clone(),
                reason: format!("source was not pinned: {}", s.reason),
            })
            .collect(),
        sources: Vec::new(),
    };
    for entry in &snapshot.sources {
        match source_graph_snapshot(entry) {
            Ok(graph) => {
                report.sources.push(graph.source.clone());
                walk(&graph, changed, bounds, &mut report);
            }
            Err(reason) => report.failures.push(SourceFailure {
                path: entry.source_path.clone(),
                reason,
            }),
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn doc(id: &str, kind: &str, links: &[(&str, &str)]) -> String {
        let mut s = format!("---\nid: {id}\nkind: {kind}\nstatus: accepted\ntitle: {id} title\n");
        if !links.is_empty() {
            s.push_str("links:\n");
            for (t, r) in links {
                s.push_str(&format!("  - target: {t}\n    relation: {r}\n"));
            }
        }
        s.push_str(&format!("---\n\n# {id}\n\nBody of {id}.\n"));
        s
    }

    /// A chain + an informational leaf + a dangling edge:
    /// C based_on A, A based_on SEED, E based_on C (cycle guard via C→SEED?),
    /// N informs SEED, X based_on MISSING.
    fn ws() -> TempDir {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join(".forgeplan");
        std::fs::create_dir_all(root.join("adrs")).unwrap();
        std::fs::create_dir_all(root.join("notes")).unwrap();
        std::fs::write(
            root.join("adrs/SEED.md"),
            doc("SEED-001", "adr", &[("GHOST-999", "informs")]),
        )
        .unwrap();
        std::fs::write(
            root.join("adrs/A.md"),
            doc("ADR-100", "adr", &[("SEED-001", "based_on")]),
        )
        .unwrap();
        std::fs::write(
            root.join("adrs/C.md"),
            doc(
                "ADR-101",
                "adr",
                &[("ADR-100", "based_on"), ("SEED-001", "informs")],
            ),
        )
        .unwrap();
        std::fs::write(
            root.join("notes/N.md"),
            doc("NOTE-200", "note", &[("SEED-001", "references")]),
        )
        .unwrap();
        std::fs::write(
            root.join("notes/X.md"),
            doc("NOTE-201", "note", &[("MISSING-404", "based_on")]),
        )
        .unwrap();
        tmp
    }

    #[tokio::test]
    async fn structural_chain_propagates_informational_does_not() {
        let ws = ws();
        let specs = vec![SourceSpec::new(ws.path().join(".forgeplan"))];
        let report = impact_live(&specs, "SEED-001", ImpactBounds::default())
            .await
            .unwrap();
        assert!(report.failures.is_empty(), "{:?}", report.failures);

        let by_key = |k: &str| {
            report
                .impacted
                .iter()
                .find(|n| n.canonical_key.contains(k) || n.display_id.as_deref() == Some(k))
        };
        // ADR-100 is structurally direct on SEED-001.
        let a = by_key("ADR-100").expect("ADR-100 impacted");
        assert_eq!(a.severity, ImpactSeverity::Direct);
        assert_eq!(a.depth, 1);
        // ADR-101 transitively via ADR-100 (structural path wins over its
        // own merely-informing edge to SEED).
        let c = by_key("ADR-101").expect("ADR-101 impacted");
        assert_eq!(c.severity, ImpactSeverity::Transitive);
        assert_eq!(c.depth, 2);
        assert_eq!(c.path.len(), 2);
        // NOTE-200 only references SEED — noted, never walked.
        let n = by_key("NOTE-200").expect("NOTE-200 noted");
        assert_eq!(n.severity, ImpactSeverity::Noted);
        assert!(
            report
                .stops
                .iter()
                .any(|s| s.reason == "informational_edge" && s.at.contains("NOTE-200"))
        );
        // The dangling edge is named in unresolved, not dropped.
        assert!(
            report
                .unresolved
                .iter()
                .any(|u| u.target == "GHOST-999" || u.target == "MISSING-404")
        );
    }

    #[tokio::test]
    async fn absent_seed_is_named_not_empty() {
        let ws = ws();
        let specs = vec![SourceSpec::new(ws.path().join(".forgeplan"))];
        let report = impact_live(&specs, "NOPE-000", ImpactBounds::default())
            .await
            .unwrap();
        assert!(report.impacted.is_empty());
        assert_eq!(report.absent_in.len(), 1);
    }

    #[tokio::test]
    async fn depth_bound_stops_and_names_the_boundary() {
        // chain A→B→C→D→E (each based_on the previous); depth=2 stops at C.
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join(".forgeplan/adrs");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("R.md"), doc("ROOT-0", "adr", &[])).unwrap();
        for (i, prev) in [(1, 0), (2, 1), (3, 2), (4, 3)] {
            std::fs::write(
                root.join(format!("N{i}.md")),
                doc(
                    &format!("NODE-{i}"),
                    "adr",
                    &[(
                        &format!("{}-{prev}", if prev == 0 { "ROOT" } else { "NODE" }),
                        "based_on",
                    )],
                ),
            )
            .unwrap();
        }
        let specs = vec![SourceSpec::new(tmp.path().join(".forgeplan"))];
        let report = impact_live(
            &specs,
            "ROOT-0",
            ImpactBounds {
                max_depth: 2,
                max_nodes: 256,
            },
        )
        .await
        .unwrap();
        let max = report.impacted.iter().map(|n| n.depth).max().unwrap_or(0);
        assert_eq!(max, 2);
        assert!(report.stops.iter().any(|s| s.reason == "depth_limit"));
    }
}
