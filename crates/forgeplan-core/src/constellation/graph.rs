//! The federated relation graph — the raw node and edge set the other
//! read paths walk over, emitted as data.
//!
//! `impact` answers "who should re-check when K changes"; this view
//! answers "what does the constellation actually look like": every
//! artifact as a source-qualified node, every declared link as a typed
//! edge carrying the relation and its provenance — which source's
//! relations table or frontmatter block declared it, and which file
//! inside that source when the channel records one.
//!
//! Same source boundary as every federated read: sources come from the
//! active constellation or a snapshot lockfile, never from a caller path.
//! Same honesty envelope too — an unreadable source lands in `failures`
//! and flips `partial`, it is never folded into a smaller graph. The
//! answer is bounded: `limit` caps nodes and edges alike, and wherever
//! the cap cuts, `truncated` says the view is partial by bound, not by
//! corpus.
//!
//! Resolution is same-source only, matching `impact` and `export`: an
//! edge's `target` string resolves to a canonical key inside the source
//! that declared it, or stays a dangling target with `resolved: null` —
//! `ADR-001` in project A never silently merges with `ADR-001` in
//! project B.

use serde::Serialize;

use super::ConstellationError;
use super::federation::{
    QueryMode, SourceFailure, SourceRef, SourceSpec, failure_is_fatal, source_unavailable,
};
use super::impact::{self, SourceGraph};
use super::snapshot::ConstellationSnapshot;

/// The bound one graph read runs under — a single cap applied to the
/// node set and the edge set alike, the role `limit` plays on the
/// search surface.
#[derive(Debug, Clone, Copy)]
pub struct GraphBounds {
    /// Maximum nodes emitted, and separately maximum edges emitted.
    pub limit: usize,
}

impl Default for GraphBounds {
    fn default() -> Self {
        Self { limit: 500 }
    }
}

/// One artifact as a graph node — source-qualified, versioned.
///
/// `source` carries the full [`SourceRef`]: `source_id` is the node's
/// federation identity and `version_key` the revision the answer was
/// read at (a generation id when pinned, a capture digest on markdown,
/// the `oid:tree:digest` triple on a snapshot).
#[derive(Debug, Clone, Serialize)]
pub struct GraphNode {
    pub source: SourceRef,
    pub canonical_key: String,
    pub display_id: Option<String>,
    pub title: String,
    /// `None` when the source did not declare a `kind` — absent stays
    /// absent rather than arriving as an empty string.
    pub kind: Option<String>,
    /// The file inside the source declaring this node, when the channel
    /// knows it; `None` on the pinned channel.
    pub relative_path: Option<String>,
}

/// A declared link, exactly as the source wrote it, plus what it
/// resolves to inside the same source.
#[derive(Debug, Clone, Serialize)]
pub struct GraphEdge {
    /// Provenance: the `source_id` whose relations table or frontmatter
    /// links declared this edge.
    pub source_id: String,
    /// The declaring artifact's canonical key in that source.
    pub from: String,
    /// The relation kind as written (`based_on`, `informs`, …).
    pub relation: String,
    /// The target as written — a canonical key, a display id, or a
    /// string nothing in the source resolves.
    pub target: String,
    /// The canonical key `target` resolves to in the same source, or
    /// `None` when the edge dangles — named, never silently dropped.
    pub resolved: Option<String>,
    /// The file inside the source that declared the edge, when the
    /// channel records one (markdown and pinned-git carry it; the
    /// pinned relations table does not).
    pub declared_in: Option<String>,
}

/// The envelope every graph read returns — the raw sets plus the
/// honesty fields the other federated reads carry.
#[derive(Debug, Serialize)]
pub struct GraphOutcome {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    /// Sources that answered, with the version each answer was read at.
    pub sources: Vec<SourceRef>,
    /// Sources that did not — `partial` is true iff this is non-empty.
    pub failures: Vec<SourceFailure>,
    /// `true` when the node or the edge set was cut at `limit`.
    pub truncated: bool,
    /// `true` when at least one source did not answer.
    pub partial: bool,
    /// The bound the read ran under — echoed so a consumer can tell a
    /// capped view from a genuinely small graph.
    pub limit: usize,
}

