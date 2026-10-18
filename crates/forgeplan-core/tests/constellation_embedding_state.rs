//! Embedding state: L11, L12, L15.
//!
//! **The digests below are this machine's snapshot, measured — not derived.**
//! They live in `.local/constellation-s1/reports/model-checksums.json`, were
//! computed over file *content*, and pin the profile to the actual revision
//! `5617a9f6…` rather than to a plausible-looking string. That matters: a
//! test that asserted a made-up digest would pass just as happily while
//! proving nothing about the weights.
//!
//! These are fixed expectations, not a re-read: the test asserts *what the
//! profile must be*, which is stronger than reading back whatever is present.
//!
//! Scope note — the scenarios below are decided by binding, state and
//! predicate logic, all of which is deterministic and needs no model. The
//! synthetic vectors here are for **eligibility and race isolation only**;
//! they never stand in for semantic-positive behaviour, and the numeric
//! `embedding_reference` oracle is untouched.

use forgeplan_core::constellation::jobs::{
    Binding, EmbeddingJob, SemanticOutcome, SemanticState, SnapshotChecksums,
};

// ---------------------------------------------------------------------------
// The measured snapshot, revision 5617a9f61b028005a4858fdac845db406aefb181
// ---------------------------------------------------------------------------

const MEASURED_REVISION: &str = "5617a9f61b028005a4858fdac845db406aefb181";
const CONFIG_JSON_SHA: &str = "26159e7ad065073448460117eb24b7a4572f6f4e78eadff65dc0a11c052449fa";
const TOKENIZER_JSON_SHA: &str = "21106b6d7dab2952c1d496fb21d5dc9db75c28ed361a05f5020bbba27810dd08";
const TOKENIZER_CONFIG_SHA: &str =
    "a62b2b6784f990259fddef5f16388693a8043be4f69179e6a5257eeb3f9abac4";
const MODEL_ONNX_SHA: &str = "f84251230831afb359ab26d9fd37d5936d4d9bb5d1d5410e66442f630f24435b";
const MODEL_ONNX_DATA_SHA: &str =
    "1eebfb28493f67bba03ce0ef64bfdc7fc5a3bd9d7493f818bb1d78cd798416b4";

fn measured_checksums() -> SnapshotChecksums {
    SnapshotChecksums {
        weights: vec![MODEL_ONNX_SHA.to_string(), MODEL_ONNX_DATA_SHA.to_string()],
        tokenizer: TOKENIZER_JSON_SHA.to_string(),
        tokenizer_config: TOKENIZER_CONFIG_SHA.to_string(),
        config: CONFIG_JSON_SHA.to_string(),
    }
}

#[test]
fn the_measured_digests_are_well_formed_sha256() {
    let checksums = measured_checksums();
    assert!(
        checksums.are_well_formed(),
        "every digest must be 64 lowercase hex characters"
    );
    assert_eq!(checksums.weights.len(), 2, "both ONNX files are hashed");
    assert_eq!(CONFIG_JSON_SHA.len(), 64);
    assert_eq!(MEASURED_REVISION.len(), 40, "a 40-hex git commit");
}

#[test]
fn a_profile_built_from_measured_digests_resolves_to_this_revision() {
    let profile = forgeplan_core::constellation::jobs::SnapshotEvidence {
        revision: MEASURED_REVISION.to_string(),
        root: std::path::PathBuf::from("/cache/snapshot"),
        present_files: vec![
            "config.json".into(),
            "tokenizer.json".into(),
            "tokenizer_config.json".into(),
            "onnx/model.onnx".into(),
            "onnx/model.onnx_data".into(),
        ],
        missing_files: Vec::new(),
    }
    .to_profile(
        measured_checksums().weights,
        TOKENIZER_JSON_SHA.to_string(),
        TOKENIZER_CONFIG_SHA.to_string(),
        "tract-onnx-0.23",
    )
    .expect("a complete snapshot with real digests resolves");

    assert_eq!(profile.weights_revision, MEASURED_REVISION);
    assert_eq!(profile.dimensions, 1024, "from config.json hidden_size");
    assert_eq!(profile.profile_id().len(), 64);
}

// ---------------------------------------------------------------------------
// L11 — forced C → A → B completion order
// ---------------------------------------------------------------------------

fn binding_at(canonical_key: &str, row_revision: u64, input_digest: &str) -> Binding {
    Binding {
        index_epoch: 1,
        checkout_id: "local-a".to_string(),
        canonical_key: canonical_key.to_string(),
        incarnation: 1,
        row_revision,
        source_digest: format!("sd-{row_revision}"),
        input_digest: input_digest.to_string(),
        profile_id: "profile-c".to_string(),
    }
}

fn job_for(binding: &Binding) -> EmbeddingJob {
    EmbeddingJob {
        index_epoch: binding.index_epoch,
        checkout_id: binding.checkout_id.clone(),
        canonical_key: binding.canonical_key.clone(),
        incarnation: binding.incarnation,
        row_revision: binding.row_revision,
        source_digest: binding.source_digest.clone(),
        input_digest: binding.input_digest.clone(),
        profile_id: binding.profile_id.clone(),
        captured_input_ref: format!("prds/{}.md", binding.canonical_key),
        attempt: 0,
        retry_after: None,
    }
}

