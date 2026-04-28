//! Playbook runtime — declarative YAML-based orchestration.
//!
//! See [`SPEC-003`](../../../.forgeplan/specs/SPEC-003-playbook-yaml-schema.md)
//! for the YAML schema contract and [`PRD-065`](../../../.forgeplan/prds/PRD-065-playbook-yaml-schema-runtime-executor.md)
//! for goals and acceptance criteria.
//!
//! # Module layout
//!
//! - [`types`]    — Wave 1: serde + `JsonSchema` types mirroring SPEC-003.
//! - [`loader`]   — Wave 2: parse YAML, enforce SPEC-003 §"Errors" matrix.
//! - [`dispatch`] — Wave 2: trait + stubs (mock / recording). Wave 3 wires
//!   the five real delegate variants (plugin, agent, skill, command,
//!   forgeplan_core).
//! - [`executor`] — Wave 2: sequential runner (topological order, journal
//!   integration, `on_error` policy).
//! - [`journal`]  — Wave 2: append-only JSONL writer for resumable runs
//!   (PRD-065 FR-6).

pub mod dispatch;
pub mod executor;
pub mod journal;
pub mod loader;
pub mod types;

pub use dispatch::{
    DispatchError, DispatchOutcome, Dispatcher, MockDispatcher, RecordingDispatcher, SecurityError,
    validate_command_delegate_security,
};
pub use executor::{
    ExecutionReport, Executor, ExecutorConfig, ExecutorError, StepReport, StepStatus,
};
pub use journal::{Journal, JournalEntry, JournalEntryKind, RunId};
pub use loader::{LoaderError, SUPPORTED_SCHEMA_RANGE, load_playbook};
pub use types::{
    Delegation, ForgeplanOp, OnError, Playbook, PluginRequirement, Requirements, SchemaVersion,
    SkillRequirement, Step, TriggeredBy,
};
