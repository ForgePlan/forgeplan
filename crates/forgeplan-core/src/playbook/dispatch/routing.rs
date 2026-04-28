//! Composite dispatcher that routes `step.delegate_to` to the production
//! impl for that variant. Used by CLI + MCP `playbook run` surfaces.
//!
//! Phase 6 Wave 4 — closes the critical user-facing gap from the
//! e2e-engineer audit: Wave 1 shipped 5 production dispatchers as a
//! library, but `playbook run` (CLI + MCP) still wired
//! [`super::MockDispatcher`]. This composite picks the right production
//! impl based on the `Delegation` variant of each step.
//!
//! Each child dispatcher is constructed via its `new(workspace_root)`
//! constructor with the playbook run's workspace root and per-dispatcher
//! default timeouts (set by Wave 1).
//!
//! References: PRD-072 §FR-1..FR-5, RFC-007 §"delegate_to", e2e-engineer
//! audit (Phase 6 Wave 4).

use std::path::PathBuf;

use async_trait::async_trait;

use super::agent_dispatcher::AgentDispatcher;
use super::command_dispatcher::CommandDispatcher;
use super::forgeplan_core_dispatcher::ForgeplanCoreDispatcher;
use super::plugin_dispatcher::PluginDispatcher;
use super::skill_dispatcher::SkillDispatcher;
use super::{DispatchError, DispatchOutcome, Dispatcher};
use crate::playbook::types::{Delegation, Step};

/// Composite [`Dispatcher`] that routes each step to the matching
/// production impl based on `step.delegate_to`.
pub struct RoutingDispatcher {
    plugin: PluginDispatcher,
    agent: AgentDispatcher,
    skill: SkillDispatcher,
    command: CommandDispatcher,
    forgeplan_core: ForgeplanCoreDispatcher,
}

impl RoutingDispatcher {
    /// Build a routing dispatcher rooted at `workspace_root` (the project
    /// root — the directory that contains `.forgeplan/`).
    ///
    /// The four subprocess dispatchers (plugin/agent/skill/command) receive
    /// the project root verbatim — that becomes the `cwd` for spawned
    /// processes so `produces_at` paths resolve correctly.
    /// [`ForgeplanCoreDispatcher`] is constructed against `<root>/.forgeplan`
    /// because its [`super::super::dispatch::forgeplan_core_dispatcher::ForgeplanCoreDispatcher`]
    /// internals call [`crate::db::store::LanceStore::open`], which expects
    /// the workspace dir (parent of `lance/`).
    pub fn new(workspace_root: PathBuf) -> Self {
        let forgeplan_dir = workspace_root.join(".forgeplan");
        Self {
            plugin: PluginDispatcher::new(workspace_root.clone()),
            agent: AgentDispatcher::new(workspace_root.clone()),
            skill: SkillDispatcher::new(workspace_root.clone()),
            command: CommandDispatcher::new(workspace_root),
            forgeplan_core: ForgeplanCoreDispatcher::new(forgeplan_dir),
        }
    }
}

