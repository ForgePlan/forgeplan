//! Stage 6 — ContextPack: an explainable, budgeted context bundle for an
//! agent's task.
//!
//! The pack answers "which documents should the agent read for this task,
//! what do they cost, and what did we leave out". Every unit carries its
//! source identity, the version it was read at, exact source text,
//! and the reasons for selection.
//!
//! * **Lexical ranking** — case-insensitive term scoring over canonical
//!   key, display id, title and body; title/key hits outrank body density.
//! * **Evidence units** retain kind categories without conferring authority.
//! * **Tokenizer & receipt** — tokens are estimated from UTF-8 bytes.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;

use super::capture::parse_artifact_file;
use super::federation::{
    QueryMode, SourceChannel, SourceRef, SourceSpec, failure_is_fatal, source_unavailable,
};
use super::jobs::DEFAULT_BODY_PREFIX_CHARS;
use super::snapshot::ConstellationSnapshot;
use super::{ConstellationError, sha256_hex};

/// Document-kind categories; none confers authority or a verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitRole {
    /// How the project wants work done (guides, conventions).
    Instruction,
    /// Requirements and decision document kinds; this does not imply authority.
    Decision,
    /// Evidence-kind material, without implying a positive result.
    Evidence,
    /// Everything else: notes, problems, solutions, epics…
    Material,
}

fn role_for(kind: &str) -> UnitRole {
    match kind {
        "adr" | "rfc" | "spec" | "prd" => UnitRole::Decision,
        "evidence" => UnitRole::Evidence,
        "guide" | "playbook" | "template" => UnitRole::Instruction,
        _ => UnitRole::Material,
    }
}

/// One selected fragment — an evidence unit.
#[derive(Debug, Clone, Serialize)]
pub struct EvidenceUnit {
    pub unit_id: String,
    pub source: SourceRef,
    pub canonical_key: String,
    pub display_id: Option<String>,
    pub title: String,
    pub kind: String,
    pub status: String,
    /// Declared evidence expiry, if the source provides one.
    pub valid_until: Option<String>,
    pub role: UnitRole,
    /// Exact body text. The span is relative to the captured body, not frontmatter.
    pub excerpt: String,
    pub relative_path: String,
    pub source_digest: String,
    pub span: EvidenceSpan,
    /// Estimated token count based on UTF-8 bytes, labelled by `token_kind`.
    pub tokens: usize,
    pub score: f64,
    /// Why this unit was selected — "term 'auth' in title (x3)", …
    pub reasons: Vec<String>,
}

/// Half-open UTF-8 byte offsets relative to the captured artifact body.
#[derive(Debug, Clone, Serialize)]
pub struct EvidenceSpan {
    pub coordinate_basis: &'static str,
    pub byte_start: usize,
    pub byte_end: usize,
}

/// A document that did not make the pack, with why.
#[derive(Debug, Clone, Serialize)]
pub struct Omission {
    pub canonical_key: String,
    pub source_id: String,
    /// `below_threshold` | `over_budget`
    pub reason: String,
    pub score: f64,
}

/// The spend report the spec asks for: budget, spend, per-role counts,
/// per-source counts, and the estimator's label.
#[derive(Debug, Clone, Serialize)]
pub struct BudgetReceipt {
    pub budget_tokens: usize,
    pub used_tokens: usize,
    pub token_kind: &'static str,
    pub units: usize,
    pub per_role: BTreeMap<String, usize>,
    pub per_source: BTreeMap<String, usize>,
    pub measured_bytes: usize,
    pub byte_cap: usize,
}

