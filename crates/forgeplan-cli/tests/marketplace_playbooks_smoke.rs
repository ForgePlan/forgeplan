//! Smoke tests for canonical marketplace playbooks.
//!
//! Track 4-A8 (HANDOFF Phase B follow-up). Walks every YAML file in
//! `marketplace/playbooks/` and asserts:
//!
//! 1. The file loads via `forgeplan_core::playbook::loader::load_playbook`
//!    (validates schema_version, structural invariants, DAG cycles, mapping
//!    consistency).
//!
//! 2. Every `Delegation::ForgeplanCore` step has the right `step.input` shape
//!    for its `target`. The schema validator does NOT reach into delegate-
//!    specific input shapes (`Step.input` is free-form `serde_yaml::Value`),
//!    so a typo like top-level `mapping: docs-to-forge` (which serialises
//!    into `Step.mapping: Option<String>` — a SOFT WARNING field, never read
//!    by `ForgeplanCoreDispatcher::Ingest`) passes validation today and
//!    fails ONLY at runtime with `Transport: "Ingest input missing
//!    'mapping_path'"`.
//!
//!    R1 audit CRITICAL (Track 4-A8 code-review): this exact defect made
//!    `brownfield-docs.yaml` non-functional end-to-end despite passing
//!    `forgeplan playbook validate`. The smoke test below is the
//!    regression guard.
//!
//! Per-target contract enforced (see
//! `crates/forgeplan-core/src/playbook/dispatch/forgeplan_core_dispatcher.rs`
//! `parse_op_input`):
//!
//! - `Ingest` → `step.input` MUST have `mapping_path` AND `source_path`
//! - `New` → `step.input` MUST have `kind` AND `title`
//! - `Validate` → `step.input` MUST have `id`
//! - `Activate` → `step.input` MUST have `id`
//! - `Search` → `step.input` MUST have `query`

use std::fs;
use std::path::{Path, PathBuf};

use forgeplan_core::playbook::loader::load_playbook;
use forgeplan_core::playbook::types::{Delegation, ForgeplanOp, Playbook, Step};

/// Resolve the workspace root containing `marketplace/playbooks/`. The test
/// runs from `crates/forgeplan-cli/`, so the workspace root is two levels up.
fn workspace_root() -> PathBuf {
    let cargo_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    cargo_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("CARGO_MANIFEST_DIR has two parents")
        .to_path_buf()
}

/// Collect every `*.yaml` file under `marketplace/playbooks/`.
fn list_marketplace_playbooks() -> Vec<PathBuf> {
    let dir = workspace_root().join("marketplace").join("playbooks");
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("failed to read {dir:?}: {e}"))
        .filter_map(|entry| entry.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("yaml"))
        .collect();
    paths.sort();
    assert!(
        !paths.is_empty(),
        "no marketplace playbooks found at {dir:?}"
    );
    paths
}

/// Helper: pull a string field from `step.input` if present.
fn input_field<'a>(step: &'a Step, key: &str) -> Option<&'a serde_yaml::Value> {
    step.input.as_ref()?.get(key)
}

/// Assert that a `Step` whose delegate is `ForgeplanCore { target: <op> }`
/// has the input shape required by `ForgeplanCoreDispatcher::parse_op_input`.
fn assert_forgeplan_core_input_shape(playbook_name: &str, step: &Step, op: ForgeplanOp) {
    let required_fields: &[&str] = match op {
        ForgeplanOp::Ingest => &["mapping_path", "source_path"],
        ForgeplanOp::New => &["kind", "title"],
        ForgeplanOp::Validate => &["id"],
        ForgeplanOp::Activate => &["id"],
        ForgeplanOp::Search => &["query"],
    };
    for field in required_fields {
        assert!(
            input_field(step, field).is_some(),
            "playbook `{playbook_name}` step `{}`: forgeplan_core target {op:?} requires \
             `step.input.{field}` (per parse_op_input contract). Without it the dispatcher \
             returns Transport(\"<Op> input missing '{field}'\") at runtime, even though the \
             YAML schema validator passes (input is free-form Value).",
            step.id,
        );
    }
}