/// Fold one source's graph into the envelope under the bound.
///
/// Output order is deterministic — nodes by canonical key, edges by
/// `(from, target-as-written, relation)` tuple order — because two
/// agents reading the same graph must be able to diff it. The cap is
/// checked per item; once it fires, `truncated` stays set.
fn absorb(graph: SourceGraph, bounds: GraphBounds, out: &mut GraphOutcome) {
    out.sources.push(graph.source.clone());
    let mut nodes: Vec<&impact::Node> = graph.nodes.values().collect();
    nodes.sort_by(|a, b| a.canonical_key.cmp(&b.canonical_key));
    for node in nodes {
        if out.nodes.len() >= bounds.limit {
            out.truncated = true;
            break;
        }
        out.nodes.push(GraphNode {
            source: graph.source.clone(),
            canonical_key: node.canonical_key.clone(),
            display_id: node.display_id.clone(),
            title: node.title.clone(),
            kind: if node.kind.is_empty() {
                None
            } else {
                Some(node.kind.clone())
            },
            relative_path: node.relative_path.clone(),
        });
    }
    let mut edges: Vec<&(String, String, String)> = graph.edges.iter().collect();
    edges.sort();
    for (from, target, relation) in edges {
        if out.edges.len() >= bounds.limit {
            out.truncated = true;
            break;
        }
        out.edges.push(GraphEdge {
            source_id: graph.source.source_id.clone(),
            from: from.clone(),
            relation: relation.clone(),
            target: target.clone(),
            resolved: graph.resolve(target),
            declared_in: graph.nodes.get(from).and_then(|n| n.relative_path.clone()),
        });
    }
}

/// The relation graph of every named live source.
///
/// Each source answers on the channel it honestly offers — pinned
/// relations table when a generation is published, parsed frontmatter
/// links otherwise — the same per-source rule `impact` and `federation`
/// apply. A source that cannot answer is a named failure, never an
/// empty piece of the graph.
///
/// `mode` applies the required/optional contract: a `required` source
/// that fails aborts the strict read with `source_unavailable` instead
/// of producing a graph that looks complete; `optional` — or
/// [`QueryMode::AllowPartial`] — reports the failure in `failures` and
/// flips `partial`.
pub async fn graph_live(
    specs: &[SourceSpec],
    bounds: GraphBounds,
    mode: QueryMode,
) -> Result<GraphOutcome, ConstellationError> {
    if bounds.limit == 0 {
        return Err(ConstellationError::Io {
            detail: "a federated graph read with limit 0 cannot return anything".to_string(),
        });
    }
    let mut out = GraphOutcome {
        nodes: Vec::new(),
        edges: Vec::new(),
        sources: Vec::new(),
        failures: Vec::new(),
        truncated: false,
        partial: false,
        limit: bounds.limit,
    };
    for spec in specs {
        match impact::source_graph_live(spec).await {
            Ok(graph) => absorb(graph, bounds, &mut out),
            Err(f) => {
                if failure_is_fatal(spec, mode) {
                    return Err(source_unavailable(&f));
                }
                out.failures.push(f);
            }
        }
    }
    out.partial = !out.failures.is_empty();
    Ok(out)
}

