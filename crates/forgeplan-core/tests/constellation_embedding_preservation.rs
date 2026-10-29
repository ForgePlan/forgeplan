//! Embedding preservation across reconcile, profile migration, and
//! canonical-key migration — the regression coverage for three confirmed
//! contract defects.
//!
//! * **C01** — a reconcile that stages *any* change must not wipe the
//!   semantic columns of rows whose `source_digest` is unchanged. The
//!   artifacts batch carries null `embedding`/`profile_id` for capture
//!   rows, so an unconditional `when_matched_update_all` invalidates the
//!   whole corpus on every pass.
//! * **C07** — a row bound to an old `profile_id` must be able to *adopt*
//!   the drain's current profile. The guarded publish used to require the
//!   stored profile to be the new one or NULL, so an eligible row's
//!   completion always matched zero rows and the queue never converged.
//! * **C02** — a canonical-key change at the same path (slug or display id
//!   edit) must delete the *old* key's relations and sections. Deleting
//!   only under the new key leaves the old edges in place, fails
//!   post-commit verification, and livelocks every later pass.
//!
//! All three go through `maintain_index` and the
//! `prepare_embedding_batch` → compute → `complete_embedding_batch` cycle on
//! real temporary workspaces — no model, no CLI. Each compute is a constant
//! fill, which is enough to prove *which* rows kept or adopted a vector; it
//! is never a semantic-quality claim.

use std::collections::BTreeMap;
use std::path::Path;

use arrow_array::{Array, StringArray};
use forgeplan_core::constellation::capture::CaptureLimits;
use forgeplan_core::constellation::facade::{
    EmbeddingBatchOutcome, MaintainCapability, PinnedFilter, adopt_embedding_profile,
    complete_embedding_batch, fail_embedding_batch, get_pinned_artifact, list_pinned_documents,
    maintain_index, prepare_embedding_batch, resume_admission, search_pinned_semantic,
};
use forgeplan_core::constellation::jobs::{
    DEFAULT_BODY_PREFIX_CHARS, DEFAULT_MAX_TOKENS, DEFAULT_MODEL_REPO, EmbeddingProfile,
};
use forgeplan_core::constellation::manifest::{
    BootstrapState, ProtocolMarker, STORAGE_SCHEME_FILE_OBJECT_STORE,
};
use forgeplan_core::constellation::{ConstellationError, FORMAT_VERSION};
use forgeplan_core::workspace::ARTIFACT_DIRS;
use tempfile::TempDir;

fn deadline() -> std::time::Instant {
    std::time::Instant::now() + std::time::Duration::from_secs(120)
}

/// No kind/status restriction, for the searches that are not about filtering.
const NO_FILTER: PinnedFilter = PinnedFilter {
    kind: None,
    status: None,
};

/// A profile whose dimensions match the table, so admission accepts it.
///
/// The two profiles differ in `body_prefix_chars`, which changes `profile_id`
/// and therefore the whole binding identity — that is what makes a migration
/// between them observable here, and it is exactly what a real model swap
/// does.
fn profile(prefix_chars: usize, revision: &str) -> EmbeddingProfile {
    EmbeddingProfile {
        model_repo: DEFAULT_MODEL_REPO.to_string(),
        weights_revision: revision.to_string(),
        weights_checksums: vec!["weights-sha".to_string()],
        tokenizer_checksum: "tok-sha".to_string(),
        tokenizer_config_checksum: "tokcfg-sha".to_string(),
        dimensions: forgeplan_core::db::schema::EMBEDDING_DIM as u32,
        engine_version: "tract-onnx-0.23".to_string(),
        pooling: "cls".to_string(),
        normalization: "l2".to_string(),
        query_prefix: String::new(),
        document_prefix: String::new(),
        body_prefix_chars: prefix_chars,
        max_tokens: DEFAULT_MAX_TOKENS,
        padding: "none".to_string(),
        truncation: "longest_first".to_string(),
        // Part of `profile_id`, so two profiles differing only in layout are
        // two embedding spaces. A fixed value is enough here: the property
        // under test is the artifact-prefix budget, not the layout.
        section_layout_fingerprint: "layout-fixture".to_string(),
    }
}

/// Profile A: the corpus-wide default prefix, revision `rev-a`.
fn profile_a() -> EmbeddingProfile {
    profile(DEFAULT_BODY_PREFIX_CHARS, "rev-a")
}

/// Profile B: a different verified snapshot, so a different `profile_id`.
fn profile_b() -> EmbeddingProfile {
    profile(DEFAULT_BODY_PREFIX_CHARS, "rev-b")
}

/// One admission → constant compute → completion cycle, with no model.
///
/// `fill` is the marker that lets one cycle's vectors be told from another's.
/// Returns the outcome so a test can read `published`, `stale` and
/// `cannot_judge` separately rather than collapsing them into a count.
async fn embed_once(
    capability: &MaintainCapability,
    profile: &EmbeddingProfile,
    fill: f32,
) -> Result<EmbeddingBatchOutcome, ConstellationError> {
    let prepared = prepare_embedding_batch(capability, profile, deadline()).await?;
    if prepared.inputs.is_empty() {
        return Ok(EmbeddingBatchOutcome {
            profile_id: profile.profile_id(),
            eligible_keys: Vec::new(),
            published: Vec::new(),
            stale: Vec::new(),
            cannot_judge: Vec::new(),
        });
    }
    let results: Vec<forgeplan_core::constellation::jobs::ComputedEmbedding> = prepared
        .inputs
        .iter()
        .map(
            |input| forgeplan_core::constellation::jobs::ComputedEmbedding {
                job: input.job.clone(),
                vector: vec![fill; forgeplan_core::db::schema::EMBEDDING_DIM as usize],
            },
        )
        .collect();
    complete_embedding_batch(capability, &prepared, &results, deadline()).await
}

/// Three artifacts, one of them slug-keyed so a later edit can change the
/// canonical key without touching the path.
///
/// * `prds/prd-alpha.md` — key `prd-alpha` (ADR-012 slug wins over `id`),
///   links to `ADR-002`.
/// * `adrs/ADR-002-storage.md` — key `ADR-002` (no slug, display id keys).
/// * `specs/SPEC-009-view.md` — key `SPEC-009`, links to `prd-alpha`.
fn linked_workspace() -> TempDir {
    let tmp = TempDir::new().unwrap();
    for dir in ARTIFACT_DIRS {
        std::fs::create_dir_all(tmp.path().join(dir)).unwrap();
    }
    std::fs::write(
        tmp.path().join("prds").join("prd-alpha.md"),
        "---\nid: PRD-001\nslug: prd-alpha\nkind: prd\nstatus: draft\ntitle: Alpha\nlinks:\n  - target: ADR-002\n    relation: refines\n---\n\nAlpha body.\n",
    )
    .unwrap();
    std::fs::write(
        tmp.path().join("adrs").join("ADR-002-storage.md"),
        "---\nid: ADR-002\nkind: adr\nstatus: active\ntitle: Storage\n---\n\nStorage body.\n",
    )
    .unwrap();
    std::fs::write(
        tmp.path().join("specs").join("SPEC-009-view.md"),
        "---\nid: SPEC-009\nkind: spec\nstatus: draft\ntitle: View\nlinks:\n  - target: prd-alpha\n    relation: based_on\n---\n\nView body.\n",
    )
    .unwrap();
    tmp
}

fn capability_for(tmp: &TempDir) -> MaintainCapability {
    MaintainCapability::for_strict_maintenance(tmp.path().canonicalize().unwrap())
}

/// The marker a pinned reader needs, built the way `facade::marker_for`
/// builds it — the fields are the protocol's, not test conveniences.
fn marker_for(
    manifest: &forgeplan_core::constellation::manifest::GenerationManifest,
) -> ProtocolMarker {
    ProtocolMarker {
        protocol_version: FORMAT_VERSION,
        state: BootstrapState::Active,
        index_epoch: manifest.index_epoch,
        checkout_id: manifest.checkout_id.clone(),
        table_names: ProtocolMarker::active_table_names(),
        storage_scheme: STORAGE_SCHEME_FILE_OBJECT_STORE.to_string(),
        bootstrap_attempt: 1,
    }
}

async fn pinned_descriptor_and_manifest(
    index_root: &Path,
) -> (
    forgeplan_core::constellation::manifest::CurrentDescriptor,
    forgeplan_core::constellation::manifest::GenerationManifest,
) {
    let descriptor = forgeplan_core::constellation::storage::read_current(index_root)
        .unwrap()
        .expect("a published generation must exist");
    let manifest =
        forgeplan_core::constellation::storage::read_manifest(index_root, &descriptor).unwrap();
    (descriptor, manifest)
}

/// One artifacts row's semantic columns, read at the pinned version.
#[derive(Debug, PartialEq)]
struct SemanticRow {
    profile_id: Option<String>,
    semantic_state: String,
    input_digest: Option<String>,
    has_embedding: bool,
    source_digest: String,
    relative_path: String,
}

/// Every artifacts row at the pinned version, keyed by `canonical_key`.
async fn pinned_artifact_rows(index_root: &Path) -> BTreeMap<String, SemanticRow> {
    let (descriptor, manifest) = pinned_descriptor_and_manifest(index_root).await;
    let version = descriptor.effective_artifacts_version(&manifest).unwrap();
    let marker = marker_for(&manifest);
    let table = forgeplan_core::constellation::lance_tables::open_table_at_version(
        index_root,
        &marker,
        "artifacts",
        version,
    )
    .await
    .unwrap();
    let batches = table.query_batches().await.unwrap();

    let mut rows = BTreeMap::new();
    for batch in &batches {
        let col = |name: &str| batch.column_by_name(name).unwrap();
        let str_col = |name: &str| {
            col(name)
                .as_any()
                .downcast_ref::<StringArray>()
                .expect("a string column")
        };
        let keys = str_col("canonical_key");
        let profiles = str_col("profile_id");
        let states = str_col("semantic_state");
        let inputs = str_col("input_digest");
        let digests = str_col("source_digest");
        let paths = str_col("relative_path");
        let embeddings = col("embedding");
        for row in 0..batch.num_rows() {
            rows.insert(
                keys.value(row).to_string(),
                SemanticRow {
                    profile_id: if profiles.is_null(row) {
                        None
                    } else {
                        Some(profiles.value(row).to_string())
                    },
                    semantic_state: states.value(row).to_string(),
                    input_digest: if inputs.is_null(row) {
                        None
                    } else {
                        Some(inputs.value(row).to_string())
                    },
                    has_embedding: !embeddings.is_null(row),
                    source_digest: digests.value(row).to_string(),
                    relative_path: paths.value(row).to_string(),
                },
            );
        }
    }
    rows
}

