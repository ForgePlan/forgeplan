//! Step dispatcher trait and stub implementations.
//!
//! Wave 2 deliverable: a typed [`Dispatcher`] trait plus mock/recording
//! implementations for executor tests. Wave 3 will wire the five real
//! delegate variants from SPEC-003 §"delegate_to":
//!
//! 1. `plugin` → invoked via Task tool (external plugin subprocess)
//! 2. `agent`  → invoked via Task tool (subagent)
//! 3. `skill`  → loaded into agent context, invoked inline
//! 4. `command` → opt-in shell, gated by `--yes` (see
//!    [`validate_command_delegate_security`])
//! 5. `forgeplan_core` → internal call into the matching CLI op
//!    (`ingest`, `new`, `validate`, `activate`, `search`)
//!
//! The trait is async because all five real backends are I/O bound. Stubs
//! pre-compute outcomes synchronously and return them via `async fn`.
//!
//! References: SPEC-003 §"delegate_to", PRD-065 FR-4.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use async_trait::async_trait;
use thiserror::Error;

use super::types::{Delegation, Step};

// =====================================================================
// Outcomes & errors
// =====================================================================

/// Result of a single step dispatch.
///
/// Matches the contract Wave 3 real dispatchers will satisfy: the trait
/// reports whether the underlying delegate finished cleanly, the artifact
/// path it produced (if any), and a stderr blob for diagnostics.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DispatchOutcome {
    /// `true` if the underlying delegate completed without error.
    pub success: bool,
    /// Captured `produces_at` path if the step writes an artifact.
    /// May be `None` even on success when the step has no `produces_at`.
    pub output_path: Option<PathBuf>,
    /// Captured stderr, populated for diagnostics regardless of success.
    pub stderr: Option<String>,
}

impl DispatchOutcome {
    /// Convenience constructor: successful, no output path, no stderr.
    pub fn success() -> Self {
        Self {
            success: true,
            output_path: None,
            stderr: None,
        }
    }

    /// Convenience constructor for failed outcomes carrying stderr text.
    pub fn failure(stderr: impl Into<String>) -> Self {
        Self {
            success: false,
            output_path: None,
            stderr: Some(stderr.into()),
        }
    }
}

/// Dispatcher errors that abort dispatch before producing a [`DispatchOutcome`].
///
/// Distinct from "delegate ran and reported failure" (`success: false`):
/// these errors mean we couldn't even invoke the delegate.
#[derive(Error, Debug)]
pub enum DispatchError {
    /// Delegate type isn't installed on this host (e.g. plugin missing).
    /// Wave 3 plugin engine populates this with the install hint copied
    /// from `Step::fallback_hint`.
    #[error("delegate `{delegate}` is not available: {reason}")]
    DelegateMissing {
        /// Human label describing the missing delegate ("plugin:c4-architecture").
        delegate: String,
        /// Why it can't be invoked + remediation hint when known.
        reason: String,
    },

    /// `delegate_to: command` step attempted without the `--yes` opt-in.
    #[error("command delegate refused: {reason}")]
    SecurityRefused {
        /// Why the security gate blocked execution.
        reason: String,
    },

    /// Wave 3 surface — variant for I/O / transport errors. Stubs do not
    /// emit this directly but real dispatchers will.
    #[error("dispatch transport error: {0}")]
    Transport(String),
}

/// Security-check error returned by [`validate_command_delegate_security`].
/// Mapped to [`DispatchError::SecurityRefused`] by the executor when bubbling
/// through an actual dispatch call.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum SecurityError {
    /// `delegate_to: command` step needs `--yes` opt-in.
    #[error(
        "step `{step_id}` uses `delegate_to: command` — pass `--yes` to acknowledge \
         arbitrary shell execution"
    )]
    ShellRequiresYes {
        /// Step ID that triggered the refusal.
        step_id: String,
    },
}

// =====================================================================
// Dispatcher trait
// =====================================================================

/// Async trait every concrete dispatcher implements. Wave 2 ships only
/// stubs ([`MockDispatcher`], [`RecordingDispatcher`]); Wave 3 layers
/// real Task-tool / shell / forgeplan_core implementations atop.
///
/// `Send + Sync` so [`super::executor::Executor`] is itself Send across
/// `tokio::spawn` boundaries when executor parallelism lands in v2.
#[async_trait]
pub trait Dispatcher: Send + Sync {
    /// Execute one step. Implementations should:
    ///
    /// 1. Map [`Step::delegate_to`] to the appropriate backend.
    /// 2. Run with provided `Step::input` payload.
    /// 3. Return [`DispatchOutcome`] reflecting success, output path, stderr.
    /// 4. On non-runtime errors (missing plugin, security refusal) return
    ///    [`DispatchError`] — the executor maps these to step failure with
    ///    `Step::on_error` policy applied.
    async fn dispatch(&self, step: &Step) -> Result<DispatchOutcome, DispatchError>;
}