/// The pack itself.
#[derive(Debug, Clone, Serialize)]
pub struct ContextPack {
    /// What the pack was built for — echoed so the receipt is auditable.
    pub task: String,
    /// Version keys of the sources that answered (generation ids or
    /// capture digests or git OIDs — the channel says which).
    pub sources: Vec<SourceRef>,
    pub units: Vec<EvidenceUnit>,
    pub omitted: Vec<Omission>,
    pub receipt: BudgetReceipt,
    /// Sources that could not answer at all — the pack is `partial`.
    pub failures: Vec<super::federation::SourceFailure>,
    pub retrieval_mode: &'static str,
    pub closure_profile: &'static str,
    pub graph: PackGraph,
    pub source_complete: bool,
    pub traversal_complete: bool,
    pub bounds: TraversalBounds,
    pub traversal_limits: Vec<String>,
    pub delivery_complete: bool,
    pub next_cursor: Option<String>,
    pub mermaid_navigation: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraversalBounds {
    pub max_depth: usize,
    pub max_nodes: usize,
    pub max_edges: usize,
    pub max_millis: u64,
}

/// Caller-supplied limits and selection parameters.
#[derive(Debug, Clone, Serialize)]
pub struct PackOptions {
    pub budget_tokens: usize,
    pub byte_cap: usize,
    pub seeds: Vec<String>,
    pub max_depth: usize,
    pub max_nodes: usize,
    pub max_edges: usize,
    pub max_millis: u64,
    pub require_complete: bool,
    pub cursor: Option<String>,
    pub presentation: String,
    pub retrieval_mode: String,
    /// Caller-visible scope or snapshot identity, included in cursor binding.
    pub request_context: String,
    pub mermaid: bool,
}

impl Default for PackOptions {
    fn default() -> Self {
        Self {
            budget_tokens: 8000,
            byte_cap: 1_000_000,
            seeds: Vec::new(),
            max_depth: 4,
            max_nodes: 256,
            max_edges: 1024,
            max_millis: 5000,
            require_complete: false,
            cursor: None,
            presentation: "json".into(),
            request_context: String::new(),
            retrieval_mode: "lexical".into(),
            mermaid: false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PackNode {
    pub source_id: String,
    pub canonical_key: String,
    pub display_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackEdge {
    pub from: PackNode,
    pub to: Option<PackNode>,
    pub target: String,
    pub relation: String,
    pub provenance: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackGraph {
    pub nodes: Vec<PackNode>,
    pub edges: Vec<PackEdge>,
    /// This graph contains edges incident to units delivered on this page.
    pub scope: &'static str,
    /// Counts refer to the bounded selected graph, not the entire corpus.
    pub total_nodes: usize,
    pub total_edges: usize,
    /// True when this page alone contains the entire bounded selected graph.
    pub complete: bool,
}

const TOKEN_KIND: &str = "estimated_utf8_bytes_div_4";

/// Task text → normalized query terms (3+ chars, de-duplicated).
fn terms_of(task: &str) -> Vec<String> {
    let mut terms: Vec<String> = task
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 3)
        .map(str::to_string)
        .collect();
    terms.sort();
    terms.dedup();
    terms
}

type Packable = Arc<super::capture::ParsedArtifact>;

/// Read every artifact in one live source via its markdown channel.
async fn collect_live(
    spec: &SourceSpec,
) -> Result<(SourceRef, Vec<Packable>), super::federation::SourceFailure> {
    let canonical = super::federation::resolve_source(spec)?;
    let captured = super::capture::capture_canonical_census(
        &super::capture::CaptureRequest::new(&canonical),
        super::capture::CaptureLimits::default(),
    )
    .await
    .map_err(|e| super::federation::SourceFailure {
        path: spec.path.display().to_string(),
        reason: format!("{e}"),
    })?;
    if !captured.is_complete() {
        return Err(super::federation::SourceFailure {
            path: spec.path.display().to_string(),
            reason: format!(
                "canonical source capture is incomplete: {:?}",
                captured.diagnostics
            ),
        });
    }
    let source = SourceRef {
        source_id: format!(
            "src:{}",
            &sha256_hex(canonical.to_string_lossy().as_bytes())[..16]
        ),
        path: canonical.display().to_string(),
        channel: SourceChannel::Markdown,
        version_key: captured.source_set_digest.clone(),
        source_set_digest: Some(captured.source_set_digest.clone()),
    };
    let docs = captured.artifacts;
    Ok((source, docs))
}

/// Every artifact in a snapshot source, parsed from pinned git objects.
fn collect_snapshot(
    entry: &super::snapshot::SnapshotSourceEntry,
) -> Result<(SourceRef, Vec<Packable>), String> {
    let repo = PathBuf::from(&entry.checkout_root);
    let names =
        super::snapshot::list_files_at(&repo, &entry.git_oid).map_err(|e| format!("{e}"))?;
    let source = SourceRef {
        source_id: format!(
            "src:{}",
            &sha256_hex(format!("{}@{}", entry.checkout_root, entry.git_oid).as_bytes())[..16]
        ),
        path: entry.source_path.clone(),
        channel: SourceChannel::PinnedGit,
        version_key: format!(
            "{}:{}:{}",
            entry.git_oid, entry.tree_oid, entry.bytes_digest
        ),
        source_set_digest: None,
    };
    let mut docs = Vec::new();
    let mut canonical_keys = HashSet::new();
    for rel in names {
        let bytes = super::snapshot::read_file_at(&repo, &entry.git_oid, &rel)
            .map_err(|e| format!("{e}"))?;
        let p = parse_artifact_file(&rel, Arc::from(&bytes[..]), DEFAULT_BODY_PREFIX_CHARS)
            .map_err(|error| format!("artifact {rel} could not be parsed: {error}"))?;
        if !canonical_keys.insert(p.canonical_key.clone()) {
            return Err(format!(
                "snapshot contains conflicting canonical key `{}`",
                p.canonical_key
            ));
        }
        docs.push(Arc::new(p));
    }
    Ok((source, docs))
}

fn pack_error(detail: impl Into<String>) -> ConstellationError {
    ConstellationError::InvalidPackRequest {
        detail: detail.into(),
    }
}

fn validate_options(options: &PackOptions) -> Result<usize, ConstellationError> {
    if options.budget_tokens == 0
        || options.budget_tokens > 2_000_000
        || options.byte_cap == 0
        || options.byte_cap > 8_000_000
        || options.max_depth == 0
        || options.max_depth > 64
        || options.max_nodes == 0
        || options.max_nodes > 4096
        || options.max_edges == 0
        || options.max_edges > 16384
        || options.max_millis == 0
        || options.max_millis > 60_000
        || options.seeds.len() > 128
        || options
            .seeds
            .iter()
            .any(|seed| seed.is_empty() || seed.len() > 512)
        || options.cursor.as_ref().is_some_and(|s| s.len() > 256)
        || options.presentation.len() > 64
        || options.request_context.len() > 256
    {
        return Err(pack_error("invalid pack limits; check documented maximums"));
    }
    if options.retrieval_mode != "lexical" {
        return Err(pack_error(
            "unsupported retrieval mode; only lexical is available",
        ));
    }
    let token_cap = options
        .budget_tokens
        .checked_mul(4)
        .ok_or_else(|| pack_error("budget overflow"))?;
    Ok(options.byte_cap.min(token_cap))
}

fn node(source: &SourceRef, doc: &Packable) -> PackNode {
    PackNode {
        source_id: source.source_id.clone(),
        canonical_key: doc.canonical_key.clone(),
        display_id: doc.display_id.clone(),
    }
}

fn mermaid_label(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '"' => escaped.push_str("&quot;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '\n' | '\r' => escaped.push(' '),
            other => escaped.push(other),
        }
    }
    escaped
}

fn graph_mermaid(graph: &PackGraph) -> String {
    let mut nodes: Vec<&PackNode> = graph.nodes.iter().collect();
    for edge in &graph.edges {
        for endpoint in std::iter::once(&edge.from).chain(edge.to.iter()) {
            if !nodes.iter().any(|n| {
                n.source_id == endpoint.source_id && n.canonical_key == endpoint.canonical_key
            }) {
                nodes.push(endpoint);
            }
        }
    }
    let mut out = String::from("graph LR\n");
    for (i, n) in nodes.iter().enumerate() {
        let label = mermaid_label(&format!("{} @ {}", n.display_id, n.source_id));
        out.push_str(&format!("  n{i}[\"{label}\"]\n"));
    }
    for (edge_index, edge) in graph.edges.iter().enumerate() {
        let from = nodes
            .iter()
            .position(|node| {
                node.source_id == edge.from.source_id
                    && node.canonical_key == edge.from.canonical_key
            })
            .unwrap();
        if let Some(to) = &edge.to {
            let target = nodes
                .iter()
                .position(|node| {
                    node.source_id == to.source_id && node.canonical_key == to.canonical_key
                })
                .unwrap();
            out.push_str(&format!("  n{from} -->|{}| n{target}\n", edge.relation));
        } else {
            let label = mermaid_label(&edge.target);
            out.push_str(&format!("  u{edge_index}[\"unresolved: {label}\"]\n"));
            out.push_str(&format!(
                "  n{from} -.->|{}| u{edge_index}\n",
                edge.relation
            ));
        }
    }
    out
}

/// Select a bounded declared closure and deliver whole units measured by the
/// caller's actual public serializer.
fn build_pack(
    task: &str,
    options: &PackOptions,
    sources: Vec<(SourceRef, Vec<Packable>)>,
    failures: Vec<super::federation::SourceFailure>,
    basis_context: &str,
    measure: &(dyn Fn(&ContextPack) -> Result<usize, String> + Send + Sync),
) -> Result<ContextPack, ConstellationError> {
    let cap = validate_options(options)?;
    let started = std::time::Instant::now();
    let mut docs: Vec<(SourceRef, Packable)> = sources
        .iter()
        .flat_map(|(s, ds)| ds.iter().map(move |d| (s.clone(), Arc::clone(d))))
        .collect();
    docs.sort_by(|a, b| {
        a.0.source_id
            .cmp(&b.0.source_id)
            .then(a.1.canonical_key.cmp(&b.1.canonical_key))
    });
    let mut keys: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    let mut aliases: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, (_, doc)) in docs.iter().enumerate() {
        keys.entry(&doc.canonical_key).or_default().push(i);
        let alias = crate::artifact::canonical_key::normalize_display_id(&doc.display_id)
            .unwrap_or_else(|| doc.display_id.clone());
        aliases.entry(alias).or_default().push(i);
    }
    let mut stops = BTreeSet::new();
    let resolve = |reference: &str, local: Option<usize>| -> Vec<usize> {
        let reference = reference.trim();
        let (key, qualifier) = super::federation::parse_qualified_ref(reference);
        if key.is_empty() || (reference.contains('@') && qualifier.is_none()) {
            return Vec::new();
        }
        let allowed = |i: usize| {
            let source = &docs[i].0;
            match qualifier {
                Some(q) => super::federation::qualifier_matches(&source.source_id, &source.path, q),
                None => local.is_none_or(|n| docs[n].0.source_id == source.source_id),
            }
        };
        let exact: Vec<usize> = keys
            .get(key)
            .into_iter()
            .flatten()
            .copied()
            .filter(|&i| allowed(i))
            .collect();
        let normalized = crate::artifact::canonical_key::normalize_display_id(key);
        if !exact.is_empty() && normalized.is_none() {
            return exact;
        }
        let alias_key = normalized.as_deref().unwrap_or(key);
        let mut matches = exact;
        for i in aliases
            .get(alias_key)
            .into_iter()
            .flatten()
            .copied()
            .filter(|&i| allowed(i))
        {
            if !matches.contains(&i) {
                matches.push(i);
            }
        }
        matches
    };
    // Explicit selectors are request validation, not optional retrieval work.
    // Resolve every one before a traversal deadline can stop scoring.
    let mut seeds = Vec::new();
    let mut seed_members = BTreeSet::new();
    let mut explicit = BTreeSet::new();
    for seed in &options.seeds {
        let hits = resolve(seed, None);
        if hits.len() != 1 {
            return Err(pack_error(format!("invalid or ambiguous seed: {seed}")));
        }
        explicit.insert(hits[0]);
        if seed_members.insert(hits[0]) {
            seeds.push(hits[0]);
        }
    }
    // A reverse name index permits incoming-edge discovery without resolving
    // every corpus edge. Actual targets are resolved once when encountered.
    let mut incoming: BTreeMap<String, Vec<(usize, String, String)>> = BTreeMap::new();
    'index: for (i, (_, doc)) in docs.iter().enumerate() {
        for edge in &doc.outgoing {
            if started.elapsed().as_millis() >= u128::from(options.max_millis) {
                stops.insert("elapsed_work_limit".to_string());
                break 'index;
            }
            if !matches!(
                edge.relation.as_str(),
                "informs" | "contradicts" | "supersedes"
            ) {
                continue;
            }
            let (key, _) = super::federation::parse_qualified_ref(&edge.target);
            let key = crate::artifact::canonical_key::normalize_display_id(key)
                .unwrap_or_else(|| key.to_string());
            incoming
                .entry(key)
                .or_default()
                .push((i, edge.relation.clone(), edge.target.clone()));
        }
    }
    let mut edge_cache: BTreeMap<(usize, String, String), Option<usize>> = BTreeMap::new();
    let terms = terms_of(task);
    let mut scores = Vec::with_capacity(docs.len());
    for (_, doc) in &docs {
        if started.elapsed().as_millis() >= u128::from(options.max_millis) {
            stops.insert("elapsed_work_limit".to_string());
            scores.push((0.0, Vec::new()));
            continue;
        }
        let key = doc.canonical_key.to_lowercase();
        let alias = doc.display_id.to_lowercase();
        let title = doc.title.to_lowercase();
        let body = doc.body.to_lowercase();
        let mut score = 0.0;
        let mut reasons = Vec::new();
        for term in &terms {
            if started.elapsed().as_millis() >= u128::from(options.max_millis) {
                stops.insert("elapsed_work_limit".to_string());
                break;
            }
            if key.contains(term) || alias.contains(term) {
                score += 4.0;
                reasons.push(format!("term '{term}' in id"));
            }
            if title.contains(term) {
                score += 3.0;
                reasons.push(format!("term '{term}' in title"));
            }
            let count = body.match_indices(term).take(3).count();
            if count > 0 {
                score += count.min(3) as f64;
                reasons.push(format!("term '{term}' in body"));
            }
        }
        scores.push((score, reasons));
    }
    let mut ranked = BinaryHeap::new();
    for (i, (score, _)) in scores.iter().enumerate() {
        if started.elapsed().as_millis() >= u128::from(options.max_millis) {
            stops.insert("elapsed_work_limit".to_string());
            break;
        }
        if *score > 0.0 {
            ranked.push((*score as u64, Reverse(i)));
        }
    }
    while let Some((_, Reverse(i))) = ranked.pop() {
        if started.elapsed().as_millis() >= u128::from(options.max_millis) {
            stops.insert("elapsed_work_limit".to_string());
            break;
        }
        if seed_members.insert(i) {
            seeds.push(i);
        }
    }
    let mut visited = BTreeSet::new();
    let mut selected = Vec::new();
    let mut mandatory = BTreeSet::new();
    let mut paths: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
    let mut graph_edges = Vec::new();
    let mut edge_keys = BTreeSet::new();
    for seed in seeds {
        let initial_reason = if explicit.contains(&seed) {
            "explicit seed"
        } else {
            "lexical seed"
        };
        paths
            .entry(seed)
            .or_default()
            .insert(initial_reason.to_string());
        if visited.contains(&seed) {
            continue;
        }
        let mut queue = VecDeque::from([(seed, 0usize, initial_reason.to_string())]);
        while let Some((i, depth, reason)) = queue.pop_front() {
            if visited.contains(&i) {
                continue;
            }
            if started.elapsed().as_millis() >= u128::from(options.max_millis) {
                stops.insert("elapsed_work_limit".to_string());
                break;
            }
            if visited.len() >= options.max_nodes {
                stops.insert("node_limit".to_string());
                break;
            }
            visited.insert(i);
            selected.push((i, reason));
            let mut candidates = BTreeSet::new();
            for edge in &docs[i].1.outgoing {
                if matches!(
                    edge.relation.as_str(),
                    "based_on" | "refines" | "contradicts" | "supersedes"
                ) {
                    candidates.insert((i, edge.relation.clone(), edge.target.clone()));
                }
            }
            let doc = &docs[i].1;
            for name in [&doc.canonical_key, &doc.display_id] {
                let name = crate::artifact::canonical_key::normalize_display_id(name)
                    .unwrap_or_else(|| name.clone());
                if let Some(edges) = incoming.get(&name) {
                    candidates.extend(edges.iter().cloned());
                }
            }
            let mut adjacency = Vec::new();
            for (from, relation, target) in candidates {
                if started.elapsed().as_millis() >= u128::from(options.max_millis) {
                    stops.insert("elapsed_work_limit".to_string());
                    break;
                }
                let cache_key = (from, relation.clone(), target.clone());
                let to = *edge_cache.entry(cache_key).or_insert_with(|| {
                    let hits = resolve(&target, Some(from));
                    if hits.len() == 1 { Some(hits[0]) } else { None }
                });
                if from == i || to == Some(i) {
                    adjacency.push((from, to, relation, target));
                }
            }
            for (from, to, relation, target) in &adjacency {
                let follows = (*from == i && matches!(relation.as_str(), "based_on" | "refines"))
                    || (*from != i && relation == "informs")
                    || matches!(relation.as_str(), "contradicts" | "supersedes");
                if !follows {
                    continue;
                }
                let key = (*from, *to, relation.clone(), target.clone());
                if edge_keys.insert(key) {
                    if graph_edges.len() >= options.max_edges {
                        stops.insert("edge_limit".to_string());
                        break;
                    }
                    graph_edges.push(PackEdge {
                        from: node(&docs[*from].0, &docs[*from].1),
                        to: to.map(|j| node(&docs[j].0, &docs[j].1)),
                        target: target.clone(),
                        relation: relation.clone(),
                        provenance: "frontmatter",
                    });
                }
                let next = if *from == i { *to } else { Some(*from) };
                if let Some(j) = next {
                    if j != i {
                        mandatory.insert(j);
                    }
                    paths.entry(j).or_default().insert(format!(
                        "declared {relation} relation from {}@{}",
                        docs[i].1.canonical_key, docs[i].0.source_id
                    ));
                }
                match next {
                    None => {
                        stops.insert("unresolved_declared_relation".to_string());
                    }
                    Some(j) if !visited.contains(&j) => {
                        if depth >= options.max_depth {
                            stops.insert("depth_limit".to_string());
                        } else {
                            queue.push_back((
                                j,
                                depth + 1,
                                format!("declared {relation} relation"),
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    selected.sort_by_key(|(i, _)| {
        if mandatory.contains(i) {
            0
        } else if explicit.contains(i) {
            1
        } else {
            2
        }
    });
    if graph_edges.iter().any(|edge| {
        edge.to.as_ref().is_some_and(|target| {
            !selected.iter().any(|(i, _)| {
                docs[*i].0.source_id == target.source_id
                    && docs[*i].1.canonical_key == target.canonical_key
            })
        }) || !selected.iter().any(|(i, _)| {
            docs[*i].0.source_id == edge.from.source_id
                && docs[*i].1.canonical_key == edge.from.canonical_key
        })
    }) {
        stops.insert("graph_endpoint_not_visited".to_string());
    }
    let all_units: Vec<EvidenceUnit> = selected
        .iter()
        .map(|(i, reason)| {
            let (source, doc) = &docs[*i];
            let mut reasons = scores[*i].1.clone();
            reasons.insert(0, reason.clone());
            if let Some(paths) = paths.get(i) {
                reasons.extend(paths.iter().cloned());
            }
            EvidenceUnit {
                unit_id: format!("{}@{}", doc.canonical_key, source.source_id),
                source: source.clone(),
                canonical_key: doc.canonical_key.clone(),
                display_id: Some(doc.display_id.clone()),
                title: doc.title.clone(),
                kind: doc.kind.clone(),
                status: doc.status.clone(),
                valid_until: doc.valid_until.clone(),
                role: role_for(&doc.kind),
                excerpt: doc.body.to_string(),
                relative_path: doc.relative_path.clone(),
                source_digest: doc.source_digest.clone(),
                span: EvidenceSpan {
                    coordinate_basis: "artifact_body_utf8_bytes",
                    byte_start: 0,
                    byte_end: doc.body.len(),
                },
                tokens: doc.body.len().div_ceil(4),
                score: scores[*i].0,
                reasons,
            }
        })
        .collect();
    let mut basis_options = options.clone();
    basis_options.cursor = None;
    let basis = serde_json::to_vec(&(
        task,
        "declared-closure-v1",
        "page-incident-graph-v1",
        basis_context,
        &basis_options,
        sources.iter().map(|(s, _)| s).collect::<Vec<_>>(),
        &failures,
        &all_units,
        &graph_edges,
        &stops,
    ))
    .map_err(|e| pack_error(e.to_string()))?;
    let basis = sha256_hex(&basis);
    let cursor_for = |offset: usize| {
        let payload = format!("{basis}:{offset}");
        format!("{offset}.{}", sha256_hex(payload.as_bytes()))
    };
    let offset = if let Some(cursor) = &options.cursor {
        let invalid = || {
            ConstellationError::Resolve(
                crate::artifact::canonical_key::ResolveError::InvalidSelector {
                    input: "malformed continuation".into(),
                },
            )
        };
        let (offset, digest) = cursor.split_once('.').ok_or_else(invalid)?;
        if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(invalid());
        }
        let offset = offset.parse::<usize>().map_err(|_| invalid())?;
        if offset == 0 || offset >= all_units.len() || cursor_for(offset) != *cursor {
            return Err(ConstellationError::IndexGenerationUnavailable {
                detail: "continuation is stale or its request binding changed; restart the pack"
                    .into(),
            });
        }
        offset
    } else {
        0
    };
    let mut pack = ContextPack {
        task: task.into(),
        sources: sources.into_iter().map(|(s, _)| s).collect(),
        units: Vec::new(),
        omitted: Vec::new(),
        receipt: BudgetReceipt {
            budget_tokens: options.budget_tokens,
            used_tokens: 0,
            token_kind: TOKEN_KIND,
            units: 0,
            per_role: BTreeMap::new(),
            per_source: BTreeMap::new(),
            measured_bytes: 0,
            byte_cap: cap,
        },
        source_complete: failures.is_empty(),
        traversal_complete: stops.is_empty(),
        bounds: TraversalBounds {
            max_depth: options.max_depth,
            max_nodes: options.max_nodes,
            max_edges: options.max_edges,
            max_millis: options.max_millis,
        },
        traversal_limits: stops.into_iter().collect(),
        delivery_complete: false,
        next_cursor: None,
        retrieval_mode: "lexical",
        closure_profile: "declared-closure-v1",
        graph: PackGraph {
            nodes: Vec::new(),
            edges: Vec::new(),
            scope: "page_incident",
            total_nodes: selected.len(),
            total_edges: graph_edges.len(),
            complete: false,
        },
        mermaid_navigation: None,
        failures,
    };
    if options.require_complete && (!pack.source_complete || !pack.traversal_complete) {
        return Err(ConstellationError::IncompletePack {
            detail: "incomplete sources or declared obligations".into(),
        });
    }
    // Index the captured graph once, retaining its original edge order.
    let mut incident: BTreeMap<(&str, &str), Vec<usize>> = BTreeMap::new();
    for (index, edge) in graph_edges.iter().enumerate() {
        incident
            .entry((&edge.from.source_id, &edge.from.canonical_key))
            .or_default()
            .push(index);
        if let Some(to) = &edge.to {
            incident
                .entry((&to.source_id, &to.canonical_key))
                .or_default()
                .push(index);
        }
    }
    let mut edge_indices = BTreeSet::new();
    let mut end = offset;
    let mut last_fitting = None;
    loop {
        if end < all_units.len() {
            let unit = &all_units[end];
            if let Some(indices) =
                incident.get(&(unit.source.source_id.as_str(), unit.canonical_key.as_str()))
            {
                edge_indices.extend(indices.iter().copied());
            }
            pack.graph.nodes.push(PackNode {
                source_id: unit.source.source_id.clone(),
                canonical_key: unit.canonical_key.clone(),
                display_id: unit.display_id.clone().unwrap_or_default(),
            });
            pack.units.push(unit.clone());
            end += 1;
        }
        pack.graph.edges = edge_indices
            .iter()
            .map(|&i| graph_edges[i].clone())
            .collect();
        pack.graph.complete = pack.graph.nodes.len() == pack.graph.total_nodes
            && pack.graph.edges.len() == pack.graph.total_edges;
        pack.mermaid_navigation = options.mermaid.then(|| graph_mermaid(&pack.graph));
        pack.delivery_complete = end == all_units.len();
        pack.next_cursor = (!pack.delivery_complete).then(|| cursor_for(end));
        pack.receipt.units = pack.units.len();
        pack.receipt.per_role.clear();
        pack.receipt.per_source.clear();
        for unit in &pack.units {
            *pack
                .receipt
                .per_role
                .entry(format!("{:?}", unit.role).to_lowercase())
                .or_default() += 1;
            *pack
                .receipt
                .per_source
                .entry(unit.source.source_id.clone())
                .or_default() += 1;
        }
        let mut converged = false;
        for _ in 0..16 {
            let bytes = measure(&pack).map_err(pack_error)?;
            let tokens = bytes.div_ceil(4);
            if pack.receipt.measured_bytes == bytes && pack.receipt.used_tokens == tokens {
                converged = true;
                break;
            }
            pack.receipt.measured_bytes = bytes;
            pack.receipt.used_tokens = tokens;
        }
        if !converged {
            return Err(pack_error("response measurement did not converge"));
        }
        if pack.receipt.measured_bytes > cap {
            if let Some(previous) = last_fitting {
                pack = previous;
                break;
            }
            return Err(ConstellationError::InsufficientBudget {
                budget_tokens: options.budget_tokens,
                smallest_unit_tokens: pack.receipt.measured_bytes.div_ceil(4),
            });
        }
        if pack.delivery_complete {
            break;
        }
        last_fitting = Some(pack.clone());
    }
    if options.require_complete && !pack.delivery_complete {
        return Err(ConstellationError::IncompletePack {
            detail: "paginated delivery; increase budget and byte cap".into(),
        });
    }
    Ok(pack)
}

/// Pack a task's context from live scope sources.
///
/// `mode` applies the required/optional contract: a `required` source
/// that cannot answer aborts the strict pack with `source_unavailable` —
/// a pack assembled without it would silently under-rank what the task
/// needed. An `optional` source, or any source under
/// [`QueryMode::AllowPartial`], lands in `failures` instead.
pub async fn pack_live(
    specs: &[SourceSpec],
    task: &str,
    options: &PackOptions,
    mode: QueryMode,
    measure: &(dyn Fn(&ContextPack) -> Result<usize, String> + Send + Sync),
) -> Result<ContextPack, ConstellationError> {
    validate_options(options)?;
    if task.len() > 32768 {
        return Err(pack_error("task exceeds 32768 UTF-8 bytes"));
    }
    let mut sources = Vec::new();
    let mut failures = Vec::new();
    for spec in specs {
        match collect_live(spec).await {
            Ok(pair) => sources.push(pair),
            Err(f) => {
                if failure_is_fatal(spec, mode) {
                    return Err(source_unavailable(&f));
                }
                failures.push(f);
            }
        }
    }
    let basis_context = format!(
        "{mode:?}:{:?}",
        specs.iter().map(|s| (&s.path, s.role)).collect::<Vec<_>>()
    );
    build_pack(task, options, sources, failures, &basis_context, measure)
}

/// Pack a task's context from a recorded snapshot — pinned revisions.
pub fn pack_snapshot(
    snapshot: &ConstellationSnapshot,
    task: &str,
    options: &PackOptions,
    measure: &(dyn Fn(&ContextPack) -> Result<usize, String> + Send + Sync),
) -> Result<ContextPack, ConstellationError> {
    validate_options(options)?;
    if task.len() > 32768 {
        return Err(pack_error("task exceeds 32768 UTF-8 bytes"));
    }
    let mut sources = Vec::new();
    let mut failures: Vec<super::federation::SourceFailure> = snapshot
        .skipped
        .iter()
        .map(|s| super::federation::SourceFailure {
            path: s.path.clone(),
            reason: format!("source was not pinned: {}", s.reason),
        })
        .collect();
    for entry in &snapshot.sources {
        match collect_snapshot(entry) {
            Ok(pair) => sources.push(pair),
            Err(reason) => failures.push(super::federation::SourceFailure {
                path: entry.source_path.clone(),
                reason,
            }),
        }
    }
    build_pack(
        task,
        options,
        sources,
        failures,
        &snapshot.snapshot_id,
        measure,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    fn json_size(pack: &ContextPack) -> Result<usize, String> {
        serde_json::to_string(pack)
            .map(|s| s.len() + 1)
            .map_err(|e| e.to_string())
    }

    async fn pack_live(
        specs: &[SourceSpec],
        task: &str,
        budget: usize,
        mode: QueryMode,
    ) -> Result<ContextPack, ConstellationError> {
        let options = PackOptions {
            budget_tokens: budget,
            ..PackOptions::default()
        };
        super::pack_live(specs, task, &options, mode, &json_size).await
    }

    fn pack_snapshot(
        snapshot: &ConstellationSnapshot,
        task: &str,
        budget: usize,
    ) -> Result<ContextPack, ConstellationError> {
        let options = PackOptions {
            budget_tokens: budget,
            ..PackOptions::default()
        };
        super::pack_snapshot(snapshot, task, &options, &json_size)
    }

    const AUTH_DOC: &str = r#"---
id: ADR-001
kind: adr
status: accepted
title: Auth storage decision
---
# Auth storage decision

We decided auth tokens live in the token vault service, not in cookies.
The token vault encrypts at rest and rotates every hour.
"#;

    const COOKIES_DOC: &str = r#"---
id: NOTE-007
kind: note
status: draft
---
# Frontend styling roundup

Fonts, colors and spacing. No backend concerns here.
"#;

    /// A `.forgeplan` root with `kind/` artifact directories — the same
    /// shape federation fixtures use.
    fn ws_root() -> TempDir {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join(".forgeplan");
        std::fs::create_dir_all(root.join("adrs")).unwrap();
        std::fs::create_dir_all(root.join("notes")).unwrap();
        std::fs::write(root.join("adrs/ADR-001-auth.md"), AUTH_DOC).unwrap();
        std::fs::write(root.join("notes/NOTE-007-style.md"), COOKIES_DOC).unwrap();
        tmp
    }
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

    #[tokio::test]
    async fn pack_selects_relevant_unit_with_reasons_and_receipt() {
        let ws = ws_root();
        let specs = vec![SourceSpec::new(ws.path().join(".forgeplan"))];

        let pack = pack_live(&specs, "auth token storage", 10_000, QueryMode::Strict)
            .await
            .unwrap();
        assert!(pack.failures.is_empty(), "{:?}", pack.failures);
        assert_eq!(pack.units.len(), 1);
        let unit = &pack.units[0];
        assert_eq!(unit.display_id.as_deref(), Some("ADR-001"));
        assert_eq!(unit.role, UnitRole::Decision);
        assert!(unit.reasons.iter().any(|r| r.contains("'auth' in title")));
        assert!(unit.reasons.iter().any(|r| r.contains("'auth'")));
        assert!(unit.excerpt.contains("token vault") || unit.excerpt.contains("auth"));
        assert!(pack.units.iter().all(|u| u.canonical_key != "NOTE-007"));
        assert_eq!(pack.receipt.units, 1);
        assert_eq!(pack.receipt.token_kind, "estimated_utf8_bytes_div_4");
        assert_eq!(*pack.receipt.per_role.get("decision").unwrap(), 1);
    }

    #[tokio::test]
    async fn unit_preserves_complete_unicode_body_and_span() {
        let ws = ws_root();
        let content = format!(
            "{AUTH_DOC}\n{}\nException: 雪 \"quoted\".\n",
            "padding ".repeat(100)
        );
        std::fs::write(ws.path().join(".forgeplan/adrs/ADR-001-auth.md"), &content).unwrap();
        let (_, body) = content.split_once("\n---\n").unwrap();
        let body = body.trim_start_matches('\n');
        let pack = pack_live(
            &[SourceSpec::new(ws.path().join(".forgeplan"))],
            "auth",
            10_000,
            QueryMode::Strict,
        )
        .await
        .unwrap();
        let unit = &pack.units[0];
        assert_eq!(
            unit.excerpt.as_bytes(),
            &body.as_bytes()[unit.span.byte_start..unit.span.byte_end]
        );
        assert_eq!(unit.span.coordinate_basis, "artifact_body_utf8_bytes");
        assert_eq!(unit.source_digest, sha256_hex(content.as_bytes()));
        assert!(unit.excerpt.contains("Exception: 雪 \"quoted\"."));
    }

    #[tokio::test]
    async fn same_task_packs_identically_twice() {
        let ws = ws_root();
        let specs = vec![SourceSpec::new(ws.path().join(".forgeplan"))];
        let p1 = pack_live(&specs, "auth", 10_000, QueryMode::Strict)
            .await
            .unwrap();
        let p2 = pack_live(&specs, "auth", 10_000, QueryMode::Strict)
            .await
            .unwrap();
        let j1 = serde_json::to_string(&p1.units).unwrap();
        let j2 = serde_json::to_string(&p2.units).unwrap();
        assert_eq!(j1, j2);
    }

    #[tokio::test]
    async fn a_dead_required_source_fails_the_pack_strictly() {
        // A required source's outage must not produce a pack that reads
        // as complete — the strict read errors; optional sources (or an
        // explicit AllowPartial call) keep the failures envelope.
        let ws = ws_root();
        let specs = vec![
            SourceSpec::new(ws.path().join(".forgeplan")),
            SourceSpec::new("/definitely/not/a/workspace"),
        ];
        let err = pack_live(&specs, "auth", 10_000, QueryMode::Strict)
            .await
            .unwrap_err();
        assert!(matches!(err, ConstellationError::SourceUnavailable { .. }));

        let pack = pack_live(&specs, "auth", 10_000, QueryMode::AllowPartial)
            .await
            .unwrap();
        assert_eq!(pack.failures.len(), 1);
        assert_eq!(pack.units.len(), 1);

        let specs = vec![
            SourceSpec::new(ws.path().join(".forgeplan")),
            SourceSpec::new_optional("/definitely/not/a/workspace"),
        ];
        let pack = pack_live(&specs, "auth", 10_000, QueryMode::Strict)
            .await
            .unwrap();
        assert_eq!(pack.failures.len(), 1);
        assert_eq!(pack.units.len(), 1);
    }

    #[tokio::test]
    async fn incomplete_live_source_is_failed_as_a_whole() {
        let good = ws_root();
        let bad = TempDir::new().unwrap();
        let bad_root = bad.path().join(".forgeplan");
        std::fs::create_dir_all(bad_root.join("adrs")).unwrap();
        std::fs::write(
            bad_root.join("adrs/ADR-777-valid.md"),
            AUTH_DOC.replace("ADR-001", "ADR-777"),
        )
        .unwrap();
        std::fs::write(bad_root.join("adrs/BROKEN.md"), "---\nid: [invalid\n").unwrap();
        let specs = vec![
            SourceSpec::new(good.path().join(".forgeplan")),
            SourceSpec::new(&bad_root),
        ];

        let strict = pack_live(&specs, "auth", 10_000, QueryMode::Strict)
            .await
            .expect_err("an incomplete required source must refuse a strict pack");
        assert!(matches!(
            strict,
            ConstellationError::SourceUnavailable { .. }
        ));

        let partial = pack_live(&specs, "auth", 10_000, QueryMode::AllowPartial)
            .await
            .unwrap();
        assert_eq!(partial.failures.len(), 1);
        assert_eq!(partial.failures[0].path, bad_root.display().to_string());
        assert!(partial.failures[0].reason.contains("adrs/BROKEN.md"));
        assert_eq!(partial.sources.len(), 1);
        assert!(
            partial
                .units
                .iter()
                .any(|unit| unit.display_id.as_deref() == Some("ADR-001"))
        );
        assert!(
            partial
                .units
                .iter()
                .all(|unit| unit.display_id.as_deref() != Some("ADR-777")),
            "valid documents from the invalid source must not leak into a partial pack"
        );
    }

    #[tokio::test]
    async fn canonical_key_collision_is_a_source_failure() {
        let good = ws_root();
        let bad = TempDir::new().unwrap();
        let bad_root = bad.path().join(".forgeplan");
        std::fs::create_dir_all(bad_root.join("prds")).unwrap();
        let duplicate = AUTH_DOC
            .replace("ADR-001", "PRD-777")
            .replace("kind: adr", "kind: prd");
        std::fs::write(bad_root.join("prds/PRD-777-a.md"), &duplicate).unwrap();
        std::fs::write(bad_root.join("prds/PRD-777-b.md"), &duplicate).unwrap();
        let specs = vec![
            SourceSpec::new(good.path().join(".forgeplan")),
            SourceSpec::new(&bad_root),
        ];

        let strict = pack_live(&specs, "auth", 10_000, QueryMode::Strict)
            .await
            .expect_err("a duplicate canonical key must refuse a strict pack");
        assert!(matches!(
            strict,
            ConstellationError::SourceUnavailable { .. }
        ));

        let partial = pack_live(&specs, "auth", 10_000, QueryMode::AllowPartial)
            .await
            .unwrap();
        assert_eq!(partial.failures.len(), 1);
        assert_eq!(partial.failures[0].path, bad_root.display().to_string());
        assert!(partial.failures[0].reason.contains("IdentityCollision"));
        assert_eq!(partial.sources.len(), 1);
        assert!(
            partial
                .units
                .iter()
                .all(|unit| unit.display_id.as_deref() != Some("PRD-777"))
        );
    }

    #[test]
    fn malformed_pinned_artifact_is_a_named_snapshot_failure() {
        let tmp = TempDir::new().unwrap();
        let fp = tmp.path().join(".forgeplan");
        std::fs::create_dir_all(fp.join("adrs")).unwrap();
        std::fs::write(fp.join("adrs/ADR-001-auth.md"), AUTH_DOC).unwrap();
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
        assert_eq!(snapshot.sources[0].files, 2);
        let pack = pack_snapshot(&snapshot, "auth", 10_000).unwrap();
        assert_eq!(pack.failures.len(), 1);
        assert_eq!(pack.failures[0].path, snapshot.sources[0].source_path);
        assert!(pack.failures[0].reason.contains("adrs/BROKEN.md"));
        assert!(pack.units.is_empty());
        assert!(pack.sources.is_empty());
    }

    fn add_doc(ws: &TempDir, id: &str, links: &[(&str, &str)], body: &str) {
        let mut text = format!("---\nid: {id}\nkind: adr\nstatus: draft\ntitle: {id}\n");
        if !links.is_empty() {
            text.push_str("links:\n");
            for (target, relation) in links {
                text.push_str(&format!("  - target: {target}\n    relation: {relation}\n"));
            }
        }
        text.push_str("---\n");
        text.push_str(body);
        std::fs::write(ws.path().join(format!(".forgeplan/adrs/{id}.md")), text).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn replaced_source_root_symlink_is_refused_without_reading_target() {
        let original = ws_root();
        let outside = ws_root();
        let root = original.path().join(".forgeplan");
        std::fs::rename(&root, original.path().join("saved")).unwrap();
        std::os::unix::fs::symlink(outside.path().join(".forgeplan"), &root).unwrap();
        let spec = SourceSpec::new(&root);
        let sources = std::slice::from_ref(&spec);
        let strict = pack_live(sources, "auth", 10_000, QueryMode::Strict).await;
        assert!(matches!(
            strict,
            Err(ConstellationError::SourceUnavailable { .. })
        ));
        let partial = pack_live(sources, "auth", 10_000, QueryMode::AllowPartial)
            .await
            .unwrap();
        assert!(partial.units.is_empty());
        assert_eq!(partial.failures.len(), 1);
        assert!(partial.failures[0].reason.contains("symlink"));
    }

    #[tokio::test]
    async fn grounds_of_lower_ranked_seed_precede_unrelated_higher_ranked_seed() {
        let ws = ws_root();
        add_doc(&ws, "ADR-010", &[], "orbital orbital orbital orbital");
        add_doc(&ws, "ADR-011", &[("ADR-012", "based_on")], "orbital");
        add_doc(&ws, "ADR-012", &[], "independent ground");
        let options = PackOptions::default();
        let result = super::pack_live(
            &[SourceSpec::new(ws.path().join(".forgeplan"))],
            "orbital",
            &options,
            QueryMode::Strict,
            &json_size,
        )
        .await
        .unwrap();
        assert_eq!(result.units[0].canonical_key, "ADR-012");
        let a = result
            .units
            .iter()
            .position(|unit| unit.canonical_key == "ADR-010")
            .unwrap();
        assert!(a > 0);
        assert!(
            result.units[0]
                .reasons
                .iter()
                .any(|reason| reason.contains("based_on"))
        );
    }

    #[tokio::test]
    async fn elapsed_limit_is_reported_during_many_term_scoring() {
        let ws = ws_root();
        let body = "z".repeat(128 * 1024);
        for number in 100..132 {
            add_doc(&ws, &format!("ADR-{number}"), &[], &body);
        }
        let task = (0..128)
            .map(|n| format!("term{n:04}"))
            .collect::<Vec<_>>()
            .join(" ");
        let options = PackOptions {
            max_millis: 1,
            ..PackOptions::default()
        };
        let result = super::pack_live(
            &[SourceSpec::new(ws.path().join(".forgeplan"))],
            &task,
            &options,
            QueryMode::Strict,
            &json_size,
        )
        .await
        .unwrap();
        assert!(!result.traversal_complete);
        assert!(
            result
                .traversal_limits
                .iter()
                .any(|limit| limit == "elapsed_work_limit")
        );
    }

    #[tokio::test]
    async fn invalid_and_ambiguous_explicit_seeds_refuse_even_when_scoring_would_exhaust_deadline()
    {
        let first = ws_root();
        let second = ws_root();
        let body = "z".repeat(128 * 1024);
        for number in 100..132 {
            add_doc(&first, &format!("ADR-{number}"), &[], &body);
        }
        let specs = [
            SourceSpec::new(first.path().join(".forgeplan")),
            SourceSpec::new(second.path().join(".forgeplan")),
        ];
        let task = (0..128)
            .map(|n| format!("term{n:04}"))
            .collect::<Vec<_>>()
            .join(" ");
        let base = PackOptions {
            max_millis: 1,
            ..PackOptions::default()
        };
        let control = super::pack_live(&specs, &task, &base, QueryMode::Strict, &json_size)
            .await
            .unwrap();
        assert!(
            control
                .traversal_limits
                .iter()
                .any(|limit| limit == "elapsed_work_limit")
        );
        for seed in ["ADR-999", "ADR-001"] {
            let options = PackOptions {
                seeds: vec![seed.into()],
                ..base.clone()
            };
            let result =
                super::pack_live(&specs, &task, &options, QueryMode::Strict, &json_size).await;
            assert!(
                matches!(result, Err(ConstellationError::InvalidPackRequest { .. })),
                "seed {seed} was silently ignored: {result:?}"
            );
        }
    }

    #[tokio::test]
    async fn many_matching_seeds_are_selected_once_when_also_explicit() {
        let ws = ws_root();
        for number in 100..180 {
            add_doc(&ws, &format!("ADR-{number}"), &[], "orbital");
        }
        let options = PackOptions {
            seeds: vec!["ADR-100".into(), "adr-100".into()],
            budget_tokens: 100_000,
            ..PackOptions::default()
        };
        let result = super::pack_live(
            &[SourceSpec::new(ws.path().join(".forgeplan"))],
            "orbital",
            &options,
            QueryMode::Strict,
            &json_size,
        )
        .await
        .unwrap();
        let identities: BTreeSet<_> = result.units.iter().map(|unit| &unit.unit_id).collect();
        assert_eq!(identities.len(), 80);
        assert_eq!(identities.len(), result.units.len());
    }

    #[tokio::test]
    async fn declared_closure_keeps_ground_refutation_and_historical_identity() {
        let ws = ws_root();
        add_doc(&ws, "ADR-010", &[("ADR-011", "based_on")], "orbital task");
        add_doc(&ws, "ADR-011", &[], "independent ground");
        add_doc(
            &ws,
            "ADR-012",
            &[("ADR-010", "informs")],
            "Verdict: refuted",
        );
        add_doc(
            &ws,
            "ADR-013",
            &[("ADR-010", "supersedes")],
            "newer decision",
        );
        let options = PackOptions {
            seeds: vec!["adr-10".into()],
            ..PackOptions::default()
        };
        let result = super::pack_live(
            &[SourceSpec::new(ws.path().join(".forgeplan"))],
            "unmatched query",
            &options,
            QueryMode::Strict,
            &json_size,
        )
        .await
        .unwrap();
        let ids: Vec<_> = result
            .units
            .iter()
            .map(|u| u.canonical_key.as_str())
            .collect();
        assert!(
            ids.contains(&"ADR-010")
                && ids.contains(&"ADR-011")
                && ids.contains(&"ADR-012")
                && ids.contains(&"ADR-013")
        );
        assert!(
            result
                .units
                .iter()
                .any(|u| u.canonical_key == "ADR-012" && u.excerpt.contains("refuted"))
        );
        assert!(result.graph.edges.iter().any(|e| e.relation == "supersedes"
            && e.from.canonical_key == "ADR-013"
            && e.to.as_ref().unwrap().canonical_key == "ADR-010"));
        assert!(result.traversal_complete);
    }

    #[tokio::test]
    async fn ambiguous_seeds_and_unadmitted_targets_do_not_cross_sources() {
        let a = ws_root();
        let b = ws_root();
        add_doc(
            &a,
            "ADR-010",
            &[("ADR-999@outside", "based_on")],
            "unique phrase",
        );
        let specs = [
            SourceSpec::new(a.path().join(".forgeplan")),
            SourceSpec::new(b.path().join(".forgeplan")),
        ];
        let ambiguous = PackOptions {
            seeds: vec!["ADR-001".into()],
            ..PackOptions::default()
        };
        assert!(
            super::pack_live(&specs, "none", &ambiguous, QueryMode::Strict, &json_size)
                .await
                .is_err()
        );
        let qualified = PackOptions {
            seeds: vec![format!(
                "adr-1@{}",
                super::super::federation::spec_source_id(&specs[0]).unwrap()
            )],
            ..PackOptions::default()
        };
        let selected = super::pack_live(
            &specs,
            "unmatched",
            &qualified,
            QueryMode::Strict,
            &json_size,
        )
        .await
        .unwrap();
        assert_eq!(selected.units.len(), 1);
        assert_eq!(
            selected.units[0].source.path,
            a.path()
                .join(".forgeplan")
                .canonicalize()
                .unwrap()
                .display()
                .to_string()
        );
        let options = PackOptions {
            seeds: vec!["ADR-010".into()],
            ..PackOptions::default()
        };
        let pack = super::pack_live(&specs, "none", &options, QueryMode::Strict, &json_size)
            .await
            .unwrap();
        assert!(!pack.traversal_complete);
        assert!(
            pack.graph
                .edges
                .iter()
                .any(|e| e.target == "ADR-999@outside" && e.to.is_none())
        );
        let strict = PackOptions {
            require_complete: true,
            ..options
        };
        assert!(
            super::pack_live(&specs, "none", &strict, QueryMode::Strict, &json_size)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn qualified_cross_source_ground_retains_both_endpoint_identities() {
        let a = ws_root();
        let b = ws_root();
        let specs = [
            SourceSpec::new(a.path().join(".forgeplan")),
            SourceSpec::new(b.path().join(".forgeplan")),
        ];
        let target_source = super::super::federation::spec_source_id(&specs[1]).unwrap();
        let target = format!("adr-1@{target_source}");
        add_doc(&a, "ADR-010", &[(&target, "based_on")], "start");
        let options = PackOptions {
            seeds: vec!["ADR-010".into()],
            ..PackOptions::default()
        };
        let pack = super::pack_live(&specs, "unmatched", &options, QueryMode::Strict, &json_size)
            .await
            .unwrap();
        let edge = pack
            .graph
            .edges
            .iter()
            .find(|e| e.relation == "based_on")
            .unwrap();
        assert_ne!(edge.from.source_id, edge.to.as_ref().unwrap().source_id);
        assert_eq!(edge.to.as_ref().unwrap().source_id, target_source);
        assert_eq!(edge.to.as_ref().unwrap().canonical_key, "ADR-001");
    }

    #[tokio::test]
    async fn cycle_depth_limit_is_reported_without_duplicate_units() {
        let ws = ws_root();
        add_doc(&ws, "ADR-010", &[("ADR-011", "based_on")], "first");
        add_doc(
            &ws,
            "ADR-011",
            &[("ADR-010", "based_on"), ("ADR-012", "based_on")],
            "second",
        );
        add_doc(&ws, "ADR-012", &[], "third");
        let options = PackOptions {
            seeds: vec!["ADR-010".into()],
            max_depth: 1,
            ..PackOptions::default()
        };
        let pack = super::pack_live(
            &[SourceSpec::new(ws.path().join(".forgeplan"))],
            "none",
            &options,
            QueryMode::Strict,
            &json_size,
        )
        .await
        .unwrap();
        assert!(!pack.traversal_complete);
        assert!(pack.traversal_limits.iter().any(|s| s == "depth_limit"));
        let ids: BTreeSet<_> = pack.units.iter().map(|u| &u.unit_id).collect();
        assert_eq!(ids.len(), pack.units.len());
    }

    #[tokio::test]
    async fn continuation_reconstructs_units_and_rejects_changed_basis() {
        let ws = ws_root();
        for id in ["ADR-010", "ADR-011", "ADR-012"] {
            add_doc(
                &ws,
                id,
                &[],
                &format!("orbital \"雪\" {}\n", "long ".repeat(500)),
            );
        }
        let specs = [SourceSpec::new(ws.path().join(".forgeplan"))];
        let mut options = PackOptions {
            budget_tokens: 100_000,
            byte_cap: 8000,
            ..PackOptions::default()
        };
        let first = super::pack_live(&specs, "orbital", &options, QueryMode::Strict, &json_size)
            .await
            .unwrap();
        let cursor = first
            .next_cursor
            .clone()
            .expect("fixture spans multiple pages");
        let changed_request = PackOptions {
            cursor: Some(cursor.clone()),
            ..options.clone()
        };
        assert!(matches!(
            super::pack_live(
                &specs,
                "different",
                &changed_request,
                QueryMode::Strict,
                &json_size
            )
            .await,
            Err(ConstellationError::IndexGenerationUnavailable { .. })
        ));
        let changed_options = PackOptions {
            cursor: Some(cursor.clone()),
            mermaid: true,
            ..options.clone()
        };
        assert!(matches!(
            super::pack_live(
                &specs,
                "orbital",
                &changed_options,
                QueryMode::Strict,
                &json_size
            )
            .await,
            Err(ConstellationError::IndexGenerationUnavailable { .. })
        ));
        let complete = PackOptions {
            require_complete: true,
            ..options.clone()
        };
        assert!(
            super::pack_live(&specs, "orbital", &complete, QueryMode::Strict, &json_size)
                .await
                .is_err()
        );
        let tiny = PackOptions {
            byte_cap: 200,
            ..options.clone()
        };
        assert!(matches!(
            super::pack_live(&specs, "orbital", &tiny, QueryMode::Strict, &json_size).await,
            Err(ConstellationError::InsufficientBudget { .. })
        ));
        let mut ids = BTreeSet::new();
        loop {
            let page = super::pack_live(&specs, "orbital", &options, QueryMode::Strict, &json_size)
                .await
                .unwrap();
            assert!(json_size(&page).unwrap() <= options.byte_cap);
            assert_eq!(page.receipt.measured_bytes, json_size(&page).unwrap());
            for unit in &page.units {
                assert!(ids.insert(unit.unit_id.clone()));
            }
            match page.next_cursor {
                Some(cursor) => {
                    assert!(!page.units.is_empty());
                    options.cursor = Some(cursor);
                }
                None => break,
            }
        }
        assert_eq!(ids.len(), 3);
        add_doc(&ws, "ADR-010", &[], "orbital changed");
        let stale = PackOptions {
            cursor: Some(cursor),
            ..options.clone()
        };
        assert!(matches!(
            super::pack_live(&specs, "orbital", &stale, QueryMode::Strict, &json_size).await,
            Err(ConstellationError::IndexGenerationUnavailable { .. })
        ));
        options.cursor = Some("1.invalid".into());
        assert!(
            super::pack_live(&specs, "orbital", &options, QueryMode::Strict, &json_size)
                .await
                .is_err()
        );
    }

    #[test]
    fn mermaid_keeps_unresolved_edges_and_escapes_target_labels() {
        let from = PackNode {
            source_id: "src:test".into(),
            canonical_key: "ADR-001".into(),
            display_id: "ADR-001".into(),
        };
        let graph = PackGraph {
            nodes: vec![from.clone()],
            edges: vec![PackEdge {
                from,
                to: None,
                target: "ADR-002@outside \"]\nattacker[\"fake\"]".into(),
                relation: "based_on".into(),
                provenance: "declared",
            }],
            scope: "page_incident",
            total_nodes: 1,
            total_edges: 1,
            complete: true,
        };
        let diagram = graph_mermaid(&graph);
        assert!(diagram.contains("unresolved: ADR-002@outside"));
        assert!(diagram.contains("|based_on|"));
        assert_eq!(
            diagram
                .lines()
                .filter(|line| line.contains("-.->|"))
                .count(),
            1
        );
        assert!(!diagram.contains("\nattacker["));
        assert!(!diagram.contains("outside \"]"));
    }

    #[tokio::test]
    async fn fixed_eval_corpus_pages_reconstruct_selected_graph() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../eval/corpus");
        let specs = [
            SourceSpec::new(root.join("ws-a/.forgeplan")),
            SourceSpec::new(root.join("ws-b/.forgeplan")),
        ];
        for (task, expected) in [
            (
                "drain queue reliability owner lock TTL retry policy ack lease",
                "PRD-001",
            ),
            (
                "binding predicate contract required fields epoch checkout incarnation",
                "SPEC-001",
            ),
        ] {
            let generous = PackOptions {
                budget_tokens: 100_000,
                ..PackOptions::default()
            };
            let full = super::pack_live(&specs, task, &generous, QueryMode::Strict, &json_size)
                .await
                .unwrap();
            let full_units: Vec<_> = full
                .units
                .iter()
                .map(|u| serde_json::to_string(u).unwrap())
                .collect();
            let full_nodes: BTreeSet<_> = full
                .graph
                .nodes
                .iter()
                .map(|n| serde_json::to_string(n).unwrap())
                .collect();
            let full_edges: BTreeSet<_> = full
                .graph
                .edges
                .iter()
                .map(|e| serde_json::to_string(e).unwrap())
                .collect();
            let mut options = PackOptions {
                budget_tokens: 2000,
                byte_cap: 8000,
                ..PackOptions::default()
            };
            let mut units = Vec::new();
            let mut nodes = BTreeSet::new();
            let mut edges = BTreeSet::new();
            let mut found = false;
            loop {
                let page = super::pack_live(&specs, task, &options, QueryMode::Strict, &json_size)
                    .await
                    .unwrap();
                assert!(!page.units.is_empty());
                assert_eq!(page.receipt.measured_bytes, json_size(&page).unwrap());
                assert!(page.receipt.measured_bytes <= 8000);
                assert_eq!(page.graph.scope, "page_incident");
                assert_eq!(page.graph.total_nodes, full.graph.total_nodes);
                assert_eq!(page.graph.total_edges, full.graph.total_edges);
                found |= page.units.iter().any(|u| u.canonical_key == expected);
                units.extend(page.units.iter().map(|u| serde_json::to_string(u).unwrap()));
                nodes.extend(
                    page.graph
                        .nodes
                        .iter()
                        .map(|n| serde_json::to_string(n).unwrap()),
                );
                edges.extend(
                    page.graph
                        .edges
                        .iter()
                        .map(|e| serde_json::to_string(e).unwrap()),
                );
                match page.next_cursor {
                    Some(cursor) => options.cursor = Some(cursor),
                    None => break,
                }
            }
            assert!(found);
            assert_eq!(units, full_units);
            assert_eq!(nodes, full_nodes);
            assert_eq!(edges, full_edges);
        }
    }

    #[test]
    fn snapshot_pack_uses_pinned_source_bytes_after_live_edit() {
        let ws = ws_root();
        git_in(ws.path(), &["init", "-q"]);
        git_in(ws.path(), &["config", "user.email", "test@example.invalid"]);
        git_in(ws.path(), &["config", "user.name", "test"]);
        git_in(ws.path(), &["add", "-A"]);
        git_in(ws.path(), &["commit", "-qm", "snapshot"]);
        let spec = SourceSpec::new(ws.path().join(".forgeplan"));
        let snapshot = super::super::snapshot::capture_snapshot(&[spec], None).unwrap();
        std::fs::write(
            ws.path().join(".forgeplan/adrs/ADR-001-auth.md"),
            AUTH_DOC.replace("token vault", "changed"),
        )
        .unwrap();
        let options = PackOptions {
            seeds: vec!["ADR-001".into()],
            ..PackOptions::default()
        };
        let pack = super::pack_snapshot(&snapshot, "unmatched", &options, &json_size).unwrap();
        assert_eq!(pack.units.len(), 1);
        assert!(pack.units[0].excerpt.contains("token vault"));
        assert!(!pack.units[0].excerpt.contains("changed"));
    }

    #[tokio::test]
    async fn unsupported_mode_is_refused_before_source_capture() {
        let options = PackOptions {
            retrieval_mode: "semantic".into(),
            ..PackOptions::default()
        };
        let result = super::pack_live(
            &[SourceSpec::new("/unavailable")],
            "task",
            &options,
            QueryMode::Strict,
            &json_size,
        )
        .await;
        assert!(matches!(
            result,
            Err(ConstellationError::InvalidPackRequest { .. })
        ));
    }
}