/// Every relations row at the pinned version, as `(key, target, relation)`.
async fn pinned_relation_rows(index_root: &Path) -> Vec<(String, String, String)> {
    let (_descriptor, manifest) = pinned_descriptor_and_manifest(index_root).await;
    let marker = marker_for(&manifest);
    let table = forgeplan_core::constellation::lance_tables::open_table_at_version(
        index_root,
        &marker,
        "relations",
        manifest.table_versions.relations,
    )
    .await
    .unwrap();
    let batches = table.query_batches().await.unwrap();
    let mut edges = Vec::new();
    for batch in &batches {
        let col = |name: &str| {
            batch
                .column_by_name(name)
                .unwrap()
                .as_any()
                .downcast_ref::<StringArray>()
                .expect("a string column")
        };
        let keys = col("canonical_key");
        let targets = col("target");
        let relations = col("relation");
        for row in 0..batch.num_rows() {
            edges.push((
                keys.value(row).to_string(),
                targets.value(row).to_string(),
                relations.value(row).to_string(),
            ));
        }
    }
    edges
}

/// Every sections row at the pinned version, as `(key, parent_key)`.
async fn pinned_section_rows(index_root: &Path) -> Vec<(String, String)> {
    let (descriptor, manifest) = pinned_descriptor_and_manifest(index_root).await;
    let marker = marker_for(&manifest);
    let version = descriptor
        .effective_sections_version(&manifest)
        .expect("this build always pins a sections version");
    let table = forgeplan_core::constellation::lance_tables::open_table_at_version(
        index_root, &marker, "sections", version,
    )
    .await
    .unwrap();
    let batches = table.query_batches().await.unwrap();
    let mut sections = Vec::new();
    for batch in &batches {
        let col = |name: &str| {
            batch
                .column_by_name(name)
                .unwrap()
                .as_any()
                .downcast_ref::<StringArray>()
                .expect("a string column")
        };
        let keys = col("canonical_key");
        let parents = col("parent_key");
        for row in 0..batch.num_rows() {
            sections.push((keys.value(row).to_string(), parents.value(row).to_string()));
        }
    }
    sections
}

// ---------------------------------------------------------------------------
// C01 — a reconcile with one edit must not wipe the whole corpus's embeddings
// ---------------------------------------------------------------------------

/// A drain completes every row, then a single-file edit goes through a
/// reconcile. The two untouched rows must keep their vector, their profile
/// and their `ready` state; only the edited row may be invalidated. The
/// descriptor's claimed space and the pinned heads must survive too.
#[tokio::test]
async fn a_reconcile_with_one_edit_preserves_embeddings_on_unchanged_rows() {
    resume_admission();
    let tmp = linked_workspace();
    let capability = capability_for(&tmp);
    let index_root = capability.index_root().to_path_buf();

    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    // Publish the semantic state through the same admission → completion path a
    // real queue uses — no shortcut that writes columns by another route.
    let profile_a = profile_a();
    let profile_a_id = profile_a.profile_id();
    let drained = embed_once(&capability, &profile_a, 0.25).await.unwrap();
    assert!(
        !drained.published.is_empty(),
        "the completion must land vectors: {drained:?}"
    );
    assert!(drained.stale.is_empty(), "nothing moved: {drained:?}");
    assert!(drained.cannot_judge.is_empty(), "{drained:?}");

    let before = pinned_artifact_rows(&index_root).await;
    assert_eq!(before.len(), 3);
    assert!(
        before.values().all(|row| {
            row.semantic_state == "ready"
                && row.profile_id.as_deref() == Some(profile_a_id.as_str())
                && row.has_embedding
                && row.input_digest.is_some()
        }),
        "every row must be embedded and bound to profile-a before the edit: {before:?}"
    );

    // One edit, on one artifact. The other two rows are byte-identical.
    std::fs::write(
        tmp.path().join("adrs").join("ADR-002-storage.md"),
        "---\nid: ADR-002\nkind: adr\nstatus: active\ntitle: Storage\n---\n\nRevised body.\n",
    )
    .unwrap();
    let (_, reconcile) = maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();
    assert!(
        !reconcile.commits.is_empty(),
        "a real edit must produce commits"
    );

    let after = pinned_artifact_rows(&index_root).await;
    for key in ["prd-alpha", "SPEC-009"] {
        let untouched = &after[key];
        let prior = &before[key];
        assert_eq!(untouched.semantic_state, "ready", "{key} must stay ready");
        assert_eq!(
            untouched.profile_id.as_deref(),
            Some(profile_a_id.as_str()),
            "{key} must keep its profile binding"
        );
        assert!(
            untouched.has_embedding,
            "{key} must keep its vector — an unchanged row losing it is the C01 wipe"
        );
        assert_eq!(
            untouched.input_digest, prior.input_digest,
            "{key}'s input binding must not be rewritten"
        );
        assert_eq!(
            untouched.source_digest, prior.source_digest,
            "{key}'s bytes did not move — its digest is the same claim"
        );
        assert_eq!(
            untouched.relative_path, prior.relative_path,
            "{key}'s path did not move"
        );
    }
    let edited = &after["ADR-002"];
    assert_eq!(
        edited.semantic_state, "unavailable",
        "the edited row's vector is stale and must be invalidated, not kept"
    );
    assert!(
        edited.profile_id.is_none(),
        "the edited row's old profile claim must be dropped"
    );
    assert!(
        !edited.has_embedding,
        "the edited row's vector must be dropped"
    );

    // The descriptor keeps claiming the completion's space — the manifest cannot
    // express it, so a reconcile that dropped it would unpublish the profile.
    let (descriptor, _manifest) = pinned_descriptor_and_manifest(&index_root).await;
    assert_eq!(
        descriptor.profile_id.as_deref(),
        Some(profile_a_id.as_str()),
        "the reconcile must carry the completion's claimed space forward"
    );

    // And the pinned semantic surface still answers in that space.
    let query = vec![0.25f32; forgeplan_core::db::schema::EMBEDDING_DIM as usize];
    let found = search_pinned_semantic(&index_root, &query, &profile_a_id, 10, &NO_FILTER)
        .await
        .unwrap();
    assert!(
        found
            .hits
            .iter()
            .any(|hit| hit.canonical_key == "prd-alpha"),
        "the preserved vector must still be queryable at the pinned generation: {:?}",
        found.hits
    );

    // Only the edited artifact and its section need fresh vectors; unchanged
    // documents and sections must not be republished.
    let second = embed_once(&capability, &profile_a, 0.25).await.unwrap();
    assert!(second.stale.is_empty(), "no row moved: {second:?}");
    assert_eq!(
        second.published,
        vec!["ADR-002".to_string(), "ADR-002#s0".to_string()],
        "only the edited document and its section were still outstanding"
    );
    let converged = pinned_artifact_rows(&index_root).await;
    assert!(
        converged.values().all(|row| row.semantic_state == "ready"),
        "every row must be ready once the edited one is re-embedded: {converged:?}"
    );
}

// ---------------------------------------------------------------------------
// C07 — a new profile republishes rows bound to an old one
// ---------------------------------------------------------------------------

/// Rows bound to profile A are eligible work for an admission under profile
/// B — every binding field is identical except the space. Admission must
/// re-point the row at B **first**, so the completion's exact-profile guard
/// matches and the row adopts the new space instead of being refused forever.
#[tokio::test]
async fn a_completion_under_a_new_profile_republishes_rows_bound_to_an_old_one() {
    resume_admission();
    let tmp = linked_workspace();
    let capability = capability_for(&tmp);
    let index_root = capability.index_root().to_path_buf();

    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    let profile_a = profile_a();
    let profile_a_id = profile_a.profile_id();
    let first = embed_once(&capability, &profile_a, 0.25).await.unwrap();
    assert!(!first.published.is_empty(), "{first:?}");
    assert!(first.stale.is_empty(), "{first:?}");

    // The model changed: same input digests, a different embedding space.
    // Every profile-a row is eligible work, and every completion must land —
    // a stale completion here is the C07 non-convergence.
    let profile_b = profile_b();
    let profile_b_id = profile_b.profile_id();
    // The operator moves the index onto B before draining it. Admission
    // admits only in the space `CURRENT` serves, so this is what makes the
    // migration happen at all — and it is deliberately a separate, named act
    // rather than something `embed_once` does as a side effect.
    assert!(
        adopt_embedding_profile(&capability, &profile_b, deadline())
            .await
            .unwrap(),
        "adopting profile-b must move CURRENT off profile-a"
    );
    let migrated = embed_once(&capability, &profile_b, 0.5).await.unwrap();
    assert!(
        !migrated.published.is_empty(),
        "rows bound to the old profile are work for the new one: {migrated:?}"
    );
    assert!(
        migrated.stale.is_empty(),
        "a same-content row bound to another profile must adopt the new \
         profile, not be refused as stale: {migrated:?}"
    );
    assert!(migrated.cannot_judge.is_empty(), "{migrated:?}");

    let rows = pinned_artifact_rows(&index_root).await;
    for (key, row) in &rows {
        assert_eq!(row.semantic_state, "ready", "{key} must be ready under b");
        assert_eq!(
            row.profile_id.as_deref(),
            Some(profile_b_id.as_str()),
            "{key} must carry the new profile — adopted, not relabeled-only: \
             the vector was recomputed under profile-b"
        );
        assert!(row.has_embedding);
    }

    // The old space holds no vectors: a search under profile-a sees zero
    // eligible rows, while profile-b answers with the migrated vectors.
    let query = vec![0.5f32; forgeplan_core::db::schema::EMBEDDING_DIM as usize];
    let under_b = search_pinned_semantic(&index_root, &query, &profile_b_id, 10, &NO_FILTER)
        .await
        .unwrap();
    assert!(under_b.eligible_rows > 0);
    let under_a = search_pinned_semantic(&index_root, &query, &profile_a_id, 10, &NO_FILTER)
        .await
        .unwrap();
    assert_eq!(
        under_a.eligible_rows, 0,
        "no row may remain bound to the old space"
    );

    // Same-profile cycles stay current and do no work.
    let again = embed_once(&capability, &profile_b, 0.5).await.unwrap();
    assert!(
        again.published.is_empty(),
        "a completed row must not be re-embedded: {again:?}"
    );
    assert!(again.stale.is_empty(), "{again:?}");
}