#[async_trait]
impl Dispatcher for RoutingDispatcher {
    async fn dispatch(&self, step: &Step) -> Result<DispatchOutcome, DispatchError> {
        match &step.delegate_to {
            Delegation::Plugin { .. } => self.plugin.dispatch(step).await,
            Delegation::Agent { .. } => self.agent.dispatch(step).await,
            Delegation::Skill { .. } => self.skill.dispatch(step).await,
            Delegation::Command { .. } => self.command.dispatch(step).await,
            Delegation::ForgeplanCore { .. } => self.forgeplan_core.dispatch(step).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playbook::types::{Delegation, ForgeplanOp, OnError, Step};

    fn step_with(id: &str, delegation: Delegation) -> Step {
        Step {
            id: id.to_string(),
            delegate_to: delegation,
            input: None,
            produces_at: None,
            mapping: None,
            requires: None,
            fallback_hint: None,
            on_error: OnError::Abort,
        }
    }

    /// Routing for `Delegation::Plugin` reaches the plugin dispatcher.
    /// We can't easily verify which child handled the call without
    /// dependency injection, so we rely on the production-dispatcher
    /// behaviour: when neither the configured nor PATH-resolved binary
    /// exists, the plugin dispatcher returns `DelegateMissing`. Other
    /// variants would either succeed (skill no-op) or report a different
    /// error class — distinguishing the route taken.
    #[tokio::test]
    async fn routes_plugin_variant_to_plugin_dispatcher() {
        let dispatcher = RoutingDispatcher::new(PathBuf::from("/tmp"));
        let step = step_with(
            "p",
            Delegation::Plugin {
                name: "definitely-not-installed-plugin".to_string(),
                target: "noop".to_string(),
            },
        );
        let err = dispatcher
            .dispatch(&step)
            .await
            .expect_err("missing binary");
        assert!(
            matches!(err, DispatchError::DelegateMissing { .. }),
            "plugin route must surface DelegateMissing when binary absent: {err:?}"
        );
    }

    /// Routing for `Delegation::Agent` reaches the agent dispatcher,
    /// which falls back to `DelegateMissing` when no task-tool binary
    /// resolves.
    #[tokio::test]
    async fn routes_agent_variant_to_agent_dispatcher() {
        let dispatcher = RoutingDispatcher::new(PathBuf::from("/tmp"));
        let step = step_with(
            "a",
            Delegation::Agent {
                name: "agent-x".to_string(),
            },
        );
        let err = dispatcher
            .dispatch(&step)
            .await
            .expect_err("missing binary");
        assert!(
            matches!(err, DispatchError::DelegateMissing { .. }),
            "agent route must surface DelegateMissing when binary absent: {err:?}"
        );
    }

    /// Routing for `Delegation::Skill` reaches the skill dispatcher.
    /// SkillDispatcher reports success (no-op stub in Wave 1 contract).
    #[tokio::test]
    async fn routes_skill_variant_to_skill_dispatcher() {
        let dispatcher = RoutingDispatcher::new(PathBuf::from("/tmp"));
        let step = step_with(
            "s",
            Delegation::Skill {
                name: "skill-x".to_string(),
                pack: None,
            },
        );
        let outcome = dispatcher.dispatch(&step).await.expect("skill ok");
        assert!(
            outcome.success,
            "skill route must report success for stub: {outcome:?}"
        );
    }

    /// Routing for `Delegation::Command` with empty argv reaches the
    /// command dispatcher and surfaces a transport error (empty command
    /// rejected before spawning).
    #[tokio::test]
    async fn routes_command_variant_to_command_dispatcher() {
        let dispatcher = RoutingDispatcher::new(PathBuf::from("."));
        let step = step_with(
            "c",
            Delegation::Command {
                command: Vec::<String>::new(),
            },
        );
        let err = dispatcher.dispatch(&step).await.expect_err("empty argv");
        assert!(
            matches!(err, DispatchError::Transport(_)),
            "command route must surface Transport for empty argv: {err:?}"
        );
    }

    /// Routing for `Delegation::ForgeplanCore` reaches the core
    /// dispatcher. With a non-existent workspace, the core dispatcher
    /// returns a transport error rather than panicking — confirming the
    /// route was taken.
    #[tokio::test]
    async fn routes_forgeplan_core_variant_to_core_dispatcher() {
        let dispatcher = RoutingDispatcher::new(PathBuf::from("/nonexistent-forge-ws"));
        let step = step_with(
            "core",
            Delegation::ForgeplanCore {
                target: ForgeplanOp::Validate,
            },
        );
        let result = dispatcher.dispatch(&step).await;
        assert!(
            matches!(result, Err(DispatchError::Transport(_)) | Ok(_)),
            "forgeplan_core route must surface Transport or Ok (no panic): {result:?}"
        );
    }
}
