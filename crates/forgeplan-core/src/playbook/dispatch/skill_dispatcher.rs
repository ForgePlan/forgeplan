//! Production [`Dispatcher`] for `Delegation::Skill` variant (FR-3).
//!
//! Phase 6 Wave 1 — owner: **skill-dispatcher** teammate.
//!
//! # What is a "skill" in this dispatch model
//!
//! Skills (в смысле Claude Code и марketplace `dev-toolkit`) загружаются в
//! **активный агентский контекст** через slash-command (`/skill-name`) или
//! MCP tool call. В отличие от `plugin` или `agent` delegations, skill **не
//! запускается отдельным subprocess** — он расширяет текущего агента
//! инлайн, делая доступными новые prompt-инструкции и tool routing.
//!
//! Это означает, что `SkillDispatcher` принципиально отличается от
//! [`super::plugin_dispatcher::PluginDispatcher`] /
//! [`super::agent_dispatcher::AgentDispatcher`]: ему не нужен
//! `helpers::run_subprocess`, не нужен `tokio::process::Command`,
//! не нужны env-allowlist'ы и timeout.
//!
//! # Phase 6 v1 limitation (intentional stub)
//!
//! Phase 6 Wave 1 — minimal viable: `SkillDispatcher::dispatch` **не
//! загружает skill в реальный агентский контекст**. В рантайме исполнителя
//! playbook (executor под тестами) у нас нет доступа к runtime-агента
//! Claude Code — он сам и есть тот процесс, кто читает stdout. Поэтому v1
//! ограничивается тем, что:
//!
//! 1. Эмитит **одну строку trace** в stdout: `[skill-invoke] /<name>` или
//!    `[skill-invoke] /<pack>/<name>` (если `pack` задан). Эту строку
//!    Claude Code-харнесс может перехватывать и интерпретировать как
//!    инструкцию активировать соответствующий skill в текущем сеансе.
//! 2. Возвращает `DispatchOutcome { success: true, output_path: step.produces_at.clone(), .. }`.
//!    Никаких реальных артефактов skill сам по себе не пишет — `produces_at`
//!    приходит из step-конфига и просто прокидывается дальше для совместимости
//!    с executor-агрегацией artifact-paths.
//! 3. Не падает на отсутствие skill в registry: registry в v1 ещё нет.
//!
//! # Wave 5+ plan (real skill loader)
//!
//! Когда в Phase 6 Wave 5 появится **skill registry** (либо чтение
//! `~/.claude/skills/*` либо запрос к MCP-серверу marketplace), этот
//! dispatcher будет:
//! - резолвить `name` (+`pack`) в filesystem-путь skill manifest;
//! - валидировать что skill установлен и совместим с текущим harness;
//! - инжектировать skill инструкции в активный turn (через MCP tool call,
//!   либо через слот в Task input — TBD по результатам Wave 5 spike);
//! - возвращать `DispatchError::DelegateMissing` если skill не найден;
//! - сохранять идемпотентность: повторный dispatch того же skill в одном
//!   playbook-run должен быть no-op (skill уже активен).
//!
//! Deviation from EVID-090 contract (subprocess-based dispatchers) задокументирован
//! как Wave 5 follow-up в EVID-090 §"Open questions" — этот файл служит
//! reference impl для in-process delegation pattern.

use std::path::PathBuf;

use async_trait::async_trait;

use super::{DispatchError, DispatchOutcome, Dispatcher};
use crate::playbook::types::{Delegation, Step};

/// FR-3: Production skill dispatcher (Phase 6 v1 stub — see module doc).
///
/// `workspace_root` хранится для будущей резолюции skill registry (Wave 5+);
/// в v1 не используется на dispatch-пути, но конструктор уже принимает его,
/// чтобы Wave 5 миграция не требовала изменения call-site executor'а.
pub struct SkillDispatcher {
    #[allow(dead_code)] // Wave 5: skill-registry path resolution.
    workspace_root: PathBuf,
}

impl SkillDispatcher {
    /// Construct dispatcher with workspace root (used by Wave 5 skill registry).
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    /// Format the v1 skill-invoke trace line. Stable contract — Claude Code
    /// harness может grep'ать эту префиксную строку.
    fn format_invoke_trace(name: &str, pack: Option<&str>) -> String {
        match pack {
            Some(p) => format!("[skill-invoke] /{p}/{name}"),
            None => format!("[skill-invoke] /{name}"),
        }
    }
}

#[async_trait]
impl Dispatcher for SkillDispatcher {
    async fn dispatch(&self, step: &Step) -> Result<DispatchOutcome, DispatchError> {
        let (name, pack) = match &step.delegate_to {
            Delegation::Skill { name, pack } => (name.as_str(), pack.as_deref()),
            other => {
                return Err(DispatchError::Transport(format!(
                    "SkillDispatcher invoked on non-Skill delegation `{}` for step `{}`",
                    delegation_kind(other),
                    step.id,
                )));
            }
        };

        // v1: emit trace line — Claude Code harness can intercept stdout
        // and translate into actual slash-command invocation. See module doc.
        println!("{}", Self::format_invoke_trace(name, pack));

        Ok(DispatchOutcome {
            success: true,
            output_path: step.produces_at.clone(),
            stderr: None,
        })
    }
}

