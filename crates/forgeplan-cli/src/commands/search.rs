use forgeplan_core::hints::{self, Hint};

use crate::commands::common;
use crate::ui;

/// Search mode determined by CLI flags.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchMode {
    /// Default: keyword + semantic + boosters (graceful degradation if no embeddings)
    Smart,
    /// Forced keyword-only (substring grep)
    Keyword,
    /// Forced semantic-only (vector similarity)
    Semantic,
}

#[allow(clippy::too_many_arguments)]
pub async fn run(
    query: &str,
    kind: Option<&str>,
    status: Option<&str>,
    depth: Option<&str>,
    with_evidence: bool,
    no_evidence: bool,
    since: Option<&str>,
    no_expand: bool,
    mode: SearchMode,
    limit: usize,
    json: bool,
) -> anyhow::Result<()> {
    if query.trim().is_empty() {
        anyhow::bail!("Search query cannot be empty.");
    }

    // Parse --since (YYYY-MM-DD) once, fail fast on bad input.
    let since_dt = if let Some(s) = since {
        match chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
            Ok(d) => Some(d.and_hms_opt(0, 0, 0).unwrap()),
            Err(e) => anyhow::bail!("Invalid --since date '{}' (expected YYYY-MM-DD): {}", s, e),
        }
    } else {
        None
    };

    // Build composite filter from CLI flags.
    let filter = build_filter(kind, status, depth, with_evidence, no_evidence, since_dt);

    match mode {
        SearchMode::Keyword => run_keyword(query, kind, json).await,
        SearchMode::Semantic => run_semantic_only(query, kind, json).await,
        SearchMode::Smart => run_smart(query, filter, no_expand, limit, json).await,
    }
}

fn build_filter(
    kind: Option<&str>,
    status: Option<&str>,
    depth: Option<&str>,
    with_evidence: bool,
    no_evidence: bool,
    since: Option<chrono::NaiveDateTime>,
) -> Option<forgeplan_core::search::filter::ArtifactFilter> {
    use forgeplan_core::search::filter::ArtifactFilter;
    let mut filters: Vec<ArtifactFilter> = Vec::new();
    if let Some(k) = kind {
        filters.push(ArtifactFilter::Kind(k.to_string()));
    }
    if let Some(s) = status {
        filters.push(ArtifactFilter::Status(s.to_string()));
    }
    if let Some(d) = depth {
        filters.push(ArtifactFilter::Depth(d.to_string()));
    }
    if with_evidence {
        filters.push(ArtifactFilter::HasEvidence);
    }
    if no_evidence {
        filters.push(ArtifactFilter::NoEvidence);
    }
    if let Some(dt) = since {
        filters.push(ArtifactFilter::CreatedAfter(dt));
    }
    if filters.is_empty() {
        None
    } else if filters.len() == 1 {
        Some(filters.into_iter().next().unwrap())
    } else {
        Some(ArtifactFilter::And(filters))
    }
}

