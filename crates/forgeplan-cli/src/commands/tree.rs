use std::collections::{BTreeMap, HashSet};

use console::style;

use forgeplan_core::db::store::ArtifactRecord;
use forgeplan_core::hints::{self, Hint};

use crate::commands::common;
use crate::ui;

/// `forgeplan tree [ID] [--depth N] [--json]` — ASCII tree view of artifact hierarchy.
pub async fn run(id: Option<&str>, depth: usize, json: bool) -> anyhow::Result<()> {
    let (workspace, source, _read_guard) = common::open_read_source().await?;
    let relations = match &source {
        common::ReadSource::Legacy(store) => store.get_all_relations(&_read_guard).await?,
        common::ReadSource::Captured(read) => read.get_all_relations(),
    };
    let legacy_records;
    let records = match &source {
        common::ReadSource::Legacy(store) => {
            legacy_records = store.list_records(&_read_guard, None).await?;
            legacy_records.as_slice()
        }
        common::ReadSource::Captured(read) => read.records(),
    };
    let root = match id {
        Some(selector) => Some(common::resolve_required_identity(&source, &workspace, selector).await?.0),
        None => None,
    };
    let id = root.as_deref();
    let mut parent_keys = BTreeMap::new();
    for record in records {
        if let Some(parent) = record.parent_epic.as_deref().filter(|p| !p.is_empty()) {
            if parent.contains('@') {
                forgeplan_core::db::store::validate_link_target(parent)?;
                parent_keys.insert(record.canonical_key.clone(), parent.to_string());
                continue;
            }
            let resolution = match &source {
                common::ReadSource::Legacy(_) => common::resolve_against_workspace(&workspace, parent).await?,
                common::ReadSource::Captured(read) => common::resolve_against_capture(read, parent),
            };
            if matches!(&resolution.outcome, forgeplan_mcp::convert::Resolution::IncompleteCapture { .. }) {
                common::require_found_identity(parent, &resolution)?;
            }
            let candidates = match &source {
                common::ReadSource::Legacy(_) => forgeplan_core::artifact::documents::workspace_candidates(&workspace).await?,
                common::ReadSource::Captured(read) => forgeplan_core::artifact::documents::capture_candidates(read),
            };
            let outcome: forgeplan_mcp::convert::Resolution =
                forgeplan_core::artifact::canonical_key::resolve_structural_target(parent, &candidates).into();
            let named = match &outcome {
                forgeplan_mcp::convert::Resolution::Ambiguous { candidates, .. }
                | forgeplan_mcp::convert::Resolution::ConflictingCanonicalKey { candidates, .. } => candidates.clone(),
                _ => Vec::new(),
            };
            let resolution = common::WorkspaceResolution { outcome, candidates: named };
            let key = match &resolution.outcome {
                forgeplan_mcp::convert::Resolution::NotFound => parent.to_string(),
                _ => common::require_found_identity(parent, &resolution)?.0,
            };
            parent_keys.insert(record.canonical_key.clone(), key);
        }
    }
    let (children_map, all_records) = build_hierarchy(records, &relations, &parent_keys);

    if json {
        return render_json(id, depth, &children_map, &all_records);
    }

    if let Some(root_id) = id {
        if !all_records.contains_key(root_id) {
            // PRD-071: error path — direct user to listing for valid IDs.
            let fix_hints: Vec<Hint> = vec![
                Hint::warning(format!("Artifact '{}' not found", root_id))
                    .with_action("forgeplan list".to_string()),
            ];
            ui::error_hint(
                &format!("Artifact '{}' not found", root_id),
                "Run `forgeplan list` to see available artifacts",
            );
            if let Some(fix) = hints::primary_action(&fix_hints) {
                eprintln!("Fix: {}", fix);
            }
            anyhow::bail!("Artifact '{}' not found", root_id);
        }
        println!();
        print_subtree(root_id, &children_map, &all_records, 0, depth, "");
    } else {
        let has_parent: HashSet<&str> = children_map
            .values()
            .flat_map(|kids| kids.iter().map(|s| s.as_str()))
            .collect();

        let mut roots: Vec<&str> = all_records
            .keys()
            .map(|s| s.as_str())
            .filter(|id| !has_parent.contains(id))
            .collect();
        roots.sort();

        if roots.is_empty() {
            // PRD-071: empty workspace — primary action is to create.
            let next_hints: Vec<Hint> = vec![
                Hint::info("Empty workspace")
                    .with_action("forgeplan new prd \"<title>\"".to_string()),
            ];
            ui::info("No artifacts found. Run `forgeplan new` to create one.");
            print!("{}", hints::render_next_action_line(&next_hints));
            return Ok(());
        }

        // Header — data LEFT, tree RIGHT
        println!();
        println!("{}", style("Forgeplan Tree").bold());
        println!("{}", style("═".repeat(90)).dim());
        println!(
            "{:<10}  {:<4}  {:<12}  {:<8}  {}",
            style("PROGRESS").bold().underlined(),
            style("R_EFF").bold().underlined(),
            style("STATUS").bold().underlined(),
            style("KIND").bold().underlined(),
            style("TREE").bold().underlined(),
        );
        println!("{}", style("─".repeat(90)).dim());

        for root in &roots {
            print_subtree(root, &children_map, &all_records, 0, depth, "");
        }

        // Summary
        let total = all_records.len();
        let active = all_records
            .values()
            .filter(|r| r.status == "active")
            .count();
        let draft = all_records.values().filter(|r| r.status == "draft").count();
        let deprecated = all_records
            .values()
            .filter(|r| r.status == "deprecated")
            .count();
        println!("{}", style("─".repeat(80)).dim());
        println!(
            "  {} artifacts | {} active | {} draft | {} deprecated",
            style(total).bold(),
            style(active).green(),
            style(draft).dim(),
            style(deprecated).red(),
        );
    }

    // PRD-071: tree is exploratory — drop user into health to surface
    // any structural issues (orphans, blind spots).
    let next_hints: Vec<Hint> = vec![
        Hint::info("Tree rendered — check workspace health")
            .with_action("forgeplan health".to_string()),
    ];
    print!("{}", hints::render_next_action_line(&next_hints));

    Ok(())
}

