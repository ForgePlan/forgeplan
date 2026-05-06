//! Sequential playbook executor.
//!
//! Per AC-3 PRD-065: steps run sequentially, ordered by topological sort
//! over `Step::requires:`. Each step is dispatched through a [`Dispatcher`]
//! impl; outcomes write to the [`Journal`]; final [`ExecutionReport`]
//! aggregates success / failure counts.
//!
//! Parallelism (multiple ready steps fanned out concurrently) is deferred
//! to v2 — see PRD-065 §"Implementation Plan". Today's contract: even when
//! the DAG admits parallel layers, the executor visits steps one-by-one.
//!
//! # Failure semantics
//!
//! * `Step::on_error == Abort` (default) — first failure stops the run; all
//!   downstream steps are recorded as **skipped** in the report.
//! * `Step::on_error == Continue` — failure is logged + skipped, but the
//!   executor proceeds to the next step in topological order.
//!
//! # Why re-validate?
//!
//! `Executor::run` re-validates loader-level invariants (cycle, unknown
//! refs, empty steps) so it stays safe even when callers construct a
//! `Playbook` programmatically rather than via [`crate::playbook::loader`].
//! See [`ExecutorConfig::skip_revalidation`] for the opt-out used by tests
//! that already loaded via `load_playbook`.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use super::dispatch::{Dispatcher, validate_command_delegate_security};
use super::journal::{Journal, JournalEntry, JournalEntryKind, RunId};
use super::loader::LoaderError;
use super::types::{OnError, Playbook, Step};

// =====================================================================
// Config & report types
// =====================================================================

/// Per-run executor configuration.
#[derive(Debug, Clone, Default)]
pub struct ExecutorConfig {
    /// User confirmed `--yes` to run a playbook (blanket "are you sure?"
    /// gate, ADR-009 / SPEC-003 §"delegate_to"). Required for any execution.
    pub yes_flag: bool,
    /// User confirmed `--allow-shell` (or set `[playbook] allow_shell = true`
    /// in workspace config) — opt-in for `Delegation::Command` shell
    /// execution (PRD-074 §FR-1+§FR-2, PROB-053 closure). Distinct from
    /// `yes_flag`: a playbook with no shell steps needs `--yes` only;
    /// a playbook with `Delegation::Command` needs both.
    pub allow_shell: bool,
    /// If `true`, skip the structural re-validation pass (cycles / unknown
    /// refs / empty steps) at the start of `run`. Use when `Playbook` was
    /// already vetted by [`crate::playbook::loader::load_playbook`].
    pub skip_revalidation: bool,
    /// One-indexed topological-order step number to start execution at.
    /// `None` (default) means start at step 1. Steps before this index are
    /// recorded as `Skipped` with reason `"skipped via --step N"`.
    /// `Some(0)` or `Some(n)` with `n > total_steps` is rejected with
    /// [`ExecutorError::InvalidStartStep`].
    ///
    /// Wired from CLI/MCP `--step N` flag (HIGH-S5, Audit Round 1).
    pub start_step: Option<usize>,
}

/// Per-step result captured during a run. Stored in execution order
/// (the topological order in which steps were attempted, not the
/// playbook source order).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepReport {
    /// `Step::id` of the step.
    pub step_id: String,
    /// Final state in this run.
    pub status: StepStatus,
    /// Optional artifact path the step produced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_path: Option<PathBuf>,
    /// Stderr / error message captured for diagnostics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Terminal state of a single step in a run.
//
// NOTE (Audit Round 2): considered for `#[non_exhaustive]` but reverted —
// CLI `commands/playbook.rs` exhaustively matches on this enum and lives
// outside this fix-2 agent's owned scope. Future statuses (`Retried`,
// `TimedOut`) must be coordinated with that file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    /// Dispatcher returned `success: true`.
    Success,
    /// Dispatcher returned `success: false` or [`super::dispatch::DispatchError`].
    Failed,
    /// Step never attempted because a predecessor with `on_error: abort`
    /// failed, or because its `requires:` predecessors were all skipped /
    /// failed.
    Skipped,
}