#[test]
fn every_marketplace_playbook_loads_via_loader() {
    for path in list_marketplace_playbooks() {
        let yaml = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
        let pb = load_playbook(&yaml)
            .unwrap_or_else(|e| panic!("playbook {path:?} failed to load: {e}"));
        // Defensive: a playbook with zero steps should have been rejected
        // by load_playbook already, but pin the contract explicitly.
        assert!(
            !pb.steps.is_empty(),
            "playbook {path:?} has zero steps after load",
        );
    }
}

#[test]
fn forgeplan_core_steps_have_required_input_fields() {
    for path in list_marketplace_playbooks() {
        let yaml = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
        let pb: Playbook = load_playbook(&yaml)
            .unwrap_or_else(|e| panic!("playbook {path:?} failed to load: {e}"));
        let pb_name = pb.name.clone();
        for step in &pb.steps {
            if let Delegation::ForgeplanCore { target } = &step.delegate_to {
                assert_forgeplan_core_input_shape(&pb_name, step, *target);
            }
        }
    }
}

/// `Step.mapping` is a SOFT WARNING field in `loader.rs:156` — it must NOT
/// be used as a substitute for `step.input.mapping_path` on Ingest steps.
/// This test catches the exact regression that made `brownfield-docs.yaml`
/// non-functional: `mapping: docs-to-forge` at top level + no `step.input` at
/// all silently passes validation but fails at runtime.
#[test]
fn ingest_steps_do_not_rely_on_top_level_mapping_field_alone() {
    for path in list_marketplace_playbooks() {
        let yaml = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
        let pb: Playbook = load_playbook(&yaml)
            .unwrap_or_else(|e| panic!("playbook {path:?} failed to load: {e}"));
        for step in &pb.steps {
            let is_ingest = matches!(
                &step.delegate_to,
                Delegation::ForgeplanCore {
                    target: ForgeplanOp::Ingest
                },
            );
            if is_ingest {
                let has_input_mapping = input_field(step, "mapping_path").is_some();
                assert!(
                    has_input_mapping,
                    "playbook `{}` step `{}`: forgeplan_core: ingest requires \
                     `step.input.mapping_path`. Top-level `step.mapping` is a soft warning \
                     only — never read by ForgeplanCoreDispatcher. Track 4-A8 R1 audit C-1 \
                     regression guard.",
                    pb.name, step.id,
                );
            }
        }
    }
}

/// Every playbook must declare a non-empty `description:` so that
/// `forgeplan playbook list` surfaces what each one does. (Schema treats
/// description as Option, but for canonical marketplace artifacts we require
/// it.)
#[test]
fn marketplace_playbooks_have_non_empty_description() {
    for path in list_marketplace_playbooks() {
        let yaml = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
        let pb: Playbook = load_playbook(&yaml)
            .unwrap_or_else(|e| panic!("playbook {path:?} failed to load: {e}"));
        let desc = pb.description.as_deref().unwrap_or("");
        assert!(
            !desc.trim().is_empty(),
            "playbook {path:?} has no description — required for `forgeplan playbook list`",
        );
    }
}

/// Spot check: every step's `requires:` references must be other step ids in
/// the SAME playbook. The loader already enforces this
/// (`find_unknown_step_refs`), but pin it as a smoke-level guarantee so a
/// future loader regression is caught here too.
#[test]
fn step_requires_references_resolve() {
    for path in list_marketplace_playbooks() {
        let yaml = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
        let pb: Playbook = load_playbook(&yaml)
            .unwrap_or_else(|e| panic!("playbook {path:?} failed to load: {e}"));
        let known_ids: std::collections::HashSet<_> =
            pb.steps.iter().map(|s| s.id.as_str()).collect();
        for step in &pb.steps {
            for req in step.requires.as_deref().unwrap_or(&[]) {
                assert!(
                    known_ids.contains(req.as_str()),
                    "playbook {path:?} step `{}` requires unknown step `{req}`",
                    step.id,
                );
            }
        }
    }
}

#[test]
fn workspace_root_resolves_to_actual_marketplace_dir() {
    // Sanity guard so the test infrastructure itself doesn't drift.
    let mp = workspace_root().join("marketplace").join("playbooks");
    assert!(
        Path::new(&mp).is_dir(),
        "expected marketplace/playbooks at {mp:?}",
    );
}