/// Record info needed for display.
struct DisplayRecord {
    id: String,
    kind: String,
    title: String,
    status: String,
    r_eff: f64,
}

/// Build parent->children mapping from all relations and parent_epic fields.
/// Both read sources feed this single hierarchy algorithm.
fn build_hierarchy(
    records: &[ArtifactRecord],
    relations: &[(String, String, String)],
    parent_keys: &BTreeMap<String, String>,
) -> (
    BTreeMap<String, Vec<String>>,
    BTreeMap<String, DisplayRecord>,
) {
    let mut records_map = BTreeMap::new();
    for r in records {
        records_map.insert(
            r.canonical_key.clone(),
            DisplayRecord {
                id: r.id.clone(),
                kind: r.kind.clone(),
                title: r.title.clone(),
                status: r.status.clone(),
                r_eff: r.r_eff_score,
            },
        );
    }

    // Edge direction: source -> target (source is child, target is parent).
    // e.g., RFC-001 --based_on--> PRD-001 means RFC is child of PRD.
    let child_relations: HashSet<&str> =
        ["based_on", "refines", "informs", "belongs_to", "child_of"]
            .into_iter()
            .collect();

    let mut children_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut has_parent: HashSet<String> = HashSet::new();

    for (source, target, relation) in relations {
        if child_relations.contains(relation.as_str())
            && records_map.contains_key(source)
            && records_map.contains_key(target)
            && !has_parent.contains(source)
        {
            has_parent.insert(source.clone());
            children_map
                .entry(target.clone())
                .or_default()
                .push(source.clone());
        }
    }

    for r in records {
        if let Some(parent) = parent_keys.get(&r.canonical_key)
            && !parent.is_empty()
            && records_map.contains_key(parent)
            && !has_parent.contains(&r.canonical_key)
        {
            has_parent.insert(r.canonical_key.clone());
            children_map
                .entry(parent.clone())
                .or_default()
                .push(r.canonical_key.clone());
        }
    }

    for kids in children_map.values_mut() {
        kids.sort();
    }

    (children_map, records_map)
}

/// Print the tree rooted at `id`. This is the entry point that prints the root
/// node without any connector, then recurses into children.
fn print_subtree(
    id: &str,
    children_map: &BTreeMap<String, Vec<String>>,
    records: &BTreeMap<String, DisplayRecord>,
    current_depth: usize,
    max_depth: usize,
    _prefix: &str,
) {
    print_node_recursive(id, children_map, records, current_depth, max_depth, "", "");
}

