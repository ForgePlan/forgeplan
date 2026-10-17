use forgeplan_core::artifact::sanitize::sanitize_for_hint;
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

    // A workspace whose index moved to the constellation protocol has no
    // legacy store to open — route reads to the pinned generation instead
    // of letting Lance report a missing table.
    let cwd = std::env::current_dir()?;
    if let Some(ws) = forgeplan_core::workspace::find_workspace(&cwd)
        && common::index_migrated(&ws)
    {
        let unsupported = [
            (depth.is_some(), "--depth"),
            (with_evidence, "--with-evidence"),
            (no_evidence, "--no-evidence"),
            (since.is_some(), "--since"),
        ]
        .into_iter()
        .find_map(|(set, flag)| set.then_some(flag));
        return run_constellation(&ws, query, kind, status, unsupported, mode, limit, json).await;
    }

    // Build composite filter from CLI flags.
    let filter = build_filter(kind, status, depth, with_evidence, no_evidence, since_dt);

    match mode {
        SearchMode::Keyword => run_keyword(query, kind, json).await,
        SearchMode::Semantic => run_semantic_only(query, kind, json).await,
        SearchMode::Smart => run_smart(query, filter, no_expand, limit, json).await,
    }
}

/// Read path for a workspace whose index is constellation-owned: hits come
/// from the pinned generation, not the retired legacy store.
///
/// The pinned row carries `kind`/`status` but not depth, evidence or
/// timestamps — a filter the index cannot answer is refused up front rather
/// than silently dropped (a filtered-out hit that was never evaluated reads
/// exactly like an empty index).
///
/// `Smart` merges both channels: semantic hits ordered by similarity first,
/// then lexical hits that were not already named. When no profile is pinned
/// the semantic half is skipped and reported, not faked.
#[allow(clippy::too_many_arguments)]
async fn run_constellation(
    ws: &std::path::Path,
    query: &str,
    kind: Option<&str>,
    status: Option<&str>,
    unsupported: Option<&'static str>,
    mode: SearchMode,
    limit: usize,
    json: bool,
) -> anyhow::Result<()> {
    use forgeplan_core::constellation::facade;
    let index_root = ws.join("lance");

    if let Some(flag) = unsupported {
        anyhow::bail!(
            "This workspace's index is constellation-owned and the pinned read \
             carries kind/status only — {flag} is not answerable.\n\
             Fix: drop {flag} (kind/status still apply)."
        );
    }

    struct Hit {
        id: String,
        key: String,
        kind: Option<String>,
        status: Option<String>,
        title: String,
        score: Option<f64>,
        channel: &'static str,
    }

    let kindf = kind.map(str::to_string);
    let statusf = status.map(str::to_string);
    let mut hits: Vec<Hit> = Vec::new();

    if matches!(mode, SearchMode::Keyword | SearchMode::Smart) {
        let out = facade::search_pinned_lexical(&index_root, query, limit).await?;
        hits.extend(out.hits.into_iter().map(|h| {
            Hit {
                id: h
                    .display_id
                    .clone()
                    .unwrap_or_else(|| h.canonical_key.clone()),
                key: h.canonical_key,
                kind: h.kind,
                status: h.status,
                title: h.title,
                score: None,
                channel: "keyword",
            }
        }));
    }

    let mut semantic_available = false;
    #[cfg(feature = "semantic-search")]
    if matches!(mode, SearchMode::Semantic | SearchMode::Smart) {
        // The descriptor's profile_id names the space ready vectors live in;
        // without it there is nothing to query.
        let profile = forgeplan_core::constellation::storage::read_current(&index_root)?
            .and_then(|d| d.profile_id);
        match profile {
            Some(profile) => {
                let v = forgeplan_core::embed::Embedder::new()?.embed(query)?;
                let out = facade::search_pinned_semantic(&index_root, &v, &profile, limit.max(10))
                    .await?;
                semantic_available = true;
                // Semantic hits carry no kind/status — resolve them through
                // the pinned get only when a filter actually needs it.
                let mut sem = Vec::new();
                for h in out.hits {
                    let (k, s) = if kindf.is_some() || statusf.is_some() {
                        match facade::get_pinned_artifact(&index_root, &h.canonical_key).await? {
                            Some((a, _, _)) => (a.kind, a.status),
                            None => (None, None),
                        }
                    } else {
                        (None, None)
                    };
                    sem.push(Hit {
                        id: h.display_id.unwrap_or_else(|| h.canonical_key.clone()),
                        key: h.canonical_key,
                        kind: k,
                        status: s,
                        title: h.title,
                        score: Some(h.similarity),
                        channel: "semantic",
                    });
                }
                // Semantic first (similarity order), then lexical leftovers.
                let seen: std::collections::BTreeSet<_> =
                    sem.iter().map(|h| h.key.clone()).collect();
                hits.retain(|h| !seen.contains(&h.key));
                sem.extend(hits);
                hits = sem;
            }
            None if matches!(mode, SearchMode::Semantic) => {
                anyhow::bail!(
                    "No embedding profile is pinned for this index — vectors were \
                     never published.\nFix: forgeplan index --embed-profile <profile-id>"
                );
            }
            None => {} // Smart: lexical half stands on its own; reported below.
        }
    }
    #[cfg(not(feature = "semantic-search"))]
    if matches!(mode, SearchMode::Semantic) {
        anyhow::bail!("Semantic search requires the `semantic-search` build feature.");
    }

    hits.retain(|h| {
        kindf
            .as_ref()
            .is_none_or(|k| h.kind.as_deref() == Some(k.as_str()))
            && statusf
                .as_ref()
                .is_none_or(|s| h.status.as_deref() == Some(s.as_str()))
    });
    hits.truncate(limit);

    if hits.is_empty() {
        let advisory = forgeplan_core::hints::search_hints(query, 0);
        if json {
            let payload = serde_json::json!({
                "results": [],
                "index": "constellation",
                "_next_action": hints::primary_action(&advisory),
            });
            println!("{}", serde_json::to_string_pretty(&payload)?);
        } else {
            ui::info(&format!("No results for \"{query}\""));
            print!("{}", forgeplan_core::hints::format_hints(&advisory));
            print!("{}", hints::render_next_action_line(&advisory));
        }
        return Ok(());
    }

    let top_id = hits[0].id.clone();
    if json {
        let rows: Vec<_> = hits
            .iter()
            .map(|h| {
                serde_json::json!({
                    "id": h.id,
                    "canonical_key": h.key,
                    "kind": h.kind,
                    "status": h.status,
                    "title": h.title,
                    "mode": h.channel,
                    "score": h.score,
                })
            })
            .collect();
        let payload = serde_json::json!({
            "results": rows,
            "index": "constellation",
            "_next_action": format!("forgeplan get {top_id}"),
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    let mut lines = vec![format!("{:<14} {:<22} {}", "ID", "KIND", "TITLE")];
    for h in &hits {
        let score = h.score.map(|s| format!(" {:.3}", s)).unwrap_or_default();
        lines.push(format!(
            "{:<14} {:<22} {}{}",
            h.id,
            h.kind.as_deref().unwrap_or("—"),
            h.title,
            score
        ));
    }
    ui::info(&lines.join("\n"));
    if matches!(mode, SearchMode::Smart) && !semantic_available {
        let advisory = vec![
            Hint::info("Smart search ran without embeddings (no profile pinned)")
                .with_action("forgeplan index --embed-profile <profile-id>".to_string()),
        ];
        print!("{}", hints::format_hints(&advisory));
    }
    print!(
        "{}",
        hints::render_next_action_line(&[
            Hint::info("Top result").with_action(format!("forgeplan get {top_id}"))
        ])
    );
    Ok(())
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
        // SEC-H1 (CWE-117 / CWE-150): titles are attacker-controllable via
        // frontmatter / CSV import; sanitize before TTY emission to
        // neutralise ANSI/bidi/control bytes.
        println!(
            "  {} [{}] \"{}\"",
            record.id,
            record.kind,
            sanitize_for_hint(&record.title)
        );

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

/// How many artifacts carry no vector, and are therefore invisible to a
/// semantic query no matter how well it is phrased.
///
/// PROB-093: a semantic search over a partially-indexed workspace returns a
/// smaller world and says nothing about it. That is the failure mode the whole
/// problem is about — not an error, a plausible answer with the relevant
/// artifact quietly missing. Reporting the count turns "no match" into "no
/// match among the N I could see".
///
/// Cost is one read of the record list; no inference. Failure here must never
/// fail the search — an unavailable count is a missing warning, not a missing
/// result — so this returns 0 rather than propagating.
#[cfg(feature = "semantic-search")]
async fn unindexed_count(store: &forgeplan_core::db::store::LanceStore) -> usize {
    match store.list_records(None).await {
        Ok(records) => records.iter().filter(|r| r.embedding.is_none()).count(),
        Err(_) => 0,
    }
}

/// Emit the "N artifacts are not indexed" warning, when there are any.
#[cfg(feature = "semantic-search")]
fn warn_unindexed(n: usize) {
    if n == 0 {
        return;
    }
    ui::info(&format!(
        "{n} artifact(s) have no embedding and cannot appear in semantic results. \
         Run `forgeplan embed` — it now only encodes what changed."
    ));
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

        // PROB-093: how much of the workspace this query could not reach.
        let unindexed = unindexed_count(&store).await;

        if hits.is_empty() {
            // PRD-071: surface a primary fix-action.
            let next_hints: Vec<Hint> = vec![
                Hint::warning(format!("No semantic results for \"{}\"", query))
                    .with_action("forgeplan embed".to_string()),
            ];
            if json {
                let payload = serde_json::json!({
                    "results": [],
                    "unindexed_artifacts": unindexed,
                    "_next_action": hints::primary_action(&next_hints),
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                ui::info(&format!("No semantic results for \"{}\"", query));
                warn_unindexed(unindexed);
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
                "unindexed_artifacts": unindexed,
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
            // SEC-H1: sanitize attacker-controllable title before TTY emission.
            println!(
                "  {:.2}  {} [{}] \"{}\"",
                h.similarity(),
                h.record.id,
                h.record.kind,
                sanitize_for_hint(&h.record.title)
            );
        }

        // Deliberately printed AFTER a non-empty result list, where it matters
        // most: results on screen read as the whole answer. "Found 10" plus a
        // silent hole is more misleading than "found nothing".
        println!();
        warn_unindexed(unindexed);

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
            // SEC-H1: sanitize attacker-controllable title before TTY emission.
            println!(
                "  {:.2}  {} [{}|{}] \"{}\"",
                r.score,
                r.id,
                r.kind,
                r.status,
                sanitize_for_hint(&r.title)
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
///
/// The implementation now lives in `forgeplan_core::search::semantic`, where
/// MCP can reach it too. It used to live here, which meant MCP search passed
/// `None` and answered differently from the CLI for the same query. It also
/// resolved its engine through `Embedder::new()` → `ensure_model(.., true)`,
/// whose last argument is `show_progress`, not permission: a cold cache meant a
/// multi-gigabyte download **on the query path**. The core version is
/// cached-only and constructs nothing when the snapshot is absent.
async fn get_semantic_scores(
    store: &forgeplan_core::db::store::LanceStore,
    query: &str,
) -> (Option<std::collections::HashMap<String, f64>>, bool) {
    forgeplan_core::search::semantic::semantic_scores(store, query).await
}
