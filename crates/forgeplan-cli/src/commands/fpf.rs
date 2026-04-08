use std::env;
use std::path::PathBuf;

use console::style;
use forgeplan_core::db::store::{FpfChunk, LanceStore};
use forgeplan_core::fpf;
use forgeplan_core::fpf::ext::rules::{Condition, NumericExpr, Rule, ValueMatch};
use forgeplan_core::fpf::knowledge;
use forgeplan_core::workspace;

use crate::ui;

/// FPF Dashboard (original command, now `forgeplan fpf dashboard`)
pub async fn run_dashboard() -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let store = LanceStore::open(&ws).await?;
    let config = workspace::load_config(&ws).map_err(|e| anyhow::anyhow!("Config error: {e}"))?;
    let fpf_config = config.fpf.as_ref();
    let dashboard = fpf::dashboard(&store, fpf_config).await?;
    print!("{dashboard}");

    Ok(())
}

/// `forgeplan fpf ingest [--path <dir>]`
pub async fn run_ingest(path: Option<&str>) -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let fpf_path = match path {
        Some(p) => PathBuf::from(p),
        None => knowledge::default_fpf_path()
            .ok_or_else(|| anyhow::anyhow!("FPF spec not found. Use --path to specify location"))?,
    };

    println!("  Ingesting FPF spec from {}...", fpf_path.display());

    let chunks = knowledge::ingest_fpf_directory(&fpf_path).await?;
    println!("  Parsed {} sections", chunks.len());

    // Use init() to ensure fpf_spec table exists
    let store = LanceStore::init(&ws).await?;

    // Clear existing FPF data and re-ingest
    if store.has_fpf() {
        store.clear_fpf().await?;
    }

    // Convert IngestChunk to FpfChunk
    let now = chrono::Utc::now().to_rfc3339();
    let fpf_chunks: Vec<FpfChunk> = chunks
        .iter()
        .map(|c| FpfChunk {
            id: c.id.clone(),
            section_id: c.section_id.clone(),
            parent_section: c.parent_section.clone(),
            title: c.title.clone(),
            body: c.body.clone(),
            line_count: c.line_count,
            file_path: c.file_path.clone(),
            created_at: now.clone(),
        })
        .collect();

    let count = store.insert_fpf_chunks(&fpf_chunks).await?;
    println!("  Ingested {} FPF sections into LanceDB", count);
    Ok(())
}

/// `forgeplan fpf search <query> [--limit N]`
pub async fn run_search(query: &str, limit: usize) -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let store = LanceStore::open(&ws).await?;
    let results = store.search_fpf(query, limit).await?;

    if results.is_empty() {
        println!("  No FPF sections match '{}'", query);
        println!("  Hint: Run `forgeplan fpf ingest` first");
        return Ok(());
    }

    println!();
    for (i, chunk) in results.iter().enumerate() {
        let snippet: String = chunk
            .body
            .lines()
            .take(3)
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(200)
            .collect();
        println!("  {}. [{}] {}", i + 1, chunk.section_id, chunk.title);
        println!("     {} ({} lines)", snippet, chunk.line_count);
        println!();
    }
    Ok(())
}

/// `forgeplan fpf section <id> [--summary]`
pub async fn run_section(id: &str, summary: bool) -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let store = LanceStore::open(&ws).await?;
    let chunk = store
        .get_fpf_section(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("FPF section '{}' not found", id))?;

    println!();
    println!("## {} — {}", chunk.section_id, chunk.title);
    println!();
    if summary {
        let preview: String = chunk.body.chars().take(500).collect();
        println!("{}", preview);
        if chunk.body.len() > 500 {
            println!(
                "\n  ... ({} more chars. Use without --summary for full text)",
                chunk.body.len() - 500
            );
        }
    } else {
        println!("{}", chunk.body);
    }
    Ok(())
}

/// `forgeplan fpf status`
pub async fn run_status() -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    println!();
    println!("FPF Knowledge Base Status");
    println!("{}", "=".repeat(40));

    // Check source
    let source_path = knowledge::default_fpf_path();
    let source_count = match &source_path {
        Some(p) => {
            println!("  Source:    {} (exists)", p.display());
            count_md_files(p).await
        }
        None => {
            println!("  Source:    not found (set fpf.path in config or install fpf-simple skill)");
            0
        }
    };
    if source_count > 0 {
        println!("  Files:     {} markdown files", source_count);
    }

    // Check ingested
    let store = LanceStore::open(&ws).await?;
    if store.has_fpf() {
        let sections = store.list_fpf_sections().await?;
        if sections.is_empty() {
            println!("  Ingested:  empty (run `forgeplan fpf ingest`)");
        } else {
            let total_lines: i32 = sections.iter().map(|s| s.line_count).sum();
            println!(
                "  Ingested:  {} sections, {} total lines",
                sections.len(),
                total_lines
            );

            // Staleness check
            if source_count > 0 && source_count != sections.len() {
                println!(
                    "  Status:    STALE — source has {} files, ingested has {} sections",
                    source_count,
                    sections.len()
                );
                println!("  Action:    Run `forgeplan fpf ingest` to re-sync");
            } else if source_count > 0 {
                println!("  Status:    UP TO DATE");
            }
        }
    } else {
        println!("  Ingested:  not initialized");
        println!("  Action:    Run `forgeplan fpf ingest` to load FPF spec");
    }

    println!();
    Ok(())
}

