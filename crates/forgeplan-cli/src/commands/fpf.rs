use std::env;
use std::path::PathBuf;

use console::style;
use forgeplan_core::db::store::{FpfChunk, LanceStore};
use forgeplan_core::fpf;
use forgeplan_core::fpf::ext::rules::Rule;
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

    // PRD-042 FR-002: Encode embeddings for each chunk when semantic-search feature is enabled.
    #[cfg(feature = "semantic-search")]
    let embeddings: Option<Vec<Vec<f32>>> = {
        println!(
            "  Encoding {} sections with BGE-M3 (first run downloads model ~150MB)...",
            chunks.len()
        );
        let mut embedder = forgeplan_core::embed::Embedder::new()?;
        // Sprint 13.7 hotfix FIX-E: runtime guard against future model swap.
        // The fpf_spec Arrow schema hardcodes `embedding: FixedSizeList<Float32,
        // EMBEDDING_DIM>` (1024 for BGE-M3). Swapping the Embedder to a model
        // with a different output dim would silently corrupt the table on the
        // next insert. Fail fast here with a clear message.
        if embedder.dim() != forgeplan_core::db::schema::EMBEDDING_DIM as usize {
            anyhow::bail!(
                "Embedder dim mismatch: model '{}' outputs {} dims but the \
                 fpf_spec schema expects {}. Sprint 13.7 hardcodes BGE-M3 \
                 (1024 dims); swapping the embedding model requires a schema \
                 migration.",
                embedder.model_name(),
                embedder.dim(),
                forgeplan_core::db::schema::EMBEDDING_DIM as usize
            );
        }
        let texts: Vec<String> = chunks
            .iter()
            .map(|c| format!("{}: {}", c.title, c.body))
            .collect();
        let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        let vecs = embedder.embed_batch(&text_refs)?;
        println!(
            "  Encoded {} embeddings (dim={})",
            vecs.len(),
            vecs.first().map(|v| v.len()).unwrap_or(0)
        );
        Some(vecs)
    };
    #[cfg(not(feature = "semantic-search"))]
    let embeddings: Option<Vec<Vec<f32>>> = None;

    let count = store
        .insert_fpf_chunks(&fpf_chunks, embeddings.as_deref())
        .await?;
    #[cfg(feature = "semantic-search")]
    println!("  Ingested {} FPF sections with embeddings", count);
    #[cfg(not(feature = "semantic-search"))]
    println!(
        "  Ingested {} FPF sections (keyword-only — build with --features semantic-search for vector search)",
        count
    );
    Ok(())
}

/// Helper for the semantic-search path with injectable encoder.
///
/// Extracted so fallback logic (embedder init failure, encode failure, vector
/// search failure) can be unit-tested via closure injection without requiring
/// the real fastembed model. Encoder is a sync closure because BGE-M3's
/// `Embedder::embed` is sync; the vector search itself is async and handled
/// via `store.search_fpf_by_vector`.
///
/// Returns the results on success, or a propagated error on any of the 3
/// failure modes. Callers convert that error into a stderr warning and fall
/// back to keyword search — this helper does NOT perform the fallback itself,
/// so tests can assert on which failure mode was hit.
#[cfg_attr(not(feature = "semantic-search"), allow(dead_code))]
pub async fn try_semantic_search<F>(
    store: &LanceStore,
    query: &str,
    limit: usize,
    encoder: F,
) -> anyhow::Result<Vec<FpfChunk>>
where
    F: FnOnce(&str) -> anyhow::Result<Vec<f32>>,
{
    let query_vec = encoder(query)?;
    store
        .search_fpf_by_vector(&query_vec, limit)
        .await
        .map_err(|e| anyhow::anyhow!("vector search failed: {e}"))
}