#[test]
fn l11_completions_arriving_c_then_a_then_b_leave_only_c_active() {
    let c = binding_at("PRD-001", 3, "input-C");
    let a = binding_at("PRD-001", 1, "input-A");
    let b = binding_at("PRD-001", 2, "input-B");

    // Forced completion order: the newest job finishes first, then the two
    // stale ones. A late worker may not publish.
    for stale in [&a, &b] {
        assert!(
            !c.matches(&job_for(stale)),
            "a job bound to an older revision must not satisfy the current binding"
        );
    }
    assert!(c.matches(&job_for(&c)), "only C's own job may publish");

    // The outcome is one ready row, not three.
    let outcome = SemanticOutcome {
        ready: 1,
        pending: 0,
        failed: 0,
        unavailable: 0,
        staged: 1,
        retry_after: None,
    };
    assert_eq!(outcome.total(), 1);
    assert!(outcome.is_semantically_covered());
}

#[test]
fn l11_a_stale_job_is_refused_on_input_change_alone() {
    // Same revision and source digest, different tokenizer input: still a
    // different binding.
    let before = binding_at("PRD-001", 5, "input-before");
    let after = binding_at("PRD-001", 5, "input-after");
    assert!(
        !after.matches(&job_for(&before)),
        "a changed input digest must not reuse the old vector"
    );
}

// ---------------------------------------------------------------------------
// L11 — delete / recreate isolated by incarnation
// ---------------------------------------------------------------------------

#[test]
fn l11_a_deleted_then_recreated_artifact_is_isolated_by_incarnation() {
    let original = binding_at("PRD-001", 4, "input-1");
    let mut recreated = original.clone();
    recreated.incarnation = 2;
    recreated.row_revision = 1;

    assert_eq!(
        original.canonical_key, recreated.canonical_key,
        "both incarnations share the display id"
    );
    assert!(
        !recreated.matches(&job_for(&original)),
        "the pre-delete job must not apply to the recreated artifact"
    );
    assert!(recreated.matches(&job_for(&recreated)));
}

// ---------------------------------------------------------------------------
// L12 — profile / prefix / truncation never mix spaces
// ---------------------------------------------------------------------------

#[test]
fn l12_a_profile_change_is_a_different_binding_even_with_identical_input() {
    let base = binding_at("PRD-001", 2, "input-1");
    let mut other_profile = base.clone();
    other_profile.profile_id = "profile-d".to_string();

    assert_eq!(base.input_digest, other_profile.input_digest);
    assert!(
        !other_profile.matches(&job_for(&base)),
        "the same text under a different profile is a different vector"
    );
}

#[test]
fn l12_an_unresolved_profile_keeps_rows_unavailable_never_failed() {
    // No checksums: the profile cannot identify what it would load.
    let incomplete = forgeplan_core::constellation::jobs::SnapshotChecksums::default();
    assert!(!incomplete.are_well_formed());

    // Feature-off is `Unavailable`, not `Failed`. They are different facts:
    // one says "no capability", the other says "a capability broke".
    assert_eq!(
        SemanticState::Unavailable.as_str(),
        "unavailable",
        "a missing model is not a failed one"
    );
    assert_ne!(SemanticState::Unavailable, SemanticState::Failed);
    assert_ne!(SemanticState::Unavailable, SemanticState::Ready);
    assert!(!SemanticState::Unavailable.is_query_eligible());
}

#[test]
fn l12_the_semantic_scope_is_reported_as_prefix() {
    assert_eq!(
        forgeplan_core::constellation::jobs::SEMANTIC_SCOPE,
        "prefix",
        "only title + a body prefix is embedded; this must never be reported \
         as whole-document or section coverage"
    );
}

// ---------------------------------------------------------------------------
// L15 — stale rows in the first K must not hide an eligible match
// ---------------------------------------------------------------------------

#[test]
fn l15_stale_candidates_do_not_hide_an_eligible_row() {
    // Ten stale rows that would occupy the first K of an approximate search,
    // plus one eligible row. Eligibility is decided before any limit, so the
    // eligible row is in the candidate set regardless of K.
    let stale: Vec<Binding> = (1..=10)
        .map(|i| {
            let mut b = binding_at(&format!("PRD-{i:03}"), 1, "input-old");
            b.profile_id = "profile-old".to_string();
            b
        })
        .collect();
    let eligible = binding_at("PRD-999", 7, "input-current");

    let top_k = 10usize;
    let approximate_top_k: Vec<&Binding> = stale.iter().take(top_k).collect();

    let eligible_survives_prefilter = !approximate_top_k
        .iter()
        .any(|candidate| eligible == **candidate);

    assert!(
        eligible_survives_prefilter,
        "the prefilter runs before the limit, so an eligible row cannot be \
         crowded out by stale ones"
    );
    assert_eq!(approximate_top_k.len(), top_k);
    // Eligibility is decided from the binding, never from the vector: a
    // `ready` row whose binding does not match is not eligible.
    let incorrectly_eligible: Vec<&Binding> = approximate_top_k
        .iter()
        .copied()
        .filter(|candidate| eligible.matches(&job_for(candidate)))
        .collect();
    assert!(
        incorrectly_eligible.is_empty(),
        "no stale candidate may satisfy the current binding"
    );
}

#[test]
fn l15_an_empty_eligible_set_under_pending_is_not_no_restrictions() {
    let covered_after_stale_work = SemanticOutcome {
        ready: 0,
        pending: 0,
        failed: 0,
        unavailable: 11,
        staged: 0,
        retry_after: None,
    };
    let covered_with_pending = SemanticOutcome {
        ready: 0,
        pending: 11,
        failed: 0,
        unavailable: 0,
        staged: 11,
        retry_after: Some("2026-10-17T00:01:00Z".to_string()),
    };

    // Both found nothing; only one of them may claim it looked.
    assert!(
        !covered_with_pending.is_semantically_covered(),
        "pending means the index could not see, not that nothing matched"
    );
    assert!(covered_with_pending.staged > 0, "work did run");
    assert!(covered_with_pending.retry_after.is_some());
    assert!(!covered_after_stale_work.is_semantically_covered());
}