// ---------------------------------------------------------------------------
// C02 — a canonical-key change at the same path must not leak old relations
// ---------------------------------------------------------------------------

/// Two artifacts change their canonical key in place — `prd-alpha` via a
/// slug edit, `ADR-002` via a display-id edit. The reconcile must delete
/// the outgoing edges and the sections the *old* keys owned; otherwise the
/// post-commit edge check disagrees and every later reconcile livelocks.
#[tokio::test]
async fn canonical_key_change_at_same_path_must_not_leak_old_relations() {
    resume_admission();
    let tmp = linked_workspace();
    let capability = capability_for(&tmp);
    let index_root = capability.index_root().to_path_buf();

    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    // Same paths, new canonical keys. Both edits change the source digest,
    // so the reconciler sees Updates, not Creates.
    std::fs::write(
        tmp.path().join("prds").join("prd-alpha.md"),
        "---\nid: PRD-001\nslug: prd-alpha-v2\nkind: prd\nstatus: draft\ntitle: Alpha\nlinks:\n  - target: ADR-002\n    relation: refines\n---\n\nAlpha body.\n",
    )
    .unwrap();
    std::fs::write(
        tmp.path().join("adrs").join("ADR-002-storage.md"),
        "---\nid: ADR-007\nkind: adr\nstatus: active\ntitle: Storage\n---\n\nStorage body.\n",
    )
    .unwrap();

    // RED: before the fix this reconcile fails post-commit verification —
    // the old key's edges leak and the edge count disagrees forever.
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    let page = list_pinned_documents(&index_root, 0, 100).await.unwrap();
    let keys: std::collections::BTreeSet<&str> = page
        .documents
        .iter()
        .map(|doc| doc.canonical_key.as_str())
        .collect();
    assert_eq!(
        keys,
        std::collections::BTreeSet::from(["prd-alpha-v2", "ADR-007", "SPEC-009"]),
        "the old keys must be gone and the new keys published"
    );

    // The old key's outgoing edges are gone; the new key's replaced set is
    // exactly the captured one. `SPEC-009 -> prd-alpha` is a dangling
    // target by name, but the edge row itself is legitimately stored.
    let edges = pinned_relation_rows(&index_root).await;
    assert_eq!(edges.len(), 2, "one edge per linked artifact, no leaks");
    assert!(
        edges
            .iter()
            .all(|(key, _, _)| key == "prd-alpha-v2" || key == "SPEC-009"),
        "no edge may survive under the replaced key `prd-alpha`: {edges:?}"
    );

    // Sections keyed by the old parents are gone too.
    let sections = pinned_section_rows(&index_root).await;
    assert!(
        sections
            .iter()
            .all(|(_, parent)| parent != "prd-alpha" && parent != "ADR-002"),
        "no section may survive under a replaced parent: {sections:?}"
    );

    // Point reads agree: the old key is absent, the new one serves — at the
    // path that always owned it.
    assert!(
        get_pinned_artifact(&index_root, "prd-alpha")
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        get_pinned_artifact(&index_root, "prd-alpha-v2")
            .await
            .unwrap()
            .is_some()
    );
    let rows = pinned_artifact_rows(&index_root).await;
    assert_eq!(
        rows["prd-alpha-v2"].relative_path, "prds/prd-alpha.md",
        "the new key must claim the path the update happened on"
    );

    // And the index converges: the next reconcile over the same source is a
    // no-op, not another verification failure.
    let (_, third) = maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();
    assert!(
        third.commits.is_empty(),
        "an unchanged source must produce no commits — the pass is converged"
    );
}

// ---------------------------------------------------------------------------
// C01 — a metadata-only edit must not invalidate a row whose input did not move
// ---------------------------------------------------------------------------

/// A `status:` edit changes the file's bytes, so `source_digest` and
/// `row_revision` both move. It does not change the title or the body, so the
/// digest of what the tokenizer receives is identical. The stored vector must
/// survive that edit untouched — requiring the source digest would throw away
/// the whole corpus the first time anyone edited a frontmatter line.
///
/// This is the case the C01 fix originally missed: it preserved a row by
/// comparing source digests, which preserved an *unedited* row and dropped an
/// *edited* one.
#[tokio::test]
async fn a_metadata_only_edit_preserves_the_vector_and_the_claimed_space() {
    resume_admission();
    let tmp = linked_workspace();
    let capability = capability_for(&tmp);
    let index_root = capability.index_root().to_path_buf();

    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();
    let profile_a = profile_a();
    let profile_a_id = profile_a.profile_id();
    let drained = embed_once(&capability, &profile_a, 0.25).await.unwrap();
    assert!(!drained.published.is_empty(), "{drained:?}");

    let before = pinned_artifact_rows(&index_root).await;
    assert!(
        before
            .values()
            .all(|row| row.has_embedding && row.semantic_state == "ready"),
        "every row must start embedded: {before:?}"
    );

    // Frontmatter only: `status` and a new tag. Same path, same title, same
    // body — so the input digest is unchanged and the incarnation holds.
    std::fs::write(
        tmp.path().join("prds").join("prd-alpha.md"),
        "---\nid: PRD-001\nslug: prd-alpha\nkind: prd\nstatus: active\ntitle: Alpha\ntags:\n  - reviewed\nlinks:\n  - target: ADR-002\n    relation: refines\n---\n\nAlpha body.\n",
    )
    .unwrap();

    let (_, reconcile) = maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();
    assert!(
        !reconcile.commits.is_empty(),
        "a real edit must produce commits"
    );

    let after = pinned_artifact_rows(&index_root).await;
    let edited = &after["prd-alpha"];
    let prior = &before["prd-alpha"];
    assert_ne!(
        edited.source_digest, prior.source_digest,
        "the bytes did move; this is not a no-op pass"
    );
    assert_eq!(
        edited.input_digest, prior.input_digest,
        "the tokenizer input is what the vector answers to, and it did not move"
    );
    assert!(
        edited.has_embedding,
        "an input that did not move keeps its vector — dropping it here is the \
         metadata-only invalidation bug"
    );
    assert_eq!(
        edited.semantic_state, "ready",
        "a preserved row keeps the state it had"
    );
    assert_eq!(
        edited.profile_id.as_deref(),
        Some(profile_a_id.as_str()),
        "a preserved row keeps its space"
    );

    // The untouched rows are unaffected, and the queue has nothing to do:
    // a preserved row is not work.
    for key in ["ADR-002", "SPEC-009"] {
        assert_eq!(after[key].semantic_state, "ready", "{key} must stay ready");
        assert!(after[key].has_embedding, "{key} must keep its vector");
    }
    let second = embed_once(&capability, &profile_a, 0.25).await.unwrap();
    assert!(
        second.published.is_empty(),
        "a metadata-only edit must not make the corpus eligible work again: \
         {second:?}"
    );
    assert!(second.stale.is_empty(), "{second:?}");

    // The descriptor still names the space, through a reconcile that had no
    // pending job to infer it from.
    let (descriptor, _manifest) = pinned_descriptor_and_manifest(&index_root).await;
    assert_eq!(
        descriptor.profile_id.as_deref(),
        Some(profile_a_id.as_str()),
        "an empty pending_jobs set must not unpublish the claimed space"
    );

    // And the preserved vector is still the one the pinned reader serves.
    let query = vec![0.25f32; forgeplan_core::db::schema::EMBEDDING_DIM as usize];
    let found = search_pinned_semantic(&index_root, &query, &profile_a_id, 10, &NO_FILTER)
        .await
        .unwrap();
    assert!(
        found
            .hits
            .iter()
            .any(|hit| hit.canonical_key == "prd-alpha"),
        "the preserved vector must still be queryable: {:?}",
        found.hits
    );
}

// ---------------------------------------------------------------------------
// Identity — a document replaced at the same path must not inherit a vector
// ---------------------------------------------------------------------------

