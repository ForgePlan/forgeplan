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

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::artifact::store::ArtifactSummary;
use crate::db::store::{ArtifactRecord, ListFilter, compute_content_hash};
use crate::graph::knowledge::{ArtifactNode, KnowledgeGraph};
use crate::scoring::evidence::parse_evidence_from_record;
use crate::scoring::reff;

use super::capture::{CapturedSourceView, ParsedArtifact};
use super::view::ReadView;

/// Read-only view of one workspace, over one captured/pinned [`ReadView`].
///
/// Construct it through [`super::facade::open_workspace_read`] so the capture
/// and the view come from one call. The public constructor exists for callers
/// that already hold both halves — a test, or a server caching a capture across
/// two questions.
#[derive(Debug, Clone)]
pub struct WorkspaceRead {
    workspace: PathBuf,
    index_root: PathBuf,
    view: ReadView,
    captured: Arc<CapturedSourceView>,
    /// Sorted by `id`, exactly as the store's `list_records` is.
    records: Vec<ArtifactRecord>,
    /// `id` (display form) → index into `records`.
    by_id: HashMap<String, usize>,
    /// Lower-cased ADR-012 slug → canonical display id.
    by_slug: HashMap<String, String>,
    /// Source display id → outgoing `(target, relation)`, sorted.
    outgoing: BTreeMap<String, Vec<(String, String)>>,
    /// Target id → incoming `(source, relation)`, sorted.
    incoming: BTreeMap<String, Vec<(String, String)>>,
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

        let mut outgoing: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        let mut incoming: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        for artifact in &parsed {
            let edges: Vec<(String, String)> = artifact
                .outgoing
                .iter()
                .map(|edge| (edge.target.clone(), edge.relation.clone()))
                .collect();
            for (target, relation) in &edges {
                incoming
                    .entry(target.clone())
                    .or_default()
                    .push((artifact.display_id.clone(), relation.clone()));
            }
            outgoing.insert(artifact.display_id.clone(), edges);
        }
        for sources in incoming.values_mut() {
            sources.sort();
        }

        let mut records: Vec<ArtifactRecord> =
            parsed.iter().map(|a| record_from_parsed(a)).collect();
        let scores = compute_r_eff_scores(&records, &outgoing, &incoming);
        for record in &mut records {
            if let Some(score) = scores.get(&record.id) {
                record.r_eff_score = *score;
            }
        }
        records.sort_by(|a, b| a.id.cmp(&b.id));

        let mut by_id = HashMap::with_capacity(records.len());
        for (index, record) in records.iter().enumerate() {
            by_id.insert(record.id.clone(), index);
        }
        let mut by_slug = HashMap::with_capacity(parsed.len());
        for artifact in &parsed {
            if let Some(slug) = &artifact.identity.slug {
                by_slug.insert(slug.to_ascii_lowercase(), artifact.display_id.clone());
            }
        }

