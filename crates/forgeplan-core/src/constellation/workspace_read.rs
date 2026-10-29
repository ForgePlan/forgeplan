//! [`WorkspaceRead`] — the read adapter a migrated workspace is served through.
//!
//! A `LanceStore` is not a read model; it is a *second writer* that runs schema
//! migrations on every open. A read handler that reaches it therefore acquires
//! write-shaped side effects it never asked for — on a workspace that has been
//! cut over to Constellation, that means creating legacy tables next to the
//! published ones. This module is the alternative: one adapter over **one**
//! captured/pinned [`ReadView`], exposing exactly the read contracts the
//! documented handlers already call, with the same result shapes
//! ([`ArtifactRecord`], [`ListFilter`], relation triples).
//!
//! # What it is, and what it is not
//!
//! It is **not** a second store, a cache with its own lifecycle, or a fallback
//! that quietly reads somewhere else. It is a projection of the confirmed
//! capture: every record, edge and body byte it returns came from the same
//! capture the caller handed in, and the same capture the view tag was derived
//! from.
//!
//! # Records come from the capture, not from a table
//!
//! [`super::facade::ensure_read_view`] returns [`ReadView::Persisted`] only when
//! the capture's `source_set_digest` and `config_digest` equal the ones the
//! published descriptor names. So for a persisted view the capture *is* the
//! published source set, and serving records from it is serving that generation
//! — not a second, drifting read. An ephemeral view is the confirmed capture
//! itself, which is what a workspace with no pointer (or with moved bytes)
//! gets.
//!
//! # What a caller cannot do through it
//!
//! There is no `&mut` anything, no table handle, no owner, no
//! `WorkspaceLock`, and no maintenance capability reachable from this type. A
//! read handler can answer or fail; it cannot publish, migrate or create.
//!
//! # Identity
//!
//! A document's identity is its **canonical key** (ADR-030); the display id
//! (`PRD-074`) is a repeatable alias. Every lookup map in this adapter is keyed
//! by the canonical key, and the display id is resolved through the shared
//! algorithm in [`crate::artifact::canonical_key::resolve_artifact`] — the one
//! resolution order the specification froze. A display id two documents carry
//! names **neither** of them: [`Self::resolve_artifact`] reports that as
//! [`ResolveError::Ambiguous`] with every candidate, and the convenience
//! accessors below answer "not this one" rather than picking a winner.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::artifact::canonical_key::{
    ArtifactCandidate, ResolveError, ResolvedArtifact, resolve_artifact as resolve_shared,
};
use crate::artifact::store::ArtifactSummary;
use crate::db::store::{ArtifactRecord, ListFilter, compute_content_hash};
use crate::graph::knowledge::{ArtifactNode, KnowledgeGraph};
use crate::scoring::evidence::parse_evidence_from_record;
use crate::scoring::reff;

use super::capture::{CapturedSourceView, ParsedArtifact};
use super::view::ReadView;

/// What a declared relation target turned out to name.
///
/// The three outcomes stay apart: a target that names exactly one document, a
/// target that names none (a forward reference, preserved verbatim), and a
/// target that names several (an ambiguous reference — neither a forward
/// reference nor evidence for a score).
#[derive(Debug, Clone, PartialEq, Eq)]
enum EdgeTarget {
    /// The declared target resolved to exactly one document.
    Key(String),
    /// The declared target names no document in this capture.
    Unresolved,
    /// The declared target names more than one document.
    Ambiguous,
}

/// One outgoing edge of one document.
///
/// The declared target is kept verbatim: it is the document's own word, and
/// validation compares it against the frontmatter. The resolved key is what
/// the edge points at, and it is absent when the declared target named no
/// document or named several — an edge is never attributed to a winner.
#[derive(Debug, Clone)]
struct Edge {
    /// The target exactly as the frontmatter declared it.
    declared: String,
    /// The relation kind, exactly as declared.
    relation: String,
    /// What the declared target named.
    target: EdgeTarget,
}

impl Edge {
    /// The endpoint this edge points at: the canonical key when the target
    /// named exactly one document, the declared text otherwise.
    fn endpoint(&self) -> String {
        match &self.target {
            EdgeTarget::Key(key) => key.clone(),
            EdgeTarget::Unresolved | EdgeTarget::Ambiguous => self.declared.clone(),
        }
    }
}

/// Read-only view of one workspace, over one captured/pinned [`ReadView`].
///
/// Construct it through [`super::facade::open_workspace_read`] so the capture
/// and the view come from one call. The public constructor exists for callers
/// that already hold both halves — a test, or a server caching a capture across
/// two questions.
///
/// # Identity
///
/// A document's identity is its **canonical key** (ADR-030). The display id
/// (`PRD-074`) is a *repeatable alias*: two documents in one workspace may
/// carry the same one, and this adapter never picks a winner between them.
/// Every lookup map here is keyed by the canonical key; the display id is
/// resolved through the shared algorithm in
/// [`crate::artifact::canonical_key::resolve_artifact`], which reports an
/// ambiguity instead of a guess.
#[derive(Debug, Clone)]
pub struct WorkspaceRead {
    workspace: PathBuf,
    index_root: PathBuf,
    view: ReadView,
    captured: Arc<CapturedSourceView>,
    /// Every captured document, sorted by display id then canonical key — the
    /// store's `list_records` order, made total.
    records: Vec<ArtifactRecord>,
    /// Canonical key → index into `records`.
    ///
    /// A canonical key names exactly one document: the capture excludes every
    /// row whose key another row claims, so no key here has two claimants.
    by_key: HashMap<String, usize>,
    /// Lower-cased display alias → every index of `records` carrying it.
    ///
    /// A multimap on purpose: a display id is a repeatable alias, and two
    /// documents may carry the same one. The old slug map overwrote the first
    /// claimant with the second, which is how one of them disappeared.
    by_alias: HashMap<String, Vec<usize>>,
    /// Source canonical key → its outgoing edges.
    outgoing: BTreeMap<String, Vec<Edge>>,
    /// Target canonical key → the sources pointing at it, as
    /// `(source canonical key, relation)`.
    incoming: BTreeMap<String, Vec<(String, String)>>,
    /// One candidate per document — the document set the shared resolver reads.
    candidates: Vec<ArtifactCandidate>,
}