/// Keyword-only search (substring grep on title + body).
async fn run_keyword(query: &str, kind: Option<&str>, json: bool) -> anyhow::Result<()> {
    let store = common::store().await?;
    let hits = store.search_body(query, kind).await?;

    if hits.is_empty() {
        // PRD-071: empty-result hints from search_hints are advisory; pick
        // primary action and surface it as Next:/_next_action.
        let advisory = forgeplan_core::hints::search_hints(query, 0);
        if json {
            let payload = serde_json::json!({
                "results": [],
                "_next_action": hints::primary_action(&advisory),
            });
            println!("{}", serde_json::to_string_pretty(&payload)?);
        } else {
            ui::info(&format!("No results for \"{}\"", query));
            print!("{}", forgeplan_core::hints::format_hints(&advisory));
            print!("{}", hints::render_next_action_line(&advisory));
        }
        return Ok(());
    }

    let top_id = hits[0].id.clone();
    let next_hints: Vec<Hint> =
        vec![Hint::info("Top result").with_action(format!("forgeplan get {}", top_id))];

    if json {
        let json_data: Vec<_> = hits
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id,
                    "kind": r.kind,
                    "status": r.status,
                    "title": r.title,
                    "mode": "keyword",
                })
            })
            .collect();
        let payload = serde_json::json!({
            "results": json_data,
            "_next_action": hints::primary_action(&next_hints),
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    println!("Found {} artifact(s) matching \"{}\":\n", hits.len(), query);

    for record in &hits {
        println!("  {} [{}] \"{}\"", record.id, record.kind, record.title);

        let query_lower = query.to_lowercase();
        let mut match_count = 0;
        for (i, line) in record.body.lines().enumerate() {
            if line.to_lowercase().contains(&query_lower) {
                let display = if line.chars().count() > 100 {
                    format!("{}...", line.chars().take(100).collect::<String>())
                } else {
                    line.to_string()
                };
                println!("    L{}: {}", i + 1, display.trim());
                match_count += 1;
                if match_count >= 3 {
                    let remaining: usize = record
                        .body
                        .lines()
                        .skip(i + 1)
                        .filter(|l| l.to_lowercase().contains(&query_lower))
                        .count();
                    if remaining > 0 {
                        println!("    ... and {} more match(es)", remaining);
                    }
                    break;
                }
            }
        }
        println!();
    }

    print!("{}", hints::render_next_action_line(&next_hints));

    Ok(())
}

/// Semantic-only search (vector similarity).
async fn run_semantic_only(query: &str, kind: Option<&str>, json: bool) -> anyhow::Result<()> {
    #[cfg(feature = "semantic-search")]
    {
        use forgeplan_core::embed::Embedder;

        let store = common::store().await?;

        let mut embedder = Embedder::new()?;
        let query_vec = embedder.embed(query)?;
        let all_hits = store.vector_search(&query_vec, 50).await?;
        let hits: Vec<_> = if let Some(k) = kind {
            all_hits
                .into_iter()
                .filter(|h| h.record.kind.eq_ignore_ascii_case(k))
                .take(10)
                .collect()
        } else {
            all_hits.into_iter().take(10).collect()
        };

        if hits.is_empty() {
            // PRD-071: surface a primary fix-action.
            let next_hints: Vec<Hint> = vec![
                Hint::warning(format!("No semantic results for \"{}\"", query))
                    .with_action("forgeplan embed".to_string()),
            ];
            if json {
                let payload = serde_json::json!({
                    "results": [],
                    "_next_action": hints::primary_action(&next_hints),
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                ui::info(&format!("No semantic results for \"{}\"", query));
                print!("{}", hints::render_next_action_line(&next_hints));
            }
            return Ok(());
        }

        let top_id = hits[0].record.id.clone();
        let next_hints: Vec<Hint> = vec![
            Hint::info("Top semantic result").with_action(format!("forgeplan get {}", top_id)),
        ];

        if json {
            let json_data: Vec<_> = hits
                .iter()
                .map(|h| {
                    serde_json::json!({
                        "id": h.record.id,
                        "kind": h.record.kind,
                        "status": h.record.status,
                        "title": h.record.title,
                        "similarity": (h.similarity() * 100.0).round() / 100.0,
                        "distance": (h.distance * 1000.0).round() / 1000.0,
                        "mode": "semantic",
                    })
                })
                .collect();
            let payload = serde_json::json!({
                "results": json_data,
                "_next_action": hints::primary_action(&next_hints),
            });
            println!("{}", serde_json::to_string_pretty(&payload)?);
            return Ok(());
        }

        println!(
            "Found {} artifact(s) semantically similar to \"{}\":\n",
            hits.len(),
            query
        );
        for h in &hits {
            println!(
                "  {:.2}  {} [{}] \"{}\"",
                h.similarity(),
                h.record.id,
                h.record.kind,
                h.record.title
            );
        }

        print!("{}", hints::render_next_action_line(&next_hints));

        Ok(())
    }
    #[cfg(not(feature = "semantic-search"))]
    {
        let _ = (query, kind, json);
        anyhow::bail!(
            "Semantic search not available. Rebuild with: \
             cargo build --features semantic-search\n\
             For automatic fallback, use smart search (default) without --semantic."
        );
    }
}

/// Smart search: keyword + semantic + graph boosters.
/// Graceful degradation: if embeddings unavailable, uses keyword only + hint.
async fn run_smart(
    query: &str,
    filter: Option<forgeplan_core::search::filter::ArtifactFilter>,
    no_expand: bool,
    limit: usize,
    json: bool,
) -> anyhow::Result<()> {
    use forgeplan_core::graph::knowledge::KnowledgeGraph;
    use forgeplan_core::search::smart;

    let store = common::store().await?;

    // Load all records — filter pushed down into smart_search.
    let records = store.list_records(None).await?;
    if records.is_empty() {
        // PRD-071: empty workspace — primary action is to create something.
        let next_hints: Vec<Hint> = vec![
            Hint::info("No artifacts in workspace")
                .with_action("forgeplan new prd \"<title>\"".to_string()),
        ];
        if json {
            let payload = serde_json::json!({
                "results": [],
                "_next_action": hints::primary_action(&next_hints),
            });
            println!("{}", serde_json::to_string_pretty(&payload)?);
        } else {
            ui::info("No artifacts found.");
            let advisory = forgeplan_core::hints::search_hints(query, 0);
            print!("{}", forgeplan_core::hints::format_hints(&advisory));
            print!("{}", hints::render_next_action_line(&next_hints));
        }
        return Ok(());
    }

    // Try to get semantic scores (graceful degradation)
    let (semantic_scores, has_embeddings) = get_semantic_scores(&store, query).await;

    // Build knowledge graph for centrality booster + neighbor expansion.
    // --no-expand disables expansion (graph passed as None) AND drops the
    // centrality booster — acceptable trade-off for "strict matches only".
    let graph = if no_expand {
        None
    } else {
        KnowledgeGraph::from_store(&store).await.ok()
    };

    // Run smart search with composite filter pushed down into core.
    let results = smart::smart_search(
        &records,
        query,
        graph.as_ref(),
        semantic_scores.as_ref(),
        filter.as_ref(),
        limit,
    );

    if results.is_empty() {
        let advisory = forgeplan_core::hints::search_hints(query, 0);
        if json {
            let payload = serde_json::json!({
                "results": [],
                "_next_action": hints::primary_action(&advisory),
            });
            println!("{}", serde_json::to_string_pretty(&payload)?);
        } else {
            ui::info(&format!("No results for \"{}\"", query));
            print!("{}", forgeplan_core::hints::format_hints(&advisory));
            print!("{}", hints::render_next_action_line(&advisory));
        }
        return Ok(());
    }

    let top_id = results[0].id.clone();
    let next_hints: Vec<Hint> = if !has_embeddings {
        vec![
            Hint::info("Smart search ran without embeddings")
                .with_action("forgeplan embed".to_string()),
        ]
    } else {
        vec![Hint::info("Top result").with_action(format!("forgeplan get {}", top_id))]
    };

    if json {
        let json_data: Vec<_> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id,
                    "kind": r.kind,
                    "status": r.status,
                    "title": r.title,
                    "score": (r.score * 100.0).round() / 100.0,
                    "keyword_score": (r.keyword_score * 100.0).round() / 100.0,
                    "semantic_score": (r.semantic_score * 100.0).round() / 100.0,
                    "r_eff": (r.r_eff * 100.0).round() / 100.0,
                    "graph_centrality": (r.graph_centrality * 100.0).round() / 100.0,
                    "expanded_from": r.expanded_from,
                    "mode": "smart",
                    "semantic_available": has_embeddings,
                })
            })
            .collect();
        let payload = serde_json::json!({
            "results": json_data,
            "_next_action": hints::primary_action(&next_hints),
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!(
            "Found {} result(s) for \"{}\" (smart search):\n",
            results.len(),
            query
        );
        for r in &results {
            // PROB-032 fix: pre-fix displayed `r.keyword_score` (substring
            // match) but `combined_score` actually uses
            // `max(bm25_score, keyword_score)`. When a query matched via BM25
            // tokenization but had no substring presence (e.g. "auth" matches
            // "authentication" via stemming but not substring), `kw=0.0` was
            // shown despite the keyword channel contributing the entire base
            // score — total > 0 with all displayed components 0.0. Display
            // the MAX of both channels so the breakdown reflects the actual
            // contributor. Precision bumped к {:.2} so small contributions
            // (0.02–0.09) no longer round-down к 0.0.
            let kw_channel = r.bm25_score.max(r.keyword_score);
            let signals = format!(
                "kw={:.2} sem={:.2} r={:.2} g={:.2}",
                kw_channel, r.semantic_score, r.r_eff, r.graph_centrality
            );
            println!(
                "  {:.2}  {} [{}|{}] \"{}\"",
                r.score, r.id, r.kind, r.status, r.title
            );
            if let Some(parent) = &r.expanded_from {
                println!("        via {} (expanded neighbor)", parent);
            }
            println!("        {}", signals);
        }

        if !has_embeddings {
            println!();
            println!("  Tip: run `forgeplan embed` to enable semantic search for better results.");
        }
        print!("{}", hints::render_next_action_line(&next_hints));
    }

    Ok(())
}

/// Try to compute semantic scores for all artifacts.
/// Returns (Some(map), true) if embeddings available, (None, false) otherwise.
async fn get_semantic_scores(
    store: &forgeplan_core::db::store::LanceStore,
    query: &str,
) -> (Option<std::collections::HashMap<String, f64>>, bool) {
    #[cfg(feature = "semantic-search")]
    {
        use forgeplan_core::embed::Embedder;

        // Try to create embedder — if model not available, degrade gracefully
        let mut embedder = match Embedder::new() {
            Ok(e) => e,
            Err(_) => return (None, false),
        };

        let query_vec = match embedder.embed(query) {
            Ok(v) => v,
            Err(_) => return (None, false),
        };

        let hits = match store.vector_search(&query_vec, 50).await {
            Ok(h) if !h.is_empty() => h,
            _ => return (None, false),
        };

        // Use real cosine similarity from LanceDB distance column
        let mut map = std::collections::HashMap::new();
        for hit in &hits {
            map.insert(hit.record.id.clone(), hit.similarity());
        }

        (Some(map), true)
    }
    #[cfg(not(feature = "semantic-search"))]
    {
        let _ = (store, query);
        (None, false)
    }
}
