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

use crate::artifact::canonical_key::{
    ArtifactCandidate, ResolveError, normalize_display_id, resolve_borrowed,
};

use super::ConstellationError;
use super::capture::{
    CaptureLimits, CaptureRequest, capture_canonical_census, parse_artifact_file,
};
use super::federation::{
    QueryMode, SourceChannel, SourceFailure, SourceRef, SourceSpec, failure_is_fatal,
    source_id_for, source_unavailable,
};
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
    /// Source containing the dependent artifact.
    pub source_id: String,
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
    pub(crate) candidate: ArtifactCandidate,
    pub(crate) has_display: bool,
    pub(crate) has_path: bool,
    pub(crate) kind: String,
}

/// The per-source graph: nodes keyed canonically, edges as written.
pub(crate) struct SourceGraph {
    pub(crate) source: SourceRef,
    /// Identity used by live-qualified links when this graph is pinned.
    live_id: Option<String>,
    /// canonical_key → node
    pub(crate) nodes: HashMap<String, Node>,
    /// Exact literal or normalized numeric alias → every canonical claimant.
    pub(crate) by_display: HashMap<String, Vec<String>>,
    /// (from canonical_key, target-as-written, relation)
    pub(crate) edges: Vec<(String, String, String)>,
}

impl SourceGraph {
    /// Resolve the complete indexed closure of canonical and alias claims.
    fn resolve(
        &self,
        target: &str,
        structural: bool,
    ) -> Result<Option<&ArtifactCandidate>, ResolveError> {
        let selector = if structural { target } else { target.trim() };
        let alias = normalize_display_id(selector)
            .map(std::borrow::Cow::Owned)
            .unwrap_or(std::borrow::Cow::Borrowed(selector));
        let exact = self.nodes.get(selector);
        let aliases = self
            .by_display
            .get(alias.as_ref())
            .into_iter()
            .flatten()
            .filter(move |key| key.as_str() != selector)
            .map(|key| &self.nodes[key].candidate);
        resolve_borrowed(
            target,
            exact.into_iter().map(|node| &node.candidate).chain(aliases),
            structural,
        )
    }
}

