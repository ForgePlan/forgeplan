//! Plugin detection and self-describing playbook recommendations.
//!
//! Extends ADR-008 hint contract with playbook recommendations based on
//! installed plugins and project signals. See
//! [`PRD-067`](../../../../.forgeplan/prds/PRD-067-plugin-detection-self-describing-hints-playbook-recommendations.md).
//!
//! # Wave layout
//!
//! - **Wave 1** (`types`): data contracts — `PluginRegistry`, `PluginInfo`,
//!   `InstalledPlugin`, `ProjectSignals`, `RecommendedPlaybookHint`,
//!   `default_registry()`.
//! - **Wave 2** (this commit):
//!   - `detection` — `PluginScanner` trait + `FilesystemScanner` /
//!     `StubScanner`, `detect_plugins()` convenience.
//!   - `registry` — `extended_registry()` and `merge_user_registry()` on top
//!     of Wave 1 default.
//!   - `signals` — `detect_signals()` + filesystem-only
//!     `signals_from_tempdir()` for tests.
//!   - `hints` — `KnownPlaybook`, `build_recommendations()`,
//!     `format_recommendations()` formatter for ADR-008 self-describing
//!     output. Does NOT modify the existing `forgeplan-core::hints` module —
//!     Wave 3 will integrate.
//! - **Wave 3** (planned): CLI / MCP surfaces (`forgeplan plugins
//!   {list,doctor,info}`) and integration with the existing `hints.rs` Hint
//!   stream so `forgeplan init` emits playbook recommendations on stderr.

pub mod detection;
pub mod hints;
pub mod registry;
pub mod signals;
pub mod types;

pub use types::{
    InstalledPlugin, PlaybookRecommendation, PluginInfo, PluginRegistry, PluginSource,
    ProjectSignals, RecommendedPlaybookHint, TriggeredBy, default_registry,
};

pub use detection::{FilesystemScanner, PluginScanner, StubScanner, detect_plugins};
pub use hints::{
    KnownPlaybook, build_recommendations, format_recommendations, render_recommendations,
};
pub use registry::{extended_registry, merge_user_registry};
pub use signals::{SignalError, detect_signals, signals_from_tempdir};