impl WorkspaceRead {
    /// Build the adapter from a capture and the view derived from it.
    pub fn from_parts(
        workspace: impl Into<PathBuf>,
        view: ReadView,
        captured: Arc<CapturedSourceView>,
    ) -> Self {
        let workspace = workspace.into();
        let index_root = workspace.join(super::facade::MaintainCapability::INDEX_DIR);

        // Sorted by canonical key first, so the edge indices below and the
        // record order that follows are both deterministic functions of the
        // capture — never of iteration order.
        let mut parsed: Vec<&ParsedArtifact> =
            captured.artifacts.iter().map(|a| a.as_ref()).collect();
        parsed.sort_by(|a, b| a.canonical_key.cmp(&b.canonical_key));

        // A record and the candidate that names it, kept together so the
        // candidate set always describes exactly the records above it.
        let mut entries: Vec<(ArtifactRecord, ArtifactCandidate)> = parsed
            .iter()
            .map(|artifact| {
                (
                    record_from_parsed(artifact),
                    ArtifactCandidate {
                        canonical_key: artifact.canonical_key.clone(),
                        display_id: artifact.display_id.clone(),
                        title: artifact.title.clone(),
                        relative_path: PathBuf::from(&artifact.relative_path),
                    },
                )
            })
            .collect();
        // The store's `list_records` order is by display id; the canonical key
        // breaks a tie, so two documents sharing a display id still have one
        // order.
        entries.sort_by(|(left, _), (right, _)| {
            left.id
                .cmp(&right.id)
                .then_with(|| left.canonical_key.cmp(&right.canonical_key))
        });
        let (mut records, candidates): (Vec<ArtifactRecord>, Vec<ArtifactCandidate>) =
            entries.into_iter().unzip();

        // Canonical key → the record claiming it, and display alias → every
        // record carrying it. The alias map is a multimap: a name that two
        // documents answer to is an ambiguity, and an index that keeps only one
        // of them is how a document disappears.
        let mut by_key: HashMap<String, usize> = HashMap::with_capacity(records.len());
        let mut by_alias: HashMap<String, Vec<usize>> = HashMap::with_capacity(records.len());
        for (index, record) in records.iter().enumerate() {
            by_key.entry(record.canonical_key.clone()).or_insert(index);
            by_alias
                .entry(record.id.to_ascii_lowercase())
                .or_default()
                .push(index);
        }

        // Edges: outgoing by source canonical key, incoming by target canonical
        // key. A declared target is resolved through the shared algorithm, so a
        // target that names no document — or names several — is attributed to
        // none of them.
        let mut outgoing: BTreeMap<String, Vec<Edge>> = BTreeMap::new();
        let mut incoming: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        let mut resolved_targets: HashMap<String, EdgeTarget> = HashMap::new();
        for artifact in &parsed {
            let mut edges: Vec<Edge> = Vec::with_capacity(artifact.outgoing.len());
            for edge in &artifact.outgoing {
                let target = resolved_targets
                    .entry(edge.target.clone())
                    .or_insert_with(|| resolve_edge_target(&candidates, &edge.target))
                    .clone();
                if let EdgeTarget::Key(target_key) = &target {
                    incoming
                        .entry(target_key.clone())
                        .or_default()
                        .push((artifact.canonical_key.clone(), edge.relation.clone()));
                }
                edges.push(Edge {
                    declared: edge.target.clone(),
                    relation: edge.relation.clone(),
                    target,
                });
            }
            outgoing.insert(artifact.canonical_key.clone(), edges);
        }
        for sources in incoming.values_mut() {
            sources.sort();
        }

        let scores = compute_r_eff_scores(&records, &outgoing, &incoming);
        for record in &mut records {
            if let Some(score) = scores.get(&record.canonical_key) {
                record.r_eff_score = *score;
            }
        }

        Self {
            workspace,
            index_root,
            view,
            captured,
            records,
            by_key,
            by_alias,
            outgoing,
            incoming,
            candidates,
        }
    }

    /// The `.forgeplan/` directory this view was captured from.
    pub fn workspace(&self) -> &Path {
        &self.workspace
    }

    /// The index root a vector channel reads at its pinned versions.
    pub fn index_root(&self) -> &Path {
        &self.index_root
    }

    /// The view this adapter serves: persisted (published generation) or
    /// ephemeral (confirmed capture).
    pub fn view(&self) -> &ReadView {
        &self.view
    }

    /// The capture every record, edge and body byte here came from.
    pub fn captured(&self) -> &CapturedSourceView {
        &self.captured
    }

    /// Whether a published generation backs this view.
    pub fn is_persisted(&self) -> bool {
        self.view.is_persisted()
    }

    /// Stable identity of the snapshot this view answers from.
    pub fn snapshot_id(&self) -> &str {
        self.view.snapshot_id()
    }

    /// Digest of the source set behind this view.
    pub fn source_set_digest(&self) -> &str {
        self.view.source_set_digest()
    }