/// Recursive tree printer. `line_prefix` is printed before this node's line.
/// `child_prefix` is the base prefix for this node's children lines.
fn print_node_recursive(
    id: &str,
    children_map: &BTreeMap<String, Vec<String>>,
    records: &BTreeMap<String, DisplayRecord>,
    current_depth: usize,
    max_depth: usize,
    line_prefix: &str,
    child_prefix: &str,
) {
    // Data LEFT, Tree RIGHT: "BAR  R_EFF  STATUS    KIND      prefix ID title"
    let data_left = format_data_cols(id, records);
    let data_width = 44; // fixed left columns width
    let prefix_width = line_prefix.chars().count();
    let id_width = id.len() + 3; // ID + ' "' + '"'
    let term_width = console::Term::stdout().size().1 as usize;
    let available = term_width
        .saturating_sub(data_width + prefix_width + id_width + 2)
        .max(10);
    let title = records
        .get(id)
        .map(|d| truncate(&d.title, available))
        .unwrap_or_else(|| "?".into());
    println!(
        "{}  {}{} \"{}\"",
        data_left,
        line_prefix,
        style(records.get(id).map(|r| r.id.as_str()).unwrap_or(id)).bold(),
        title,
    );

    if current_depth >= max_depth {
        return;
    }

    let empty = Vec::new();
    let kids = children_map.get(id).unwrap_or(&empty);

    for (i, child) in kids.iter().enumerate() {
        let is_last = i == kids.len() - 1;
        let connector = if is_last {
            "\u{2514}\u{2500} "
        } else {
            "\u{251c}\u{2500} "
        };
        let continuation = if is_last { "   " } else { "\u{2502}  " };

        print_node_recursive(
            child,
            children_map,
            records,
            current_depth + 1,
            max_depth,
            &format!("{}{}", child_prefix, connector),
            &format!("{}{}", child_prefix, continuation),
        );
    }
}

/// Format fixed-width data columns (LEFT side): BAR  R_EFF  STATUS      KIND
/// Total width: 10 + 2 + 4 + 2 + 12 + 2 + 8 = ~40 chars (always same width)
fn format_data_cols(id: &str, records: &BTreeMap<String, DisplayRecord>) -> String {
    let display = records.get(id);
    let kind = display.map(|d| d.kind.as_str()).unwrap_or("?");
    let status = display.map(|d| d.status.as_str()).unwrap_or("?");
    let r_eff = display.map(|d| d.r_eff).unwrap_or(0.0);

    // Evidence, note, refresh don't have R_eff — show dash instead of bar
    let is_non_scorable = matches!(kind, "evidence" | "note" | "refresh");
    let (bar, reff_str) = if is_non_scorable {
        (
            style("··········".to_string()).dim().to_string(),
            style(" ·· ").dim().to_string(),
        )
    } else {
        (reff_bar(r_eff), ui::styled_reff(r_eff))
    };

    let status_styled = ui::styled_status(status);
    let status_pad = " ".repeat(12_usize.saturating_sub(status.len()));
    let kind_pad = " ".repeat(8_usize.saturating_sub(kind.len()));

    format!(
        "{}  {}  {}{}  {}{}",
        bar,
        reff_str,
        status_styled,
        status_pad,
        style(kind).dim(),
        kind_pad,
    )
}

/// Render R_eff as a 10-char bar.
fn reff_bar(score: f64) -> String {
    let filled = (score * 10.0).round() as usize;
    let filled = filled.min(10);
    let empty = 10 - filled;
    let bar = format!("{}{}", "\u{2588}".repeat(filled), "\u{2591}".repeat(empty));
    if score >= 0.5 {
        style(bar).green().to_string()
    } else if score >= 0.1 {
        style(bar).yellow().to_string()
    } else {
        style(bar).red().dim().to_string()
    }
}

/// Truncate a string to max_len chars, appending "..." if truncated.
fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len - 3).collect();
        format!("{}...", truncated)
    }
}

/// Render JSON output.
///
/// **Backward-compat (PRD-071)**: stdout MUST be a bare JSON shape
/// (array of root nodes when no `id`, single node when `id` is given) so
/// existing `forgeplan tree --json | jq '.[]'` consumers keep working.
/// The `Next:` hint is additive and emitted to stderr — agents and
/// tooling that look for hints across stdout+stderr still see it; raw
/// JSON parsers see only the array/object on stdout.
fn render_json(
    id: Option<&str>,
    depth: usize,
    children_map: &BTreeMap<String, Vec<String>>,
    records: &BTreeMap<String, DisplayRecord>,
) -> anyhow::Result<()> {
    // PRD-071: emit primary next-action so JSON consumers (agents) get the
    // same `forgeplan health` deterministic hint as text mode — but on
    // stderr so the stdout JSON shape stays bw-compatible.
    let next_action = if records.is_empty() {
        Some("forgeplan new prd \"<title>\"".to_string())
    } else {
        Some("forgeplan health".to_string())
    };

    if let Some(root_id) = id {
        let tree = build_json_node(root_id, children_map, records, 0, depth);
        println!("{}", serde_json::to_string_pretty(&tree)?);
    } else {
        let has_parent: HashSet<&str> = children_map
            .values()
            .flat_map(|kids| kids.iter().map(|s| s.as_str()))
            .collect();

        let mut roots: Vec<&str> = records
            .keys()
            .map(|s| s.as_str())
            .filter(|id| !has_parent.contains(id))
            .collect();
        roots.sort();

        let trees: Vec<serde_json::Value> = roots
            .iter()
            .map(|root| build_json_node(root, children_map, records, 0, depth))
            .collect();

        println!("{}", serde_json::to_string_pretty(&trees)?);
    }

    if let Some(next) = next_action {
        eprintln!("Next: {}", next);
    }
    Ok(())
}

