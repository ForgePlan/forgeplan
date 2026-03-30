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

pub async fn run(
    query: &str,
    kind: Option<&str>,
    mode: SearchMode,
    json: bool,
) -> anyhow::Result<()> {
    if query.trim().is_empty() {
        anyhow::bail!("Search query cannot be empty.");
    }
    match mode {
        SearchMode::Keyword => run_keyword(query, kind, json).await,
        SearchMode::Semantic => run_semantic_only(query, kind, json).await,
        SearchMode::Smart => run_smart(query, kind, json).await,
    }
}

/// Keyword-only search (substring grep on title + body).
async fn run_keyword(query: &str, kind: Option<&str>, json: bool) -> anyhow::Result<()> {
    let store = common::store().await?;
    let hits = store.search_body(query, kind).await?;

    if hits.is_empty() {
        if json {
            println!("[]");
        } else {
            ui::info(&format!("No results for \"{}\"", query));
        }
        return Ok(());
    }

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
        println!("{}", serde_json::to_string_pretty(&json_data)?);
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
            all_hits.into_iter().filter(|r| r.kind.eq_ignore_ascii_case(k)).take(10).collect()
        } else {
            all_hits.into_iter().take(10).collect()
        };

        if hits.is_empty() {
            if json {
                println!("[]");
            } else {
                ui::info(&format!("No semantic results for \"{}\"", query));
                println!("  Hint: run `forgeplan embed` to generate embeddings.");
            }
            return Ok(());
        }

        if json {
            let json_data: Vec<_> = hits
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.id,
                        "kind": r.kind,
                        "status": r.status,
                        "title": r.title,
                        "mode": "semantic",
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&json_data)?);
            return Ok(());
        }

        println!(
            "Found {} artifact(s) semantically similar to \"{}\":\n",
            hits.len(),
            query
        );
        for record in &hits {
            println!("  {} [{}] \"{}\"", record.id, record.kind, record.title);
        }

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
async fn run_smart(query: &str, kind: Option<&str>, json: bool) -> anyhow::Result<()> {
    use forgeplan_core::graph::knowledge::KnowledgeGraph;
    use forgeplan_core::search::smart;

    let store = common::store().await?;

    // Load all records (with optional kind filter applied after scoring)
    let records = store.list_records(None).await?;
    if records.is_empty() {
        if json {
            println!("[]");
        } else {
            ui::info("No artifacts found.");
        }
        return Ok(());
    }

    // Try to get semantic scores (graceful degradation)
    let (semantic_scores, has_embeddings) = get_semantic_scores(&store, query).await;

    // Build knowledge graph for centrality booster
    let graph = KnowledgeGraph::from_store(&store).await.ok();

    // Run smart search
    let results = smart::smart_search(
        &records,
        query,
        semantic_scores.as_ref(),
        graph.as_ref(),
        20,
    );

    // Apply kind filter post-scoring (so boosters still use full graph)
    let results: Vec<_> = if let Some(k) = kind {
        results
            .into_iter()
            .filter(|r| r.kind.eq_ignore_ascii_case(k))
            .collect()
    } else {
        results
    };

    if results.is_empty() {
        if json {
            println!("[]");
        } else {
            ui::info(&format!("No results for \"{}\"", query));
        }
        return Ok(());
    }

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
                    "mode": "smart",
                    "semantic_available": has_embeddings,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_data)?);
    } else {
        println!(
            "Found {} result(s) for \"{}\" (smart search):\n",
            results.len(),
            query
        );
        for r in &results {
            let signals = format!(
                "kw={:.1} sem={:.1} r={:.1} g={:.1}",
                r.keyword_score, r.semantic_score, r.r_eff, r.graph_centrality
            );
            println!(
                "  {:.2}  {} [{}|{}] \"{}\"",
                r.score, r.id, r.kind, r.status, r.title
            );
            println!("        {}", signals);
        }

        if !has_embeddings {
            println!();
            println!(
                "  Tip: run `forgeplan embed` to enable semantic search for better results."
            );
        }
    }

    Ok(())
}

/// Try to compute semantic scores for all artifacts.
/// Returns (Some(map), true) if embeddings available, (None, false) otherwise.
async fn get_semantic_scores(
    store: &forgeplan_core::db::store::LanceStore,
    query: &str,
) -> (
    Option<std::collections::HashMap<String, f64>>,
    bool,
) {
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

        // Normalize cosine distances to similarity scores [0..1]
        // LanceDB returns distance (lower = more similar), convert to similarity
        let mut map = std::collections::HashMap::new();
        let max_rank = hits.len() as f64;
        for (i, record) in hits.iter().enumerate() {
            // Rank-based score: top result = 1.0, last = close to 0
            let score = 1.0 - (i as f64 / max_rank);
            map.insert(record.id.clone(), score);
        }

        (Some(map), true)
    }
    #[cfg(not(feature = "semantic-search"))]
    {
        let _ = (store, query);
        (None, false)
    }
}