/// Human label for a Delegation variant — used in error messages.
fn delegation_kind(d: &Delegation) -> &'static str {
    match d {
        Delegation::Plugin { .. } => "plugin",
        Delegation::Agent { .. } => "agent",
        Delegation::Skill { .. } => "skill",
        Delegation::Command { .. } => "command",
        Delegation::ForgeplanCore { .. } => "forgeplan_core",
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::playbook::types::{Delegation, OnError, Step};

    fn skill_step(id: &str, name: &str, pack: Option<&str>) -> Step {
        Step {
            id: id.to_string(),
            delegate_to: Delegation::Skill {
                name: name.to_string(),
                pack: pack.map(str::to_string),
            },
            input: None,
            produces_at: None,
            mapping: None,
            requires: None,
            fallback_hint: None,
            on_error: OnError::Abort,
            timeout_seconds: None,
        }
    }

    fn agent_step(id: &str) -> Step {
        Step {
            id: id.to_string(),
            delegate_to: Delegation::Agent {
                name: "some-agent".to_string(),
            },
            input: None,
            produces_at: None,
            mapping: None,
            requires: None,
            fallback_hint: None,
            on_error: OnError::Abort,
            timeout_seconds: None,
        }
    }

    /// v1 stub returns success and emits a trace line containing the skill name.
    #[tokio::test]
    async fn skill_dispatcher_returns_success_for_v1_stub() {
        let dispatcher = SkillDispatcher::new(PathBuf::from("/tmp/ws"));
        let step = skill_step("s1", "rust-expert", None);

        let outcome = dispatcher.dispatch(&step).await.expect("ok");

        assert!(outcome.success);
        assert!(outcome.stderr.is_none());

        // Independently verify the trace formatter (we can't capture stdout
        // here cross-platform, but the formatter is the contract surface).
        let trace = SkillDispatcher::format_invoke_trace("rust-expert", None);
        assert_eq!(trace, "[skill-invoke] /rust-expert");
        assert!(trace.contains("rust-expert"));
    }

    /// Non-skill delegations are rejected as Transport errors — wrong dispatcher.
    #[tokio::test]
    async fn skill_dispatcher_rejects_non_skill_delegation() {
        let dispatcher = SkillDispatcher::new(PathBuf::from("/tmp/ws"));
        let step = agent_step("wrong");

        let err = dispatcher
            .dispatch(&step)
            .await
            .expect_err("must reject Agent delegation");

        match err {
            DispatchError::Transport(msg) => {
                assert!(msg.contains("non-Skill"), "unexpected msg: {msg}");
                assert!(msg.contains("agent"), "should name the kind: {msg}");
                assert!(msg.contains("wrong"), "should include step id: {msg}");
            }
            other => panic!("expected Transport, got {other:?}"),
        }
    }

    /// `pack=Some(..)` produces `[skill-invoke] /<pack>/<name>`.
    #[tokio::test]
    async fn skill_dispatcher_includes_pack_prefix_when_set() {
        let dispatcher = SkillDispatcher::new(PathBuf::from("/tmp/ws"));
        let step = skill_step("s2", "skill-x", Some("brownfield"));

        let outcome = dispatcher.dispatch(&step).await.expect("ok");
        assert!(outcome.success);

        let trace = SkillDispatcher::format_invoke_trace("skill-x", Some("brownfield"));
        assert_eq!(trace, "[skill-invoke] /brownfield/skill-x");
        assert!(trace.contains("brownfield/skill-x"));
    }

    /// `produces_at` is propagated into `DispatchOutcome.output_path` verbatim.
    #[tokio::test]
    async fn skill_dispatcher_returns_step_produces_at_as_output_path() {
        let dispatcher = SkillDispatcher::new(PathBuf::from("/tmp/ws"));
        let mut step = skill_step("s3", "doc-writer", None);
        step.produces_at = Some(PathBuf::from(".forgeplan/notes/skill-out.md"));

        let outcome = dispatcher.dispatch(&step).await.expect("ok");

        assert!(outcome.success);
        assert_eq!(
            outcome.output_path.as_deref(),
            Some(std::path::Path::new(".forgeplan/notes/skill-out.md")),
        );
    }

    /// Absent `produces_at` ⇒ `output_path` is None (no synthetic path injected).
    #[tokio::test]
    async fn skill_dispatcher_propagates_none_output_path() {
        let dispatcher = SkillDispatcher::new(PathBuf::from("/tmp/ws"));
        let step = skill_step("s4", "linter", None);

        let outcome = dispatcher.dispatch(&step).await.expect("ok");
        assert!(outcome.success);
        assert!(outcome.output_path.is_none());
    }

    /// Trace-formatter pure-function check: no pack ⇒ no double-slash.
    #[test]
    fn format_invoke_trace_no_pack_has_single_slash() {
        let trace = SkillDispatcher::format_invoke_trace("foo", None);
        assert_eq!(trace, "[skill-invoke] /foo");
        assert!(!trace.contains("//"));
    }
}