fn build_graph(source: SourceRef, artifacts: Vec<GraphArtifact>) -> SourceGraph {
    let mut nodes = HashMap::new();
    let mut by_display = HashMap::new();
    let mut edges = Vec::new();
    for a in artifacts {
        if let Some(d) = &a.display_id {
            let alias = normalize_display_id(d).unwrap_or_else(|| d.clone());
            by_display
                .entry(alias)
                .or_insert_with(Vec::new)
                .push(a.canonical_key.clone());
        }
        for (target, relation) in &a.edges {
            edges.push((a.canonical_key.clone(), target.clone(), relation.clone()));
        }
        let has_display = a.display_id.is_some();
        let has_path = a.relative_path.is_some();
        nodes.insert(
            a.canonical_key.clone(),
            Node {
                candidate: ArtifactCandidate {
                    canonical_key: a.canonical_key,
                    display_id: a.display_id.unwrap_or_default(),
                    title: a.title,
                    relative_path: a.relative_path.unwrap_or_default().into(),
                },
                has_display,
                has_path,
                kind: a.kind,
            },
        );
    }
    let live_id = (source.channel == SourceChannel::PinnedGit)
        .then(|| source_id_for(std::path::Path::new(&source.path)));
    SourceGraph {
        source,
        live_id,
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
    let canonical = super::federation::resolve_source(spec)?;
    let source = SourceRef {
        source_id: source_id_for(&canonical),
        path: canonical.display().to_string(),
        channel: SourceChannel::Markdown,
        version_key: String::new(),
        source_set_digest: None,
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
        s.source_set_digest = Some(graph.source_set_digest);
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
    let captured =
        capture_canonical_census(&CaptureRequest::new(&canonical), CaptureLimits::default())
            .await
            .map_err(|e| SourceFailure {
                path: spec.path.display().to_string(),
                reason: format!("{e}"),
            })?;
    if !captured.is_complete() {
        return Err(SourceFailure {
            path: spec.path.display().to_string(),
            reason: format!(
                "canonical source capture is incomplete: {:?}",
                captured.diagnostics
            ),
        });
    }
    let mut s = source;
    s.version_key = captured.source_set_digest.clone();
    s.source_set_digest = Some(captured.source_set_digest.clone());
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
    let mut canonical_keys = HashSet::new();
    for rel in &names {
        let bytes = super::snapshot::read_file_at(&repo, &entry.git_oid, rel)
            .map_err(|e| format!("{e}"))?;
        let p = parse_artifact_file(
            rel,
            Arc::from(&bytes[..]),
            super::jobs::DEFAULT_BODY_PREFIX_CHARS,
        )
        .map_err(|error| format!("artifact {rel} could not be parsed: {error}"))?;
        if !canonical_keys.insert(p.canonical_key.clone()) {
            return Err(format!(
                "snapshot contains conflicting canonical key `{}`",
                p.canonical_key
            ));
        }
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
        source_set_digest: None,
    };
    Ok(build_graph(source, artifacts))
}

/// Structural relations propagate; informational ones only *note*.
fn propagates(relation: &str) -> bool {
    crate::graph::topological::is_structural_relation(relation)
}

/// Resolve an edge using only the captured, admitted graphs.
pub(super) fn resolve_edge<'a>(
    graphs: &'a [SourceGraph],
    from: usize,
    target: &str,
) -> Option<(usize, &'a str)> {
    let (bare, qualifier) = super::federation::parse_qualified_ref(target);
    let Some(qualifier) = qualifier else {
        return graphs[from]
            .resolve(target, true)
            .ok()
            .flatten()
            .map(|candidate| (from, candidate.canonical_key.as_str()));
    };
    let mut found = None;
    for (i, graph) in graphs.iter().enumerate() {
        let matches_source = graph.source.path.contains(qualifier)
            || graph.source.source_id.starts_with(qualifier)
            || graph
                .live_id
                .as_deref()
                .is_some_and(|id| id.starts_with(qualifier));
        if !matches_source {
            continue;
        }
        match graph.resolve(bare, true) {
            Ok(Some(candidate)) if found.is_none() => {
                found = Some((i, candidate.canonical_key.as_str()))
            }
            Ok(Some(_)) | Err(_) => return None,
            Ok(None) => {}
        }
    }
    found
}

/// Walk the combined graph; identities include the declaring source.
fn walk(
    graphs: &[SourceGraph],
    seeds: Vec<(usize, String)>,
    bounds: ImpactBounds,
    report: &mut ImpactReport,
) {
    type Key<'a> = (usize, &'a str);
    let mut dependents: HashMap<Key<'_>, Vec<(Key<'_>, &str)>> = HashMap::new();
    for (i, graph) in graphs.iter().enumerate() {
        for (from, target, relation) in &graph.edges {
            if let Some(to) = resolve_edge(graphs, i, target) {
                dependents
                    .entry(to)
                    .or_default()
                    .push(((i, from.as_str()), relation));
            } else {
                report.unresolved.push(UnresolvedEdge {
                    source_id: graph.source.source_id.clone(),
                    from: from.clone(),
                    target: target.clone(),
                    relation: relation.clone(),
                });
            }
        }
    }
    for outgoing in dependents.values_mut() {
        outgoing.sort_by_key(|(_, relation)| !propagates(relation));
    }
    let mut reached: HashMap<Key<'_>, (u8, usize)> = HashMap::new();
    let mut index_of: HashMap<Key<'_>, usize> = HashMap::new();
    let mut enqueued: HashSet<Key<'_>> = seeds
        .iter()
        .map(|(source, key)| (*source, key.as_str()))
        .collect();
    let mut queue: VecDeque<(Key<'_>, usize, Vec<PathStep>, Key<'_>)> = seeds
        .iter()
        .map(|(source, key)| {
            let key = (*source, key.as_str());
            (key, 0, Vec::new(), key)
        })
        .collect();
    while let Some((key, depth, path, root)) = queue.pop_front() {
        let Some(outgoing) = dependents.get(&key) else {
            continue;
        };
        for (dependent, relation) in outgoing {
            let graph = &graphs[dependent.0];
            let next_depth = depth + 1;
            let structural = propagates(relation);
            let severity = if !structural {
                ImpactSeverity::Noted
            } else if next_depth == 1 {
                ImpactSeverity::Direct
            } else {
                ImpactSeverity::Transitive
            };
            let rank = match severity {
                ImpactSeverity::Direct => 3,
                ImpactSeverity::Transitive => 2,
                ImpactSeverity::Noted => 1,
            };
            let better = reached
                .get(dependent)
                .is_none_or(|&(r, d)| rank > r || (rank == r && next_depth < d));
            if better
                && report.impacted.len() >= bounds.max_nodes
                && !index_of.contains_key(dependent)
            {
                report.stops.push(ImpactStop {
                    at: dependent.1.to_owned(),
                    source_id: graph.source.source_id.clone(),
                    reason: "node_limit".into(),
                });
                return;
            }
            let mut next_path = path.clone();
            next_path.push(PathStep {
                dependent: dependent.1.to_owned(),
                source_id: graph.source.source_id.clone(),
                relation: (*relation).to_string(),
            });
            if better {
                let node = &graph.nodes[dependent.1];
                let unit = ImpactedNode {
                    source: graph.source.clone(),
                    canonical_key: node.candidate.canonical_key.clone(),
                    display_id: node.has_display.then(|| node.candidate.display_id.clone()),
                    title: node.candidate.title.clone(),
                    kind: node.kind.clone(),
                    severity,
                    depth: next_depth,
                    path: next_path.clone(),
                };
                if let Some(&index) = index_of.get(dependent) {
                    report.impacted[index] = unit;
                } else {
                    index_of.insert(*dependent, report.impacted.len());
                    report.impacted.push(unit);
                }
                reached.insert(*dependent, (rank, next_depth));
            }
            if !structural {
                report.stops.push(ImpactStop {
                    at: dependent.1.to_owned(),
                    source_id: graph.source.source_id.clone(),
                    reason: "informational_edge".into(),
                });
            } else if next_depth >= bounds.max_depth && dependents.contains_key(dependent) {
                report.stops.push(ImpactStop {
                    at: dependent.1.to_owned(),
                    source_id: graph.source.source_id.clone(),
                    reason: "depth_limit".into(),
                });
            } else if next_depth < bounds.max_depth {
                if enqueued.insert(*dependent) {
                    queue.push_back((*dependent, next_depth, next_path, root));
                } else if path.iter().any(|step| {
                    step.source_id == graph.source.source_id && step.dependent == dependent.1
                }) || dependent == &root
                {
                    report.stops.push(ImpactStop {
                        at: dependent.1.to_owned(),
                        source_id: graph.source.source_id.clone(),
                        reason: "cycle".into(),
                    });
                }
            }
        }
    }
}
fn resolve_seeds(
    graphs: &[SourceGraph],
    selected: &[bool],
    bare: &str,
    report: &mut ImpactReport,
) -> Result<Vec<(usize, String)>, ConstellationError> {
    let mut seeds = Vec::new();
    let mut error: Option<ResolveError> = None;
    let mut valid = false;
    for (i, graph) in graphs.iter().enumerate() {
        if !selected[i] {
            continue;
        }
        match graph.resolve(bare, false) {
            Ok(Some(candidate)) => {
                valid = true;
                seeds.push((i, candidate.canonical_key.clone()));
            }
            Ok(None) => {
                valid = true;
                report.absent_in.push(graph.source.source_id.clone());
            }
            Err(ResolveError::InvalidSelector { .. }) => {}
            Err(next) => {
                valid = true;
                let next =
                    super::federation::qualify_resolve_candidates(next, &graph.source.source_id);
                match (&mut error, next) {
                    (
                        Some(ResolveError::Ambiguous { candidates, .. }),
                        ResolveError::Ambiguous {
                            candidates: extra, ..
                        },
                    ) => candidates.extend(extra),
                    (slot @ None, next) => *slot = Some(next),
                    _ => {}
                }
            }
        }
    }
    if !valid && let Err(invalid) = resolve_borrowed(bare, std::iter::empty(), false) {
        error = Some(invalid);
    }
    if let Some(mut error) = error {
        if let ResolveError::Ambiguous { candidates, .. } = &mut error {
            for (i, key) in &seeds {
                let mut candidate = graphs[*i].nodes[key].candidate.clone();
                candidate.relative_path =
                    PathBuf::from(&graphs[*i].source.source_id).join(candidate.relative_path);
                candidates.push(candidate);
            }
            candidates.sort_by(|a, b| {
                a.canonical_key
                    .cmp(&b.canonical_key)
                    .then_with(|| a.relative_path.cmp(&b.relative_path))
            });
        }
        return Err(ConstellationError::ResolveRead {
            error,
            sources: std::mem::take(&mut report.sources),
            failures: std::mem::take(&mut report.failures),
        });
    }
    Ok(seeds)
}