/// Delete a file and write a different one at the same path. The key is the
/// same, so nothing but the content can tell the rows apart — and the vector
/// the old document had describes bytes that no longer exist.
///
/// The `input_digest` is what clears it here: the replacement has a different
/// body, so the digest the vector was bound to no longer matches.
/// `incarnation` is the second line of defence and now actually moves —
/// `maintain_index` captures against the published manifest, so a confirmed
/// delete/recreate skips past the incarnation the delete retired. That fence
/// is what covers the harder case, where the replacement is *byte-identical*
/// and only the tombstone distinguishes it; `a_byte_identical_recreate`
/// covers that one. The test asserts the observable outcome, not which of the
/// two fired here.
#[tokio::test]
async fn a_document_replaced_at_the_same_path_does_not_inherit_the_old_vector() {
    resume_admission();
    let tmp = linked_workspace();
    let capability = capability_for(&tmp);
    let index_root = capability.index_root().to_path_buf();

    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();
    let profile_a = profile_a();
    let first = embed_once(&capability, &profile_a, 0.25).await.unwrap();
    assert!(!first.published.is_empty(), "{first:?}");
    let before = pinned_artifact_rows(&index_root).await;
    assert!(before["ADR-002"].has_embedding);

    // Same path, same key, different document. The bytes are unrelated.
    std::fs::remove_file(tmp.path().join("adrs").join("ADR-002-storage.md")).unwrap();
    std::fs::write(
        tmp.path().join("adrs").join("ADR-002-storage.md"),
        "---\nid: ADR-002\nkind: adr\nstatus: active\ntitle: Storage\n---\n\nA completely different decision record.\n",
    )
    .unwrap();

    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    let after = pinned_artifact_rows(&index_root).await;
    let replaced = &after["ADR-002"];
    assert_ne!(
        replaced.source_digest, before["ADR-002"].source_digest,
        "this test only means something if the bytes actually changed"
    );
    assert_ne!(
        replaced.input_digest, before["ADR-002"].input_digest,
        "the replacement has a different body, so the input the vector answers to moved"
    );
    assert!(
        !replaced.has_embedding,
        "a replaced document must not inherit the old one's vector"
    );
    assert_eq!(
        replaced.profile_id, None,
        "a cleared row must not keep a profile claim"
    );
    assert_eq!(
        replaced.semantic_state, "unavailable",
        "a cleared row says unavailable, not ready"
    );

    // And the invalidation is honest work, not a silent hole: admission sees
    // exactly the replaced document and its section, and nothing else.
    let prepared = prepare_embedding_batch(&capability, &profile_a, deadline())
        .await
        .unwrap();
    let admitted: std::collections::BTreeSet<String> = prepared
        .inputs
        .iter()
        .map(|input| input.job.canonical_key.clone())
        .collect();
    assert_eq!(
        admitted,
        std::collections::BTreeSet::from(["ADR-002".to_string(), "ADR-002#s0".to_string()]),
        "only the recreated document's row and its section are outstanding work"
    );

    // And that work completes: the recreated row earns a vector of its own,
    // and the completion is not stale.
    let results: Vec<forgeplan_core::constellation::jobs::ComputedEmbedding> = prepared
        .inputs
        .iter()
        .map(
            |input| forgeplan_core::constellation::jobs::ComputedEmbedding {
                job: input.job.clone(),
                vector: vec![0.25; forgeplan_core::db::schema::EMBEDDING_DIM as usize],
            },
        )
        .collect();
    let next = complete_embedding_batch(&capability, &prepared, &results, deadline())
        .await
        .unwrap();
    assert!(
        next.stale.is_empty(),
        "the completion must not be stale: {next:?}"
    );
    assert_eq!(
        next.published
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>(),
        admitted,
        "exactly the admitted rows must publish"
    );

    // The replaced row now carries its own vector, bound to its own input.
    let rebuilt = pinned_artifact_rows(&index_root).await;
    assert_eq!(rebuilt["ADR-002"].semantic_state, "ready");
    assert!(rebuilt["ADR-002"].has_embedding);
}

// ---------------------------------------------------------------------------
// The delete/recreate fence at full production resolution
// ---------------------------------------------------------------------------

/// Delete a document, let the index publish the confirmed absence, then
/// recreate it with **byte-identical** content at the same path.
///
/// This is the case that nothing but the incarnation can decide. The path is
/// the same, the canonical key is the same, the `source_digest` is the same and
/// so is the `input_digest` — every field `SemanticColumns::is_preserved_for`
/// compares is identical to the deleted row's. The only thing that
/// distinguishes the replacement from the document that was deleted is that a
/// reconcile in between **confirmed the absence**, recorded it as a tombstone,
/// and the next capture skipped past the incarnation that tombstone retired.
///
/// Before the published manifest became the capture baseline, `incarnation`
/// was `1` on every pass, so this reproduction carried the deleted row's whole
/// identity: its vector was spliced onto the replacement, and an outstanding
/// job bound to the deleted document was admitted against it.
#[tokio::test]
async fn a_byte_identical_recreate_does_not_inherit_the_deleted_documents_vector() {
    resume_admission();
    let tmp = linked_workspace();
    let capability = capability_for(&tmp);
    let index_root = capability.index_root().to_path_buf();

    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();
    let profile_a = profile_a();
    let first = embed_once(&capability, &profile_a, 0.25).await.unwrap();
    assert!(!first.published.is_empty(), "{first:?}");

    let before = pinned_artifact_rows(&index_root).await;
    assert!(
        before["ADR-002"].has_embedding,
        "the document must start embedded, or this proves nothing: {before:?}"
    );
    let (_, before_manifest) = pinned_descriptor_and_manifest(&index_root).await;
    let deleted_entry = before_manifest
        .source_entries
        .iter()
        .find(|entry| entry.canonical_key == "ADR-002")
        .expect("the published generation must describe ADR-002")
        .clone();
    assert_eq!(
        deleted_entry.incarnation, 1,
        "the first observation is incarnation 1; the recreate must not be too"
    );

    // Pass one confirms the delete. The file is gone and the capture is
    // complete, so absence is proven rather than assumed.
    let adr = tmp.path().join("adrs").join("ADR-002-storage.md");
    let original_bytes = std::fs::read(&adr).unwrap();
    std::fs::remove_file(&adr).unwrap();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    let (_, deleted_manifest) = pinned_descriptor_and_manifest(&index_root).await;
    let tombstone = deleted_manifest
        .tombstones
        .iter()
        .find(|tombstone| tombstone.canonical_key == "ADR-002")
        .expect("a complete capture that proved an absence must record it")
        .clone();
    assert_eq!(
        tombstone.incarnation, 1,
        "the tombstone names the incarnation the delete retired"
    );
    assert!(
        !deleted_manifest
            .source_entries
            .iter()
            .any(|entry| entry.canonical_key == "ADR-002"),
        "a deleted key must leave the published source entries"
    );
    let after_delete = pinned_artifact_rows(&index_root).await;
    assert!(
        !after_delete.contains_key("ADR-002"),
        "the row is gone, so nothing can be inherited from it"
    );

    // Pass two recreates it — the very same bytes, at the very same path.
    std::fs::write(&adr, &original_bytes).unwrap();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    let (_, recreated_manifest) = pinned_descriptor_and_manifest(&index_root).await;
    let recreated_entry = recreated_manifest
        .source_entries
        .iter()
        .find(|entry| entry.canonical_key == "ADR-002")
        .expect("the recreated document is published again");
    assert_eq!(
        recreated_entry.source_digest, deleted_entry.source_digest,
        "this test only means something if the recreated bytes are identical"
    );
    assert_eq!(
        recreated_entry.input_digest, deleted_entry.input_digest,
        "and identical content means an identical tokenizer input too — so \
         the input digest cannot be what saves us here"
    );
    assert_eq!(
        recreated_entry.incarnation, 2,
        "the recreate must skip past the incarnation the confirmed delete \
         retired; reproducing it is what let the deleted vector be preserved"
    );

    let after = pinned_artifact_rows(&index_root).await;
    assert!(
        !after["ADR-002"].has_embedding,
        "a byte-identical recreate must not inherit the deleted document's \
         vector: {:?}",
        after["ADR-002"]
    );
    assert_eq!(
        after["ADR-002"].semantic_state, "unavailable",
        "and it must say unavailable, not claim the deleted document's state"
    );
    assert_eq!(
        after["ADR-002"].profile_id, None,
        "nor keep the deleted document's profile claim"
    );

    // The untouched rows are genuinely untouched — the fence is per-key, not a
    // corpus-wide invalidation wearing a narrower name.
    for key in ["prd-alpha", "SPEC-009"] {
        assert_eq!(after[key].semantic_state, "ready", "{key} must stay ready");
        assert!(after[key].has_embedding, "{key} must keep its vector");
    }
}

/// The recreate is real work, not a hole: once it carries a new incarnation,
/// admission issues it a job and completion publishes onto it.
#[tokio::test]
async fn a_byte_identical_recreate_is_issued_as_its_own_work() {
    resume_admission();
    let tmp = linked_workspace();
    let capability = capability_for(&tmp);
    let index_root = capability.index_root().to_path_buf();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();
    let profile_a = profile_a();
    embed_once(&capability, &profile_a, 0.25).await.unwrap();

    let adr = tmp.path().join("adrs").join("ADR-002-storage.md");
    let original_bytes = std::fs::read(&adr).unwrap();
    std::fs::remove_file(&adr).unwrap();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();
    std::fs::write(&adr, &original_bytes).unwrap();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    // The recreate is outstanding work: admission must see the recreated row
    // and its section, and nothing else.
    let prepared = prepare_embedding_batch(&capability, &profile_a, deadline())
        .await
        .unwrap();
    let admitted: std::collections::BTreeSet<String> = prepared
        .inputs
        .iter()
        .map(|input| input.job.canonical_key.clone())
        .collect();
    assert_eq!(
        admitted,
        std::collections::BTreeSet::from(["ADR-002".to_string(), "ADR-002#s0".to_string()]),
        "the recreated document's row and its section are the outstanding work"
    );

    let outcome = embed_once(&capability, &profile_a, 0.9).await.unwrap();
    assert_eq!(outcome.stale, Vec::<String>::new(), "{outcome:?}");
    assert_eq!(
        outcome
            .published
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>(),
        admitted,
        "the recreate must be able to earn a vector of its own"
    );
    assert!(
        pinned_artifact_rows(&index_root).await["ADR-002"].has_embedding,
        "and the vector must be readable at the pinned version"
    );
}

// ---------------------------------------------------------------------------
// The prepare → compute → complete contract
// ---------------------------------------------------------------------------

/// Constant vectors for every input of a prepared batch.
fn vectors_for(
    prepared: &forgeplan_core::constellation::jobs::PreparedEmbeddingBatch,
    fill: f32,
) -> Vec<forgeplan_core::constellation::jobs::ComputedEmbedding> {
    prepared
        .inputs
        .iter()
        .map(
            |input| forgeplan_core::constellation::jobs::ComputedEmbedding {
                job: input.job.clone(),
                vector: vec![fill; forgeplan_core::db::schema::EMBEDDING_DIM as usize],
            },
        )
        .collect()
}