async fn count_md_files(dir: &std::path::Path) -> usize {
    let mut count = 0;
    if let Ok(mut rd) = tokio::fs::read_dir(dir).await {
        while let Ok(Some(entry)) = rd.next_entry().await {
            let p = entry.path();
            if p.is_dir()
                && let Ok(mut sub) = tokio::fs::read_dir(&p).await
            {
                while let Ok(Some(sub_entry)) = sub.next_entry().await {
                    let sp = sub_entry.path();
                    if sp.extension().is_some_and(|e| e == "md")
                        && sp.file_name().is_some_and(|n| n != "_index.md")
                    {
                        count += 1;
                    }
                }
            }
        }
    }
    count
}

/// `forgeplan fpf list`
pub async fn run_list() -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let store = LanceStore::open(&ws).await?;
    let sections = store.list_fpf_sections().await?;

    if sections.is_empty() {
        println!("  No FPF sections loaded. Run `forgeplan fpf ingest` first.");
        return Ok(());
    }

    println!();
    println!("  {:10}  {:5}  Title", "Section", "Lines");
    println!("  {}", "-".repeat(60));
    for s in &sections {
        println!("  {:10}  {:5}  {}", s.section_id, s.line_count, s.title);
    }
    println!();
    println!("  {} sections total", sections.len());
    Ok(())
}

// ──────────────────────────────────────────────────────────────────
// PRD-041 FR-001: `forgeplan fpf rules`
// ──────────────────────────────────────────────────────────────────

/// Render a single `Condition` as a human-readable "k=v AND k>0.5" string.
///
/// Condition is a flat implicit-AND struct; every Some(...) field becomes one
/// "key op value" clause joined with " AND ". Truncates to 120 chars with "…".
pub(crate) fn summarize_condition(cond: &Condition) -> String {
    if cond.is_empty() {
        return "(always matches)".to_string();
    }

    let mut parts: Vec<String> = Vec::new();

    if let Some(v) = &cond.kind {
        parts.push(format!("kind={}", format_value_match(v)));
    }
    if let Some(v) = &cond.status {
        parts.push(format!("status={}", format_value_match(v)));
    }
    if let Some(v) = &cond.depth {
        parts.push(format!("depth={}", format_value_match(v)));
    }
    if let Some(n) = &cond.r_eff {
        parts.push(format!("r_eff{}", format_numeric(n)));
    }
    if let Some(n) = &cond.overall {
        parts.push(format!("overall{}", format_numeric(n)));
    }
    if let Some(n) = &cond.link_count {
        parts.push(format!("link_count{}", format_numeric(n)));
    }
    if let Some(b) = cond.is_stale {
        parts.push(format!("is_stale={b}"));
    }
    if let Some(links) = &cond.links_missing {
        parts.push(format!("links_missing={}", links.join(",")));
    }
    if let Some(n) = &cond.days_until_expiry {
        parts.push(format!("days_until_expiry{}", format_numeric(n)));
    }

    let mut joined = parts.join(" AND ");
    if joined.chars().count() > 120 {
        joined = joined.chars().take(119).collect::<String>();
        joined.push('…');
    }
    joined
}

fn format_value_match(v: &ValueMatch) -> String {
    match v {
        ValueMatch::Single(s) => s.clone(),
        ValueMatch::Multiple(list) => format!("[{}]", list.join("|")),
    }
}

fn format_numeric(n: &NumericExpr) -> String {
    match n {
        NumericExpr::Lt(v) => format!("<{v}"),
        NumericExpr::Le(v) => format!("<={v}"),
        NumericExpr::Gt(v) => format!(">{v}"),
        NumericExpr::Ge(v) => format!(">={v}"),
        NumericExpr::Eq(v) => format!("=={v}"),
        NumericExpr::Range(lo, hi) => format!("={lo}..{hi}"),
    }
}