/// Impact over live scope sources.
///
/// `mode` is the same required/optional contract [`super::federation`]
/// applies: under [`QueryMode::Strict`] a failed required source aborts
/// the walk with `source_unavailable` — a report built without it would
/// read as complete; an optional failure lands in `failures`.
pub async fn impact_live(
    specs: &[SourceSpec],
    changed: &str,
    bounds: ImpactBounds,
    mode: QueryMode,
) -> Result<ImpactReport, ConstellationError> {
    let (bare, qualifier) = super::federation::parse_qualified_ref(changed);
    if let Some(q) = qualifier
        && super::federation::specs_matching(specs, q).is_empty()
    {
        return Err(ConstellationError::SourceUnavailable {
            detail: format!("qualifier @{q} matches no source in this scope"),
        });
    }
    let mut graphs = Vec::new();
    let mut selected = Vec::new();
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
                selected.push(qualifier.is_none_or(|q| {
                    !super::federation::specs_matching(std::slice::from_ref(spec), q).is_empty()
                }));
                report.sources.push(graph.source.clone());
                graphs.push(graph);
            }
            Err(f) => {
                if failure_is_fatal(spec, mode) {
                    return Err(source_unavailable(&f));
                }
                report.failures.push(f);
            }
        }
    }
    let seeds = resolve_seeds(&graphs, &selected, bare, &mut report)?;
    walk(&graphs, seeds, bounds, &mut report);
    Ok(report)
}