/// `forgeplan fpf search <query> [--limit N] [--semantic]`
pub async fn run_search(query: &str, limit: usize, semantic: bool) -> anyhow::Result<()> {
    // Input validation (Sprint 13.7 audit M1): fail fast on empty / oversized
    // queries before touching the store, so both keyword and semantic paths
    // get consistent UX.
    if query.trim().is_empty() {
        anyhow::bail!("Search query cannot be empty");
    }
    if query.len() > 8192 {
        anyhow::bail!(
            "Search query too long (max 8192 chars, got {})",
            query.len()
        );
    }

    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let store = LanceStore::open(&ws).await?;

    // PRD-042 FR-003: Semantic search with graceful fallback to keyword.
    // Sprint 13.7 wave 2 C2: match MCP parity — ANY runtime failure in the
    // semantic path (embedder init, encode, vector search) degrades to
    // keyword search with a warning on stderr instead of bubbling up.
    let (results, header): (Vec<FpfChunk>, Option<&'static str>) = if semantic {
        #[cfg(feature = "semantic-search")]
        {
            let encoder = |q: &str| -> anyhow::Result<Vec<f32>> {
                let mut embedder = forgeplan_core::embed::Embedder::new()
                    .map_err(|e| anyhow::anyhow!("failed to initialize embedder: {e}"))?;
                embedder
                    .embed(q)
                    .map_err(|e| anyhow::anyhow!("failed to encode query: {e}"))
            };
            match try_semantic_search(&store, query, limit, encoder).await {
                Ok(vecs) => (vecs, Some("[semantic search: BGE-M3]")),
                Err(e) => {
                    eprintln!(
                        "{} {}; falling back to keyword search",
                        style("⚠").yellow().bold(),
                        e
                    );
                    (store.search_fpf(query, limit).await?, None)
                }
            }
        }
        #[cfg(not(feature = "semantic-search"))]
        {
            eprintln!(
                "{} semantic-search feature not compiled in; falling back to keyword search",
                style("⚠").yellow().bold()
            );
            (store.search_fpf(query, limit).await?, None)
        }
    } else {
        (store.search_fpf(query, limit).await?, None)
    };

    if results.is_empty() {
        // Sprint 13.7 audit M2: strip control chars from echoed query so a
        // crafted query can't inject ANSI escapes into user-facing output.
        let safe_query: String = query.chars().filter(|c| !c.is_control()).collect();
        println!("  No FPF sections match '{}'", safe_query);
        println!("  Hint: Run `forgeplan fpf ingest` first");
        return Ok(());
    }

    println!();
    if let Some(h) = header {
        println!("  {}", style(h).dim());
    }
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
                    "condition_summary": r.condition.summarize(),
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
                r.condition.summarize(),
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
        let plural = if in_group.len() == 1 { "rule" } else { "rules" };
        println!(
            "  {} {} ({} {}) — {}",
            branch,
            style_action(action),
            in_group.len(),
            plural,
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
                style(rule.condition.summarize()).dim()
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
        Ok(Some(r)) => r,
        Ok(None) => {
            ui::error_hint(
                &format!("Artifact '{id}' not found"),
                "forgeplan list --kind prd",
            );
            std::process::exit(1);
        }
        Err(_) => {
            ui::error_hint(
                &format!("Failed to check artifact '{id}'"),
                "forgeplan health",
            );
            std::process::exit(1);
        }
    };

    if json {
        let mut val = serde_json::to_value(&result)?;
        if let Some(obj) = val.as_object_mut() {
            obj.insert(
                "summary".to_string(),
                serde_json::Value::String(result.summary_line()),
            );
        }
        println!("{}", serde_json::to_string_pretty(&val)?);
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
    use forgeplan_core::fpf::ext::rules::{Condition, Rule};

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

    #[tokio::test]
    async fn run_search_empty_query_errors() {
        let err = super::run_search("", 5, false).await;
        assert!(err.is_err(), "empty query must error");
        let msg = format!("{:?}", err.unwrap_err());
        assert!(msg.contains("empty"), "error must mention empty: {msg}");
    }

    #[tokio::test]
    async fn run_search_whitespace_query_errors() {
        let err = super::run_search("   ", 5, false).await;
        assert!(err.is_err(), "whitespace query must error");
    }

    #[tokio::test]
    async fn run_search_oversized_query_errors() {
        let big = "a".repeat(9000);
        let err = super::run_search(&big, 5, false).await;
        assert!(err.is_err(), "oversized query must error");
        let msg = format!("{:?}", err.unwrap_err());
        assert!(
            msg.contains("too long"),
            "error must mention too long: {msg}"
        );
    }

    // ── Sprint 13.7 wave 2 C3: fallback helper tests ────────────

    async fn make_fpf_store() -> (tempfile::TempDir, LanceStore) {
        let tmp = tempfile::TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let store = LanceStore::init(&ws).await.unwrap();
        (tmp, store)
    }

    #[tokio::test]
    async fn semantic_fallback_on_embedder_init_fail() {
        // Encoder simulates Embedder::new() failing (e.g. model download error).
        let (_tmp, store) = make_fpf_store().await;
        let encoder =
            |_q: &str| -> anyhow::Result<Vec<f32>> { Err(anyhow::anyhow!("embedder init failed")) };
        let err = try_semantic_search(&store, "trust", 5, encoder)
            .await
            .unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("embedder init failed"),
            "error must bubble through: {msg}"
        );
    }

    #[tokio::test]
    async fn semantic_fallback_on_encode_fail() {
        // Encoder simulates embed() failing on a valid embedder.
        let (_tmp, store) = make_fpf_store().await;
        let encoder = |_q: &str| -> anyhow::Result<Vec<f32>> {
            Err(anyhow::anyhow!("failed to encode query"))
        };
        let err = try_semantic_search(&store, "ctx", 5, encoder)
            .await
            .unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("encode"), "error must mention encode: {msg}");
    }

    #[tokio::test]
    async fn semantic_fallback_on_search_fail() {
        // Encoder returns an invalid-dim vector, which makes
        // search_fpf_by_vector fail with the dim-mismatch check. The helper
        // must propagate that error so the caller can fall back.
        let (_tmp, store) = make_fpf_store().await;
        let encoder = |_q: &str| -> anyhow::Result<Vec<f32>> { Ok(vec![0.0f32; 512]) };
        let err = try_semantic_search(&store, "q", 5, encoder)
            .await
            .unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("vector search failed") || msg.contains("wrong dim"),
            "error must surface vector search failure: {msg}"
        );
    }

    #[tokio::test]
    async fn semantic_success_returns_results() {
        // Happy path through the helper: seed 2 chunks, encoder returns a
        // valid 1024-dim vector, and the helper returns the store results.
        let (_tmp, store) = make_fpf_store().await;
        let dim = forgeplan_core::db::schema::EMBEDDING_DIM as usize;
        let chunks = vec![
            FpfChunk {
                id: "fpf-h-1".into(),
                section_id: "H.1".into(),
                parent_section: None,
                title: "T1".into(),
                body: "body".into(),
                line_count: 1,
                file_path: "a.md".into(),
                created_at: "2026-01-01".into(),
            },
            FpfChunk {
                id: "fpf-h-2".into(),
                section_id: "H.2".into(),
                parent_section: None,
                title: "T2".into(),
                body: "body".into(),
                line_count: 1,
                file_path: "a.md".into(),
                created_at: "2026-01-01".into(),
            },
        ];
        let mut e1 = vec![0.0f32; dim];
        e1[0] = 1.0;
        let mut e2 = vec![0.0f32; dim];
        e2[1] = 1.0;
        store
            .insert_fpf_chunks(&chunks, Some(&[e1.clone(), e2]))
            .await
            .unwrap();
        let encoder = move |_q: &str| -> anyhow::Result<Vec<f32>> { Ok(e1.clone()) };
        let results = try_semantic_search(&store, "q", 2, encoder).await.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "fpf-h-1");
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
