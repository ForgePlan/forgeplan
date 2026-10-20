//! Two pinned-search contracts that a caller reads as one number and one
//! result list, and that the implementation once got wrong in the same way:
//! both were computed *after* a cut that had already been applied.
//!
//! * [`coverage_is_counted_before_the_semantic_limit`] — `eligible_rows` is
//!   how much of the corpus is embedded, so counting the rows a `limit`
//!   admitted would let a fully-unembedded corpus report itself as covered.
//! * [`a_kind_filter_is_applied_before_the_limit_not_after`] — `search x
//!   --kind prd --limit 10` must return PRDs that rank below the top ten ADRs,
//!   not "no results".
//!
//! Everything runs through `maintain_index` and the real
//! `prepare_embedding_batch` → compute → `complete_embedding_batch` cycle on
//! temporary workspaces. No model, no CLI: each "vector" is a constant fill,
//! which is enough to prove *which* rows are eligible and *which* rows a
//! filter admits, and is never a similarity-quality claim.

use forgeplan_core::constellation::capture::CaptureLimits;
use forgeplan_core::constellation::facade::{
    EmbeddingBatchOutcome, MaintainCapability, PinnedFilter, complete_embedding_batch,
    maintain_index, prepare_embedding_batch, resume_admission, search_pinned_lexical,
    search_pinned_semantic,
};
use forgeplan_core::constellation::jobs::{
    DEFAULT_BODY_PREFIX_CHARS, DEFAULT_MAX_TOKENS, DEFAULT_MODEL_REPO, EmbeddingProfile,
};
use forgeplan_core::constellation::lance_tables::open_active_tables_readonly;
use forgeplan_core::constellation::manifest::{
    BootstrapState, ProtocolMarker, STORAGE_SCHEME_FILE_OBJECT_STORE,
};
use forgeplan_core::constellation::{ConstellationError, FORMAT_VERSION};
use forgeplan_core::workspace::ARTIFACT_DIRS;
use tempfile::TempDir;

fn deadline() -> std::time::Instant {
    std::time::Instant::now() + std::time::Duration::from_secs(120)
}

const NO_FILTER: PinnedFilter = PinnedFilter {
    kind: None,
    status: None,
};

/// A profile whose dimensions match the table, so admission accepts it.
fn profile() -> EmbeddingProfile {
    EmbeddingProfile {
        model_repo: DEFAULT_MODEL_REPO.to_string(),
        weights_revision: "test-revision".to_string(),
        weights_checksums: vec!["weights-sha".to_string()],
        tokenizer_checksum: "tok-sha".to_string(),
        tokenizer_config_checksum: "tokcfg-sha".to_string(),
        dimensions: forgeplan_core::db::schema::EMBEDDING_DIM as u32,
        // Spelled out rather than read from `facade::ENGINE_VERSION`: that
        // constant is `#[cfg(feature = "semantic-search")]`, and these tests
        // run in every feature configuration — the admission and completion
        // contract they exercise is not feature-gated. A profile's engine
        // version only has to be *some* consistent string for these purposes;
        // it is what a real profile hashes to matter, and nothing here hashes
        // against a published one.
        engine_version: "tract-onnx-0.23".to_string(),
        pooling: "cls".to_string(),
        normalization: "l2".to_string(),
        query_prefix: String::new(),
        document_prefix: String::new(),
        body_prefix_chars: DEFAULT_BODY_PREFIX_CHARS,
        max_tokens: DEFAULT_MAX_TOKENS,
        padding: "none".to_string(),
        truncation: "longest_first".to_string(),
    }
}

/// A workspace with `adrs` ADRs and `prds` PRDs, every one of them carrying
/// the shared body text the lexical search matches on.
///
/// The two kinds must be otherwise identical for the filter test to mean
/// anything: if the ADRs merely matched the query better, a post-limit filter
/// would return the same empty list a correct one does, and the test would
/// pass without proving the ordering rule.
fn mixed_workspace(adrs: usize, prds: usize) -> TempDir {
    let tmp = TempDir::new().unwrap();
    for dir in ARTIFACT_DIRS {
        std::fs::create_dir_all(tmp.path().join(dir)).unwrap();
    }
    for index in 0..adrs {
        std::fs::write(
            tmp.path()
                .join("adrs")
                .join(format!("ADR-{index:03}-shared.md")),
            format!(
                "---\nid: ADR-{index:03}\nkind: adr\nstatus: active\ntitle: Shared {index}\n\
                 ---\n\nshared body text.\n"
            ),
        )
        .unwrap();
    }
    for index in 0..prds {
        std::fs::write(
            tmp.path()
                .join("prds")
                .join(format!("PRD-{index:03}-shared.md")),
            format!(
                "---\nid: PRD-{index:03}\nkind: prd\nstatus: draft\ntitle: Shared {index}\n\
                 ---\n\nshared body text.\n"
            ),
        )
        .unwrap();
    }
    tmp
}