    /// Every record, sorted by display id then canonical key.
    pub fn records(&self) -> &[ArtifactRecord] {
        &self.records
    }

    /// The document a reference names, by display id or canonical key.
    ///
    /// Resolution runs through the shared algorithm, so a canonical key, a
    /// display id and the normalised display form all reach the same document,
    /// and a reference two documents answer to reaches **neither**: this
    /// returns `None` rather than a winner. Use [`Self::resolve_artifact`] when
    /// the caller must tell "not found" from "ambiguous".
    pub fn get_record(&self, id: &str) -> Option<&ArtifactRecord> {
        // Two documents carrying one display id: the alias names neither.
        if self.alias_is_contested(id) {
            return None;
        }
        let key = self.resolve_key(id)?;
        self.get_record_by_key(&key)
    }

    /// The document with this canonical key, or `None`.
    ///
    /// The key is the identity, so this lookup is exact: one key, one
    /// document, no ambiguity to report.
    pub fn get_record_by_key(&self, canonical_key: &str) -> Option<&ArtifactRecord> {
        self.by_key.get(canonical_key).map(|&index| &self.records[index])
    }

    /// Every document carrying this display alias, in record order.
    ///
    /// The alias index is a multimap, so a shared alias keeps **every**
    /// claimant: the old slug map overwrote the first with the second, and
    /// one of the two documents disappeared. This is where the second one is
    /// still visible.
    ///
    /// The match is case-insensitive and exact on the display id; the
    /// normalised forms (`prd-74` for `PRD-074`) are the shared
    /// algorithm's business, and [`Self::resolve_artifact`] answers those.
    pub fn records_with_alias(&self, alias: &str) -> Vec<&ArtifactRecord> {
        self.by_alias
            .get(&alias.trim().to_ascii_lowercase())
            .map(|indices| indices.iter().map(|&index| &self.records[index]).collect())
            .unwrap_or_default()
    }

    /// Whether this display alias names more than one document.
    ///
    /// A contested alias names **neither** of its claimants: the convenience
    /// accessors answer "not this one" rather than picking a winner, and
    /// [`Self::resolve_artifact`] is where the caller learns why.
    fn alias_is_contested(&self, alias: &str) -> bool {
        self.by_alias
            .get(&alias.trim().to_ascii_lowercase())
            .is_some_and(|indices| indices.len() > 1)
    }

    /// Records matching a kind/status filter, sorted by id.
    pub fn list_records(&self, filter: Option<&ListFilter>) -> Vec<ArtifactRecord> {
        self.records
            .iter()
            .filter(|record| match filter {
                None => true,
                Some(f) => {
                    f.kind
                        .as_ref()
                        .is_none_or(|kind| kind.eq_ignore_ascii_case(&record.kind))
                        && f.status
                            .as_ref()
                            .is_none_or(|status| status.eq_ignore_ascii_case(&record.status))
                }
            })
            .cloned()
            .collect()
    }

    /// Summaries (no body), for the list/status surfaces.
    pub fn list_artifacts(&self, filter: Option<&ListFilter>) -> Vec<ArtifactSummary> {
        self.list_records(filter)
            .iter()
            .map(ArtifactRecord::to_summary)
            .collect()
    }

    /// Resolve a caller-supplied reference to the display id it names.
    ///
    /// The three paths the store resolver uses, in the same order: an exact
    /// canonical key, the display id in any accepted spelling, then the
    /// ADR-012 slug — which is the canonical key itself, so the third path is
    /// the first one. A reference two documents answer to resolves to `None`:
    /// the alias names neither of them.
    pub fn resolve_id(&self, input: &str) -> Option<String> {
        self.resolve_artifact(input)
            .ok()
            .flatten()
            .map(|resolved| resolved.identity.display_id)
    }

    /// Resolve a caller-supplied reference against this view's documents.
    ///
    /// This is the one resolution path: the shared algorithm, over exactly the
    /// documents this read saw. The outcomes stay apart —
    ///
    /// * `Ok(Some(resolved))` — exactly one document answers;
    /// * `Ok(None)` — no document answers;
    /// * [`ResolveError::Ambiguous`] — several documents answer by display
    ///   alias, and every candidate is named;
    /// * [`ResolveError::ConflictingCanonicalKey`] — several documents claim
    ///   one canonical key;
    /// * [`ResolveError::InvalidSelector`] — the reference is not usable;
    /// * [`ResolveError::IncompleteCapture`] — the capture could not be read
    ///   in full, so any answer would be partial.
    ///
    /// A partial document set answers nothing: a read that could not see every
    /// document cannot say which of them a name belongs to.
    pub fn resolve_artifact(&self, input: &str) -> Result<Option<ResolvedArtifact>, ResolveError> {
        if !self.captured.is_complete() {
            return Err(ResolveError::IncompleteCapture {
                message: self.incomplete_capture_message(),
            });
        }
        resolve_shared(input, &self.candidates)
    }

    /// The canonical key a reference names, when it names exactly one.
    fn resolve_key(&self, input: &str) -> Option<String> {
        self.resolve_artifact(input)
            .ok()
            .flatten()
            .map(|resolved| resolved.identity.canonical_key)
    }

    /// Why this capture could not be read in full.
    ///
    /// Workspace-relative paths only: an absolute path never leaves this
    /// adapter.
    fn incomplete_capture_message(&self) -> String {
        let coverage = &self.captured.coverage;
        let mut message = format!(
            "canonical source capture is incomplete: {} of {} enumerated paths produced no current row ({} invalid, {} unavailable, {} excluded)",
            coverage.expected.saturating_sub(coverage.read),
            coverage.expected,
            coverage.invalid,
            coverage.unavailable,
            coverage.excluded,
        );
        let unusable: Vec<&str> = self
            .captured
            .unusable
            .iter()
            .map(|source| source.relative_path.as_str())
            .collect();
        if !unusable.is_empty() {
            message.push_str("; unusable: ");
            message.push_str(&unusable.join(", "));
        }
        message
    }