/// The owner is released between admission and completion.
///
/// This is the whole reason the drain is split. The barrier is a channel
/// handshake rather than a sleep: admission returns, the test then takes the
/// owner itself, and only afterwards completes the batch. Before the split
/// the drain held that owner across the compute, so this acquire would block
/// for the length of the inference — and with the inference paused, forever.
#[tokio::test]
async fn admission_releases_the_owner_before_the_compute_runs() {
    resume_admission();
    let tmp = linked_workspace();
    let capability = capability_for(&tmp);
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    let profile_a = profile_a();
    let prepared = prepare_embedding_batch(&capability, &profile_a, deadline())
        .await
        .unwrap();
    assert!(
        !prepared.inputs.is_empty(),
        "a fresh workspace owes vectors for every row"
    );

    // Between the two phases, nothing is held. A second maintenance pass
    // proves it, and it proves it against the real owner rather than a
    // stand-in.
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .expect(
            "a second maintenance pass must be able to take the owner while a \
                 prepared batch is outstanding",
        );

    // The batch is still completable afterwards, because its jobs live in the
    // durable manifest and not in an in-memory lock.
    let outcome = complete_embedding_batch(
        &capability,
        &prepared,
        &vectors_for(&prepared, 0.25),
        deadline(),
    )
    .await;
    assert!(
        outcome.is_ok(),
        "a batch prepared before another pass must still complete: {outcome:?}"
    );
}

/// A completion whose source moved under it publishes nothing.
///
/// Admission binds the exact bytes it captured. The file is then edited and
/// reconciled, so the row's binding no longer matches the job's. The
/// completion must report the row as stale and leave the vector off it —
/// never rebind the vector onto the bytes that replaced it.
#[tokio::test]
async fn a_completion_whose_source_moved_is_stale_and_writes_nothing() {
    resume_admission();
    let tmp = linked_workspace();
    let capability = capability_for(&tmp);
    let index_root = capability.index_root().to_path_buf();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    let profile_a = profile_a();
    let prepared = prepare_embedding_batch(&capability, &profile_a, deadline())
        .await
        .unwrap();
    let target = prepared
        .inputs
        .iter()
        .find(|input| input.job.canonical_key == "ADR-002")
        .expect("ADR-002 must be admitted")
        .job
        .canonical_key
        .clone();

    // The source moves while the "inference" is outstanding.
    std::fs::write(
        tmp.path().join("adrs").join("ADR-002-storage.md"),
        "---\nid: ADR-002\nkind: adr\nstatus: active\ntitle: Storage\n---\n\nMoved body.\n",
    )
    .unwrap();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    let outcome = complete_embedding_batch(
        &capability,
        &prepared,
        &vectors_for(&prepared, 0.25),
        deadline(),
    )
    .await
    .unwrap();
    assert!(
        outcome.stale.contains(&target),
        "the row whose bytes moved must be reported stale: {outcome:?}"
    );
    assert!(
        !outcome.published.contains(&target),
        "a stale row must not be published onto: {outcome:?}"
    );

    let rows = pinned_artifact_rows(&index_root).await;
    assert!(
        !rows["ADR-002"].has_embedding,
        "the moved row must not carry the vector computed from its old bytes: {:?}",
        rows["ADR-002"]
    );
}

/// A delete and recreate between admission and completion cannot be
/// resurrected by the outstanding job.
///
/// The path, the key and even the title come back; only the bytes differ.
/// `incarnation` is the fence for exactly this, and the completion must
/// honour it rather than rebind.
#[tokio::test]
async fn a_document_recreated_during_the_compute_is_not_revived_by_the_old_job() {
    resume_admission();
    let tmp = linked_workspace();
    let capability = capability_for(&tmp);
    let index_root = capability.index_root().to_path_buf();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    let profile_a = profile_a();
    let prepared = prepare_embedding_batch(&capability, &profile_a, deadline())
        .await
        .unwrap();

    let path = tmp.path().join("adrs").join("ADR-002-storage.md");
    std::fs::remove_file(&path).unwrap();
    std::fs::write(
        &path,
        "---\nid: ADR-002\nkind: adr\nstatus: active\ntitle: Storage\n---\n\nRecreated body.\n",
    )
    .unwrap();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    let outcome = complete_embedding_batch(
        &capability,
        &prepared,
        &vectors_for(&prepared, 0.25),
        deadline(),
    )
    .await
    .unwrap();
    assert!(
        outcome.stale.contains(&"ADR-002".to_string()),
        "the recreated row must be stale for the pre-delete job: {outcome:?}"
    );

    let rows = pinned_artifact_rows(&index_root).await;
    assert!(
        !rows["ADR-002"].has_embedding,
        "the recreated row must not inherit the deleted document's vector"
    );
}

/// A result for a key this admission never issued is refused outright, and
/// nothing at all is written.
///
/// The refusal is the point: a batch carrying one stranger is not partially
/// applied. The two rows that *were* admitted stay `pending`.
#[tokio::test]
async fn an_unknown_result_key_is_refused_before_any_write() {
    resume_admission();
    let tmp = linked_workspace();
    let capability = capability_for(&tmp);
    let index_root = capability.index_root().to_path_buf();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    let profile_a = profile_a();
    let prepared = prepare_embedding_batch(&capability, &profile_a, deadline())
        .await
        .unwrap();
    let mut results = vectors_for(&prepared, 0.25);
    let stranger = forgeplan_core::constellation::jobs::EmbeddingJob {
        canonical_key: "PRD-404".to_string(),
        ..results[0].job.clone()
    };
    results.push(forgeplan_core::constellation::jobs::ComputedEmbedding {
        job: stranger,
        vector: vec![0.9; forgeplan_core::db::schema::EMBEDDING_DIM as usize],
    });

    let err = complete_embedding_batch(&capability, &prepared, &results, deadline())
        .await
        .expect_err("an unknown key must be refused");
    assert!(
        matches!(err, ConstellationError::Io { .. }),
        "an unknown key is a caller bug, not an index fault: {err}"
    );
    assert!(
        err.to_string().contains("PRD-404"),
        "the refusal must name the offending key: {err}"
    );

    // Nothing was written: every admitted row is still `pending`, so the
    // refusal really did happen before the first update.
    let rows = pinned_artifact_rows(&index_root).await;
    assert!(
        rows.values().all(|row| row.semantic_state == "pending"),
        "a refused completion must leave every admitted row pending: {rows:?}"
    );
    assert!(
        rows.values().all(|row| !row.has_embedding),
        "a refused completion must write no vector: {rows:?}"
    );
}

/// Two results for one key are refused, not deduplicated.
///
/// Two vectors for a single row is either a worker that answered twice or a
/// caller that meant two rows. Picking one would silently discard a vector,
/// so the whole batch is refused instead.
#[tokio::test]
async fn a_duplicate_result_key_is_refused_rather_than_deduplicated() {
    resume_admission();
    let tmp = linked_workspace();
    let capability = capability_for(&tmp);
    let index_root = capability.index_root().to_path_buf();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    let profile_a = profile_a();
    let prepared = prepare_embedding_batch(&capability, &profile_a, deadline())
        .await
        .unwrap();
    let mut results = vectors_for(&prepared, 0.25);
    results.push(results[0].clone());

    let err = complete_embedding_batch(&capability, &prepared, &results, deadline())
        .await
        .expect_err("a duplicate key must be refused");
    assert!(
        err.to_string().contains(&results[0].job.canonical_key),
        "the refusal must name the duplicated key: {err}"
    );

    let rows = pinned_artifact_rows(&index_root).await;
    assert!(
        rows.values().all(|row| !row.has_embedding),
        "a refused completion must write no vector: {rows:?}"
    );
}

/// A result whose job is not the one admission issued is refused.
///
/// Every binding field matches except one — this is a job that was forged or
/// carried over from an earlier admission. Only the exact job may publish.
#[tokio::test]
async fn a_result_whose_job_drifted_from_the_admitted_one_is_refused() {
    resume_admission();
    let tmp = linked_workspace();
    let capability = capability_for(&tmp);
    let index_root = capability.index_root().to_path_buf();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    let profile_a = profile_a();
    let prepared = prepare_embedding_batch(&capability, &profile_a, deadline())
        .await
        .unwrap();
    let mut results = vectors_for(&prepared, 0.25);
    results[0].job.input_digest = "a-digest-this-admission-never-recorded".to_string();

    let err = complete_embedding_batch(&capability, &prepared, &results, deadline())
        .await
        .expect_err("a drifted job must be refused");
    assert!(
        err.to_string().contains(&results[0].job.canonical_key),
        "the refusal must name the offending key: {err}"
    );

    let rows = pinned_artifact_rows(&index_root).await;
    assert!(
        rows.values().all(|row| !row.has_embedding),
        "a refused completion must write no vector: {rows:?}"
    );
}