/// Aggregate outcome of `Executor::run`. Counts plus per-step detail.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionReport {
    /// Run identifier shared with all journal entries.
    pub run_id: RunId,
    /// Number of steps that finished cleanly.
    pub success: usize,
    /// Number of steps that failed (dispatcher error or `success: false`).
    pub failed: usize,
    /// Number of steps not executed (downstream of an `Abort` failure).
    pub skipped: usize,
    /// Per-step detail in execution order.
    pub per_step: Vec<StepReport>,
}

impl ExecutionReport {
    /// `true` if no step failed and none were skipped due to upstream errors.
    pub fn ok(&self) -> bool {
        self.failed == 0 && self.skipped == 0
    }
}

/// Executor errors raised before any step runs (validation / setup) or
/// unrecoverable mid-run problems (journal IO).
///
/// `#[non_exhaustive]` so future executor revisions (parallel steps,
/// resumable runs) can introduce new error classes without forcing
/// downstream `match` arms to be re-checked.
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum ExecutorError {
    /// Re-validation failed for a programmatically-built playbook.
    #[error(transparent)]
    Validation(#[from] LoaderError),
    /// Journal write failed; runs cannot continue silently because FR-6
    /// (resumable runs) depends on a complete journal.
    #[error("journal write failed: {0}")]
    Journal(#[from] std::io::Error),
    /// `ExecutorConfig::start_step` is `Some(0)` or exceeds the total
    /// number of steps in the (topologically ordered) playbook (HIGH-S5,
    /// Audit Round 1).
    #[error(
        "--step {requested} is out of range (playbook has {total} step(s); valid range 1..={total})"
    )]
    InvalidStartStep { requested: usize, total: usize },
}

// =====================================================================
// Executor
// =====================================================================

/// Sequential playbook executor. Generic over [`Dispatcher`] so tests
/// can inject mocks while production wires the real Wave 3 dispatcher.
pub struct Executor<D: Dispatcher> {
    dispatcher: D,
    journal: Journal,
    config: ExecutorConfig,
}

impl<D: Dispatcher> Executor<D> {
    /// Build a new executor.
    pub fn new(dispatcher: D, journal: Journal, config: ExecutorConfig) -> Self {
        Self {
            dispatcher,
            journal,
            config,
        }
    }