    /// Outgoing edges of one artifact: `(target, relation)`, sorted.
    ///
    /// The target is the canonical key of the document the declared target
    /// names; a target that names no document — or names several — keeps its
    /// declared text, so an unresolved reference is preserved rather than
    /// dropped.
    pub fn get_relations(&self, id: &str) -> Vec<(String, String)> {
        // Two documents carrying one display id: the alias names neither.
        if self.alias_is_contested(id) {
            return Vec::new();
        }
        match self.resolve_key(id) {
            Some(key) => self.get_relations_by_key(&key),
            None => Vec::new(),
        }
    }

    /// Outgoing edges of the document with this canonical key:
    /// `(target, relation)`, sorted.
    pub fn get_relations_by_key(&self, canonical_key: &str) -> Vec<(String, String)> {
        self.outgoing
            .get(canonical_key)
            .map(|edges| {
                edges
                    .iter()
                    .map(|edge| (edge.endpoint(), edge.relation.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Incoming edges of one artifact: `(source, relation)`, sorted.
    ///
    /// The source is the canonical key of the document the edge comes from, so
    /// two documents sharing a display id stay two sources.
    pub fn get_incoming_relations(&self, id: &str) -> Vec<(String, String)> {
        // Two documents carrying one display id: the alias names neither.
        if self.alias_is_contested(id) {
            return Vec::new();
        }
        match self.resolve_key(id) {
            Some(key) => self.incoming.get(&key).cloned().unwrap_or_default(),
            None => Vec::new(),
        }
    }

    /// Every edge as `(source key, target, relation)`, sorted by source.
    ///
    /// The source is the canonical key — the identity — so two documents
    /// sharing a display id contribute two edges, not one merged edge.
    pub fn get_all_relations(&self) -> Vec<(String, String, String)> {
        let mut out: Vec<(String, String, String)> = Vec::new();
        for (source, edges) in &self.outgoing {
            for edge in edges {
                out.push((source.clone(), edge.endpoint(), edge.relation.clone()));
            }
        }
        out.sort();
        out
    }

    /// Case-insensitive substring match over title and body.
    ///
    /// Same scan-the-corpus semantics as the store: no index, no ranking, and a
    /// `kind` filter applied before the match.
    pub fn search_body(&self, query: &str, kind_filter: Option<&str>) -> Vec<ArtifactRecord> {
        let needle = query.to_lowercase();
        self.records
            .iter()
            .filter(|record| {
                kind_filter.is_none_or(|kind| kind.eq_ignore_ascii_case(&record.kind))
                    && (record.title.to_lowercase().contains(&needle)
                        || record.body.to_lowercase().contains(&needle))
            })
            .cloned()
            .collect()
    }

    /// Artifacts whose `valid_until` has passed, earliest expiry first.
    pub fn find_stale(&self) -> Vec<ArtifactRecord> {
        let today = chrono::Utc::now().date_naive();
        let mut stale: Vec<ArtifactRecord> = self
            .records
            .iter()
            .filter(|record| {
                record.valid_until.as_ref().is_some_and(|vu| {
                    chrono::NaiveDate::parse_from_str(vu, "%Y-%m-%d")
                        .or_else(|_| {
                            chrono::DateTime::parse_from_rfc3339(vu).map(|dt| dt.date_naive())
                        })
                        .map(|date| date < today)
                        .unwrap_or(false)
                })
            })
            .cloned()
            .collect();
        stale.sort_by(|a, b| a.valid_until.cmp(&b.valid_until));
        stale
    }

    /// #446 parity — [`ArtifactRecord::frontmatter_map`] plus this artifact's
    /// real outgoing edges as a `links` sequence.
    ///
    /// The same map [`crate::db::store::LanceStore::frontmatter_map_with_links`]
    /// builds, from the same shared helper, fed with the edges THIS capture
    /// observed. Validation rules that compare against
    /// `extract_frontmatter_link_targets(fm)` therefore see identical input on
    /// both layouts — a rule that fired only on the legacy projection would be
    /// a workspace-layout bug wearing a methodology rule's clothes.
    ///
    /// The links carry the *declared* target text, not the resolved key: the
    /// frontmatter is the document's own word, and that is what the rules
    /// compare against.
    pub fn frontmatter_map_with_links(
        &self,
        record: &ArtifactRecord,
    ) -> BTreeMap<String, serde_yaml::Value> {
        let relations: Vec<(String, String)> = self
            .outgoing
            .get(&record.canonical_key)
            .map(|edges| {
                edges
                    .iter()
                    .map(|edge| (edge.declared.clone(), edge.relation.clone()))
                    .collect()
            })
            .unwrap_or_default();
        crate::db::store::frontmatter_map_with_links_from(record, &relations)
    }

    /// The graph over exactly this view's records and edges.
    ///
    /// Built from parts rather than from a store, so graph expansion in search
    /// can never walk an artifact this read did not see.
    ///
    /// The node id is the **canonical key** — the identity — so two documents
    /// carrying one display id stay two vertices instead of collapsing into one.
    /// The label a renderer shows (display id and title) lives on the record
    /// this read returns; [`Self::get_record_by_key`] joins the two.
    pub fn knowledge_graph(&self) -> KnowledgeGraph {
        let nodes: Vec<ArtifactNode> = self
            .records
            .iter()
            .map(|record| ArtifactNode {
                id: record.canonical_key.clone(),
                label: ArtifactNode::label_for(&record.id, &record.title),
                kind: record.kind.clone(),
                status: record.status.clone(),
            })
            .collect();
        let mut edges: Vec<(String, String, String)> = Vec::new();
        for (source, source_edges) in &self.outgoing {
            for edge in source_edges {
                if let EdgeTarget::Key(target) = &edge.target {
                    edges.push((source.clone(), target.clone(), edge.relation.clone()));
                }
            }
        }
        KnowledgeGraph::from_parts(nodes, edges)
    }
}

/// What a declared relation target names, resolved through the shared
/// algorithm.
///
/// The shared resolver is the only place that decides which document a name
/// belongs to; this maps its verdict onto the three states an edge can be in.
fn resolve_edge_target(candidates: &[ArtifactCandidate], declared: &str) -> EdgeTarget {
    match resolve_shared(declared, candidates) {
        Ok(Some(resolved)) => EdgeTarget::Key(resolved.identity.canonical_key),
        Ok(None) => EdgeTarget::Unresolved,
        // An ambiguous reference is neither a forward reference nor a link.
        Err(ResolveError::Ambiguous { .. }) => EdgeTarget::Ambiguous,
        // A qualified reference or an unusable string names nothing here.
        Err(_) => EdgeTarget::Unresolved,
    }
}

/// Convert one captured artifact into the record shape the read contracts
/// return.
///
/// This is the single conversion point, deliberately: a second one is how a read
/// path starts disagreeing with itself about what a body is.
///
/// The body keeps its **frontmatter**. The store's `body` column holds the
/// whole markdown file, and the documented consumers parse frontmatter out of it
/// (`refs_form_from_body`, the identity triple). Returning only the
/// post-frontmatter body would silently drop every slug and identity field.
pub fn record_from_parsed(artifact: &ParsedArtifact) -> ArtifactRecord {
    let body = String::from_utf8_lossy(&artifact.raw_bytes).into_owned();
    let body_hash = Some(compute_content_hash(&artifact.title, &body));
    ArtifactRecord {
        canonical_key: artifact.canonical_key.clone(),
        id: artifact.display_id.clone(),
        kind: artifact.kind.clone(),
        status: artifact.status.clone(),
        title: artifact.title.clone(),
        body,
        depth: artifact.depth.clone().unwrap_or_default(),
        author: artifact.author.clone(),
        parent_epic: artifact.parent_epic.clone(),
        r_eff_score: 0.0,
        valid_until: artifact.valid_until.clone(),
        created_at: artifact.created_at.clone().unwrap_or_default(),
        updated_at: artifact.updated_at.clone().unwrap_or_default(),
        tags: artifact.tags.clone(),
        body_hash,
        // No vector reaches a read through the capture: vectors live in the
        // pinned tables, and a record claiming one here would be claiming a
        // binding nothing in this view verified.
        embedding: None,
    }
}

/// Pure R_eff over this view's records and edges.
///
/// The capture does not carry the store's *cached* score — that cache is a
/// write-side projection, and reading it back would mean reading a second
/// source. So the read computes the weakest link from the same bytes: the
/// evidence packs linked in either direction. Terminal packs are excluded by
/// [`reff::r_eff`] itself (ADR-020), so this function only decides *which*
/// packs count as linked.
///
/// The score map is keyed by the canonical key: a score belongs to a document's
/// identity, and two documents sharing a display id get two scores.
fn compute_r_eff_scores(
    records: &[ArtifactRecord],
    outgoing: &BTreeMap<String, Vec<Edge>>,
    incoming: &BTreeMap<String, Vec<(String, String)>>,
) -> BTreeMap<String, f64> {
    let evidence: Vec<&ArtifactRecord> = records
        .iter()
        .filter(|record| record.kind.eq_ignore_ascii_case("evidence"))
        .collect();
    let pack_of = |key: &str| evidence.iter().copied().find(|pack| pack.canonical_key == key);

    let mut scores = BTreeMap::new();
    for record in records {
        let mut linked: Vec<&ArtifactRecord> = Vec::new();
        if let Some(edges) = outgoing.get(&record.canonical_key) {
            for edge in edges {
                if let EdgeTarget::Key(target) = &edge.target
                    && let Some(pack) = pack_of(target)
                {
                    linked.push(pack);
                }
            }
        }
        if let Some(sources) = incoming.get(&record.canonical_key) {
            for (source, _) in sources {
                if let Some(pack) = pack_of(source) {
                    linked.push(pack);
                }
            }
        }
        let items: Vec<reff::EvidenceItem> =
            linked.into_iter().map(parse_evidence_from_record).collect();
        scores.insert(record.canonical_key.clone(), reff::r_eff(&items));
    }
    scores
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constellation::capture::{CaptureLimits, CaptureRequest, capture_source};
    use crate::workspace::ARTIFACT_DIRS;
    use tempfile::TempDir;

    /// A workspace with one linked PRD → Evidence pair and an unrelated RFC.
    ///
    /// The link is declared from the PRD's side, so the outgoing index and the
    /// incoming index are built from different artifacts — a test that only
    /// checked one direction would pass on a half-built index.
    fn workspace() -> TempDir {
        let tmp = TempDir::new().unwrap();
        for dir in ARTIFACT_DIRS {
            std::fs::create_dir_all(tmp.path().join(dir)).unwrap();
        }
        std::fs::write(
            tmp.path().join("prds").join("PRD-001-auth.md"),
            "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\nslug: prd-auth-system\ndepth: standard\ntags:\n  - security\nlinks:\n  - target: EVID-001\n    relation: informs\n---\n\n## Summary\n\nAuth rewrite.\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("evidence").join("EVID-001-stress.md"),
            "---\nid: EVID-001\nkind: evidence\nstatus: active\ntitle: Stress run\ndepth: standard\n---\n\nverdict: supports\ncongruence_level: 3\nevidence_type: measurement\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("rfcs").join("RFC-002-wire.md"),
            "---\nid: RFC-002\nkind: rfc\nstatus: active\ntitle: Wire format\ndepth: standard\n---\n\n## Summary\n\nUnrelated.\n",
        )
        .unwrap();
        tmp
    }

    /// A workspace where two documents carry the **same** display id.
    ///
    /// Their canonical keys differ — the slug is the key — so neither row is a
    /// collision and both survive the capture. The display id is the alias they
    /// share, and it is the alias that must not pick a winner.
    fn workspace_with_duplicate_display_id() -> TempDir {
        let tmp = TempDir::new().unwrap();
        for dir in ARTIFACT_DIRS {
            std::fs::create_dir_all(tmp.path().join(dir)).unwrap();
        }
        std::fs::write(
            tmp.path().join("prds").join("PRD-001-alpha.md"),
            "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Alpha\nslug: prd-alpha\n---\n\nAlpha.\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("prds").join("PRD-001-beta.md"),
            "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Beta\nslug: prd-beta\n---\n\nBeta.\n",
        )
        .unwrap();
        tmp
    }

    /// A workspace whose capture cannot be read in full.
    ///
    /// One valid document and one unparsable sibling: the capture counts the
    /// broken file as invalid, so the document set is partial.
    fn workspace_with_unreadable_source() -> TempDir {
        let tmp = TempDir::new().unwrap();
        for dir in ARTIFACT_DIRS {
            std::fs::create_dir_all(tmp.path().join(dir)).unwrap();
        }
        std::fs::write(
            tmp.path().join("prds").join("PRD-001-auth.md"),
            "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\nslug: prd-auth-system\n---\n\nAuth rewrite.\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("prds").join("PRD-002-broken.md"),
            "---\nid: [unclosed\n",
        )
        .unwrap();
        tmp
    }

    async fn read_of(root: &Path) -> WorkspaceRead {
        let captured = Arc::new(
            capture_source(&CaptureRequest::new(root), CaptureLimits::default())
                .await
                .unwrap(),
        );
        WorkspaceRead::from_parts(
            root,
            ReadView::from_capture(Arc::clone(&captured)),
            captured,
        )
    }

    #[tokio::test]
    async fn a_read_returns_the_captured_bodies_and_frontmatter() {
        let tmp = workspace();
        let read = read_of(tmp.path()).await;

        let record = read.get_record("PRD-001").expect("PRD-001 is captured");
        assert_eq!(record.kind, "prd");
        assert_eq!(record.status, "draft");
        assert_eq!(record.tags, vec!["security".to_string()]);
        assert_eq!(record.depth, "standard");
        // The body keeps its frontmatter: consumers parse slug and refs out of
        // it, so a post-frontmatter-only body would silently drop both.
        assert!(
            record.body.starts_with("---") && record.body.contains("slug: prd-auth-system"),
            "body must retain frontmatter, got {:?}",
            record.body
        );
        assert!(
            record.embedding.is_none(),
            "a read must not invent a vector"
        );

        assert_eq!(
            read.get_record("EVID-001")
                .map(|record| record.title.as_str()),
            Some("Stress run")
        );
        assert_eq!(
            read.get_record("RFC-002")
                .map(|record| record.title.as_str()),
            Some("Wire format")
        );
        for missing in ["A-000", "PRD-002", "ZZZ-999"] {
            assert!(
                read.get_record(missing).is_none(),
                "unexpected record for {missing}"
            );
        }
    }

    #[tokio::test]
    async fn resolve_id_accepts_display_forms_and_slugs_and_rejects_unknown_refs() {
        let tmp = workspace();
        let read = read_of(tmp.path()).await;

        assert_eq!(read.resolve_id("PRD-001").as_deref(), Some("PRD-001"));
        // Normalized display form: `prd-1` → `PRD-001`.
        assert_eq!(read.resolve_id("prd-1").as_deref(), Some("PRD-001"));
        assert_eq!(read.resolve_id("prd-001").as_deref(), Some("PRD-001"));
        // ADR-012 slugs retain the legacy lowercase-only input shape.
        assert_eq!(
            read.resolve_id("prd-auth-system").as_deref(),
            Some("PRD-001")
        );
        assert_eq!(read.resolve_id("PRD-AUTH-SYSTEM"), None);
        assert_eq!(
            read.resolve_id("  RFC-002  ").as_deref(),
            Some("RFC-002"),
            "surrounding whitespace is trimmed, so the direct id path still matches"
        );
        assert_eq!(
            read.resolve_id("rfc-2").as_deref(),
            Some("RFC-002"),
            "a display-id shape normalises regardless of which kind the prefix names"
        );
        // `prd-2` normalises to `PRD-002`, which this fixture does not have.
        // It must NOT fall through to the slug scan and bind to PRD-001.
        assert_eq!(read.resolve_id("prd-2").as_deref(), None);
        assert_eq!(read.resolve_id("prd-999").as_deref(), None);
        assert_eq!(read.resolve_id("nope-nothing").as_deref(), None);
        assert_eq!(read.resolve_id("   ").as_deref(), None);
    }

    #[tokio::test]
    async fn relations_are_exact_in_both_directions() {
        let tmp = workspace();
        let read = read_of(tmp.path()).await;

        assert_eq!(
            read.get_relations("PRD-001"),
            vec![("EVID-001".to_string(), "informs".to_string())]
        );
        assert_eq!(
            read.get_incoming_relations("EVID-001"),
            vec![("prd-auth-system".to_string(), "informs".to_string())],
            "the source is the canonical key: the identity, not the alias"
        );
        assert!(read.get_relations("RFC-002").is_empty());
        assert!(read.get_incoming_relations("PRD-001").is_empty());
        assert!(read.get_relations("NO-SUCH-ID").is_empty());

        assert_eq!(
            read.get_all_relations(),
            vec![(
                "prd-auth-system".to_string(),
                "EVID-001".to_string(),
                "informs".to_string()
            )],
            "the edge set is reported with canonical keys, once, sorted"
        );
    }

    #[tokio::test]
    async fn filters_narrow_by_kind_and_status_without_changing_order() {
        let tmp = workspace();
        let read = read_of(tmp.path()).await;

        assert_eq!(
            read.list_records(None)
                .iter()
                .map(|r| r.id.as_str())
                .collect::<Vec<_>>(),
            vec!["EVID-001", "PRD-001", "RFC-002"],
            "records are sorted by id, deterministically"
        );

        let drafts = read.list_records(Some(&ListFilter {
            kind: None,
            status: Some("draft".into()),
        }));
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].id, "PRD-001");

        let rfcs = read.list_records(Some(&ListFilter {
            kind: Some("RFC".into()),
            status: None,
        }));
        assert_eq!(rfcs.len(), 1, "kind matching is case-insensitive");
        assert_eq!(rfcs[0].id, "RFC-002");

        assert!(
            read.list_records(Some(&ListFilter {
                kind: Some("prd".into()),
                status: Some("active".into()),
            }))
            .is_empty(),
            "kind and status compose as AND"
        );
    }

    #[tokio::test]
    async fn r_eff_comes_from_the_linked_packs_in_this_view() {
        let tmp = workspace();
        let read = read_of(tmp.path()).await;

        let prd = read.get_record("PRD-001").unwrap();
        assert!(
            prd.r_eff_score > 0.0,
            "a PRD linked to a `supports`/CL3 pack must score above 0, got {}",
            prd.r_eff_score
        );

        let rfc = read.get_record("RFC-002").unwrap();
        assert_eq!(
            rfc.r_eff_score, 0.0,
            "an artifact with no linked evidence has no weakest link to report"
        );
    }

    /// The ranking layer's `HasEvidence` / `NoEvidence` filters read
    /// `r_eff_score`, so a view whose scores were left at the struct default
    /// would answer every evidence filter wrongly — silently, because a
    /// filtered corpus legitimately returns fewer rows.
    #[tokio::test]
    async fn the_evidence_filters_see_the_computed_scores() {
        use crate::search::filter::ArtifactFilter as SearchFilter;

        let tmp = workspace();
        let read = read_of(tmp.path()).await;

        let linked = read.get_record("PRD-001").unwrap();
        assert!(SearchFilter::HasEvidence.matches(linked));
        let unlinked = read.get_record("RFC-002").unwrap();
        assert!(SearchFilter::NoEvidence.matches(unlinked));
    }

    #[tokio::test]
    async fn search_body_matches_title_and_body_case_insensitively() {
        let tmp = workspace();
        let read = read_of(tmp.path()).await;

        let by_title = read.search_body("WIRE FORMAT", None);
        assert_eq!(by_title.len(), 1);
        assert_eq!(by_title[0].id, "RFC-002");

        let by_body = read.search_body("auth rewrite", None);
        assert_eq!(by_body.len(), 1);
        assert_eq!(by_body[0].id, "PRD-001");

        let kind_scoped = read.search_body("Auth", Some("rfc"));
        assert!(
            kind_scoped.is_empty(),
            "the kind filter applies before the match, not after"
        );
    }

    #[tokio::test]
    async fn find_stale_reports_only_expired_valid_until_values() {
        let tmp = TempDir::new().unwrap();
        for dir in ARTIFACT_DIRS {
            std::fs::create_dir_all(tmp.path().join(dir)).unwrap();
        }
        std::fs::write(
            tmp.path().join("adrs").join("ADR-001-old.md"),
            "---\nid: ADR-001\nkind: adr\nstatus: active\ntitle: Old\nvalid_until: 2000-01-01\n---\n\nBody.\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("adrs").join("ADR-002-fresh.md"),
            "---\nid: ADR-002\nkind: adr\nstatus: active\ntitle: Fresh\nvalid_until: 2999-01-01\n---\n\nBody.\n",
        )
        .unwrap();
        let read = read_of(tmp.path()).await;

        let stale = read.find_stale();
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0].id, "ADR-001");
    }

    #[tokio::test]
    async fn the_graph_covers_exactly_this_views_records_and_edges() {
        let tmp = workspace();
        let read = read_of(tmp.path()).await;

        let graph = read.knowledge_graph();
        // One 1-hop neighbour on each side of the single declared edge; the
        // RFC has none in either direction. The node id is the canonical key.
        assert_eq!(graph.neighbors_with_relations("prd-auth-system").len(), 1);
        assert_eq!(graph.neighbors_with_relations("EVID-001").len(), 1);
        assert!(graph.neighbors_with_relations("RFC-002").is_empty());
    }

    #[tokio::test]
    async fn an_ephemeral_view_reports_no_published_generation() {
        let tmp = workspace();
        let read = read_of(tmp.path()).await;

        assert!(!read.is_persisted(), "no CURRENT pointer exists here");
        assert_eq!(read.snapshot_id(), read.captured().snapshot_id);
        assert_eq!(read.source_set_digest(), read.captured().source_set_digest);
    }

    /// A document is addressed by its canonical key, whatever the display id
    /// says.
    #[tokio::test]
    async fn a_document_is_retrievable_by_its_canonical_key() {
        let tmp = workspace();
        let read = read_of(tmp.path()).await;

        let by_key = read
            .get_record_by_key("prd-auth-system")
            .expect("the canonical key names the document");
        assert_eq!(by_key.title, "Auth");
        assert_eq!(
            by_key.id, "PRD-001",
            "the key names the document; the display id stays its alias"
        );
        assert_eq!(
            read.get_record_by_key("EVID-001")
                .map(|record| record.title.as_str()),
            Some("Stress run"),
            "a legacy document whose key is its display id resolves the same way"
        );
        assert!(
            read.get_record_by_key("PRD-001").is_none(),
            "a display id is not a canonical key: the two identities stay apart"
        );
        assert!(read.get_record_by_key("prd-missing").is_none());
    }

    /// Two documents sharing one display id stay two documents: the graph
    /// keeps two vertices, and the alias names neither of them.
    #[tokio::test]
    async fn two_documents_sharing_a_display_id_stay_two_vertices() {
        let tmp = workspace_with_duplicate_display_id();
        let read = read_of(tmp.path()).await;

        let graph = read.knowledge_graph();
        assert_eq!(
            graph.node_count(),
            2,
            "the node id is the canonical key, so the shared display id cannot merge them"
        );
        assert!(
            graph.get("prd-alpha").is_some() && graph.get("prd-beta").is_some(),
            "each document keeps its own vertex, keyed by its own key"
        );
        assert!(
            graph.get("PRD-001").is_none(),
            "the shared display id is an alias, never a node id"
        );

        // Both records are still there: the alias did not overwrite one.
        assert_eq!(
            read.records()
                .iter()
                .map(|record| record.canonical_key.as_str())
                .collect::<Vec<_>>(),
            vec!["prd-alpha", "prd-beta"]
        );
        assert_eq!(read.records().iter().filter(|r| r.id == "PRD-001").count(), 2);
    }

    /// A duplicate alias is an ambiguity with both candidates named — never a
    /// silent winner, and never an overwrite.
    #[tokio::test]
    async fn a_duplicate_alias_yields_the_ambiguity_not_a_winner() {
        let tmp = workspace_with_duplicate_display_id();
        let read = read_of(tmp.path()).await;

        let error = read
            .resolve_artifact("PRD-001")
            .expect_err("a shared display id is an ambiguity, not a winner");
        match error {
            ResolveError::Ambiguous { input, candidates } => {
                assert_eq!(input, "PRD-001");
                assert_eq!(
                    candidates.len(),
                    2,
                    "both claimants must be named, or the ambiguity hides one"
                );
                let keys: Vec<&str> = candidates
                    .iter()
                    .map(|candidate| candidate.canonical_key.as_str())
                    .collect();
                assert!(keys.contains(&"prd-alpha") && keys.contains(&"prd-beta"));
            }
            other => panic!("expected an ambiguity, got {other:?}"),
        }

        assert!(
            read.get_record("PRD-001").is_none(),
            "the convenience accessor must not pick one of the two claimants"
        );
        assert!(
            read.get_relations("PRD-001").is_empty(),
            "and it must not attribute one claimant's edges to the alias"
        );
        assert!(
            read.get_record_by_key("prd-alpha").is_some()
                && read.get_record_by_key("prd-beta").is_some(),
            "each document is still addressable by its own key"
        );
    }

    /// Absence and ambiguity are different answers.
    #[tokio::test]
    async fn absence_is_distinct_from_ambiguity() {
        let tmp = workspace_with_duplicate_display_id();
        let read = read_of(tmp.path()).await;

        let absent = read
            .resolve_artifact("PRD-999")
            .expect("a missing reference is not an error");
        assert!(
            absent.is_none(),
            "no document answers a missing reference"
        );
        assert!(
            read.resolve_artifact("PRD-001").is_err(),
            "a shared alias is an error, not an absence"
        );
    }

    /// A partial document set answers nothing: the capture could not see every
    /// document, so any answer would be partial.
    #[tokio::test]
    async fn an_incomplete_capture_refuses_to_answer() {
        let tmp = workspace_with_unreadable_source();
        let read = read_of(tmp.path()).await;

        assert!(
            !read.captured().is_complete(),
            "the fixture must actually be an incomplete capture"
        );
        let error = read
            .resolve_artifact("PRD-001")
            .expect_err("a partial document set must not answer");
        match error {
            ResolveError::IncompleteCapture { message } => {
                assert!(
                    message.contains("incomplete"),
                    "the message must say what is missing, got {message:?}"
                );
                assert!(
                    !message.contains(tmp.path().to_string_lossy().as_ref()),
                    "an absolute path must never leave this adapter"
                );
            }
            other => panic!("expected an incomplete capture, got {other:?}"),
        }
    }

    /// A complete capture resolves normally.
    #[tokio::test]
    async fn a_complete_capture_resolves_normally() {
        let tmp = workspace();
        let read = read_of(tmp.path()).await;

        assert!(read.captured().is_complete());
        let resolved = read
            .resolve_artifact("prd-auth-system")
            .expect("a complete capture resolves")
            .expect("the key names a document");
        assert_eq!(resolved.identity.canonical_key, "prd-auth-system");
        assert_eq!(resolved.identity.display_id, "PRD-001");
        assert!(
            resolved.relative_path.is_relative(),
            "the resolved path stays workspace-relative"
        );
    }
}
