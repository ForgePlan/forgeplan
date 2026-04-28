//! `forgeplan playbook` CLI surface (PRD-065).
//!
//! Wave 3 implementation by agent **w3a-cli-playbook**. Wires
//! `forgeplan-core::playbook::{loader,executor,dispatch,journal}` to
//! user-facing CLI flags + hint contract emission per PRD-071.
//!
//! # Discovery roots
//!
//! Playbooks are searched in this order (first hit wins for `show`/`run`):
//!
//! 1. `<workspace>/.forgeplan/playbooks/*.yaml`
//! 2. `~/.claude/plugins/*/playbooks/*.yaml`
//!
//! Built-in / packaged playbooks are not yet shipped — this is intentional
//! and produces an empty list on a fresh install (Wave 4 will seed bundled
//! playbooks).
//!
//! # Wave 4 follow-up
//!
//! `run_execute` currently uses `MockDispatcher::AlwaysOk` because the real
//! Plugin/Agent/Skill/Command/ForgeplanCore dispatchers are scheduled for
//! Wave 4 (subprocess invocation via Task tool). The CLI surface, journal
//! integration, and hint contract are complete; only the dispatch backend
//! remains stubbed. A `tracing::warn!`-equivalent stderr line is emitted on
//! every real run so users are not surprised.

use std::path::{Path, PathBuf};

use forgeplan_core::playbook::{
    DispatchOutcome, ExecutorConfig, MockDispatcher, Playbook,
    executor::{ExecutionReport, Executor},
    journal::Journal,
    loader::{LoaderError, load_playbook},
    types::{Delegation, ForgeplanOp, Step},
};
use forgeplan_core::workspace;

// =====================================================================
// Public commands (wired in main.rs)
// =====================================================================