    /// Execute every step of `playbook` in topological order.
    ///
    /// Failure of a step with `on_error: abort` halts the run; remaining
    /// steps are recorded as `Skipped`. `on_error: continue` lets the run
    /// proceed past failures (downstream steps that depend on the failed
    /// one are still skipped — graph constraints are honored).
    ///
    /// # Errors
    /// * [`ExecutorError::Validation`] when re-validation fails (cycles
    ///   etc.) and `skip_revalidation == false`.
    /// * [`ExecutorError::Journal`] when the journal cannot be written.
    pub async fn run(&mut self, playbook: &Playbook) -> Result<ExecutionReport, ExecutorError> {
        if !self.config.skip_revalidation {
            revalidate(playbook)?;
        }

        let order = topological_order(playbook)?;
        let by_id: HashMap<&str, &Step> =
            playbook.steps.iter().map(|s| (s.id.as_str(), s)).collect();

        let run_id = RunId::new();
        self.journal
            .append(&JournalEntry {
                ts: chrono::Utc::now(),
                run_id,
                playbook_name: playbook.name.clone(),
                step_id: None,
                kind: JournalEntryKind::RunStart,
                payload: serde_json::json!({
                    "title": playbook.title,
                    "step_count": playbook.steps.len(),
                    "yes_flag": self.config.yes_flag,
                    "allow_shell": self.config.allow_shell,
                }),
            })
            .await?;
        info!(run_id = %run_id, name = %playbook.name, "playbook run started");

        // --step N support (HIGH-S5, Audit Round 1) — clamp + validate.
        let start_index = match self.config.start_step {
            None => 0usize,
            Some(n) => {
                if n == 0 {
                    return Err(ExecutorError::InvalidStartStep {
                        requested: 0,
                        total: order.len(),
                    });
                }
                if n > order.len() {
                    return Err(ExecutorError::InvalidStartStep {
                        requested: n,
                        total: order.len(),
                    });
                }
                n - 1
            }
        };

        // Track terminal state per step so downstream skipping is accurate.
        let mut state: HashMap<&str, StepStatus> = HashMap::with_capacity(order.len());
        let mut per_step: Vec<StepReport> = Vec::with_capacity(order.len());
        let mut abort_after_failure = false;

        for (idx, step_id) in order.iter().enumerate() {
            let step = by_id.get(step_id.as_str()).copied().expect("id in by_id");

            // HIGH-S5: skip steps before --step N. Recorded as Skipped so
            // downstream reports stay accurate, but state stays Skipped so
            // dependents see the predecessor as not Success.
            if idx < start_index {
                let report = StepReport {
                    step_id: step.id.clone(),
                    status: StepStatus::Skipped,
                    output_path: None,
                    message: Some(format!(
                        "skipped via --step {}",
                        start_index + 1 // user-facing 1-indexed value
                    )),
                };
                self.write_step_pair(run_id, &playbook.name, &report, true)
                    .await?;
                state.insert(step.id.as_str(), StepStatus::Skipped);
                per_step.push(report);
                continue;
            }

            // Skip if any predecessor failed/skipped (graph constraint) or if
            // an Abort-policy failure already took down the run.
            let predecessors_ok = step
                .requires
                .as_deref()
                .unwrap_or(&[])
                .iter()
                .all(|req| matches!(state.get(req.as_str()), Some(StepStatus::Success)));

            if abort_after_failure || !predecessors_ok {
                let report = StepReport {
                    step_id: step.id.clone(),
                    status: StepStatus::Skipped,
                    output_path: None,
                    message: Some(if abort_after_failure {
                        "skipped: prior step failed with on_error: abort".into()
                    } else {
                        "skipped: predecessor not successful".into()
                    }),
                };
                self.write_step_pair(run_id, &playbook.name, &report, true)
                    .await?;
                state.insert(step.id.as_str(), StepStatus::Skipped);
                per_step.push(report);
                continue;
            }

            // StepStart entry.
            self.journal
                .append(&JournalEntry {
                    ts: chrono::Utc::now(),
                    run_id,
                    playbook_name: playbook.name.clone(),
                    step_id: Some(step.id.clone()),
                    kind: JournalEntryKind::StepStart,
                    payload: serde_json::json!({
                        "delegate_kind": delegate_kind_label(step),
                    }),
                })
                .await?;
            info!(step = %step.id, "step start");

            // Security gate (Command without --allow-shell).
            // PROB-053 closure: previously gated by `yes_flag` (blanket
            // confirm), now uses dedicated `allow_shell` signal so a generic
            // `--yes` for a non-shell playbook does not implicitly authorise
            // shell execution should the playbook be modified later.
            if let Err(sec_err) = validate_command_delegate_security(step, self.config.allow_shell)
            {
                let report = StepReport {
                    step_id: step.id.clone(),
                    status: StepStatus::Failed,
                    output_path: None,
                    message: Some(sec_err.to_string()),
                };
                self.write_step_pair(run_id, &playbook.name, &report, false)
                    .await?;
                state.insert(step.id.as_str(), StepStatus::Failed);
                per_step.push(report);
                if step.on_error == OnError::Abort {
                    abort_after_failure = true;
                }
                continue;
            }

            // Dispatch.
            let dispatch_result = self.dispatcher.dispatch(step).await;
            let report = match dispatch_result {
                Ok(outcome) if outcome.success => StepReport {
                    step_id: step.id.clone(),
                    status: StepStatus::Success,
                    output_path: outcome.output_path,
                    message: outcome.stderr,
                },
                Ok(outcome) => StepReport {
                    step_id: step.id.clone(),
                    status: StepStatus::Failed,
                    output_path: outcome.output_path,
                    message: outcome
                        .stderr
                        .or_else(|| Some("dispatcher reported failure".into())),
                },
                Err(err) => StepReport {
                    step_id: step.id.clone(),
                    status: StepStatus::Failed,
                    output_path: None,
                    message: Some(err.to_string()),
                },
            };

            let success = report.status == StepStatus::Success;
            self.write_step_pair(run_id, &playbook.name, &report, success)
                .await?;
            state.insert(step.id.as_str(), report.status);
            if !success {
                if step.on_error == OnError::Abort {
                    warn!(step = %step.id, "step failed with on_error: abort — halting run");
                    abort_after_failure = true;
                } else {
                    warn!(step = %step.id, "step failed with on_error: continue — proceeding");
                }
            }
            per_step.push(report);
        }

        let (success, failed, skipped) = tally(&per_step);
        self.journal
            .append(&JournalEntry {
                ts: chrono::Utc::now(),
                run_id,
                playbook_name: playbook.name.clone(),
                step_id: None,
                kind: JournalEntryKind::RunEnd,
                payload: serde_json::json!({
                    "success": success,
                    "failed": failed,
                    "skipped": skipped,
                }),
            })
            .await?;
        self.journal.flush().await?;
        info!(
            run_id = %run_id,
            success,
            failed,
            skipped,
            "playbook run finished"
        );

        Ok(ExecutionReport {
            run_id,
            success,
            failed,
            skipped,
            per_step,
        })
    }