/// Impact over a recorded snapshot — the same walk at pinned OIDs.
pub fn impact_snapshot(
    snapshot: &ConstellationSnapshot,
    changed: &str,
    bounds: ImpactBounds,
) -> Result<ImpactReport, ConstellationError> {
    let (bare, qualifier) = super::federation::parse_qualified_ref(changed);
    if let Some(q) = qualifier
        && !snapshot
            .sources
            .iter()
            .any(|entry| super::snapshot::entry_matches_qualifier(entry, q))
    {
        return Err(ConstellationError::SourceUnavailable {
            detail: format!("qualifier @{q} matches no source in this snapshot"),
        });
    }
    let mut graphs = Vec::new();
    let mut selected = Vec::new();
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
                selected.push(
                    qualifier.is_none_or(|q| super::snapshot::entry_matches_qualifier(entry, q)),
                );
                report.sources.push(graph.source.clone());
                graphs.push(graph);
            }
            Err(reason) => report.failures.push(SourceFailure {
                path: entry.source_path.clone(),
                reason,
            }),
        }
    }
    let seeds = resolve_seeds(&graphs, &selected, bare, &mut report)?;
    walk(&graphs, seeds, bounds, &mut report);
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    fn git_in(dir: &std::path::Path, args: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

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
            doc("ADR-001", "adr", &[("GHOST-999", "informs")]),
        )
        .unwrap();
        std::fs::write(
            root.join("adrs/A.md"),
            doc("ADR-100", "adr", &[("ADR-001", "based_on")]),
        )
        .unwrap();
        std::fs::write(
            root.join("adrs/C.md"),
            doc(
                "ADR-101",
                "adr",
                &[("ADR-100", "based_on"), ("ADR-001", "informs")],
            ),
        )
        .unwrap();
        std::fs::write(
            root.join("notes/N.md"),
            doc("NOTE-200", "note", &[("ADR-001", "references")]),
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
        let report = impact_live(
            &specs,
            "ADR-001",
            ImpactBounds::default(),
            QueryMode::Strict,
        )
        .await
        .unwrap();
        assert!(report.failures.is_empty(), "{:?}", report.failures);

        let by_key = |k: &str| {
            report
                .impacted
                .iter()
                .find(|n| n.canonical_key.contains(k) || n.display_id.as_deref() == Some(k))
        };
        // ADR-100 is structurally direct on ADR-001.
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
        let report = impact_live(
            &specs,
            "ADR-999",
            ImpactBounds::default(),
            QueryMode::Strict,
        )
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
        std::fs::write(root.join("R.md"), doc("ADR-0", "adr", &[])).unwrap();
        for (i, prev) in [(1, 0), (2, 1), (3, 2), (4, 3)] {
            std::fs::write(
                root.join(format!("N{i}.md")),
                doc(
                    &format!("ADR-{i}"),
                    "adr",
                    &[(&format!("ADR-{prev}"), "based_on")],
                ),
            )
            .unwrap();
        }
        let specs = vec![SourceSpec::new(tmp.path().join(".forgeplan"))];
        let report = impact_live(
            &specs,
            "ADR-0",
            ImpactBounds {
                max_depth: 2,
                max_nodes: 256,
            },
            QueryMode::Strict,
        )
        .await
        .unwrap();
        let max = report.impacted.iter().map(|n| n.depth).max().unwrap_or(0);
        assert_eq!(max, 2);
        assert!(report.stops.iter().any(|s| s.reason == "depth_limit"));
    }

    #[tokio::test]
    async fn a_dead_required_source_fails_impact_before_any_report() {
        // Same contract as federated_search: a required source's outage
        // aborts the strict read — a report without it would name too few
        // dependents and read as complete.
        let ws = ws();
        let specs = vec![
            SourceSpec::new(ws.path().join(".forgeplan")),
            SourceSpec::new("/definitely/not/a/workspace"),
        ];
        let err = impact_live(
            &specs,
            "ADR-001",
            ImpactBounds::default(),
            QueryMode::Strict,
        )
        .await
        .unwrap_err();
        assert!(matches!(err, ConstellationError::SourceUnavailable { .. }));

        // Optional — or `AllowPartial` — keeps the honest-envelope path.
        let specs = vec![
            SourceSpec::new(ws.path().join(".forgeplan")),
            SourceSpec::new_optional("/definitely/not/a/workspace"),
        ];
        let report = impact_live(
            &specs,
            "ADR-001",
            ImpactBounds::default(),
            QueryMode::Strict,
        )
        .await
        .unwrap();
        assert_eq!(report.failures.len(), 1);
        assert!(!report.impacted.is_empty());

        let specs = vec![
            SourceSpec::new(ws.path().join(".forgeplan")),
            SourceSpec::new("/definitely/not/a/workspace"),
        ];
        let report = impact_live(
            &specs,
            "ADR-001",
            ImpactBounds::default(),
            QueryMode::AllowPartial,
        )
        .await
        .unwrap();
        assert_eq!(report.failures.len(), 1);
        assert!(!report.impacted.is_empty());
    }

    #[tokio::test]
    async fn cross_source_live_and_git_snapshot_impact_and_graph() {
        let tmp = TempDir::new().unwrap();
        let hub = tmp.path().join("hub");
        let shared = tmp.path().join("shared");
        for repo in [&hub, &shared] {
            std::fs::create_dir_all(repo.join(".forgeplan/adrs")).unwrap();
            std::fs::create_dir_all(repo.join(".forgeplan/prds")).unwrap();
            std::fs::write(
                repo.join(".forgeplan/adrs/ADR-001.md"),
                doc("ADR-001", "adr", &[]),
            )
            .unwrap();
        }
        std::fs::write(
            hub.join(".forgeplan/prds/PRD-002.md"),
            doc("PRD-002", "prd", &[("ADR-001@shared", "based_on")]),
        )
        .unwrap();
        std::fs::write(
            hub.join(".forgeplan/prds/PRD-003.md"),
            doc("PRD-003", "prd", &[("PRD-002", "based_on")]),
        )
        .unwrap();
        for repo in [&hub, &shared] {
            git_in(repo, &["init", "-q"]);
            git_in(repo, &["config", "user.email", "test@example.invalid"]);
            git_in(repo, &["config", "user.name", "test"]);
            git_in(repo, &["add", "-A"]);
            git_in(repo, &["commit", "-qm", "initial"]);
        }
        let specs = vec![
            SourceSpec::new(hub.join(".forgeplan")),
            SourceSpec::new(shared.join(".forgeplan")),
        ];
        let check = |report: &ImpactReport, graph: &super::super::graph::GraphOutcome| {
            assert_eq!(report.sources.len(), 2);
            assert!(report.failures.is_empty());
            let hub_id = &report
                .sources
                .iter()
                .find(|s| s.path.contains("/hub/"))
                .unwrap()
                .source_id;
            let shared_id = &report
                .sources
                .iter()
                .find(|s| s.path.contains("/shared/"))
                .unwrap()
                .source_id;
            let direct = report
                .impacted
                .iter()
                .find(|n| n.display_id.as_deref() == Some("PRD-002"))
                .unwrap();
            assert_eq!(direct.severity, ImpactSeverity::Direct);
            assert_eq!(&direct.source.source_id, hub_id);
            assert_eq!(direct.path[0].source_id, *hub_id);
            assert_eq!(direct.path[0].relation, "based_on");
            let transitive = report
                .impacted
                .iter()
                .find(|n| n.display_id.as_deref() == Some("PRD-003"))
                .unwrap();
            assert_eq!(transitive.severity, ImpactSeverity::Transitive);
            assert_eq!(transitive.depth, 2);
            assert_eq!(transitive.path.len(), 2);
            assert!(transitive.path.iter().all(|step| step.source_id == *hub_id));
            assert!(
                !report
                    .impacted
                    .iter()
                    .any(|n| n.display_id.as_deref() == Some("ADR-001"))
            );
            let edge = graph.edges.iter().find(|e| e.from == "PRD-002").unwrap();
            assert_eq!(edge.resolved.as_deref(), Some("ADR-001"));
            assert_eq!(edge.resolved_source_id.as_deref(), Some(shared_id.as_str()));
            assert_eq!(edge.source_id, *hub_id);
        };
        let live = impact_live(
            &specs,
            "ADR-001@shared",
            ImpactBounds::default(),
            QueryMode::Strict,
        )
        .await
        .unwrap();
        let live_graph =
            super::super::graph::graph_live(&specs, Default::default(), QueryMode::Strict)
                .await
                .unwrap();
        check(&live, &live_graph);
        let snapshot = super::super::snapshot::capture_snapshot(&specs, None).unwrap();
        std::fs::write(
            hub.join(".forgeplan/prds/PRD-002.md"),
            doc("PRD-002", "prd", &[]),
        )
        .unwrap();
        let pinned = impact_snapshot(&snapshot, "ADR-001@shared", ImpactBounds::default()).unwrap();
        let pinned_graph =
            super::super::graph::graph_snapshot(&snapshot, Default::default()).unwrap();
        check(&pinned, &pinned_graph);
        let shared_id = &pinned
            .sources
            .iter()
            .find(|s| s.path.contains("/shared/"))
            .unwrap()
            .source_id;
        let qualified = format!("ADR-001@{shared_id}");
        let pinned_by_id = impact_snapshot(&snapshot, &qualified, ImpactBounds::default()).unwrap();
        assert_eq!(pinned_by_id.impacted.len(), 2);
    }

    #[tokio::test]
    async fn a_qualified_key_narrows_the_walk_to_the_named_source() {
        let first = ws();
        let second = ws();
        let first_spec = SourceSpec::new(first.path().join(".forgeplan"));
        let second_spec = SourceSpec::new(second.path().join(".forgeplan"));
        let second_id =
            super::super::federation::spec_source_id(&second_spec).expect("second source resolves");
        let qualified = format!("ADR-100@{second_id}");

        let report = impact_live(
            &[first_spec.clone(), second_spec.clone()],
            &qualified,
            ImpactBounds::default(),
            QueryMode::Strict,
        )
        .await
        .expect("a qualified impact must answer");

        assert_eq!(
            report.changed, qualified,
            "the receipt echoes what was asked"
        );
        assert_eq!(
            report.sources.len(),
            2,
            "all admitted sources are witnessed"
        );
        assert!(
            report
                .sources
                .iter()
                .any(|source| source.source_id == second_id)
        );
        assert!(
            report
                .impacted
                .iter()
                .all(|node| node.source.source_id == second_id),
            "no node from the other source may appear: {:?}",
            report.impacted
        );

        let unknown = impact_live(
            &[first_spec, second_spec],
            "ADR-100@src:does-not-exist",
            ImpactBounds::default(),
            QueryMode::Strict,
        )
        .await;
        assert!(
            matches!(unknown, Err(ConstellationError::SourceUnavailable { .. })),
            "an unmatched qualifier is a named refusal"
        );
    }
    #[tokio::test]
    async fn incomplete_live_source_is_failed_as_a_whole() {
        let good = ws();
        let bad = TempDir::new().unwrap();
        let bad_root = bad.path().join(".forgeplan");
        std::fs::create_dir_all(bad_root.join("adrs")).unwrap();
        std::fs::write(bad_root.join("adrs/SEED.md"), doc("ADR-001", "adr", &[])).unwrap();
        std::fs::write(
            bad_root.join("adrs/DEPENDENT.md"),
            doc("ADR-777", "adr", &[("ADR-001", "based_on")]),
        )
        .unwrap();
        std::fs::write(bad_root.join("adrs/BROKEN.md"), "---\nid: [invalid\n").unwrap();
        let specs = vec![
            SourceSpec::new(good.path().join(".forgeplan")),
            SourceSpec::new(&bad_root),
        ];

        let strict = impact_live(
            &specs,
            "ADR-001",
            ImpactBounds::default(),
            QueryMode::Strict,
        )
        .await
        .expect_err("an incomplete required source must refuse a strict impact report");
        assert!(matches!(
            strict,
            ConstellationError::SourceUnavailable { .. }
        ));

        let partial = impact_live(
            &specs,
            "ADR-001",
            ImpactBounds::default(),
            QueryMode::AllowPartial,
        )
        .await
        .unwrap();
        assert_eq!(partial.failures.len(), 1);
        assert_eq!(partial.failures[0].path, bad_root.display().to_string());
        assert!(partial.failures[0].reason.contains("adrs/BROKEN.md"));
        assert_eq!(partial.sources.len(), 1);
        assert!(
            partial
                .impacted
                .iter()
                .any(|node| node.display_id.as_deref() == Some("ADR-100"))
        );
        assert!(
            partial
                .impacted
                .iter()
                .all(|node| node.display_id.as_deref() != Some("ADR-777")),
            "valid documents from the invalid source must not leak into impact results"
        );
    }

    #[tokio::test]
    async fn canonical_key_collision_is_a_source_failure() {
        let good = ws();
        let bad = TempDir::new().unwrap();
        let bad_root = bad.path().join(".forgeplan");
        std::fs::create_dir_all(bad_root.join("adrs")).unwrap();
        let duplicate = doc("ADR-777", "adr", &[("ADR-001", "based_on")]);
        std::fs::write(bad_root.join("adrs/A.md"), &duplicate).unwrap();
        std::fs::write(bad_root.join("adrs/B.md"), &duplicate).unwrap();
        let specs = vec![
            SourceSpec::new(good.path().join(".forgeplan")),
            SourceSpec::new(&bad_root),
        ];

        let strict = impact_live(
            &specs,
            "ADR-001",
            ImpactBounds::default(),
            QueryMode::Strict,
        )
        .await
        .expect_err("duplicate canonical keys must refuse a strict impact report");
        assert!(matches!(
            strict,
            ConstellationError::SourceUnavailable { .. }
        ));

        let partial = impact_live(
            &specs,
            "ADR-001",
            ImpactBounds::default(),
            QueryMode::AllowPartial,
        )
        .await
        .unwrap();
        assert_eq!(partial.failures.len(), 1);
        assert_eq!(partial.failures[0].path, bad_root.display().to_string());
        assert!(partial.failures[0].reason.contains("IdentityCollision"));
        assert_eq!(partial.sources.len(), 1);
        assert!(
            partial
                .impacted
                .iter()
                .all(|node| node.display_id.as_deref() != Some("ADR-777"))
        );
    }

    #[test]
    fn malformed_pinned_artifact_is_a_named_snapshot_failure() {
        let tmp = TempDir::new().unwrap();
        let fp = tmp.path().join(".forgeplan");
        std::fs::create_dir_all(fp.join("adrs")).unwrap();
        std::fs::write(fp.join("adrs/SEED.md"), doc("ADR-001", "adr", &[])).unwrap();
        std::fs::write(
            fp.join("adrs/DEPENDENT.md"),
            doc("ADR-777", "adr", &[("ADR-001", "based_on")]),
        )
        .unwrap();
        std::fs::write(fp.join("adrs/BROKEN.md"), "---\nid: [invalid\n").unwrap();
        std::fs::write(fp.join("README.md"), "---\nid: [unrelated\n").unwrap();
        git_in(tmp.path(), &["init", "-q"]);
        git_in(
            tmp.path(),
            &["config", "user.email", "test@example.invalid"],
        );
        git_in(tmp.path(), &["config", "user.name", "test"]);
        git_in(tmp.path(), &["add", "-A"]);
        git_in(tmp.path(), &["commit", "-qm", "snapshot"]);

        let snapshot =
            super::super::snapshot::capture_snapshot(&[SourceSpec::new(&fp)], None).unwrap();
        assert_eq!(snapshot.sources[0].files, 3);
        let report = impact_snapshot(&snapshot, "ADR-001", ImpactBounds::default()).unwrap();
        assert_eq!(report.failures.len(), 1);
        assert_eq!(
            std::fs::canonicalize(&report.failures[0].path).unwrap(),
            fp.canonicalize().unwrap()
        );
        assert!(report.failures[0].reason.contains("adrs/BROKEN.md"));
        assert!(report.sources.is_empty());
        assert!(report.impacted.is_empty());
    }
    #[tokio::test]
    async fn ambiguous_qualified_edge_remains_unresolved() {
        let first = ws();
        let second = ws();
        std::fs::write(
            first.path().join(".forgeplan/adrs/AMBIGUOUS.md"),
            doc("ADR-888", "adr", &[("ADR-001@.forgeplan", "based_on")]),
        )
        .unwrap();
        let specs = vec![
            SourceSpec::new(first.path().join(".forgeplan")),
            SourceSpec::new(second.path().join(".forgeplan")),
        ];
        let report = impact_live(
            &specs,
            "ADR-001",
            ImpactBounds::default(),
            QueryMode::Strict,
        )
        .await
        .unwrap();
        assert!(
            report
                .unresolved
                .iter()
                .any(|edge| edge.target == "ADR-001@.forgeplan")
        );
        assert!(
            !report
                .impacted
                .iter()
                .any(|node| node.display_id.as_deref() == Some("ADR-888"))
        );
        let graph = super::super::graph::graph_live(&specs, Default::default(), QueryMode::Strict)
            .await
            .unwrap();
        let edge = graph
            .edges
            .iter()
            .find(|edge| edge.target == "ADR-001@.forgeplan")
            .unwrap();
        assert!(edge.resolved.is_none());
        assert!(edge.resolved_source_id.is_none());
    }
}