// =====================================================================
// Security helper
// =====================================================================

/// Returns `Ok(())` if the step is safe to dispatch under the current
/// `--yes` flag, or [`SecurityError`] if a `Command` delegate was attempted
/// without explicit opt-in.
///
/// Callers (executor + CLI surface) pass `yes_flag = true` when the user
/// acknowledged arbitrary shell execution. Wave 3 wires this into the
/// CLI `forgeplan playbook run --yes` flag.
///
/// Non-`Command` steps are always allowed: typed delegates (plugin/agent/
/// skill/forgeplan_core) carry their own permission boundary.
pub fn validate_command_delegate_security(
    step: &Step,
    yes_flag: bool,
) -> Result<(), SecurityError> {
    match &step.delegate_to {
        Delegation::Command { .. } if !yes_flag => Err(SecurityError::ShellRequiresYes {
            step_id: step.id.clone(),
        }),
        _ => Ok(()),
    }
}

// =====================================================================
// Stub: MockDispatcher
// =====================================================================

/// Test-only dispatcher whose response per step ID is configured up front.
///
/// Designed for [`super::executor::Executor`] tests: caller wires expected
/// outcomes per step ID and the mock returns them in order. Default fallback
/// (when a step ID isn't in the map) is [`DispatchOutcome::success`].
pub struct MockDispatcher {
    outcomes: HashMap<String, Result<DispatchOutcome, MockError>>,
    default_outcome: DispatchOutcome,
}

impl Default for MockDispatcher {
    fn default() -> Self {
        Self {
            outcomes: HashMap::new(),
            // Successful by default — failure cases use explicit `with_*`.
            default_outcome: DispatchOutcome::success(),
        }
    }
}

/// Cloneable, owned variant of [`DispatchError`] used inside [`MockDispatcher`]
/// (because [`DispatchError`] doesn't implement `Clone` due to underlying
/// transports).
#[derive(Debug, Clone)]
enum MockError {
    DelegateMissing { delegate: String, reason: String },
    SecurityRefused { reason: String },
    Transport(String),
}

impl From<MockError> for DispatchError {
    fn from(value: MockError) -> Self {
        match value {
            MockError::DelegateMissing { delegate, reason } => {
                DispatchError::DelegateMissing { delegate, reason }
            }
            MockError::SecurityRefused { reason } => DispatchError::SecurityRefused { reason },
            MockError::Transport(s) => DispatchError::Transport(s),
        }
    }
}

impl MockDispatcher {
    /// New empty mock — all steps return `DispatchOutcome::success()`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure a successful outcome for `step_id`.
    pub fn with_success(mut self, step_id: impl Into<String>, outcome: DispatchOutcome) -> Self {
        self.outcomes.insert(step_id.into(), Ok(outcome));
        self
    }

    /// Configure a delegate-missing failure for `step_id`.
    pub fn with_missing(
        mut self,
        step_id: impl Into<String>,
        delegate: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        self.outcomes.insert(
            step_id.into(),
            Err(MockError::DelegateMissing {
                delegate: delegate.into(),
                reason: reason.into(),
            }),
        );
        self
    }

    /// Configure a security-refused failure for `step_id`.
    pub fn with_security_refused(
        mut self,
        step_id: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        self.outcomes.insert(
            step_id.into(),
            Err(MockError::SecurityRefused {
                reason: reason.into(),
            }),
        );
        self
    }

    /// Configure a transport failure for `step_id`.
    pub fn with_transport_error(
        mut self,
        step_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        self.outcomes
            .insert(step_id.into(), Err(MockError::Transport(message.into())));
        self
    }

    /// Override the default outcome returned when `step_id` isn't configured.
    pub fn with_default(mut self, outcome: DispatchOutcome) -> Self {
        self.default_outcome = outcome;
        self
    }
}

#[async_trait]
impl Dispatcher for MockDispatcher {
    async fn dispatch(&self, step: &Step) -> Result<DispatchOutcome, DispatchError> {
        match self.outcomes.get(&step.id) {
            Some(Ok(out)) => Ok(out.clone()),
            Some(Err(err)) => Err(err.clone().into()),
            None => Ok(self.default_outcome.clone()),
        }
    }
}

// =====================================================================
// Stub: RecordingDispatcher
// =====================================================================

/// Wrapper dispatcher recording every dispatched step ID. Useful for
/// asserting topological order in tests.
pub struct RecordingDispatcher<D: Dispatcher> {
    inner: D,
    calls: Mutex<Vec<String>>,
}

impl<D: Dispatcher> RecordingDispatcher<D> {
    /// Wrap an inner dispatcher.
    pub fn new(inner: D) -> Self {
        Self {
            inner,
            calls: Mutex::new(Vec::new()),
        }
    }