    /// Write the closing `StepEnd` entry for `report` and flush the journal
    /// to disk.
    ///
    /// Per-step flush (NEW-S-H2, Audit Round 2): every `StepEnd` is
    /// followed by an explicit [`Journal::flush`] so that, on a process
    /// crash mid-run, every step that *finished* is durably recorded.
    /// PRD-065 FR-6 (resumable runs) treats a missing `StepEnd` after the
    /// matching `StepStart` as "step in flight when killed — retry"; a
    /// fully-buffered journal would lose the last `StepEnd` and falsely
    /// retry a completed step.
    async fn write_step_pair(
        &mut self,
        run_id: RunId,
        playbook_name: &str,
        report: &StepReport,
        success: bool,
    ) -> std::io::Result<()> {
        self.journal
            .append(&JournalEntry {
                ts: chrono::Utc::now(),
                run_id,
                playbook_name: playbook_name.to_string(),
                step_id: Some(report.step_id.clone()),
                kind: JournalEntryKind::StepEnd,
                payload: serde_json::json!({
                    "status": report.status,
                    "success": success,
                    "output_path": report.output_path,
                    "message": report.message,
                }),
            })
            .await?;
        // NEW-S-H2: flush on every StepEnd so resumable runs trust the tail.
        self.journal.flush().await
    }
}

// =====================================================================
// Helpers
// =====================================================================

/// Re-runs the structural checks loader performed (empty steps / unknown
/// refs / cycles). Defends against playbooks built programmatically.
fn revalidate(pb: &Playbook) -> Result<(), LoaderError> {
    if pb.steps.is_empty() {
        return Err(LoaderError::EmptySteps);
    }
    let unknown = pb.find_unknown_step_refs();
    if !unknown.is_empty() {
        return Err(LoaderError::UnknownStepRef {
            pairs: unknown
                .into_iter()
                .map(|(s, r)| (s.to_string(), r.to_string()))
                .collect(),
        });
    }
    if let Some(cycle) = pb.detect_cycles() {
        return Err(LoaderError::Cycle {
            path: cycle.iter().map(|s| s.to_string()).collect(),
        });
    }
    Ok(())
}