/// A late result from a superseded admission cannot migrate the row back.
///
/// Profile A is admitted, then the operator migrates to B and B is admitted
/// for the same rows — the row now stores B and B's job is what is pending.
/// A's result arrives last. The exact-profile guard must refuse it: the row
/// keeps B, and A's vector is never written.
#[tokio::test]
async fn a_late_result_from_a_superseded_admission_cannot_move_the_row_back() {
    resume_admission();
    let tmp = linked_workspace();
    let capability = capability_for(&tmp);
    let index_root = capability.index_root().to_path_buf();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    let profile_a = profile_a();
    let profile_a_id = profile_a.profile_id();
    let profile_b = profile_b();
    let profile_b_id = profile_b.profile_id();

    // Admission A, and a result for it that is deliberately **not** completed
    // yet. A's jobs are now durable and A's rows are pending.
    let admitted_a = prepare_embedding_batch(&capability, &profile_a, deadline())
        .await
        .unwrap();
    let late_results = vectors_for(&admitted_a, 0.25);

    // The migration is explicit: `adopt_embedding_profile` is what points the
    // index at B, and admission below then works in the space `CURRENT`
    // serves. Without it, admission is refused rather than silently moving the
    // corpus — which is the same fact the late-A completion is checked
    // against.
    assert!(
        adopt_embedding_profile(&capability, &profile_b, deadline())
            .await
            .unwrap(),
        "adopting a different space must move CURRENT"
    );

    // Admission B re-points the same rows at B, and B completes.
    let admitted_b = prepare_embedding_batch(&capability, &profile_b, deadline())
        .await
        .unwrap();
    assert!(
        !admitted_b.inputs.is_empty(),
        "B's admission must re-point the rows A left pending"
    );
    let outcome_b = complete_embedding_batch(
        &capability,
        &admitted_b,
        &vectors_for(&admitted_b, 0.5),
        deadline(),
    )
    .await
    .unwrap();
    assert!(
        !outcome_b.published.is_empty() && outcome_b.stale.is_empty(),
        "B's own completion must land: {outcome_b:?}"
    );

    // Now A's result arrives late. B's admission replaced A's pending jobs, so
    // A's batch is no longer the index's current one and its completion is
    // refused — either as an outright error or with nothing published.
    let outcome_a =
        complete_embedding_batch(&capability, &admitted_a, &late_results, deadline()).await;
    if let Ok(outcome) = &outcome_a {
        assert!(
            outcome.published.is_empty(),
            "a late result must never publish: {outcome:?}"
        );
    }

    let rows = pinned_artifact_rows(&index_root).await;
    for (key, row) in &rows {
        assert_eq!(
            row.profile_id.as_deref(),
            Some(profile_b_id.as_str()),
            "{key} must stay on the newest profile"
        );
    }

    // A's vectors are nowhere: searching in A's space finds no eligible row.
    let query = vec![0.25f32; forgeplan_core::db::schema::EMBEDDING_DIM as usize];
    let under_a = search_pinned_semantic(&index_root, &query, &profile_a_id, 10, &NO_FILTER)
        .await
        .unwrap();
    assert_eq!(
        under_a.eligible_rows, 0,
        "the superseded space must hold no rows"
    );
    let under_b = search_pinned_semantic(&index_root, &query, &profile_b_id, 10, &NO_FILTER)
        .await
        .unwrap();
    assert!(
        under_b.eligible_rows > 0,
        "the current space must serve the vectors B published"
    );
}

// ---------------------------------------------------------------------------
// Failure recording, job identity across reconcile, and retirement
// ---------------------------------------------------------------------------

/// The pending jobs the published manifest currently names.
async fn pending_jobs(
    index_root: &Path,
) -> BTreeMap<String, forgeplan_core::constellation::jobs::EmbeddingJob> {
    let (_descriptor, manifest) = pinned_descriptor_and_manifest(index_root).await;
    manifest.pending_jobs
}

/// Admission must reach compute without first marking the whole corpus
/// pending. Unselected units stay eligible and complete in later batches.
#[tokio::test]
async fn admission_bounds_pending_writes_and_later_batches_finish_the_corpus() {
    resume_admission();
    let tmp = TempDir::new().unwrap();
    for dir in ARTIFACT_DIRS {
        std::fs::create_dir_all(tmp.path().join(dir)).unwrap();
    }
    let batch_limit = forgeplan_core::constellation::facade::MAX_PREPARED_BATCH;
    // More than one batch even when no cached tokenizer can produce sections.
    for ordinal in 1..=batch_limit + 1 {
        std::fs::write(
            tmp.path()
                .join("notes")
                .join(format!("NOTE-{ordinal:03}-queue.md")),
            format!(
                "---\nid: NOTE-{ordinal:03}\nkind: note\nstatus: draft\n\
                 title: Bounded queue {ordinal}\n---\n\nDistinct input {ordinal}.\n"
            ),
        )
        .unwrap();
    }
    let capability = capability_for(&tmp);
    let index_root = capability.index_root();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();
    let (_, manifest) = pinned_descriptor_and_manifest(index_root).await;
    let expected_keys: std::collections::BTreeSet<String> = manifest
        .source_entries
        .iter()
        .map(|entry| entry.canonical_key.clone())
        .chain(
            manifest
                .section_bindings
                .iter()
                .map(|section| section.section_key.clone()),
        )
        .collect();
    assert!(expected_keys.len() > batch_limit);
    let profile = profile_a();
    let mut prepared = prepare_embedding_batch(&capability, &profile, deadline())
        .await
        .unwrap();
    let mut completed = std::collections::BTreeSet::new();
    loop {
        let batch_keys: std::collections::BTreeSet<String> = prepared
            .inputs
            .iter()
            .map(|input| input.job.canonical_key.clone())
            .collect();
        assert!(batch_keys.len() <= batch_limit);
        let durable_keys: std::collections::BTreeSet<String> =
            pending_jobs(index_root).await.into_keys().collect();
        assert_eq!(
            durable_keys, batch_keys,
            "a fresh batch must not write pending state for deferred units"
        );
        let outcome = complete_embedding_batch(
            &capability,
            &prepared,
            &vectors_for(&prepared, 0.25),
            deadline(),
        )
        .await
        .unwrap();
        assert!(outcome.stale.is_empty());
        assert!(outcome.cannot_judge.is_empty());
        assert_eq!(
            outcome
                .published
                .iter()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>(),
            batch_keys
        );
        for key in outcome.published {
            assert!(completed.insert(key), "a completed unit was admitted again");
        }
        prepared = prepare_embedding_batch(&capability, &profile, deadline())
            .await
            .unwrap();
        if prepared.inputs.is_empty() {
            break;
        }
    }
    assert_eq!(completed, expected_keys, "deferred units must not be lost");
    assert!(pending_jobs(index_root).await.is_empty());
    let coverage = search_pinned_semantic(
        index_root,
        &vec![0.25; forgeplan_core::db::schema::EMBEDDING_DIM as usize],
        &profile.profile_id(),
        1,
        &NO_FILTER,
    )
    .await
    .unwrap();
    assert_eq!(coverage.eligible_rows, expected_keys.len());
}

/// A reconcile between two admissions must not remint the pending jobs.
///
/// Before this, every reconcile published a fresh generation because every
/// admission re-derived `captured_input_ref` from a new generation id and
/// reset `attempt` to zero. A permanently failing key therefore looked brand
/// new on every pass, and a batch prepared before the reconcile no longer
/// matched the pending set — so its completion was refused even though
/// nothing about its work had changed.
#[tokio::test]
async fn an_identical_pending_job_survives_a_reconcile_unchanged() {
    resume_admission();
    let tmp = linked_workspace();
    let capability = capability_for(&tmp);
    let index_root = capability.index_root().to_path_buf();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    let profile_a = profile_a();
    let first = prepare_embedding_batch(&capability, &profile_a, deadline())
        .await
        .unwrap();
    let before = pending_jobs(&index_root).await;
    assert!(
        !before.is_empty(),
        "admission must record the work it handed out"
    );

    // An unrelated maintenance pass, with no source edit at all.
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    let second = prepare_embedding_batch(&capability, &profile_a, deadline())
        .await
        .unwrap();
    let after = pending_jobs(&index_root).await;

    for (key, job) in &before {
        let still = after
            .get(key)
            .unwrap_or_else(|| panic!("`{key}` lost its pending job across a reconcile"));
        assert_eq!(
            still, job,
            "`{key}` describes the same work, so its job must be carried forward byte for byte"
        );
    }
    assert!(
        !second.inputs.is_empty(),
        "the outstanding work must still be admitted: {second:?}"
    );
    for input in &second.inputs {
        assert!(
            first.accepts(&input.job.canonical_key, &input.job),
            "`{}` was re-minted for the same work; the pending token must be reused",
            input.job.canonical_key
        );
    }

    // The batch prepared before the reconcile still completes: its token is
    // still the one this index has pending.
    let outcome =
        complete_embedding_batch(&capability, &first, &vectors_for(&first, 0.25), deadline())
            .await
            .expect("a batch prepared before an unrelated reconcile must still complete");
    assert!(
        !outcome.published.is_empty(),
        "the earlier batch's vectors must still land: {outcome:?}"
    );
    assert!(
        outcome.stale.is_empty(),
        "an unrelated reconcile must not stale an outstanding batch: {outcome:?}"
    );
}

/// A failed attempt is recorded, not merely counted.
///
/// After the model fails, the row is `failed` with a `retry_after`, and the
/// durable pending job carries an advanced attempt. That pair is what stops
/// the next pass from re-spending the same inference immediately — and what
/// makes "this key does not embed" answerable from the index.
#[tokio::test]
async fn a_failed_attempt_is_recorded_with_a_retry_delay_and_an_advanced_attempt() {
    resume_admission();
    let tmp = linked_workspace();
    let capability = capability_for(&tmp);
    let index_root = capability.index_root().to_path_buf();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    let profile_a = profile_a();
    let prepared = prepare_embedding_batch(&capability, &profile_a, deadline())
        .await
        .unwrap();
    let target = prepared
        .inputs
        .iter()
        .find(|input| input.job.canonical_key == "ADR-002")
        .expect("ADR-002 must be admitted")
        .job
        .canonical_key
        .clone();

    let failures: Vec<forgeplan_core::constellation::jobs::EmbeddingJob> = prepared
        .inputs
        .iter()
        .map(|input| input.job.clone())
        .collect();
    let outcome = fail_embedding_batch(
        &capability,
        &prepared,
        &failures,
        "the test's model refused every input",
        deadline(),
    )
    .await
    .unwrap();

    assert!(
        outcome.failed.contains(&target),
        "the failed key must be reported as failed: {outcome:?}"
    );
    assert!(
        outcome.retry_after.is_some(),
        "a recorded failure must say when the key may be tried again: {outcome:?}"
    );

    // The row is `failed`, still pending, and no vector was invented for it.
    let rows = pinned_artifact_rows(&index_root).await;
    assert_eq!(
        rows[&target].semantic_state, "failed",
        "the row must say why it has no vector"
    );
    assert!(
        !rows[&target].has_embedding,
        "a failed attempt must never publish a vector"
    );

    // The job is still pending — a failure is not a retirement — and its
    // attempt has advanced past the admission's zero.
    let jobs = pending_jobs(&index_root).await;
    let job = jobs
        .get(&target)
        .unwrap_or_else(|| panic!("`{target}` must stay in the durable pending set"));
    assert_eq!(job.attempt, 1, "the failure must count as an attempt");
    assert!(
        job.retry_after.is_some(),
        "the durable job must carry the retry delay: {job:?}"
    );

    // The retry delay holds: the next admission does not hand the same inputs
    // straight back, so a drain cannot spin on a permanently failing model.
    let next = prepare_embedding_batch(&capability, &profile_a, deadline())
        .await
        .unwrap();
    assert!(
        next.inputs.is_empty(),
        "a key inside its retry delay must not be re-admitted: {:?}",
        next.inputs
            .iter()
            .map(|input| &input.job.canonical_key)
            .collect::<Vec<_>>()
    );

    // …and the job is still there when the delay passes, not dropped.
    let jobs_after = pending_jobs(&index_root).await;
    assert!(
        jobs_after.contains_key(&target),
        "a delayed key keeps its place in the durable work list"
    );
}