/// Build a JSON tree node recursively.
fn build_json_node(
    id: &str,
    children_map: &BTreeMap<String, Vec<String>>,
    records: &BTreeMap<String, DisplayRecord>,
    current_depth: usize,
    max_depth: usize,
) -> serde_json::Value {
    let display = records.get(id);
    let title = display.map(|d| d.title.as_str()).unwrap_or("");
    let status = display.map(|d| d.status.as_str()).unwrap_or("unknown");
    let r_eff = display.map(|d| d.r_eff).unwrap_or(0.0);

    let kind = display.map(|d| d.kind.as_str()).unwrap_or("");

    let children = if current_depth < max_depth {
        let empty = Vec::new();
        let kids = children_map.get(id).unwrap_or(&empty);
        kids.iter()
            .map(|child| {
                build_json_node(child, children_map, records, current_depth + 1, max_depth)
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    serde_json::json!({
        "id": display.map(|d| d.id.as_str()).unwrap_or(id),
        "canonical_key": id,
        "title": title,
        "kind": kind,
        "status": status,
        "r_eff": r_eff,
        "children": children,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_short_string() {
        assert_eq!(truncate("hello", 40), "hello");
    }

    #[test]
    fn truncate_long_string() {
        let long = "a".repeat(50);
        let result = truncate(&long, 40);
        assert_eq!(result.len(), 40);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn reff_bar_full() {
        let bar = reff_bar(1.0);
        // Should contain 10 filled blocks (Unicode may be styled)
        assert!(!bar.is_empty());
    }

    #[test]
    fn reff_bar_empty() {
        let bar = reff_bar(0.0);
        assert!(!bar.is_empty());
    }

    #[test]
    fn reff_bar_half() {
        let bar = reff_bar(0.5);
        assert!(!bar.is_empty());
    }

    #[test]
    fn build_json_node_leaf() {
        let records = BTreeMap::from([(
            "PRD-001".to_string(),
            DisplayRecord {
                id: "PRD-001".to_string(),
                kind: "prd".to_string(),
                title: "Test".to_string(),
                status: "draft".to_string(),
                r_eff: 0.0,
            },
        )]);
        let children_map = BTreeMap::new();

        let node = build_json_node("PRD-001", &children_map, &records, 0, 99);
        assert_eq!(node["id"], "PRD-001");
        assert_eq!(node["title"], "Test");
        assert_eq!(node["kind"], "prd");
        assert_eq!(node["children"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn build_json_node_with_children() {
        let records = BTreeMap::from([
            (
                "EPIC-001".to_string(),
                DisplayRecord {
                    id: "EPIC-001".to_string(),
                    kind: "epic".to_string(),
                    title: "Epic".to_string(),
                    status: "active".to_string(),
                    r_eff: 1.0,
                },
            ),
            (
                "PRD-001".to_string(),
                DisplayRecord {
                    id: "PRD-001".to_string(),
                    kind: "prd".to_string(),
                    title: "Feature".to_string(),
                    status: "draft".to_string(),
                    r_eff: 0.0,
                },
            ),
        ]);
        let children_map = BTreeMap::from([("EPIC-001".to_string(), vec!["PRD-001".to_string()])]);

        let node = build_json_node("EPIC-001", &children_map, &records, 0, 99);
        let children = node["children"].as_array().unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0]["id"], "PRD-001");
    }

    #[test]
    fn build_json_node_respects_depth() {
        let records = BTreeMap::from([
            (
                "A-001".to_string(),
                DisplayRecord {
                    id: "A-001".to_string(),
                    kind: "prd".into(),
                    title: "A".into(),
                    status: "active".into(),
                    r_eff: 0.0,
                },
            ),
            (
                "B-001".to_string(),
                DisplayRecord {
                    id: "B-001".to_string(),
                    kind: "rfc".into(),
                    title: "B".into(),
                    status: "active".into(),
                    r_eff: 0.0,
                },
            ),
        ]);
        let children_map = BTreeMap::from([("A-001".to_string(), vec!["B-001".to_string()])]);

        // depth=0 should not include children
        let node = build_json_node("A-001", &children_map, &records, 0, 0);
        assert_eq!(node["children"].as_array().unwrap().len(), 0);

        // depth=1 should include children
        let node = build_json_node("A-001", &children_map, &records, 0, 1);
        assert_eq!(node["children"].as_array().unwrap().len(), 1);
    }
}