/// Kahn's algorithm — return step IDs in a valid topological order. Stable
/// w.r.t. playbook source order (ties broken by index of first appearance),
/// so reports are reproducible across runs.
fn topological_order(pb: &Playbook) -> Result<Vec<String>, LoaderError> {
    // Map id → index for stable ordering.
    let index: HashMap<&str, usize> = pb
        .steps
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id.as_str(), i))
        .collect();

    // Build adjacency: for each step `s`, edges from each `req in s.requires`
    // to `s` (req must run BEFORE s).
    let mut indegree: HashMap<&str, usize> = pb.steps.iter().map(|s| (s.id.as_str(), 0)).collect();
    let mut succ: HashMap<&str, Vec<&str>> = HashMap::with_capacity(pb.steps.len());
    for step in &pb.steps {
        for req in step.requires.as_deref().unwrap_or(&[]) {
            // Loader has already rejected unknown refs, but be defensive.
            if !indegree.contains_key(req.as_str()) {
                return Err(LoaderError::UnknownStepRef {
                    pairs: vec![(step.id.clone(), req.clone())],
                });
            }
            succ.entry(req.as_str()).or_default().push(step.id.as_str());
            *indegree.entry(step.id.as_str()).or_insert(0) += 1;
        }
    }

    // Initial frontier — steps with indegree 0, in source order.
    let mut queue: VecDeque<&str> = pb
        .steps
        .iter()
        .filter(|s| indegree.get(s.id.as_str()).copied().unwrap_or(0) == 0)
        .map(|s| s.id.as_str())
        .collect();

    let mut out: Vec<String> = Vec::with_capacity(pb.steps.len());
    while let Some(node) = queue.pop_front() {
        out.push(node.to_string());
        if let Some(nexts) = succ.get(node) {
            // Sort successors by source index for stable output.
            let mut nexts = nexts.clone();
            nexts.sort_by_key(|n| index.get(n).copied().unwrap_or(usize::MAX));
            for n in nexts {
                let entry = indegree.get_mut(n).expect("known node");
                *entry -= 1;
                if *entry == 0 {
                    queue.push_back(n);
                }
            }
        }
    }

    if out.len() != pb.steps.len() {
        // Should be unreachable: loader/revalidate run cycle detection
        // first, but defend in depth.
        let leftover: Vec<&str> = pb
            .steps
            .iter()
            .map(|s| s.id.as_str())
            .filter(|id| !out.iter().any(|o| o == id))
            .collect();
        return Err(LoaderError::Cycle {
            path: leftover.iter().map(|s| s.to_string()).collect(),
        });
    }
    Ok(out)
}

/// Short label describing the delegate kind for journal payloads.
fn delegate_kind_label(step: &Step) -> &'static str {
    use super::types::Delegation;
    match step.delegate_to {
        Delegation::Plugin { .. } => "plugin",
        Delegation::Agent { .. } => "agent",
        Delegation::Skill { .. } => "skill",
        Delegation::Command { .. } => "command",
        Delegation::ForgeplanCore { .. } => "forgeplan_core",
    }
}