/// A failure record must never mark a row `ready` or resurrect a vector.
#[tokio::test]
async fn a_completion_still_lands_after_a_recorded_failure_on_a_different_key() {
    resume_admission();
    let tmp = linked_workspace();
    let capability = capability_for(&tmp);
    let index_root = capability.index_root().to_path_buf();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    let profile_a = profile_a();
    let first = prepare_embedding_batch(&capability, &profile_a, deadline())
        .await
        .unwrap();

    // Fail exactly one key.
    let failures: Vec<forgeplan_core::constellation::jobs::EmbeddingJob> = first
        .inputs
        .iter()
        .filter(|input| input.job.canonical_key == "ADR-002")
        .map(|input| input.job.clone())
        .collect();
    fail_embedding_batch(
        &capability,
        &first,
        &failures,
        "one key refused",
        deadline(),
    )
    .await
    .unwrap();

    let rows = pinned_artifact_rows(&index_root).await;
    assert_eq!(rows["ADR-002"].semantic_state, "failed");
    assert!(
        rows.iter()
            .filter(|(key, _)| key.as_str() != "ADR-002")
            .all(|(_, row)| !row.has_embedding),
        "recording one failure must not publish anything for the others"
    );

    // The remaining keys of the same batch still complete.
    let live: Vec<forgeplan_core::constellation::jobs::ComputedEmbedding> =
        vectors_for(&first, 0.25)
            .into_iter()
            .filter(|result| result.job.canonical_key != "ADR-002")
            .collect();
    assert!(!live.is_empty(), "the batch has more than one key");
    let outcome = complete_embedding_batch(&capability, &first, &live, deadline())
        .await
        .unwrap();
    assert_eq!(
        outcome.published.len(),
        live.len(),
        "every un-failed key must land: {outcome:?}"
    );

    let rows = pinned_artifact_rows(&index_root).await;
    assert_eq!(
        rows["ADR-002"].semantic_state, "failed",
        "the failed key must stay failed"
    );
    assert!(!rows["ADR-002"].has_embedding);
    assert!(
        rows["prd-alpha"].has_embedding,
        "the live keys carry their vectors"
    );
}

/// A pending job for a deleted document is retired, not carried forever.
#[tokio::test]
async fn a_pending_job_for_a_deleted_document_is_retired() {
    resume_admission();
    let tmp = linked_workspace();
    let capability = capability_for(&tmp);
    let index_root = capability.index_root().to_path_buf();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    let profile_a = profile_a();
    prepare_embedding_batch(&capability, &profile_a, deadline())
        .await
        .unwrap();
    let before = pending_jobs(&index_root).await;
    assert!(
        before.contains_key("ADR-002"),
        "the artifact and its section must both be pending: {before:?}"
    );
    assert!(
        before.keys().any(|key| key.starts_with("ADR-002#s")),
        "the section row of ADR-002 must be pending too: {before:?}"
    );

    // The document and its sections are deleted, then reconciled away.
    std::fs::remove_file(tmp.path().join("adrs").join("ADR-002-storage.md")).unwrap();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    // A drain of what is left retires the jobs whose keys no longer exist.
    prepare_embedding_batch(&capability, &profile_a, deadline())
        .await
        .unwrap();
    let after = pending_jobs(&index_root).await;
    assert!(
        !after.contains_key("ADR-002"),
        "a deleted document's pending job must be retired: {after:?}"
    );
    assert!(
        !after.keys().any(|key| key.starts_with("ADR-002#s")),
        "its sections' pending jobs must be retired with it: {after:?}"
    );
    assert!(
        after.contains_key("prd-alpha"),
        "the surviving documents keep their pending work: {after:?}"
    );
}

/// A mixed batch completes the live keys and reports the superseded one.
///
/// One document is edited while a batch is outstanding, so the next admission
/// mints a **new** job for that key while the other keys keep theirs. The first
/// batch's results then straddle the line: the edited key's result was computed
/// for bytes the index no longer serves, and the rest are still live. The
/// superseded vector must not land, it must not cost the rest of the batch
/// their vectors, and the newer admission's job must survive untouched.
#[tokio::test]
async fn a_mixed_batch_completes_the_live_keys_and_reports_the_superseded_one() {
    resume_admission();
    let tmp = linked_workspace();
    let capability = capability_for(&tmp);
    let index_root = capability.index_root().to_path_buf();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    let profile_a = profile_a();
    let first = prepare_embedding_batch(&capability, &profile_a, deadline())
        .await
        .unwrap();
    let first_results = vectors_for(&first, 0.25);
    let live_key = first
        .inputs
        .iter()
        .find(|input| input.job.canonical_key == "prd-alpha")
        .expect("prd-alpha must be admitted")
        .job
        .canonical_key
        .clone();

    // SPEC-009's body moves, so its binding — and therefore its job — changes
    // at the next admission. `prd-alpha` is untouched.
    std::fs::write(
        tmp.path().join("specs").join("SPEC-009-view.md"),
        "---\nid: SPEC-009\nkind: spec\nstatus: draft\ntitle: View\nlinks:\n  - target: prd-alpha\n    relation: based_on\n---\n\nView body, edited.\n",
    )
    .unwrap();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    let second = prepare_embedding_batch(&capability, &profile_a, deadline())
        .await
        .unwrap();
    let superseded_key = first
        .inputs
        .iter()
        .find(|input| input.job.canonical_key.starts_with("SPEC-009#s"))
        .expect("the first batch embeds SPEC-009's sections")
        .job
        .canonical_key
        .clone();
    assert!(
        second
            .job_for(&superseded_key)
            .is_some_and(|job| !first.accepts(&superseded_key, job)),
        "the edited key must have been re-admitted under a new job"
    );

    // Complete the *first* batch in full.
    let outcome = complete_embedding_batch(&capability, &first, &first_results, deadline())
        .await
        .expect("a superseded result must not abort the batch");
    assert!(
        outcome.stale.contains(&superseded_key),
        "the key whose bytes moved is stale: {outcome:?}"
    );
    assert!(
        outcome.published.contains(&live_key),
        "the keys the index still has pending must land: {outcome:?}"
    );
    assert!(
        !outcome
            .published
            .iter()
            .any(|key| key.starts_with("SPEC-009#s")),
        "no vector computed from the pre-edit bytes may land: {outcome:?}"
    );

    // The newer admission's job for the superseded key was **not** retired by
    // the older batch's completion: a completion may only retire its own jobs.
    let jobs = pending_jobs(&index_root).await;
    assert!(
        jobs.contains_key(&superseded_key),
        "the newer admission's outstanding work must survive an older completion"
    );
}

// ---------------------------------------------------------------------------
// Admission may not move the index out of the space it serves
// ---------------------------------------------------------------------------

/// A caller holding a superseded profile cannot start a backward migration.
///
/// The failure this guards: a `watch` tick or a `serve` pass resolves a
/// profile, stalls, another operator migrates the index onto new weights, and
/// the stalled caller then runs again. Without the check, that run re-points
/// every eligible row back at the old space, republishes `CURRENT` onto it,
/// and begins a full re-embed of the corpus into weights that were replaced.
#[tokio::test]
async fn admission_refuses_a_profile_the_index_no_longer_serves() {
    resume_admission();
    let tmp = linked_workspace();
    let capability = capability_for(&tmp);
    let index_root = capability.index_root().to_path_buf();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    let profile_a = profile_a();
    let profile_a_id = profile_a.profile_id();
    let profile_b = profile_b();
    let profile_b_id = profile_b.profile_id();

    // The index is on profile-a and every row is ready in that space.
    let first = embed_once(&capability, &profile_a, 0.25).await.unwrap();
    assert!(!first.published.is_empty(), "{first:?}");

    // The operator migrates onto profile-b and drains it. Every row now
    // carries B, and `CURRENT` names B.
    assert!(
        adopt_embedding_profile(&capability, &profile_b, deadline())
            .await
            .unwrap()
    );
    let migrated = embed_once(&capability, &profile_b, 0.5).await.unwrap();
    assert!(!migrated.published.is_empty(), "{migrated:?}");

    // The stalled caller runs again, still holding profile-a.
    let stale_caller = prepare_embedding_batch(&capability, &profile_a, deadline()).await;
    let err = match stale_caller {
        Err(e) => e,
        Ok(prepared) => panic!(
            "a caller holding `{profile_a_id}` was admitted into an index serving \
             `{profile_b_id}`: {prepared:?}"
        ),
    };
    assert_eq!(
        err.code(),
        forgeplan_core::constellation::DiagnosticCode::ProfileIncompatible,
        "the refusal must name the profile mismatch: {err}"
    );

    // And the refusal changed nothing: the rows are still on B, and no
    // pending job for A was minted.
    let rows = pinned_artifact_rows(&index_root).await;
    for (key, row) in &rows {
        assert_eq!(
            row.profile_id.as_deref(),
            Some(profile_b_id.as_str()),
            "{key} must still carry the space the index serves"
        );
    }
    assert!(
        pending_jobs(&index_root)
            .await
            .values()
            .all(|job| job.profile_id == profile_b_id),
        "a refused admission must mint no pending job in the stale space"
    );
}