/// The same graph at a recorded snapshot — every source parsed out of
/// its pinned Git objects. Sources the lockfile recorded as skipped are
/// failures up front, the same convention `impact_snapshot` follows.
pub fn graph_snapshot(
    snapshot: &ConstellationSnapshot,
    bounds: GraphBounds,
) -> Result<GraphOutcome, ConstellationError> {
    if bounds.limit == 0 {
        return Err(ConstellationError::Io {
            detail: "a snapshot graph read with limit 0 cannot return anything".to_string(),
        });
    }
    let mut out = GraphOutcome {
        nodes: Vec::new(),
        edges: Vec::new(),
        sources: Vec::new(),
        failures: snapshot
            .skipped
            .iter()
            .map(|s| SourceFailure {
                path: s.path.clone(),
                reason: format!("source was not pinned: {}", s.reason),
            })
            .collect(),
        truncated: false,
        partial: false,
        limit: bounds.limit,
    };
    for entry in &snapshot.sources {
        match impact::source_graph_snapshot(entry) {
            Ok(graph) => absorb(graph, bounds, &mut out),
            Err(reason) => out.failures.push(SourceFailure {
                path: entry.source_path.clone(),
                reason,
            }),
        }
    }
    out.partial = !out.failures.is_empty();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constellation::capture::CaptureLimits;
    use crate::constellation::facade::{MaintainCapability, maintain_index, resume_admission};
    use crate::constellation::federation::SourceChannel;
    use crate::constellation::snapshot::capture_snapshot;
    use crate::workspace::ARTIFACT_DIRS;
    use std::process::Command;
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

    /// A `.forgeplan` directory under a tempdir — the same convention the
    /// federation tests use.
    fn workspace_with(files: &[(&str, &str, &str)]) -> TempDir {
        let tmp = TempDir::new().unwrap();
        for dir in ARTIFACT_DIRS {
            std::fs::create_dir_all(tmp.path().join(dir)).unwrap();
        }
        for (dir, name, body) in files {
            std::fs::write(tmp.path().join(dir).join(name), body).unwrap();
        }
        tmp
    }

    /// Publish a real generation so the source answers on the pinned
    /// channel — the fixture the spec requires next to a markdown one.
    async fn publish(tmp: &TempDir) {
        resume_admission();
        let capability =
            MaintainCapability::for_strict_maintenance(tmp.path().canonicalize().unwrap());
        maintain_index(
            &capability,
            CaptureLimits::default(),
            std::time::Instant::now() + std::time::Duration::from_secs(120),
        )
        .await
        .unwrap();
    }

    /// One published source + one markdown-only source must produce
    /// nodes carrying `source_id`/`version_key` on their own channels,
    /// and edges carrying relation kind + provenance.
    #[tokio::test]
    async fn graph_spans_pinned_and_markdown_channels() {
        let pinned_ws = workspace_with(&[
            ("adrs", "A.md", &doc("ADR-001", "adr", &[])),
            (
                "adrs",
                "B.md",
                &doc("ADR-002", "adr", &[("ADR-001", "based_on")]),
            ),
        ]);
        publish(&pinned_ws).await;
        let markdown_ws = workspace_with(&[
            ("notes", "N.md", &doc("NOTE-001", "note", &[])),
            (
                "notes",
                "M.md",
                &doc(
                    "NOTE-002",
                    "note",
                    &[("NOTE-001", "references"), ("GHOST-999", "informs")],
                ),
            ),
        ]);
        let specs = vec![
            SourceSpec::new(pinned_ws.path().to_path_buf()),
            SourceSpec::new(markdown_ws.path().to_path_buf()),
        ];
        let out = graph_live(&specs, GraphBounds::default(), QueryMode::Strict)
            .await
            .unwrap();
        assert!(out.failures.is_empty(), "{:?}", out.failures);
        assert!(!out.partial && !out.truncated);
        assert_eq!(out.sources.len(), 2);
        assert_eq!(out.nodes.len(), 4);

        // Every node is source-qualified and versioned, on the channel
        // its source honestly offers.
        for node in &out.nodes {
            assert!(node.source.source_id.starts_with("src:"));
            assert!(!node.source.version_key.is_empty());
        }
        assert!(
            out.nodes
                .iter()
                .any(|n| n.source.channel == SourceChannel::Pinned)
        );
        assert!(
            out.nodes
                .iter()
                .any(|n| n.source.channel == SourceChannel::Markdown)
        );

        // A markdown-channel edge: kind + provenance + same-source
        // resolution.
        let edge = out
            .edges
            .iter()
            .find(|e| e.relation == "references")
            .expect("NOTE-002 -> NOTE-001 edge");
        assert_eq!(edge.from, "NOTE-002");
        assert_eq!(edge.target, "NOTE-001");
        assert_eq!(edge.resolved.as_deref(), Some("NOTE-001"));
        assert_eq!(edge.declared_in.as_deref(), Some("notes/M.md"));
        let md_source_id = out
            .sources
            .iter()
            .find(|s| s.channel == SourceChannel::Markdown)
            .unwrap()
            .source_id
            .clone();
        assert_eq!(edge.source_id, md_source_id);

        // A dangling target is named, not dropped.
        let dangling = out
            .edges
            .iter()
            .find(|e| e.target == "GHOST-999")
            .expect("dangling edge kept");
        assert_eq!(dangling.relation, "informs");
        assert!(dangling.resolved.is_none());

        // The pinned channel emits edges from its relations table — the
        // from-side file is not recorded there.
        let pinned_edge = out
            .edges
            .iter()
            .find(|e| e.relation == "based_on")
            .expect("ADR-002 -> ADR-001 edge");
        assert_eq!(pinned_edge.resolved.as_deref(), Some("ADR-001"));
        assert!(pinned_edge.declared_in.is_none());
    }

    /// The cap is a named boundary: cutting the node set flips
    /// `truncated`, never shrinks the corpus silently.
    #[tokio::test]
    async fn truncation_fires_at_the_cap() {
        let ws = workspace_with(&[
            ("adrs", "A.md", &doc("ADR-001", "adr", &[])),
            ("adrs", "B.md", &doc("ADR-002", "adr", &[])),
            ("adrs", "C.md", &doc("ADR-003", "adr", &[])),
        ]);
        let specs = vec![SourceSpec::new(ws.path().to_path_buf())];
        let out = graph_live(&specs, GraphBounds { limit: 2 }, QueryMode::Strict)
            .await
            .unwrap();
        assert_eq!(out.nodes.len(), 2);
        assert!(out.truncated);
        assert_eq!(out.limit, 2);
    }

    /// A source that cannot be read is a diagnostic + `partial`, never
    /// an empty-looking graph.
    #[tokio::test]
    async fn unreadable_source_is_partial_not_empty() {
        let ws = workspace_with(&[("adrs", "A.md", &doc("ADR-001", "adr", &[]))]);
        let missing = ws.path().join("does-not-exist");
        // Optional — a required dead source is the strict-error arm,
        // covered by `dead_required_source_is_an_error_not_a_smaller_graph`.
        let specs = vec![
            SourceSpec::new(ws.path().to_path_buf()),
            SourceSpec::new_optional(missing.clone()),
        ];
        let out = graph_live(&specs, GraphBounds::default(), QueryMode::Strict)
            .await
            .unwrap();
        assert!(out.partial);
        assert_eq!(out.failures.len(), 1);
        assert_eq!(out.failures[0].path, missing.display().to_string());
        assert_eq!(out.nodes.len(), 1, "the readable source still answers");
    }

    /// The role contract on the graph surface: a *required* dead source
    /// aborts the strict read; `optional` or `AllowPartial` keeps the
    /// named-failure envelope.
    #[tokio::test]
    async fn dead_required_source_is_an_error_not_a_smaller_graph() {
        let ws = workspace_with(&[("adrs", "A.md", &doc("ADR-001", "adr", &[]))]);
        let specs = vec![
            SourceSpec::new(ws.path().to_path_buf()),
            SourceSpec::new(ws.path().join("does-not-exist")),
        ];
        let err = graph_live(&specs, GraphBounds::default(), QueryMode::Strict)
            .await
            .unwrap_err();
        assert!(matches!(err, ConstellationError::SourceUnavailable { .. }));

        // Required + AllowPartial: the caller opted into a partial view.
        let out = graph_live(&specs, GraphBounds::default(), QueryMode::AllowPartial)
            .await
            .unwrap();
        assert!(out.partial);
        assert_eq!(out.failures.len(), 1);
        assert_eq!(out.nodes.len(), 1);

        // Optional + Strict: same envelope — the role, not the path
        // spelling, is what decides.
        let specs = vec![
            SourceSpec::new(ws.path().to_path_buf()),
            SourceSpec::new_optional(ws.path().join("does-not-exist")),
        ];
        let out = graph_live(&specs, GraphBounds::default(), QueryMode::Strict)
            .await
            .unwrap();
        assert!(out.partial);
        assert_eq!(out.failures.len(), 1);
        assert_eq!(out.nodes.len(), 1);
    }

    fn git_in(dir: &std::path::Path, args: &[&str]) {
        let out = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    /// The snapshot channel: nodes and edges parsed out of pinned Git
    /// objects, provenance carried on the `git` channel.
    #[test]
    fn graph_at_a_snapshot_reads_pinned_objects() {
        let tmp = TempDir::new().unwrap();
        let fp = tmp.path().join(".forgeplan");
        for dir in ARTIFACT_DIRS {
            std::fs::create_dir_all(fp.join(dir)).unwrap();
        }
        std::fs::write(fp.join("adrs/A.md"), doc("ADR-001", "adr", &[])).unwrap();
        std::fs::write(
            fp.join("adrs/B.md"),
            doc("ADR-002", "adr", &[("ADR-001", "based_on")]),
        )
        .unwrap();
        git_in(tmp.path(), &["init", "-q"]);
        git_in(tmp.path(), &["config", "user.email", "t@t"]);
        git_in(tmp.path(), &["config", "user.name", "t"]);
        git_in(tmp.path(), &["add", "-A"]);
        git_in(tmp.path(), &["commit", "-qm", "init"]);

        let snapshot = capture_snapshot(&[SourceSpec::new(&fp)], Some("demo")).unwrap();
        let out = graph_snapshot(&snapshot, GraphBounds::default()).unwrap();
        assert!(out.failures.is_empty(), "{:?}", out.failures);
        assert_eq!(out.nodes.len(), 2);
        assert!(
            out.nodes
                .iter()
                .all(|n| n.source.channel == SourceChannel::PinnedGit)
        );
        // The version key IS the pin: full commit + tree + digest.
        assert!(
            out.nodes[0]
                .source
                .version_key
                .contains(&snapshot.sources[0].git_oid)
        );
        let edge = out
            .edges
            .iter()
            .find(|e| e.relation == "based_on")
            .expect("snapshot edge");
        assert_eq!(edge.from, "ADR-002");
        assert_eq!(edge.resolved.as_deref(), Some("ADR-001"));
        assert_eq!(edge.declared_in.as_deref(), Some("adrs/B.md"));
    }
}
