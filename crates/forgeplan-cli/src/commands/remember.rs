use anyhow::Result;
use console::style;

use forgeplan_core::artifact::types::slugify;
use forgeplan_core::db::store::{ArtifactFilter, NewArtifact};
use forgeplan_core::git::author::resolve_author;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::projection;

use crate::commands::common;

pub async fn run(
    text: Option<&str>,
    category: Option<&str>,
    list: bool,
    forget: Option<&str>,
) -> Result<()> {
    if list {
        return run_list().await;
    }

    if let Some(id) = forget {
        return run_forget(id).await;
    }

    let text = text.ok_or_else(|| {
        anyhow::anyhow!("Provide text to remember, or use --list / --forget <id>")
    })?;

    run_remember(text, category.unwrap_or("fact")).await
}

async fn run_remember(text: &str, category: &str) -> Result<()> {
    let (workspace, _lock, store) = common::open_store_locked().await?;

    // Generate slug from first 50 chars
    let slug_source: String = text.chars().take(50).collect();
    let slug = slugify(&slug_source);
    let id = format!("mem-{}", slug);

    // Title = first 80 chars
    let title: String = text.chars().take(80).collect();

    // Build markdown body with frontmatter
    let now = chrono::Utc::now().to_rfc3339();

    // Issue #411: real provenance instead of the hardcoded `cli`.
    // Resolved ONCE and reused by BOTH the frontmatter string below and
    // `NewArtifact.author` — writing them separately is exactly how the
    // markdown and the LanceDB row could drift.
    //
    // `None` for the caller identity: the CLI never performs the MCP
    // clientInfo handshake, so tier 2 is a no-op here and the chain is
    // git config -> "cli". An MCP-hosted `remember` passes `Some(&id)`.
    //
    // `author` is emitted as a double-quoted YAML scalar: the resolver
    // strips `"` and `\`, so no escaping is needed, and a git user.name
    // like `- foo` / `foo: bar` / `foo # bar` cannot break the mapping the
    // way the previous unquoted `author: cli` line would have.
    let author = resolve_author(&workspace, None).await;

    let body = format!(
        "---\nid: \"{}\"\nkind: memory\ncategory: {}\nstatus: active\ndepth: tactical\ntitle: \"{}\"\ncreated: {}\nauthor: \"{}\"\n---\n\n{}",
        id,
        category,
        title.replace('"', "\\\""),
        now,
        author,
        text
    );

    // Create in LanceDB
    let artifact = NewArtifact {
        id: id.clone(),
        kind: "memory".to_string(),
        status: "active".to_string(),
        title: title.clone(),
        body: body.clone(),
        depth: "tactical".to_string(),
        // Same value as the frontmatter above — single resolution, no drift.
        author: Some(author.clone()),
        parent_epic: None,
        valid_until: None,
        // C1: memory artifacts have no tags at creation; users add via `forgeplan tag`.
        tags: Vec::new(),
    };
    // PRD-073 file-first: helper writes file first, then syncs to LanceDB.
    projection::create_artifact_with_projection(
        &projection::MutationContext::new(&workspace, &store),
        &artifact,
    )
    .await?;

    println!("  Remembered: {} — \"{}\"", style(&id).bold(), title);

    // PRD-071: list memories so the agent can immediately confirm storage
    // or chain into recall.
    let next_hints: Vec<Hint> =
        vec![Hint::info("Memory stored").with_action("forgeplan remember --list".to_string())];
    print!("{}", hints::render_next_action_line(&next_hints));

    Ok(())
}

async fn run_list() -> Result<()> {
    let store = common::store().await?;

    let filter = ArtifactFilter {
        kind: Some("memory".to_string()),
        status: None,
    };
    let records = store.list_records(Some(&filter)).await?;

    if records.is_empty() {
        println!("  No memories found.");
        // PRD-071: empty list — primary action is to capture something.
        let next_hints: Vec<Hint> = vec![
            Hint::info("No memories yet")
                .with_action("forgeplan remember \"<fact to capture>\"".to_string()),
        ];
        print!("{}", hints::render_next_action_line(&next_hints));
        return Ok(());
    }

    // Issue #411: the author is stored but was invisible in the one place
    // people look. Cells are materialised BEFORE the header so the column
    // can be width-fitted the same way `id_width` already is — a workspace
    // where every memory says `cli` pays 6 chars, not the 20-char cap.
    let author_cells: Vec<String> = records
        .iter()
        .map(|r| {
            common::resolve_display_author(r.author.as_deref(), &r.body)
                .map(|a| common::shorten_author(&a, common::AUTHOR_COL_MAX))
                .unwrap_or_else(|| common::AUTHOR_MISSING.to_string())
        })
        .collect();

    // Print header
    let id_width = records.iter().map(|r| r.id.len()).max().unwrap_or(6).max(2);
    // `.max(6)` is the "Author" header floor, mirroring `.max(2)` for "ID".
    let author_width = author_cells
        .iter()
        .map(|a| a.chars().count())
        .max()
        .unwrap_or(0)
        .max("Author".len());
    println!(
        "{:<id_w$}  {:<12}  {:<a_w$}  {:<12}  {}",
        style("ID").bold().underlined(),
        style("Category").bold().underlined(),
        style("Author").bold().underlined(),
        style("Created").bold().underlined(),
        style("Text").bold().underlined(),
        id_w = id_width,
        a_w = author_width,
    );

    for (r, author) in records.iter().zip(author_cells.iter()) {
        let category = common::extract_frontmatter_field(&r.body, "category")
            .unwrap_or_else(|| "fact".to_string());
        let date: String = r.created_at.chars().take(10).collect();
        let plain_text = common::extract_plain_text(&r.body);
        let truncated: String = plain_text.chars().take(60).collect();

        println!(
            "{:<id_w$}  {:<12}  {:<a_w$}  {:<12}  {}",
            style(&r.id).bold(),
            category,
            author,
            date,
            truncated,
            id_w = id_width,
            a_w = author_width,
        );
    }

    println!("\n  {} memory(ies) total", records.len());

    // PRD-071: list output is terminal — show recall as the obvious next step.
    let next_hints: Vec<Hint> = vec![
        Hint::info("Memory list rendered").with_action("forgeplan recall \"<query>\"".to_string()),
    ];
    print!("{}", hints::render_next_action_line(&next_hints));

    Ok(())
}

async fn run_forget(id: &str) -> Result<()> {
    let (ws, _lock, store) = common::open_store_locked().await?;

    // Verify it exists and is a memory
    let record = store
        .get_record(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Memory '{}' not found", id))?;

    if record.kind != "memory" {
        anyhow::bail!("'{}' is not a memory (kind: {})", id, record.kind);
    }

    // PRD-073 file-first: helper removes file first, then cascades DB.
    projection::delete_artifact_with_projection(&projection::MutationContext::new(&ws, &store), id)
        .await?;

    println!("  Forgotten: {}", style(id).bold());

    // PRD-071: forget is destructive — direct user to surviving memory list.
    let next_hints: Vec<Hint> =
        vec![Hint::info("Memory removed").with_action("forgeplan remember --list".to_string())];
    print!("{}", hints::render_next_action_line(&next_hints));

    Ok(())
}