/// Adopting the space an index already serves is a no-op, not a publish.
///
/// Idempotence is what lets the migration callers (`forgeplan embed`,
/// `forgeplan index --embed-profile`) call this unconditionally before every
/// drain instead of having to ask first whether a migration is needed.
#[tokio::test]
async fn adopting_the_space_already_served_moves_nothing() {
    resume_admission();
    let tmp = linked_workspace();
    let capability = capability_for(&tmp);
    let index_root = capability.index_root().to_path_buf();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    let profile_a = profile_a();
    let first = embed_once(&capability, &profile_a, 0.25).await.unwrap();
    assert!(!first.published.is_empty(), "{first:?}");

    let before = forgeplan_core::constellation::storage::read_current(&index_root)
        .unwrap()
        .expect("a published generation exists");

    assert!(
        !adopt_embedding_profile(&capability, &profile_a, deadline())
            .await
            .unwrap(),
        "adopting the served space must report that it moved nothing"
    );

    let after = forgeplan_core::constellation::storage::read_current(&index_root)
        .unwrap()
        .expect("a published generation still exists");
    assert_eq!(
        after, before,
        "a no-op adoption must leave CURRENT byte-identical"
    );
}

/// A late completion cannot revert an adoption that no admission followed.
///
/// The window [`a_late_result_from_a_superseded_admission_cannot_move_the_row_back`]
/// closes by admitting B, which replaces A's pending jobs. The bare window is
/// the one an operator actually leaves behind: adoption is a cheap named act,
/// and the drain that completes under the old space may already be in flight.
/// `adopt_embedding_profile` writes no row, mints no job and leaves
/// `pending_jobs` alone, so A's jobs are still pending, A's rows still store A
/// and the config digest never changed — every per-key check passes. Only the
/// descriptor's served profile says the migration happened, so that is what
/// must be read as a fence before the first write.
#[tokio::test]
async fn a_late_completion_after_an_adoption_cannot_move_the_pointer_back() {
    resume_admission();
    let tmp = linked_workspace();
    let capability = capability_for(&tmp);
    let index_root = capability.index_root().to_path_buf();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    let profile_a = profile_a();
    let profile_a_id = profile_a.profile_id();
    let profile_b_id = profile_b().profile_id();

    // A's admission is in flight: its jobs are durable, its rows are pending.
    // The operator migrates to B and then, as the process ends, never drains B.
    let admitted_a = prepare_embedding_batch(&capability, &profile_a, deadline())
        .await
        .unwrap();
    assert!(
        !admitted_a.inputs.is_empty(),
        "A's admission must have work in flight"
    );
    assert!(
        adopt_embedding_profile(&capability, &profile_b(), deadline())
            .await
            .unwrap(),
        "the operator's explicit migration must move CURRENT"
    );
    let before_pointer = std::fs::read(index_root.join("CURRENT.json")).unwrap();
    let before_rows = pinned_artifact_rows(&index_root).await;

    // A's results arrive after the migration, with **no** intervening
    // admission. This is the exact shape the superseded test does not cover.
    let err = complete_embedding_batch(
        &capability,
        &admitted_a,
        &vectors_for(&admitted_a, 0.25),
        deadline(),
    )
    .await
    .expect_err("a batch from a replaced space must be refused outright");

    assert_eq!(
        err.code(),
        forgeplan_core::constellation::DiagnosticCode::ProfileIncompatible,
        "the refusal must name the space mismatch: {err}"
    );
    let served = forgeplan_core::constellation::storage::read_current(&index_root)
        .unwrap()
        .expect("a published generation exists");
    assert_eq!(
        served.profile_id.as_deref(),
        Some(profile_b_id.as_str()),
        "a refused completion must leave CURRENT naming the space the operator adopted"
    );
    assert_eq!(
        std::fs::read(index_root.join("CURRENT.json")).unwrap(),
        before_pointer,
        "a refused completion must not publish another generation"
    );

    // Adoption leaves A's pending bindings in place. A refused completion must
    // leave those rows unchanged, not pretend adoption already rewrote them.
    let rows = pinned_artifact_rows(&index_root).await;
    assert!(
        rows.values().all(|row| !row.has_embedding),
        "a refused completion must publish no vector: {rows:?}"
    );
    assert_eq!(
        rows, before_rows,
        "a refused completion must not alter rows"
    );

    // And the work is still outstanding under the space that owns it, so the
    // next drain picks it up rather than the index losing it.
    let jobs = pending_jobs(&index_root).await;
    assert!(
        jobs.values().any(|job| job.profile_id == profile_a_id),
        "the fenced batch's jobs must survive so the drain can re-admit them: {jobs:?}"
    );
}

/// The same window through the failure path.
///
/// `fail_embedding_batch` writes rows, advances the durable attempt and
/// republishes a manifest, so the fence is not optional there: without it the
/// record lands under an input digest the index no longer serves, resets the
/// backoff of a row the operator has already migrated, and moves `CURRENT`
/// back onto the retired space — the same silent revert as the completion
/// path, reached by the "honest" bookkeeping branch instead.
#[tokio::test]
async fn a_late_failure_after_an_adoption_writes_nothing() {
    resume_admission();
    let tmp = linked_workspace();
    let capability = capability_for(&tmp);
    let index_root = capability.index_root().to_path_buf();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    let profile_a = profile_a();
    let profile_b_id = profile_b().profile_id();
    let admitted_a = prepare_embedding_batch(&capability, &profile_a, deadline())
        .await
        .unwrap();
    assert!(
        !admitted_a.inputs.is_empty(),
        "A's admission must have work in flight"
    );
    let before_failure = pending_jobs(&index_root).await;

    assert!(
        adopt_embedding_profile(&capability, &profile_b(), deadline())
            .await
            .unwrap(),
        "the operator's explicit migration must move CURRENT"
    );

    let failures: Vec<forgeplan_core::constellation::jobs::EmbeddingJob> = admitted_a
        .inputs
        .iter()
        .map(|input| input.job.clone())
        .collect();
    let err = fail_embedding_batch(
        &capability,
        &admitted_a,
        &failures,
        "the model this drain used has since been replaced",
        deadline(),
    )
    .await
    .expect_err("a failure record from a replaced space must be refused outright");
    assert_eq!(
        err.code(),
        forgeplan_core::constellation::DiagnosticCode::ProfileIncompatible,
        "the refusal must name the space mismatch: {err}"
    );

    let served = forgeplan_core::constellation::storage::read_current(&index_root)
        .unwrap()
        .expect("a published generation exists");
    assert_eq!(
        served.profile_id.as_deref(),
        Some(profile_b_id.as_str()),
        "a refused failure record must leave CURRENT on the adopted space"
    );

    // No row was marked failed, and no attempt was spent — the durable record
    // is exactly what it was before the refused call.
    let rows = pinned_artifact_rows(&index_root).await;
    assert!(
        rows.values().all(|row| row.semantic_state == "pending"),
        "a refused failure record must move no row to failed: {rows:?}"
    );
    assert_eq!(
        pending_jobs(&index_root).await,
        before_failure,
        "a refused failure record must advance no attempt"
    );
}

/// An incomplete capture must never retire a pending job.
///
/// Retirement drops work the index can no longer describe. Under a **complete**
/// capture that is right. Under an incomplete one it is a data-loss bug: the
/// artifact is absent from the capture because it is momentarily unreadable,
/// while its row is still `pending` with its profile set — so the job is
/// destroyed, and the next admission mints the key a fresh job at
/// `attempt: 0` with no `retry_after`. A key that had earned a backoff over
/// several attempts is retried immediately on the very model that failed.
#[tokio::test]
async fn a_pending_job_survives_an_incomplete_capture() {
    resume_admission();
    let tmp = linked_workspace();
    let capability = capability_for(&tmp);
    let index_root = capability.index_root().to_path_buf();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    // One attempt is spent and recorded: the row is `failed`, the durable job
    // carries `attempt: 1` and a `retry_after`. That history is the thing a
    // partial capture must not be able to destroy.
    let prepared = prepare_embedding_batch(&capability, &profile_a(), deadline())
        .await
        .unwrap();
    let target = prepared
        .inputs
        .iter()
        .find(|input| input.job.canonical_key == "ADR-002")
        .expect("ADR-002 must be admitted")
        .job
        .canonical_key
        .clone();
    fail_embedding_batch(
        &capability,
        &prepared,
        &[prepared
            .inputs
            .iter()
            .find(|input| input.job.canonical_key == "ADR-002")
            .expect("ADR-002 must be admitted")
            .job
            .clone()],
        "the model refused this key",
        deadline(),
    )
    .await
    .unwrap();
    let spent = pending_jobs(&index_root).await;
    let job_before = spent
        .get(&target)
        .unwrap_or_else(|| panic!("`{target}` must be durably pending"))
        .clone();
    assert_eq!(job_before.attempt, 1, "{job_before:?}");
    assert!(job_before.retry_after.is_some(), "{job_before:?}");

    // The next capture cannot read this artifact: unparseable frontmatter
    // makes it `unusable`, so it is absent from `entries` while the row stays
    // exactly where the failed attempt left it.
    std::fs::write(
        tmp.path().join("adrs").join("ADR-002-storage.md"),
        "---\nid: [unclosed\n",
    )
    .unwrap();

    // Admission runs over the incomplete capture. It must not retire the job,
    // and must not mint a replacement either.
    prepare_embedding_batch(&capability, &profile_a(), deadline())
        .await
        .unwrap();

    let jobs_after = pending_jobs(&index_root).await;
    let job_after = jobs_after
        .get(&target)
        .unwrap_or_else(|| {
            panic!(
                "an incomplete capture retired `{target}`: {jobs_after:?}. Partial coverage is \
                 a diagnostic, never a confirmed delete."
            )
        })
        .clone();
    // Byte-identical, not merely present: a replacement job would be minted at
    // `attempt: 0` with no `retry_after`, which is the backoff this test
    // exists to protect.
    assert_eq!(
        job_after, job_before,
        "an incomplete capture must carry the pending job forward untouched"
    );
}