/// `forgeplan playbook list [--json]`
pub async fn run_list(json: bool) -> anyhow::Result<()> {
    let entries = discover_playbooks();

    if json {
        let arr: Vec<serde_json::Value> = entries
            .iter()
            .map(|e| {
                serde_json::json!({
                    "name": e.playbook.name,
                    "title": e.playbook.title,
                    "steps_count": e.playbook.steps.len(),
                    "source_path": e.source.display().to_string(),
                })
            })
            .collect();

        let next_action = entries
            .first()
            .map(|e| format!("forgeplan playbook show {}", e.playbook.name));

        let payload = serde_json::json!({
            "playbooks": arr,
            "_next_action": next_action,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    if entries.is_empty() {
        println!("No playbooks found.");
        println!("  Searched: .forgeplan/playbooks/*.yaml, ~/.claude/plugins/*/playbooks/*.yaml");
        println!();
        println!("Done.");
        return Ok(());
    }

    println!("Available playbooks ({}):", entries.len());
    println!();
    let name_w = entries
        .iter()
        .map(|e| e.playbook.name.len())
        .max()
        .unwrap_or(4)
        .max(4);
    let title_w = entries
        .iter()
        .map(|e| e.playbook.title.len())
        .max()
        .unwrap_or(5)
        .max(5);

    println!(
        "  {:<name_w$}  {:<title_w$}  {:>5}  SOURCE",
        "NAME",
        "TITLE",
        "STEPS",
        name_w = name_w,
        title_w = title_w
    );
    for e in &entries {
        println!(
            "  {:<name_w$}  {:<title_w$}  {:>5}  {}",
            e.playbook.name,
            e.playbook.title,
            e.playbook.steps.len(),
            e.source.display(),
            name_w = name_w,
            title_w = title_w
        );
    }
    println!();
    println!("Next: forgeplan playbook show {}", entries[0].playbook.name);
    Ok(())
}

/// `forgeplan playbook show <target> [--json]`
///
/// `target` may be a playbook name (matched against discovered playbooks) or
/// a path to a `.yaml` file.
pub async fn run_show(target: &str, json: bool) -> anyhow::Result<()> {
    let resolved = match resolve_target(target) {
        Ok(path) => path,
        Err(msg) => {
            print_resolve_error(target, &msg, json);
            std::process::exit(2);
        }
    };

    let yaml = match std::fs::read_to_string(&resolved) {
        Ok(s) => s,
        Err(e) => {
            print_io_error(&resolved, &e, json);
            std::process::exit(2);
        }
    };

    let pb = match load_playbook(&yaml) {
        Ok(pb) => pb,
        Err(err) => {
            emit_loader_error(&resolved, &err, json);
            std::process::exit(2);
        }
    };

    if json {
        let payload = serde_json::json!({
            "playbook": pb,
            "source_path": resolved.display().to_string(),
            "_next_action": format!("forgeplan playbook run {} --yes --dry-run", pb.name),
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    println!("Playbook: {}", pb.name);
    println!("Title:    {}", pb.title);
    if let Some(desc) = &pb.description {
        println!("Description:");
        for line in desc.lines() {
            println!("  {}", line);
        }
    }
    println!("Source:   {}", resolved.display());

    if let Some(reqs) = &pb.requires {
        if !reqs.plugins.is_empty() {
            println!();
            println!("Requires plugins:");
            for p in &reqs.plugins {
                match &p.version {
                    Some(v) => println!("  - {} ({})", p.name, v),
                    None => println!("  - {}", p.name),
                }
            }
        }
        if !reqs.skills.is_empty() {
            println!();
            println!("Requires skills:");
            for s in &reqs.skills {
                match &s.pack {
                    Some(pack) => println!("  - {} (pack: {})", s.name, pack),
                    None => println!("  - {}", s.name),
                }
            }
        }
    }

    println!();
    println!("Steps ({}):", pb.steps.len());
    for (idx, step) in pb.steps.iter().enumerate() {
        println!(
            "  [{}] {}: delegate={}",
            idx + 1,
            step.id,
            delegate_label(step)
        );
        if let Some(reqs) = &step.requires
            && !reqs.is_empty()
        {
            println!("      requires: {}", reqs.join(", "));
        }
        if let Some(produces) = &step.produces_at {
            println!("      produces_at: {}", produces.display());
        }
        if let Some(mapping) = &step.mapping {
            println!("      mapping: {}", mapping);
        }
        if let Some(hint) = &step.fallback_hint {
            println!("      fallback_hint: {}", hint);
        }
    }
    println!();
    println!("Next: forgeplan playbook run {} --yes --dry-run", pb.name);
    Ok(())
}

/// `forgeplan playbook validate <file> [--json]`
pub async fn run_validate(file: &Path, json: bool) -> anyhow::Result<()> {
    let yaml = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            print_io_error(file, &e, json);
            std::process::exit(2);
        }
    };

    match load_playbook(&yaml) {
        Ok(pb) => {
            if json {
                let payload = serde_json::json!({
                    "passed": true,
                    "name": pb.name,
                    "title": pb.title,
                    "steps_count": pb.steps.len(),
                    "source_path": file.display().to_string(),
                    "_next_action": format!(
                        "forgeplan playbook run {} --yes --dry-run",
                        pb.name
                    ),
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("OK: {} ({} steps)", pb.name, pb.steps.len());
                println!();
                println!("Done.");
            }
            Ok(())
        }
        Err(err) => {
            emit_loader_error(file, &err, json);
            std::process::exit(2);
        }
    }
}

/// `forgeplan playbook run <target> --yes [--dry-run] [--step N] [--json]`
pub async fn run_execute(
    target: &str,
    yes: bool,
    dry_run: bool,
    step: Option<usize>,
    json: bool,
) -> anyhow::Result<()> {
    // ADR-009 / SPEC-003 §"delegate_to": refuse without --yes.
    if !yes && !dry_run {
        let fix = format!(
            "forgeplan playbook run {} --yes",
            shell_quote_if_needed(target)
        );
        if json {
            let payload = serde_json::json!({
                "error": "playbook run requires --yes confirmation",
                "_next_action": fix,
            });
            println!("{}", serde_json::to_string_pretty(&payload)?);
        } else {
            eprintln!("Error: playbook run requires --yes confirmation (ADR-009 security gate).");
            eprintln!("Fix: {}", fix);
        }
        std::process::exit(2);
    }

    // Resolve + load.
    let resolved = match resolve_target(target) {
        Ok(p) => p,
        Err(msg) => {
            print_resolve_error(target, &msg, json);
            std::process::exit(2);
        }
    };

    let yaml = match std::fs::read_to_string(&resolved) {
        Ok(s) => s,
        Err(e) => {
            print_io_error(&resolved, &e, json);
            std::process::exit(2);
        }
    };

    let pb = match load_playbook(&yaml) {
        Ok(pb) => pb,
        Err(err) => {
            emit_loader_error(&resolved, &err, json);
            std::process::exit(2);
        }
    };

    // Validate `--step N` early so we don't get partway in.
    let start_step = match step {
        Some(n) if n == 0 || n > pb.steps.len() => {
            let msg = format!(
                "--step out of range: requested {}, playbook has {} step(s) (1..={})",
                n,
                pb.steps.len(),
                pb.steps.len()
            );
            if json {
                let payload = serde_json::json!({
                    "error": msg,
                    "_next_action": format!("forgeplan playbook show {}", pb.name),
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                eprintln!("Error: {}", msg);
                eprintln!("Fix: forgeplan playbook show {}", pb.name);
            }
            std::process::exit(2);
        }
        Some(n) => Some(n),
        None => None,
    };

    if dry_run {
        return run_dry_run(&pb, &resolved, start_step, json);
    }

    // Real run — delegate to MockDispatcher (Wave 4 wires real backends).
    eprintln!(
        "warn: Real dispatchers wired in Wave 4 — using MockDispatcher::AlwaysOk for {}",
        pb.name
    );

    // `Journal::open` expects the project root (parent of `.forgeplan/`),
    // because it builds `<root>/.forgeplan/journal/...`. `find_workspace`
    // returns the `.forgeplan/` dir itself, so we step up one level when it
    // matches.
    let cwd = std::env::current_dir()?;
    let workspace_root = match workspace::find_workspace(&cwd) {
        Some(fp_dir) => fp_dir
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| cwd.clone()),
        None => cwd.clone(),
    };
    let journal = Journal::open(&workspace_root)?;

    let dispatcher = MockDispatcher::new().with_default(DispatchOutcome::success());
    let cfg = ExecutorConfig {
        yes_flag: yes,
        // load_playbook already validated; skip duplicate work in executor.
        skip_revalidation: true,
    };
    let mut executor = Executor::new(dispatcher, journal, cfg);

    if !json {
        eprintln!("Running playbook: {} ({} steps)", pb.name, pb.steps.len());
    }

    let report = executor.run(&pb).await?;

    if json {
        let payload = serde_json::json!({
            "playbook": pb.name,
            "report": report,
            "_next_action": next_action_after_run(&pb, &report),
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    print_run_text(&pb, &report);
    Ok(())
}

// =====================================================================
// Discovery & resolution
// =====================================================================

/// One discovered playbook plus its source file path.
struct DiscoveredPlaybook {
    playbook: Playbook,
    source: PathBuf,
}

/// Discover playbooks in workspace + plugin dirs.
///
/// Failed-to-parse files are silently skipped (with a stderr `warn:` line) —
/// listing must not abort because of one corrupt entry.
fn discover_playbooks() -> Vec<DiscoveredPlaybook> {
    let mut out: Vec<DiscoveredPlaybook> = Vec::new();
    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();

    for path in playbook_search_paths() {
        let yamls = match collect_yaml_files(&path) {
            Ok(v) => v,
            Err(_) => continue,
        };
        for file in yamls {
            match std::fs::read_to_string(&file) {
                Ok(yaml) => match load_playbook(&yaml) {
                    Ok(pb) => {
                        if seen_names.insert(pb.name.clone()) {
                            out.push(DiscoveredPlaybook {
                                playbook: pb,
                                source: file,
                            });
                        }
                    }
                    Err(err) => {
                        eprintln!(
                            "warn: skipping invalid playbook {}: {}",
                            file.display(),
                            err
                        );
                    }
                },
                Err(err) => {
                    eprintln!(
                        "warn: cannot read playbook file {}: {}",
                        file.display(),
                        err
                    );
                }
            }
        }
    }

    // Stable order — by name for deterministic output.
    out.sort_by(|a, b| a.playbook.name.cmp(&b.playbook.name));
    out
}

/// Search roots for playbook discovery.
///
/// Tests set `FORGEPLAN_DISABLE_PLUGIN_DISCOVERY=1` to skip the user-home
/// plugin scan so the host machine's installed packs do not leak into
/// integration assertions.
fn playbook_search_paths() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = Vec::new();

    // 1. Workspace .forgeplan/playbooks/
    //
    // `workspace::find_workspace` returns the `.forgeplan/` directory itself,
    // so we join `playbooks` directly (not `.forgeplan/playbooks`).
    if let Ok(cwd) = std::env::current_dir()
        && let Some(ws) = workspace::find_workspace(&cwd)
    {
        paths.push(ws.join("playbooks"));
    }

    // 2. Claude plugin packs: ~/.claude/plugins/*/playbooks/
    let skip_plugins = std::env::var_os("FORGEPLAN_DISABLE_PLUGIN_DISCOVERY")
        .map(|v| v != "0" && !v.is_empty())
        .unwrap_or(false);
    if !skip_plugins && let Some(home) = dirs::home_dir() {
        let plugins_root = home.join(".claude").join("plugins");
        if let Ok(entries) = std::fs::read_dir(&plugins_root) {
            for entry in entries.flatten() {
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    paths.push(entry.path().join("playbooks"));
                }
            }
        }
    }

    paths
}

/// List all `.yaml` / `.yml` files in `dir` (non-recursive). Returns `Ok(vec)`
/// even if the dir doesn't exist (returns empty).
fn collect_yaml_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && let Some(ext) = path.extension().and_then(|e| e.to_str())
            && (ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml"))
        {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

/// Resolve a `target` argument to an absolute file path.
///
/// Strategy: if the argument contains `/` or ends with `.yaml`/`.yml` and the
/// path exists → use it as-is. Otherwise treat it as a playbook name and look
/// it up in the discovery roots.
fn resolve_target(target: &str) -> Result<PathBuf, String> {
    let as_path = Path::new(target);
    let looks_like_path = target.contains('/')
        || target.contains('\\')
        || target.ends_with(".yaml")
        || target.ends_with(".yml");

    if looks_like_path && as_path.exists() {
        return Ok(as_path.to_path_buf());
    }
    if as_path.exists() && as_path.is_file() {
        return Ok(as_path.to_path_buf());
    }

    // Name lookup.
    for entry in discover_playbooks() {
        if entry.playbook.name == target {
            return Ok(entry.source);
        }
    }

    Err(format!(
        "no playbook named `{}` and no file at that path",
        target
    ))
}

// =====================================================================
// Output helpers — text + JSON
// =====================================================================

/// Compact label for a step's delegate (for `show`).
fn delegate_label(step: &Step) -> String {
    match &step.delegate_to {
        Delegation::Plugin { name, target } => format!("plugin:{}#{}", name, target),
        Delegation::Agent { name } => format!("agent:{}", name),
        Delegation::Skill { name, pack } => match pack {
            Some(p) => format!("skill:{} (pack: {})", name, p),
            None => format!("skill:{}", name),
        },
        Delegation::Command { command } => format!("command:{}", command.join(" ")),
        Delegation::ForgeplanCore { target } => format!("forgeplan_core:{}", op_label(*target)),
    }
}

/// Render the `Run --dry-run` view (no execution, just enumerate).
fn run_dry_run(
    pb: &Playbook,
    source: &Path,
    start: Option<usize>,
    json: bool,
) -> anyhow::Result<()> {
    let from = start.unwrap_or(1);
    let next = format!("forgeplan playbook run {} --yes", pb.name);

    if json {
        let steps: Vec<serde_json::Value> = pb
            .steps
            .iter()
            .enumerate()
            .filter(|(i, _)| i + 1 >= from)
            .map(|(i, s)| {
                serde_json::json!({
                    "index": i + 1,
                    "id": s.id,
                    "delegate": delegate_label(s),
                    "requires": s.requires,
                })
            })
            .collect();
        let payload = serde_json::json!({
            "playbook": pb.name,
            "source_path": source.display().to_string(),
            "dry_run": true,
            "steps": steps,
            "_next_action": next,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    println!("Dry-run: {} ({} steps)", pb.name, pb.steps.len());
    println!("Source:  {}", source.display());
    println!();
    for (i, step) in pb.steps.iter().enumerate() {
        if i + 1 < from {
            continue;
        }
        println!(
            "  [{}] {}: delegate={}",
            i + 1,
            step.id,
            delegate_label(step)
        );
    }
    println!();
    println!("Next: {}", next);
    Ok(())
}

/// Render the final ExecutionReport in text mode.
fn print_run_text(pb: &Playbook, report: &ExecutionReport) {
    println!();
    println!("Run summary: {}", pb.name);
    println!("  run_id:  {}", report.run_id);
    println!("  success: {}", report.success);
    println!("  failed:  {}", report.failed);
    println!("  skipped: {}", report.skipped);
    println!();

    for s in &report.per_step {
        let icon = match s.status {
            forgeplan_core::playbook::StepStatus::Success => "[OK]",
            forgeplan_core::playbook::StepStatus::Failed => "[FAIL]",
            forgeplan_core::playbook::StepStatus::Skipped => "[SKIP]",
        };
        println!("  {} {}", icon, s.step_id);
        if let Some(msg) = &s.message {
            println!("       {}", msg);
        }
        if let Some(out) = &s.output_path {
            println!("       output: {}", out.display());
        }
    }
    println!();

    match next_action_after_run(pb, report) {
        Some(cmd) => println!("Next: {}", cmd),
        None => println!("Done."),
    }
}

/// Decide the canonical next-action after a run completed.
fn next_action_after_run(pb: &Playbook, report: &ExecutionReport) -> Option<String> {
    if report.failed > 0 {
        // Re-run after the user investigates the failure.
        Some(format!("forgeplan playbook show {}", pb.name))
    } else if report.skipped > 0 {
        // Some steps were skipped (predecessor failed or abort policy fired).
        Some(format!("forgeplan playbook show {}", pb.name))
    } else {
        // Clean run — terminal.
        None
    }
}

fn op_label(op: ForgeplanOp) -> &'static str {
    match op {
        ForgeplanOp::Ingest => "ingest",
        ForgeplanOp::New => "new",
        ForgeplanOp::Validate => "validate",
        ForgeplanOp::Activate => "activate",
        ForgeplanOp::Search => "search",
    }
}

/// Quote target for inclusion in a follow-up CLI command if it contains
/// shell-special characters.
fn shell_quote_if_needed(target: &str) -> String {
    if target.contains(char::is_whitespace) || target.contains('"') || target.contains('\'') {
        format!("\"{}\"", target.replace('"', "\\\""))
    } else {
        target.to_string()
    }
}

// =====================================================================
// Error printing — uniform across show/validate/run
// =====================================================================

/// Print a `LoaderError` with file context + Fix hint per the contract.
fn emit_loader_error(file: &Path, err: &LoaderError, json: bool) {
    let summary = format_loader_error(err);
    let fix = loader_error_fix_hint(err, file);

    if json {
        // Errors-as-JSON go to stdout so callers `--json` consumers parse uniformly.
        let payload = serde_json::json!({
            "passed": false,
            "source_path": file.display().to_string(),
            "error": summary.headline,
            "details": summary.details,
            "_next_action": fix,
        });
        // Best-effort serialize; if this fails, fall back to stderr.
        match serde_json::to_string_pretty(&payload) {
            Ok(s) => println!("{}", s),
            Err(_) => eprintln!("Error: {}", summary.headline),
        }
        return;
    }

    eprintln!("Error: {} ({})", summary.headline, file.display());
    for d in &summary.details {
        eprintln!("  - {}", d);
    }
    eprintln!("Fix: {}", fix);
}

/// Print an I/O error (file not found, permission denied, etc).
fn print_io_error(file: &Path, err: &std::io::Error, json: bool) {
    let msg = format!("cannot read {}: {}", file.display(), err);
    let fix = "forgeplan playbook list".to_string();
    if json {
        let payload = serde_json::json!({
            "passed": false,
            "error": msg,
            "_next_action": fix,
        });
        if let Ok(s) = serde_json::to_string_pretty(&payload) {
            println!("{}", s);
        }
    } else {
        eprintln!("Error: {}", msg);
        eprintln!("Fix: {}", fix);
    }
}

/// Print an error from `resolve_target` (no playbook by that name / path).
fn print_resolve_error(target: &str, msg: &str, json: bool) {
    let fix = "forgeplan playbook list".to_string();
    if json {
        let payload = serde_json::json!({
            "passed": false,
            "target": target,
            "error": msg,
            "_next_action": fix,
        });
        if let Ok(s) = serde_json::to_string_pretty(&payload) {
            println!("{}", s);
        }
    } else {
        eprintln!("Error: {}", msg);
        eprintln!("Fix: {}", fix);
    }
}

/// Decompose a loader error into a short headline + 0..N detail lines
/// suitable for both text and JSON rendering.
struct LoaderErrorSummary {
    headline: String,
    details: Vec<String>,
}

fn format_loader_error(err: &LoaderError) -> LoaderErrorSummary {
    match err {
        LoaderError::Yaml(e) => LoaderErrorSummary {
            headline: format!("YAML parse error: {}", e),
            details: Vec::new(),
        },
        LoaderError::EmptySteps => LoaderErrorSummary {
            headline: "playbook has no steps (must have at least one)".to_string(),
            details: vec![
                "SPEC-003 §Errors: empty `steps` array → ERROR".to_string(),
                "Add at least one step under `steps:`".to_string(),
            ],
        },
        LoaderError::UnknownStepRef { pairs } => LoaderErrorSummary {
            headline: format!(
                "{} step(s) reference unknown step IDs in `requires:`",
                pairs.len()
            ),
            details: pairs
                .iter()
                .map(|(s, r)| format!("step `{}` requires unknown step `{}`", s, r))
                .collect(),
        },
        LoaderError::Cycle { path } => LoaderErrorSummary {
            headline: "cycle detected in step `requires:` graph".to_string(),
            details: vec![format!("cycle: {}", path.join(" -> "))],
        },
        LoaderError::MappingWithoutProducesAt { step_id } => LoaderErrorSummary {
            headline: format!(
                "step `{}` has `mapping` but no `produces_at` (nothing to ingest)",
                step_id
            ),
            details: vec![
                "SPEC-003 §Errors: `mapping` without `produces_at` → ERROR".to_string(),
                "Either remove `mapping:` or add a `produces_at:` path".to_string(),
            ],
        },
        LoaderError::UnsupportedSchemaVersion { version, supported } => LoaderErrorSummary {
            headline: format!(
                "unsupported schema_version `{}` (runtime supports `{}`)",
                version, supported
            ),
            details: vec![
                "Pin the playbook to a supported version or upgrade Forgeplan".to_string(),
            ],
        },
        LoaderError::InternalRange { range, source } => LoaderErrorSummary {
            headline: format!(
                "internal: failed to parse SUPPORTED_SCHEMA_RANGE `{}`",
                range
            ),
            details: vec![source.to_string()],
        },
    }
}

/// Suggest a remediation command for a given loader error. The contract is
/// `Fix: <full command>` so we always return a runnable string.
fn loader_error_fix_hint(err: &LoaderError, file: &Path) -> String {
    match err {
        // YAML / structural: re-run validate after fixing.
        LoaderError::Yaml(_)
        | LoaderError::EmptySteps
        | LoaderError::UnknownStepRef { .. }
        | LoaderError::Cycle { .. }
        | LoaderError::MappingWithoutProducesAt { .. }
        | LoaderError::UnsupportedSchemaVersion { .. }
        | LoaderError::InternalRange { .. } => {
            format!("forgeplan playbook validate {}", file.display())
        }
    }
}

// =====================================================================
// Tests (unit) — discovery + helpers. Integration tests live in
// `crates/forgeplan-cli/tests/cli_playbook.rs`.
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use forgeplan_core::playbook::types::{Delegation, OnError};

    fn step_with_agent(id: &str) -> Step {
        Step {
            id: id.to_string(),
            delegate_to: Delegation::Agent {
                name: "alpha".into(),
            },
            input: None,
            produces_at: None,
            mapping: None,
            requires: None,
            fallback_hint: None,
            on_error: OnError::Abort,
        }
    }

    #[test]
    fn delegate_label_renders_all_5_variants() {
        let plugin = Step {
            delegate_to: Delegation::Plugin {
                name: "p".into(),
                target: "t".into(),
            },
            ..step_with_agent("plugin")
        };
        assert_eq!(delegate_label(&plugin), "plugin:p#t");

        let agent = step_with_agent("agent");
        assert_eq!(delegate_label(&agent), "agent:alpha");

        let skill_no_pack = Step {
            delegate_to: Delegation::Skill {
                name: "s".into(),
                pack: None,
            },
            ..step_with_agent("skill")
        };
        assert_eq!(delegate_label(&skill_no_pack), "skill:s");

        let skill_pack = Step {
            delegate_to: Delegation::Skill {
                name: "s".into(),
                pack: Some("pk".into()),
            },
            ..step_with_agent("skill")
        };
        assert_eq!(delegate_label(&skill_pack), "skill:s (pack: pk)");

        let cmd = Step {
            delegate_to: Delegation::Command {
                command: vec!["echo".into(), "hi".into()],
            },
            ..step_with_agent("cmd")
        };
        assert_eq!(delegate_label(&cmd), "command:echo hi");

        let core = Step {
            delegate_to: Delegation::ForgeplanCore {
                target: ForgeplanOp::Validate,
            },
            ..step_with_agent("core")
        };
        assert_eq!(delegate_label(&core), "forgeplan_core:validate");
    }

    #[test]
    fn shell_quote_if_needed_basic() {
        assert_eq!(shell_quote_if_needed("simple"), "simple");
        assert_eq!(shell_quote_if_needed("two words"), "\"two words\"");
        assert_eq!(shell_quote_if_needed("a\"b"), "\"a\\\"b\"");
    }

    #[test]
    fn op_label_covers_all_variants() {
        assert_eq!(op_label(ForgeplanOp::Ingest), "ingest");
        assert_eq!(op_label(ForgeplanOp::New), "new");
        assert_eq!(op_label(ForgeplanOp::Validate), "validate");
        assert_eq!(op_label(ForgeplanOp::Activate), "activate");
        assert_eq!(op_label(ForgeplanOp::Search), "search");
    }

    #[test]
    fn collect_yaml_files_returns_empty_for_missing_dir() {
        let path = std::path::PathBuf::from("/nonexistent/forgeplan/playbooks/xyz-test-9999");
        let v = collect_yaml_files(&path).expect("ok");
        assert!(v.is_empty());
    }

    #[test]
    fn collect_yaml_files_finds_yaml_and_yml() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.yaml"), "{}").unwrap();
        std::fs::write(dir.path().join("b.yml"), "{}").unwrap();
        std::fs::write(dir.path().join("c.txt"), "skip").unwrap();
        let v = collect_yaml_files(dir.path()).expect("ok");
        let names: Vec<_> = v
            .iter()
            .filter_map(|p| p.file_name().and_then(|n| n.to_str()))
            .collect();
        assert!(names.contains(&"a.yaml"));
        assert!(names.contains(&"b.yml"));
        assert!(!names.contains(&"c.txt"));
    }

    #[test]
    fn next_action_after_run_terminal_when_clean() {
        let pb = sample_pb_one_step();
        let report = ExecutionReport {
            run_id: forgeplan_core::playbook::RunId::new(),
            success: 1,
            failed: 0,
            skipped: 0,
            per_step: Vec::new(),
        };
        assert!(next_action_after_run(&pb, &report).is_none());
    }

    #[test]
    fn next_action_after_run_suggests_show_when_failed() {
        let pb = sample_pb_one_step();
        let report = ExecutionReport {
            run_id: forgeplan_core::playbook::RunId::new(),
            success: 0,
            failed: 1,
            skipped: 0,
            per_step: Vec::new(),
        };
        let next = next_action_after_run(&pb, &report).expect("some");
        assert!(next.contains("show"));
        assert!(next.contains(&pb.name));
    }

    #[test]
    fn loader_error_fix_hint_always_runnable() {
        let err = LoaderError::EmptySteps;
        let p = std::path::PathBuf::from("/tmp/p.yaml");
        let hint = loader_error_fix_hint(&err, &p);
        assert!(hint.starts_with("forgeplan playbook validate "));
    }

    #[test]
    fn format_loader_error_unknown_ref_lists_pairs() {
        let err = LoaderError::UnknownStepRef {
            pairs: vec![("a".into(), "b".into()), ("c".into(), "d".into())],
        };
        let s = format_loader_error(&err);
        assert!(s.headline.contains("2"));
        assert_eq!(s.details.len(), 2);
        assert!(s.details[0].contains("a") && s.details[0].contains("b"));
    }

    fn sample_pb_one_step() -> Playbook {
        let yaml = r#"
schema_version: "1.0"
name: sample-pb
title: Sample
steps:
  - id: only
    delegate_to: { type: agent, name: a }
"#;
        load_playbook(yaml).expect("loads")
    }
}