fn style_action(action: &str) -> String {
    match action {
        "EXPLORE" => style(action).cyan().bold().to_string(),
        "INVESTIGATE" => style(action).yellow().bold().to_string(),
        "EXPLOIT" => style(action).green().bold().to_string(),
        _ => action.to_string(),
    }
}

/// `forgeplan fpf rules [--flat] [--json]`
pub async fn run_rules(flat: bool, json: bool) -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;
    let config = workspace::load_config(&ws).map_err(|e| anyhow::anyhow!("Config error: {e}"))?;
    let fpf_config = config.fpf.as_ref();

    let (rules, source) = fpf::active_rules(fpf_config);
    let source_label = match source {
        fpf::RuleSource::Config => "Config",
        fpf::RuleSource::Default => "Default",
    };

    if json {
        let dump: Vec<serde_json::Value> = rules
            .iter()
            .map(|r| {
                serde_json::json!({
                    "name": r.name,
                    "priority": r.priority,
                    "action": r.action.to_string(),
                    "condition": serde_json::to_value(&r.condition).unwrap_or(serde_json::Value::Null),
                    "message": r.message,
                })
            })
            .collect();
        let out = serde_json::json!({
            "source": source_label,
            "count": rules.len(),
            "rules": dump,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    let mut sorted: Vec<&Rule> = rules.iter().collect();
    sorted.sort_by_key(|r| r.priority);

    if flat {
        ui::header(
            "FPF Rules",
            &format!("{} active (source: {source_label})", sorted.len()),
        );
        println!(
            "  {:<4}  {:<28}  {:<13}  {}",
            style("prio").bold(),
            style("name").bold(),
            style("action").bold(),
            style("condition").bold()
        );
        println!("  {}", style("-".repeat(90)).dim());
        for r in &sorted {
            let action = r.action.to_string();
            println!(
                "  [{}]   {:<28}  {:<13}  {}",
                r.priority,
                truncate(&r.name, 28),
                style_action(&action),
                summarize_condition(&r.condition),
            );
        }
        println!();
        return Ok(());
    }

    // Tree view — group by action
    ui::header(
        "FPF Rules",
        &format!("{} active (source: {source_label})", sorted.len()),
    );
    println!(
        "  {}",
        style("Evaluation order: priority ascending — first match wins").dim()
    );

    let groups: [(&str, &str, bool); 3] = [
        ("EXPLORE", "когда исследовать варианты", false),
        ("INVESTIGATE", "когда собрать больше данных", false),
        ("EXPLOIT", "когда действовать решительно", true),
    ];

    for (action, descr, is_last_group) in groups {
        let in_group: Vec<&&Rule> = sorted
            .iter()
            .filter(|r| r.action.to_string() == action)
            .collect();
        if in_group.is_empty() {
            continue;
        }
        let branch = if is_last_group { "└─" } else { "├─" };
        let vbar = if is_last_group { "   " } else { "│  " };
        println!();
        println!(
            "  {} {} ({} rules) — {}",
            branch,
            style_action(action),
            in_group.len(),
            style(descr).dim()
        );
        let last_idx = in_group.len().saturating_sub(1);
        for (i, rule) in in_group.iter().enumerate() {
            let rule_branch = if i == last_idx { "└─" } else { "├─" };
            let rule_vbar = if i == last_idx { "   " } else { "│  " };
            println!(
                "  {}{} [{}] {}",
                vbar,
                rule_branch,
                rule.priority,
                style(&rule.name).bold()
            );
            println!(
                "  {}{}     {}",
                vbar,
                rule_vbar,
                style(summarize_condition(&rule.condition)).dim()
            );
            if let Some(msg) = &rule.message {
                println!("  {}{}     {}", vbar, rule_vbar, style(msg).italic().dim());
            }
        }
    }
    println!();
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{t}…")
    }
}

// ──────────────────────────────────────────────────────────────────
// PRD-041 FR-002: `forgeplan fpf check <id>`
// ──────────────────────────────────────────────────────────────────

/// `forgeplan fpf check <id> [--verbose] [--json]`
pub async fn run_check(id: &str, verbose: bool, json: bool) -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;
    let store = LanceStore::open(&ws).await?;
    let config = workspace::load_config(&ws).map_err(|e| anyhow::anyhow!("Config error: {e}"))?;
    let fpf_config = config.fpf.as_ref();

    let result = match fpf::check_artifact_against_rules(&store, id, fpf_config).await {
        Ok(r) => r,
        Err(e) => {
            ui::error_hint(
                &format!("Artifact '{id}' not found: {e}"),
                "forgeplan list --kind prd",
            );
            return Err(anyhow::anyhow!("artifact not found"));
        }
    };

    if json {
        let out = serde_json::json!({
            "artifact_id": result.artifact_id,
            "artifact_kind": result.artifact_kind,
            "artifact_status": result.artifact_status,
            "matched": result.matched.iter().map(|m| serde_json::json!({
                "name": m.name,
                "priority": m.priority,
                "action": m.action,
                "message": m.message,
            })).collect::<Vec<_>>(),
            "unmatched": result.unmatched,
            "winning": result.winning.as_ref().map(|m| serde_json::json!({
                "name": m.name,
                "priority": m.priority,
                "action": m.action,
                "message": m.message,
            })),
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    ui::header(
        &result.artifact_id,
        &format!("[{}, {}]", result.artifact_kind, result.artifact_status),
    );

    if let Some(win) = &result.winning {
        ui::section("Winning rule");
        println!(
            "  {} {} (priority {}) → {}",
            style("★").yellow().bold(),
            style(&win.name).bold(),
            win.priority,
            style_action(&win.action),
        );
        println!("    {}", style(&win.message).dim());

        if result.matched.len() > 1 {
            ui::section("Other matched rules");
            for m in result.matched.iter().skip(1) {
                println!(
                    "  - {} (priority {}) → {}",
                    m.name,
                    m.priority,
                    style_action(&m.action)
                );
            }
        }
    } else {
        ui::section("Result");
        println!("  No rules matched this artifact.");
    }

    if verbose && !result.unmatched.is_empty() {
        ui::section("Unmatched rules");
        for name in &result.unmatched {
            println!("  - {name}");
        }
    } else if !result.unmatched.is_empty() {
        println!();
        println!(
            "  {}",
            style(format!(
                "{} other rule(s) did not match.",
                result.unmatched.len()
            ))
            .dim()
        );
    }
    println!();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use forgeplan_core::fpf::core::model::ActionType;
    use forgeplan_core::fpf::ext::rules::{Condition, NumericExpr, Rule, ValueMatch};

    #[test]
    fn summarize_empty_condition() {
        let c = Condition::default();
        assert_eq!(summarize_condition(&c), "(always matches)");
    }

    #[test]
    fn summarize_flat_condition_joined_with_and() {
        let c = Condition {
            kind: Some(ValueMatch::Single("prd".into())),
            status: Some(ValueMatch::Single("active".into())),
            r_eff: Some(NumericExpr::Lt(0.5)),
            ..Default::default()
        };
        let s = summarize_condition(&c);
        assert!(s.contains("kind=prd"));
        assert!(s.contains("status=active"));
        assert!(s.contains("r_eff<0.5"));
        assert!(s.contains(" AND "));
    }

    #[test]
    fn summarize_multi_value_match() {
        let c = Condition {
            status: Some(ValueMatch::Multiple(vec!["draft".into(), "stale".into()])),
            ..Default::default()
        };
        assert_eq!(summarize_condition(&c), "status=[draft|stale]");
    }

    #[test]
    fn summarize_truncates_long_output() {
        let links: Vec<String> = (0..60).map(|i| format!("link{i}")).collect();
        let c = Condition {
            links_missing: Some(links),
            ..Default::default()
        };
        let s = summarize_condition(&c);
        assert!(s.chars().count() <= 120);
        assert!(s.ends_with('…'));
    }

    #[test]
    fn style_action_returns_nonempty_for_known_actions() {
        assert!(!style_action("EXPLORE").is_empty());
        assert!(!style_action("INVESTIGATE").is_empty());
        assert!(!style_action("EXPLOIT").is_empty());
        assert!(!style_action("UNKNOWN").is_empty());
    }

    #[test]
    fn truncate_short_unchanged() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_long_appends_ellipsis() {
        let t = truncate("abcdefghijklmn", 5);
        assert_eq!(t.chars().count(), 5);
        assert!(t.ends_with('…'));
    }

    #[test]
    fn summarize_uses_all_numeric_operators() {
        let c = Condition {
            r_eff: Some(NumericExpr::Ge(0.7)),
            overall: Some(NumericExpr::Range(0.1, 0.5)),
            link_count: Some(NumericExpr::Eq(0.0)),
            ..Default::default()
        };
        let s = summarize_condition(&c);
        assert!(s.contains("r_eff>=0.7"));
        assert!(s.contains("overall=0.1..0.5"));
        assert!(s.contains("link_count==0"));
    }

    // Smoke: a Rule with ActionType serializes via Display as expected
    #[test]
    fn rule_action_display_matches_expected() {
        let r = Rule {
            name: "t".into(),
            priority: 1,
            condition: Condition::default(),
            action: ActionType::Explore,
            message: None,
        };
        assert_eq!(r.action.to_string(), "EXPLORE");
    }
}