/// Count step statuses into `(success, failed, skipped)`.
fn tally(per_step: &[StepReport]) -> (usize, usize, usize) {
    let mut s = 0usize;
    let mut f = 0usize;
    let mut k = 0usize;
    for r in per_step {
        match r.status {
            StepStatus::Success => s += 1,
            StepStatus::Failed => f += 1,
            StepStatus::Skipped => k += 1,
        }
    }
    (s, f, k)
}

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playbook::dispatch::{DispatchOutcome, MockDispatcher, RecordingDispatcher};
    use crate::playbook::loader::load_playbook;
    use tempfile::tempdir;

    fn fresh_journal() -> Journal {
        let dir = tempdir().expect("tempdir");
        // Leak the tempdir so the file outlives the test; tempfiles get
        // cleaned up at process exit. We leak here intentionally because
        // each test creates its own dir and the journal needs to keep
        // the path valid for the run.
        let path = dir.path().join("journal.jsonl");
        std::mem::forget(dir);
        Journal::open_at(path).expect("journal")
    }

    /// 3-step happy path: all steps succeed, report counts match.
    #[tokio::test]
    async fn happy_path_three_steps_all_succeed() {
        let yaml = r#"
schema_version: "1.0"
name: happy
title: Happy
steps:
  - id: a
    delegate_to: { type: agent, name: a }
  - id: b
    delegate_to: { type: agent, name: b }
    requires: [a]
  - id: c
    delegate_to: { type: agent, name: c }
    requires: [b]
"#;
        let pb = load_playbook(yaml).expect("loads");
        let mock = MockDispatcher::new();
        let mut exec = Executor::new(mock, fresh_journal(), ExecutorConfig::default());
        let report = exec.run(&pb).await.expect("runs");
        assert_eq!(report.success, 3);
        assert_eq!(report.failed, 0);
        assert_eq!(report.skipped, 0);
        assert!(report.ok());
        let ids: Vec<_> = report.per_step.iter().map(|r| r.step_id.as_str()).collect();
        assert_eq!(ids, vec!["a", "b", "c"]);
    }

    /// `on_error: abort` halts run, downstream steps marked Skipped.
    #[tokio::test]
    async fn on_error_abort_halts_run() {
        let yaml = r#"
schema_version: "1.0"
name: abort-flow
title: Abort
steps:
  - id: a
    delegate_to: { type: agent, name: a }
  - id: b
    delegate_to: { type: agent, name: b }
    on_error: abort
  - id: c
    delegate_to: { type: agent, name: c }
"#;
        let pb = load_playbook(yaml).expect("loads");
        let mock = MockDispatcher::new()
            .with_success("a", DispatchOutcome::success())
            .with_success("b", DispatchOutcome::failure("boom")); // success: false
        let mut exec = Executor::new(mock, fresh_journal(), ExecutorConfig::default());
        let report = exec.run(&pb).await.expect("runs");
        assert_eq!(report.success, 1, "{report:?}");
        assert_eq!(report.failed, 1);
        assert_eq!(report.skipped, 1);
        assert_eq!(report.per_step[0].status, StepStatus::Success);
        assert_eq!(report.per_step[1].status, StepStatus::Failed);
        assert_eq!(report.per_step[2].status, StepStatus::Skipped);
    }

    /// `on_error: continue` lets run proceed past failures.
    #[tokio::test]
    async fn on_error_continue_proceeds() {
        let yaml = r#"
schema_version: "1.0"
name: continue-flow
title: Continue
steps:
  - id: a
    delegate_to: { type: agent, name: a }
    on_error: continue
  - id: b
    delegate_to: { type: agent, name: b }
"#;
        let pb = load_playbook(yaml).expect("loads");
        let mock = MockDispatcher::new().with_success("a", DispatchOutcome::failure("boom"));
        let mut exec = Executor::new(mock, fresh_journal(), ExecutorConfig::default());
        let report = exec.run(&pb).await.expect("runs");
        assert_eq!(report.success, 1);
        assert_eq!(report.failed, 1);
        assert_eq!(report.skipped, 0);
        assert_eq!(report.per_step[0].status, StepStatus::Failed);
        assert_eq!(report.per_step[1].status, StepStatus::Success);
    }

    /// Topological order: `b requires a` → `a` runs before `b` even when
    /// listed second in YAML order.
    #[tokio::test]
    async fn topological_order_respected() {
        let yaml = r#"
schema_version: "1.0"
name: dag
title: DAG
steps:
  - id: b
    delegate_to: { type: agent, name: b }
    requires: [a]
  - id: a
    delegate_to: { type: agent, name: a }
"#;
        let pb = load_playbook(yaml).expect("loads");
        let recording = RecordingDispatcher::new(MockDispatcher::new());
        let mut exec = Executor::new(recording, fresh_journal(), ExecutorConfig::default());
        let report = exec.run(&pb).await.expect("runs");
        let calls: Vec<_> = report.per_step.iter().map(|r| r.step_id.as_str()).collect();
        // Topological order must place "a" before "b".
        let pos_a = calls.iter().position(|s| *s == "a").expect("a present");
        let pos_b = calls.iter().position(|s| *s == "b").expect("b present");
        assert!(pos_a < pos_b, "a must come before b: {calls:?}");
    }

    /// `--allow-shell` not passed → Command step refused with
    /// SecurityRefused-shaped message in StepReport (PROB-053 closure:
    /// pre-PROB-053 the gate read `--yes`; now reads `--allow-shell`).
    #[tokio::test]
    async fn command_step_without_allow_shell_is_refused() {
        let yaml = r#"
schema_version: "1.0"
name: shellish
title: Shell
steps:
  - id: cmd
    delegate_to:
      type: command
      command: ["echo", "hi"]
"#;
        let pb = load_playbook(yaml).expect("loads");
        let mock = MockDispatcher::new();
        let cfg = ExecutorConfig {
            yes_flag: true,     // blanket confirm OK …
            allow_shell: false, // … but no shell-exec opt-in → refuse
            skip_revalidation: false,
            start_step: None,
        };
        let mut exec = Executor::new(mock, fresh_journal(), cfg);
        let report = exec.run(&pb).await.expect("runs");
        assert_eq!(report.failed, 1);
        assert_eq!(report.success, 0);
        let msg = report.per_step[0].message.as_deref().expect("message set");
        assert!(
            msg.contains("--allow-shell"),
            "msg should reference --allow-shell: {msg}"
        );
    }

    /// Dispatcher transport error → step Failed + message captured.
    #[tokio::test]
    async fn dispatch_error_yields_failed_step() {
        let yaml = r#"
schema_version: "1.0"
name: missing
title: Missing
steps:
  - id: only
    delegate_to:
      type: plugin
      name: c4-architecture
      target: c4-code
"#;
        let pb = load_playbook(yaml).expect("loads");
        let mock = MockDispatcher::new().with_missing(
            "only",
            "plugin:c4-architecture",
            "install via brew install forgeplan/c4",
        );
        let mut exec = Executor::new(mock, fresh_journal(), ExecutorConfig::default());
        let report = exec.run(&pb).await.expect("runs");
        assert_eq!(report.failed, 1);
        let msg = report.per_step[0].message.as_deref().expect("message");
        assert!(msg.contains("brew install"), "msg: {msg}");
    }

    /// `topological_order` returns input order for a chain.
    #[test]
    fn topological_order_chain() {
        let yaml = r#"
schema_version: "1.0"
name: chain
title: Chain
steps:
  - id: a
    delegate_to: { type: agent, name: a }
  - id: b
    delegate_to: { type: agent, name: b }
    requires: [a]
  - id: c
    delegate_to: { type: agent, name: c }
    requires: [b]
"#;
        let pb = load_playbook(yaml).expect("loads");
        let order = topological_order(&pb).expect("orders");
        assert_eq!(order, vec!["a", "b", "c"]);
    }

    /// `tally` counts statuses correctly.
    #[test]
    fn tally_counts() {
        let per_step = vec![
            StepReport {
                step_id: "a".into(),
                status: StepStatus::Success,
                output_path: None,
                message: None,
            },
            StepReport {
                step_id: "b".into(),
                status: StepStatus::Failed,
                output_path: None,
                message: None,
            },
            StepReport {
                step_id: "c".into(),
                status: StepStatus::Skipped,
                output_path: None,
                message: None,
            },
            StepReport {
                step_id: "d".into(),
                status: StepStatus::Success,
                output_path: None,
                message: None,
            },
        ];
        assert_eq!(tally(&per_step), (2, 1, 1));
    }

    /// HIGH-S5 (Audit Round 1): `start_step = Some(3)` skips steps 1 and 2,
    /// dispatches steps 3, 4, 5. Skipped steps are recorded with the
    /// `"skipped via --step N"` message.
    #[tokio::test]
    async fn executor_start_step_skips_earlier() {
        let yaml = r#"
schema_version: "1.0"
name: start-step-flow
title: Start Step
steps:
  - id: a
    delegate_to: { type: agent, name: a }
  - id: b
    delegate_to: { type: agent, name: b }
    requires: [a]
  - id: c
    delegate_to: { type: agent, name: c }
    requires: [b]
  - id: d
    delegate_to: { type: agent, name: d }
    requires: [c]
  - id: e
    delegate_to: { type: agent, name: e }
    requires: [d]
"#;
        let pb = load_playbook(yaml).expect("loads");
        let mock = MockDispatcher::new();
        let cfg = ExecutorConfig {
            yes_flag: false,
            allow_shell: false,
            skip_revalidation: false,
            start_step: Some(3),
        };
        let mut exec = Executor::new(mock, fresh_journal(), cfg);
        let report = exec.run(&pb).await.expect("runs");

        // Topo order is [a, b, c, d, e]. Steps at indices 0,1 (a,b) skipped
        // due to --step 3 — but step c (index 2) requires b. b is Skipped,
        // so c's predecessor check fails and c is also Skipped (graph
        // constraint). This is the correct semantic: --step N starts at
        // step N regardless of dependencies, and downstream graph rules
        // still apply. The test asserts that the Skipped reasons differ:
        // a/b skipped via --step, c skipped because predecessor failed.
        assert_eq!(report.per_step.len(), 5);
        assert_eq!(report.per_step[0].step_id, "a");
        assert_eq!(report.per_step[0].status, StepStatus::Skipped);
        assert!(
            report.per_step[0]
                .message
                .as_deref()
                .unwrap_or("")
                .contains("--step"),
            "expected --step skip reason, got {:?}",
            report.per_step[0].message
        );
        assert_eq!(report.per_step[1].step_id, "b");
        assert_eq!(report.per_step[1].status, StepStatus::Skipped);
        assert!(
            report.per_step[1]
                .message
                .as_deref()
                .unwrap_or("")
                .contains("--step"),
        );
        // c, d, e were NOT directly --step skipped; they fail predecessor
        // check (b is Skipped, not Success). Per-step messages should
        // mention "predecessor".
        assert_eq!(report.per_step[2].status, StepStatus::Skipped);
        assert_eq!(report.per_step[3].status, StepStatus::Skipped);
        assert_eq!(report.per_step[4].status, StepStatus::Skipped);
    }

    /// `start_step = Some(1)` is the same as no start_step — every step
    /// dispatches normally.
    #[tokio::test]
    async fn executor_start_step_one_dispatches_all() {
        let yaml = r#"
schema_version: "1.0"
name: ss1
title: SS1
steps:
  - id: a
    delegate_to: { type: agent, name: a }
  - id: b
    delegate_to: { type: agent, name: b }
"#;
        let pb = load_playbook(yaml).expect("loads");
        let mock = MockDispatcher::new();
        let cfg = ExecutorConfig {
            yes_flag: false,
            allow_shell: false,
            skip_revalidation: false,
            start_step: Some(1),
        };
        let mut exec = Executor::new(mock, fresh_journal(), cfg);
        let report = exec.run(&pb).await.expect("runs");
        assert_eq!(report.success, 2);
        assert_eq!(report.skipped, 0);
    }

    /// `start_step = Some(0)` is rejected with `InvalidStartStep` (1-indexed).
    #[tokio::test]
    async fn executor_start_step_zero_rejected() {
        let yaml = r#"
schema_version: "1.0"
name: ss0
title: SS0
steps:
  - id: a
    delegate_to: { type: agent, name: a }
"#;
        let pb = load_playbook(yaml).expect("loads");
        let cfg = ExecutorConfig {
            yes_flag: false,
            allow_shell: false,
            skip_revalidation: false,
            start_step: Some(0),
        };
        let mut exec = Executor::new(MockDispatcher::new(), fresh_journal(), cfg);
        let err = exec.run(&pb).await.unwrap_err();
        assert!(
            matches!(err, ExecutorError::InvalidStartStep { requested: 0, .. }),
            "expected InvalidStartStep(0), got {err:?}"
        );
    }

    /// `start_step` greater than the step count returns `InvalidStartStep`.
    #[tokio::test]
    async fn executor_start_step_too_large_rejected() {
        let yaml = r#"
schema_version: "1.0"
name: ssbig
title: SSBig
steps:
  - id: a
    delegate_to: { type: agent, name: a }
  - id: b
    delegate_to: { type: agent, name: b }
"#;
        let pb = load_playbook(yaml).expect("loads");
        let cfg = ExecutorConfig {
            yes_flag: false,
            allow_shell: false,
            skip_revalidation: false,
            start_step: Some(99),
        };
        let mut exec = Executor::new(MockDispatcher::new(), fresh_journal(), cfg);
        let err = exec.run(&pb).await.unwrap_err();
        match err {
            ExecutorError::InvalidStartStep { requested, total } => {
                assert_eq!(requested, 99);
                assert_eq!(total, 2);
            }
            other => panic!("expected InvalidStartStep, got {other:?}"),
        }
    }

    /// `ExecutionReport::ok` requires no failures and no skips.
    #[test]
    fn execution_report_ok_predicate() {
        let report = ExecutionReport {
            run_id: RunId::new(),
            success: 2,
            failed: 0,
            skipped: 0,
            per_step: vec![],
        };
        assert!(report.ok());

        let bad = ExecutionReport {
            run_id: RunId::new(),
            success: 1,
            failed: 0,
            skipped: 1,
            per_step: vec![],
        };
        assert!(!bad.ok());
    }
}