        Self {
            workspace,
            index_root,
            view,
            captured,
            records,
            by_id,
            by_slug,
            outgoing,
            incoming,
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

    /// Every record, sorted by id.
    pub fn records(&self) -> &[ArtifactRecord] {
        &self.records
    }

    /// `id` (display form) → record, or `None`.
    ///
    /// Byte-exact, like the store's `id = '…'` predicate: a normalized display
    /// id or a slug goes through [`Self::resolve_id`] first.
    pub fn get_record(&self, id: &str) -> Option<&ArtifactRecord> {
        self.by_id.get(id).map(|index| &self.records[*index])
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

    /// Resolve a caller-supplied reference to a canonical display id.
    ///
    /// Same three paths the store resolver uses, in the same order: a literal id
    /// match, the `KIND-N` display form, then the ADR-012 slug. A display-id
    /// shape whose kind is real but which matches nothing does **not** fall
    /// through to the slug scan — such an input cannot be a slug.
    pub fn resolve_id(&self, input: &str) -> Option<String> {
        let input = input.trim();
        if input.is_empty() {
            return None;
        }
        if self.by_id.contains_key(input) {
            return Some(input.to_string());
        }

        if let Some((prefix, rest)) = input.split_once('-')
            && !prefix.is_empty()
            && !rest.is_empty()
            && rest.chars().all(|c| c.is_ascii_digit())
            && prefix.chars().all(|c| c.is_ascii_alphabetic())
            && let Ok(number) = rest.parse::<u32>()
            && crate::artifact::types::ArtifactKind::from_slug_prefix(&prefix.to_ascii_lowercase())
                .is_some()
        {
            let normalized = format!("{}-{number:03}", prefix.to_ascii_uppercase());
            return self.by_id.contains_key(&normalized).then_some(normalized);
        }

        self.by_slug.get(&input.to_ascii_lowercase()).cloned()
    }

    /// Outgoing edges of one artifact: `(target, relation)`, sorted.
    ///
    /// Targets are the strings the frontmatter declared, verbatim. Resolving an
    /// unresolved target is a read decision the caller makes; this adapter does
    /// not invent a node for it.
    pub fn get_relations(&self, id: &str) -> Vec<(String, String)> {
        self.outgoing.get(id).cloned().unwrap_or_default()
    }

    /// Incoming edges of one artifact: `(source, relation)`, sorted.
    pub fn get_incoming_relations(&self, id: &str) -> Vec<(String, String)> {
        self.incoming.get(id).cloned().unwrap_or_default()
    }

    /// Every edge as `(source_id, target_id, relation)`, sorted by source.
    ///
    /// `source_id` is the artifact's display id — the same identity the store's
    /// relations table uses — so a caller cannot tell the two sources apart.
    pub fn get_all_relations(&self) -> Vec<(String, String, String)> {
        let mut out: Vec<(String, String, String)> = Vec::new();
        for (source, edges) in &self.outgoing {
            for (target, relation) in edges {
                out.push((source.clone(), target.clone(), relation.clone()));
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
    pub fn frontmatter_map_with_links(
        &self,
        record: &ArtifactRecord,
    ) -> BTreeMap<String, serde_yaml::Value> {
        crate::db::store::frontmatter_map_with_links_from(record, &self.get_relations(&record.id))
    }

    /// The graph over exactly this view's records and edges.
    ///
    /// Built from parts rather than from a store, so graph expansion in search
    /// can never walk an artifact this read did not see.
    pub fn knowledge_graph(&self) -> KnowledgeGraph {
        let nodes: Vec<ArtifactNode> = self
            .records
            .iter()
            .map(|record| ArtifactNode {
                id: record.id.clone(),
                kind: record.kind.clone(),
                status: record.status.clone(),
            })
            .collect();
        KnowledgeGraph::from_parts(nodes, self.get_all_relations())
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
fn compute_r_eff_scores(
    records: &[ArtifactRecord],
    outgoing: &BTreeMap<String, Vec<(String, String)>>,
    incoming: &BTreeMap<String, Vec<(String, String)>>,
) -> BTreeMap<String, f64> {
    let evidence: Vec<&ArtifactRecord> = records
        .iter()
        .filter(|record| record.kind.eq_ignore_ascii_case("evidence"))
        .collect();
    let pack_of = |id: &str| evidence.iter().copied().find(|pack| pack.id == id);

    let mut scores = BTreeMap::new();
    for record in records {
        let mut linked: Vec<&ArtifactRecord> = Vec::new();
        if let Some(edges) = outgoing.get(&record.id) {
            for (target, _) in edges {
                if let Some(pack) = pack_of(target) {
                    linked.push(pack);
                }
            }
        }
        if let Some(sources) = incoming.get(&record.id) {
            for (source, _) in sources {
                if let Some(pack) = pack_of(source) {
                    linked.push(pack);
                }
            }
        }
        let items: Vec<reff::EvidenceItem> =
            linked.into_iter().map(parse_evidence_from_record).collect();
        scores.insert(record.id.clone(), reff::r_eff(&items));
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
    }

    #[tokio::test]
    async fn resolve_id_accepts_display_forms_and_slugs_and_rejects_unknown_refs() {
        let tmp = workspace();
        let read = read_of(tmp.path()).await;

        assert_eq!(read.resolve_id("PRD-001").as_deref(), Some("PRD-001"));
        // Normalized display form: `prd-1` → `PRD-001`.
        assert_eq!(read.resolve_id("prd-1").as_deref(), Some("PRD-001"));
        assert_eq!(read.resolve_id("prd-001").as_deref(), Some("PRD-001"));
        // ADR-012 slug, case-insensitively.
        assert_eq!(
            read.resolve_id("prd-auth-system").as_deref(),
            Some("PRD-001")
        );
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
            vec![("PRD-001".to_string(), "informs".to_string())]
        );
        assert!(read.get_relations("RFC-002").is_empty());
        assert!(read.get_incoming_relations("PRD-001").is_empty());
        assert!(read.get_relations("NO-SUCH-ID").is_empty());

        assert_eq!(
            read.get_all_relations(),
            vec![(
                "PRD-001".to_string(),
                "EVID-001".to_string(),
                "informs".to_string()
            )],
            "the edge set is reported with display ids, once, sorted"
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
        // RFC has none in either direction.
        assert_eq!(graph.neighbors_with_relations("PRD-001").len(), 1);
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
}