fn capability_for(tmp: &TempDir) -> MaintainCapability {
    MaintainCapability::for_strict_maintenance(tmp.path().canonicalize().unwrap())
}

/// Drain until nothing is eligible, then return what landed overall.
///
/// One admission hands back at most `MAX_PREPARED_BATCH` inputs, so a corpus
/// larger than that needs several cycles; a test that drained once would
/// measure the batch bound rather than what it means to measure.
async fn embed_everything(
    capability: &MaintainCapability,
    profile: &EmbeddingProfile,
    fill: f32,
) -> usize {
    let mut published = 0usize;
    loop {
        let prepared = match prepare_embedding_batch(capability, profile, deadline()).await {
            Ok(prepared) => prepared,
            Err(e) => panic!("admission failed: {e}"),
        };
        if prepared.inputs.is_empty() {
            return published;
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
        let outcome: EmbeddingBatchOutcome =
            complete_embedding_batch(capability, &prepared, &results, deadline())
                .await
                .expect("completion must land for an unchanged corpus");
        published += outcome.published.len();
    }
}

fn query_vector(fill: f32) -> Vec<f32> {
    vec![fill; forgeplan_core::db::schema::EMBEDDING_DIM as usize]
}

/// `eligible_rows` is coverage, and coverage does not shrink because the
/// caller asked for fewer results.
///
/// The defect this pins: `eligible_rows` was summed from the batches an ANN
/// query *returned*, and that query carries `limit`. So a corpus of 8 embedded
/// rows asked for 2 reported `eligible_rows == 2`, which a client reads as
/// "the corpus is embedded" — leaving 6 rows permanently unembedded while the
/// answer claims completeness.
#[tokio::test]
async fn coverage_is_counted_before_the_semantic_limit() {
    resume_admission();
    let tmp = mixed_workspace(4, 4);
    let capability = capability_for(&tmp);
    let index_root = capability.index_root().to_path_buf();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    let profile = profile();
    let embedded = embed_everything(&capability, &profile, 0.25).await;
    assert!(
        embedded >= 8,
        "the whole corpus must be embedded: {embedded}"
    );

    let query = query_vector(0.25);

    // The narrow ask and the wide ask rank the same rows; only how many come
    // back differs.
    let narrow = search_pinned_semantic(&index_root, &query, &profile.profile_id(), 2, &NO_FILTER)
        .await
        .unwrap();
    let wide = search_pinned_semantic(&index_root, &query, &profile.profile_id(), 100, &NO_FILTER)
        .await
        .unwrap();

    assert_eq!(
        narrow.hits.len(),
        2,
        "a limit of 2 must return 2 hits, whatever coverage is"
    );
    assert_eq!(
        narrow.eligible_rows, wide.eligible_rows,
        "eligible_rows must not depend on the limit: it is coverage, not a page size"
    );
    assert!(
        narrow.eligible_rows > narrow.hits.len(),
        "the corpus holds more embedded rows than the limit returned, so coverage must say so \
         (got eligible_rows={}, hits={})",
        narrow.eligible_rows,
        narrow.hits.len()
    );
    assert_eq!(
        narrow.eligible_rows, embedded,
        "every embedded row is eligible to rank — artifacts and their sections both count"
    );

    // And the reported coverage really is the whole ready corpus, not an
    // artifact-only subset: sections are vectors too.
    assert!(
        narrow.eligible_rows > 8,
        "sections carry ready vectors and must be counted: {}",
        narrow.eligible_rows
    );
}

/// `--kind` narrows the ranking, it does not filter an already-cut page.
///
/// The failure this pins: 12 ADRs and 3 PRDs all match `shared`, the query
/// asked for 10, and the caller filtered the 10 returned rows afterwards. All
/// ten were ADRs, so all ten were discarded and the command printed "no
/// results" — for a corpus that contains matching PRDs.
#[tokio::test]
async fn a_kind_filter_is_applied_before_the_limit_not_after() {
    resume_admission();
    // More ADRs than the limit, fewer PRDs than the limit: a post-limit filter
    // can only ever return an empty list here.
    let tmp = mixed_workspace(12, 3);
    let capability = capability_for(&tmp);
    let index_root = capability.index_root().to_path_buf();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    let limit = 10usize;

    // Lexical: `shared` matches every document's title and body.
    let prds = PinnedFilter {
        kind: Some("prd".to_string()),
        status: None,
    };
    let lexical = search_pinned_lexical(&index_root, "shared", limit, &prds)
        .await
        .unwrap();
    assert_eq!(
        lexical.hits.len(),
        3,
        "every PRD matches and none was cut by the limit: {:?}",
        lexical.hits
    );
    assert!(
        lexical
            .hits
            .iter()
            .all(|hit| hit.kind.as_deref() == Some("prd")),
        "a filtered search returns only what passed: {:?}",
        lexical.hits
    );

    // Unfiltered, the limit really is smaller than the corpus — so the test
    // above could not have passed by having everything fit anyway.
    let unfiltered = search_pinned_lexical(&index_root, "shared", limit, &NO_FILTER)
        .await
        .unwrap();
    assert_eq!(
        unfiltered.hits.len(),
        limit,
        "the unfiltered search must hit the limit, or this test proves nothing: {:?}",
        unfiltered.hits
    );

    // The same rule on the semantic channel, where the filter has to reach a
    // **section** row through its parent — a section row stores no kind of its
    // own, so a filter applied after ranking could not have excluded it.
    let profile = profile();
    embed_everything(&capability, &profile, 0.25).await;
    let query = query_vector(0.25);
    let semantic = search_pinned_semantic(&index_root, &query, &profile.profile_id(), limit, &prds)
        .await
        .unwrap();
    assert!(
        !semantic.hits.is_empty(),
        "the filtered semantic search must return the PRDs that match"
    );
    assert!(
        semantic.hits.len() <= limit,
        "a filter cannot add rows past the limit: {}",
        semantic.hits.len()
    );

    // Every hit is a PRD document — including section hits, which resolve to
    // their parent.
    let prd_keys: std::collections::BTreeSet<String> = ["PRD-000", "PRD-001", "PRD-002"]
        .iter()
        .map(|key| key.to_string())
        .collect();
    for hit in &semantic.hits {
        let document = hit.parent_key.as_deref().unwrap_or(&hit.canonical_key);
        assert!(
            prd_keys.contains(document),
            "a filtered semantic search returned a non-PRD document: {document}"
        );
    }

    // Coverage under a filter describes the filtered set, so it must be
    // smaller than the unfiltered coverage rather than the same number.
    let semantic_unfiltered = search_pinned_semantic(
        &index_root,
        &query,
        &profile.profile_id(),
        limit,
        &NO_FILTER,
    )
    .await
    .unwrap();
    assert!(
        semantic.eligible_rows < semantic_unfiltered.eligible_rows,
        "a kind filter must narrow the eligible set it counts ({} vs {})",
        semantic.eligible_rows,
        semantic_unfiltered.eligible_rows
    );
}

/// A filter that admits nothing yields an empty result, not an error and not
/// a silently unfiltered one.
///
/// Both halves matter. An unfiltered fallback would answer `--kind nonexistent`
/// with the whole corpus; an error would make "no results" indistinguishable
/// from "the index is broken".
#[tokio::test]
async fn a_filter_that_admits_nothing_returns_no_hits() {
    resume_admission();
    let tmp = mixed_workspace(2, 2);
    let capability = capability_for(&tmp);
    let index_root = capability.index_root().to_path_buf();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    let missing = PinnedFilter {
        kind: Some("no-such-kind".to_string()),
        status: None,
    };

    let lexical = search_pinned_lexical(&index_root, "shared", 10, &missing)
        .await
        .unwrap();
    assert!(
        lexical.hits.is_empty(),
        "an unsatisfiable filter must return nothing, not fall back to unfiltered: {:?}",
        lexical.hits
    );

    // With sections in play, the parent set is empty and the section half must
    // be skipped rather than run against a filter nothing satisfies.
    let profile = profile();
    embed_everything(&capability, &profile, 0.25).await;
    let semantic = search_pinned_semantic(
        &index_root,
        &query_vector(0.25),
        &profile.profile_id(),
        10,
        &missing,
    )
    .await
    .unwrap();
    assert!(
        semantic.hits.is_empty(),
        "an unsatisfiable filter must return nothing from either table: {:?}",
        semantic.hits
    );
    assert_eq!(
        semantic.eligible_rows, 0,
        "no row is eligible under a filter nothing passes"
    );
}

/// A filter value is data, never structure: a quote in it cannot end the
/// literal and append a clause.
///
/// The filter reaches SQL as a predicate term, which is the one place a quote
/// in user-supplied text becomes query syntax. This asserts the refusal is a
/// clean empty result rather than an injected predicate.
#[tokio::test]
async fn a_filter_value_cannot_become_query_syntax() {
    resume_admission();
    let tmp = mixed_workspace(2, 2);
    let capability = capability_for(&tmp);
    let index_root = capability.index_root().to_path_buf();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    let injected = PinnedFilter {
        kind: Some("prd' OR '1'='1".to_string()),
        status: None,
    };
    let outcome = search_pinned_lexical(&index_root, "shared", 10, &injected).await;
    match outcome {
        Ok(result) => assert!(
            result.hits.is_empty(),
            "a quote in a filter value must be data, so nothing matches: {:?}",
            result.hits
        ),
        // A dialect that rejects the literal is also an acceptable answer; what
        // is not acceptable is returning rows the filter did not select.
        Err(e) => assert!(
            e.to_string().to_lowercase().contains("sql")
                || e.to_string().to_lowercase().contains("parse")
                || e.to_string().to_lowercase().contains("filter"),
            "an unexpected failure mode for an escaped literal: {e}"
        ),
    }
}

/// The read-only open refuses a missing dataset; it does not create one.
///
/// This is the mechanism `forgeplan embed`'s precheck runs on, and the reason
/// it exists. The writer's `open_active_tables` treats a missing sections
/// dataset as the pre-sections upgrade path and calls `create_empty` — correct
/// under an owner whose closure barrier will sync the creation, and wrong for
/// a caller holding no such barrier, where the write would land and never be
/// part of a synced set.
///
/// So: remove the sections dataset, call the read-only open, and assert both
/// that it failed with the typed "unavailable" error **and** that the dataset
/// is still absent afterwards. The second assertion is the one that matters —
/// a constructor that created the table and then reported a different error
/// would pass the first.
#[tokio::test]
async fn the_readonly_open_refuses_a_missing_dataset_without_creating_it() {
    resume_admission();
    let tmp = mixed_workspace(1, 1);
    let capability = capability_for(&tmp);
    let index_root = capability.index_root().to_path_buf();
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .unwrap();

    let descriptor = forgeplan_core::constellation::storage::read_current(&index_root)
        .unwrap()
        .unwrap();
    let manifest =
        forgeplan_core::constellation::storage::read_manifest(&index_root, &descriptor).unwrap();
    let marker = ProtocolMarker {
        protocol_version: FORMAT_VERSION,
        state: BootstrapState::Active,
        index_epoch: manifest.index_epoch,
        checkout_id: manifest.checkout_id.clone(),
        table_names: ProtocolMarker::active_table_names(),
        storage_scheme: STORAGE_SCHEME_FILE_OBJECT_STORE.to_string(),
        bootstrap_attempt: 1,
    };

    // The pre-sections upgrade path, staged honestly: the dataset is gone and
    // the manifest still pins a version for it.
    let sections_dir = index_root.join(format!(
        "{}.lance",
        forgeplan_core::constellation::manifest::TABLE_SECTIONS_V1
    ));
    assert!(
        sections_dir.exists(),
        "a freshly built index has a sections dataset to remove"
    );
    std::fs::remove_dir_all(&sections_dir).unwrap();
    assert!(
        !sections_dir.exists(),
        "the dataset must be gone before the open"
    );

    let outcome = open_active_tables_readonly(&index_root, &marker).await;
    let err = match outcome {
        Err(e) => e,
        Ok(_) => panic!("a read-only open must not create the dataset it could not find"),
    };
    assert!(
        matches!(err, ConstellationError::IndexGenerationUnavailable { .. }),
        "a missing dataset is a typed unavailability, not a silent creation: {err}"
    );

    assert!(
        !sections_dir.exists(),
        "the failed open must have left the index exactly as it found it"
    );
}