    /// Snapshot of step IDs in dispatch order. Returns a clone so the lock
    /// is released immediately.
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned (a previous call panicked
    /// while holding the lock). Tests treat this as an unrecoverable bug.
    pub fn calls(&self) -> Vec<String> {
        self.calls
            .lock()
            .expect("RecordingDispatcher mutex poisoned")
            .clone()
    }
}

#[async_trait]
impl<D: Dispatcher> Dispatcher for RecordingDispatcher<D> {
    async fn dispatch(&self, step: &Step) -> Result<DispatchOutcome, DispatchError> {
        self.calls
            .lock()
            .expect("RecordingDispatcher mutex poisoned")
            .push(step.id.clone());
        self.inner.dispatch(step).await
    }
}

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playbook::types::{Delegation, OnError, Step};

    fn agent_step(id: &str) -> Step {
        Step {
            id: id.to_string(),
            delegate_to: Delegation::Agent {
                name: "a".to_string(),
            },
            input: None,
            produces_at: None,
            mapping: None,
            requires: None,
            fallback_hint: None,
            on_error: OnError::Abort,
        }
    }

    fn command_step(id: &str) -> Step {
        Step {
            id: id.to_string(),
            delegate_to: Delegation::Command {
                command: vec!["echo".to_string(), "hi".to_string()],
            },
            input: None,
            produces_at: None,
            mapping: None,
            requires: None,
            fallback_hint: None,
            on_error: OnError::Abort,
        }
    }

    /// `MockDispatcher::with_default` returns success for unknown step IDs.
    #[tokio::test]
    async fn mock_default_success_for_unknown_step() {
        let mock = MockDispatcher::new();
        let step = agent_step("unconfigured");
        let outcome = mock.dispatch(&step).await.expect("ok");
        assert!(outcome.success);
    }

    /// `MockDispatcher::with_success` overrides per step ID.
    #[tokio::test]
    async fn mock_with_success_returns_configured_outcome() {
        let mock = MockDispatcher::new().with_success(
            "s1",
            DispatchOutcome {
                success: true,
                output_path: Some(PathBuf::from("out.md")),
                stderr: None,
            },
        );
        let step = agent_step("s1");
        let outcome = mock.dispatch(&step).await.expect("ok");
        assert!(outcome.success);
        assert_eq!(
            outcome.output_path.as_deref(),
            Some(std::path::Path::new("out.md"))
        );
    }

    /// `MockDispatcher::with_missing` returns DelegateMissing.
    #[tokio::test]
    async fn mock_with_missing_returns_error() {
        let mock =
            MockDispatcher::new().with_missing("s1", "plugin:c4", "install via brew install c4");
        let step = agent_step("s1");
        let err = mock.dispatch(&step).await.expect_err("missing");
        match err {
            DispatchError::DelegateMissing { delegate, reason } => {
                assert_eq!(delegate, "plugin:c4");
                assert!(reason.contains("brew install"));
            }
            other => panic!("expected DelegateMissing, got {other:?}"),
        }
    }

    /// `RecordingDispatcher` captures dispatch order.
    #[tokio::test]
    async fn recording_dispatcher_captures_calls_in_order() {
        let inner = MockDispatcher::new();
        let rec = RecordingDispatcher::new(inner);
        rec.dispatch(&agent_step("alpha")).await.expect("ok");
        rec.dispatch(&agent_step("beta")).await.expect("ok");
        rec.dispatch(&agent_step("gamma")).await.expect("ok");
        assert_eq!(rec.calls(), vec!["alpha", "beta", "gamma"]);
    }

    /// `validate_command_delegate_security`: Command without --yes refused.
    #[test]
    fn security_refuses_command_without_yes() {
        let step = command_step("danger");
        let err = validate_command_delegate_security(&step, false).expect_err("must refuse");
        match err {
            SecurityError::ShellRequiresYes { step_id } => assert_eq!(step_id, "danger"),
        }
    }

    /// Command WITH --yes is allowed.
    #[test]
    fn security_allows_command_with_yes() {
        let step = command_step("danger");
        validate_command_delegate_security(&step, true).expect("--yes allows");
    }

    /// Non-command delegates always allowed regardless of yes flag.
    #[test]
    fn security_allows_non_command_steps() {
        let step = agent_step("safe");
        validate_command_delegate_security(&step, false).expect("agent always ok");
        validate_command_delegate_security(&step, true).expect("agent always ok");
    }

    /// `DispatchOutcome::failure` helper sets fields correctly.
    #[test]
    fn dispatch_outcome_failure_helper() {
        let out = DispatchOutcome::failure("boom");
        assert!(!out.success);
        assert_eq!(out.stderr.as_deref(), Some("boom"));
        assert!(out.output_path.is_none());
    }
}
