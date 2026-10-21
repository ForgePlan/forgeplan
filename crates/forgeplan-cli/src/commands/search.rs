use forgeplan_core::artifact::sanitize::sanitize_for_hint;
use forgeplan_core::constellation::workspace_read::WorkspaceRead;
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

    let cwd = std::env::current_dir()?;
    let read = match forgeplan_core::workspace::find_workspace(&cwd) {
        Some(ws) if common::index_migrated(&ws) => Some(common::workspace_read(&ws).await?),
        _ => None,
    };

    // Build composite filter from CLI flags.
    let filter = build_filter(kind, status, depth, with_evidence, no_evidence, since_dt);

    match mode {
        SearchMode::Keyword => run_keyword(query, kind, json, read.as_ref()).await,
        SearchMode::Semantic => match read.as_ref() {
            Some(read) => run_constellation(read, query, kind, status, limit, json).await,
            None => run_semantic_only(query, kind, json).await,
        },
        SearchMode::Smart => run_smart(query, filter, no_expand, limit, json, read.as_ref()).await,
    }
}

/// Spell a requested kind/status the way the captured source spells it.
///
/// The pinned predicate is byte-exact, unlike every other CLI filter. Returning
/// the capture's own spelling keeps a case-insensitive request and the coverage
/// denominator computed from the same value.
#[cfg(feature = "semantic-search")]
fn captured_case<'a, I: Iterator<Item = &'a str>>(
    wanted: Option<&str>,
    values: I,
) -> Option<String> {
    wanted.map(|wanted| {
        values
            .into_iter()
            .find(|value| value.eq_ignore_ascii_case(wanted))
            .unwrap_or(wanted)
            .to_string()
    })
}

async fn run_constellation(
    read: &WorkspaceRead,
    query: &str,
    kind: Option<&str>,
    status: Option<&str>,
    limit: usize,
    json: bool,
) -> anyhow::Result<()> {
    #[cfg(not(feature = "semantic-search"))]
    {
        let _ = (read, query, kind, status, limit, json);
        anyhow::bail!("Semantic search requires a binary built with semantic-search");
    }
    #[cfg(feature = "semantic-search")]
    {
        use forgeplan_core::constellation::{facade, jobs, runtime};
        let profile = read.view().semantic_profile_id().ok_or_else(|| {
            anyhow::anyhow!("No embedding profile is published; run forgeplan embed first")
        })?;
        // The pinned predicate compares bytes, while every other CLI path
        // accepts the forgiving form (`--type PRD`, `--status DRAFT`). Resolve
        // the request against the captured casing first: a filter the caller
        // meant to widen must not exclude everything and then be reported as
        // missing vectors.
        let kind = captured_case(
            kind,
            read.captured().artifacts.iter().map(|a| a.kind.as_str()),
        );
        let status = captured_case(
            status,
            read.captured().artifacts.iter().map(|a| a.status.as_str()),
        );
        // Nothing eligible under the pinned profile means the model load
        // cannot change the answer; refuse with the reason instead of
        // spending seconds to rank an empty set.
        match facade::count_eligible_semantic(read.index_root(), read.view(), profile).await {
            Ok(0) => anyhow::bail!(
                "No eligible vectors under the published profile: every current input is \
                 pending, failed, or changed since it was embedded. Run `forgeplan embed` first."
            ),
            Ok(_) => {}
            Err(error) => return Err(error.into()),
        }
        let (profile, vector) = runtime::cached_query_embedding(
            jobs::DEFAULT_MODEL_NAME,
            Some(profile),
            read.captured().body_prefix_chars,
            query,
        )
        .await?;
        let outcome = facade::search_semantic_in_view(
            read.index_root(),
            read.view(),
            &vector,
            &profile,
            limit.max(1),
            &facade::PinnedFilter {
                kind: kind.clone(),
                status: status.clone(),
            },
        )
        .await?;
        let expected_keys: std::collections::HashSet<_> = read
            .captured()
            .artifacts
            .iter()
            .filter(|artifact| {
                kind.as_ref().is_none_or(|kind| &artifact.kind == kind)
                    && status
                        .as_ref()
                        .is_none_or(|status| &artifact.status == status)
            })
            .map(|artifact| artifact.canonical_key.as_str())
            .collect();
        // Expected rows are the CURRENT source set under this filter: one per
        // artifact, plus one per section the capture actually holds for those
        // artifacts. See [`expected_rows_for_capture`] for why that is a
        // membership count and not a table-version check.
        let expected_rows = expected_rows_for_capture(read, &expected_keys);
        let mut rows = Vec::with_capacity(outcome.hits.len().min(limit));
        for hit in outcome.hits.into_iter().take(limit) {
            let key = hit.parent_key.as_deref().unwrap_or(&hit.canonical_key);
            let resolved = read
                .resolve_id(key)
                .ok_or_else(|| anyhow::anyhow!("Search hit {key} has no captured identity"))?;
            let record = read
                .get_record(&resolved)
                .ok_or_else(|| anyhow::anyhow!("Search hit {key} has no captured artifact"))?;
            rows.push(serde_json::json!({
                "id": record.id, "title": record.title, "kind": record.kind,
                "status": record.status, "similarity": hit.similarity,
                "score": hit.similarity, "distance": 1.0 - hit.similarity,
                "canonical_key": hit.canonical_key, "document_key": key,
                "mode": "semantic",
            }));
        }
        if json {
            // `semantic_scope` is passed through unchanged — this surface does
            // not decide it. CONSUMER-VISIBLE VALUE CHANGE: it used to be
            // derived from "a sections table version is pinned", and reported
            // `prefix+sections` whenever one was. It is now derived from the
            // published section MEMBERSHIP, so a generation that pins the
            // sections table with zero rows — what a `semantic_unavailable`
            // capture publishes — reports `prefix` instead. The field, its
            // name and its two possible values are unchanged; only which of the
            // two a given workspace receives has moved, and it moves toward the
            // honest answer (no section rows → no section coverage advertised).
            // `expected_rows` above now counts membership too, so the two
            // fields cannot disagree.
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "results": rows, "index": "constellation", "mode": "semantic",
                    "semantic_available": outcome.eligible_rows > 0,
                    "eligible_rows": outcome.eligible_rows, "expected_rows": expected_rows,
                    "semantic_scope": outcome.semantic_scope, "snapshot_id": read.snapshot_id(),
                    "persisted": read.is_persisted(),
                    "source_set_digest": read.source_set_digest(),
                }))?
            );
        } else {
            for row in &rows {
                println!(
                    "{}  {:.3}  {}",
                    row["id"].as_str().unwrap_or(""),
                    row["similarity"].as_f64().unwrap_or_default(),
                    row["title"].as_str().unwrap_or("")
                );
            }
            ui::info(&format!("{} semantic results", rows.len()));
            if outcome.eligible_rows < expected_rows {
                ui::info(&format!(
                    "Semantic coverage: {}/{} current input rows are eligible",
                    outcome.eligible_rows, expected_rows
                ));
            }
        }
        Ok(())
    }
}

/// How many semantic rows the current source set is expected to hold under the
/// filter: one per artifact, plus one per section the capture actually holds
/// for those artifacts.
///
/// This counts section MEMBERSHIP, not the presence of a pinned sections table.
/// Three tables are always pinned, so "a sections version exists" says nothing
/// about whether any section was ever published — and a
/// `semantic_unavailable` capture pins the table with zero members. Counting
/// those would claim coverage that does not exist and make
/// `eligible_rows < expected_rows` print a permanent, meaningless gap. A
/// capture with no sections contributes zero, which is exactly what
/// `semantic_scope: prefix` says alongside it.
#[cfg(feature = "semantic-search")]
fn expected_rows_for_capture(
    read: &WorkspaceRead,
    expected_keys: &std::collections::HashSet<&str>,
) -> usize {
    expected_rows(read.captured().sections.as_slice(), expected_keys)
}

/// The membership count itself, over the section slice — separated from the
/// view plumbing so it can be exercised without a capture on disk.
#[cfg(feature = "semantic-search")]
fn expected_rows(
    sections: &[forgeplan_core::constellation::capture::CapturedSection],
    expected_keys: &std::collections::HashSet<&str>,
) -> usize {
    let section_rows = sections
        .iter()
        .filter(|section| expected_keys.contains(section.parent_key.as_str()))
        .count();
    expected_keys.len() + section_rows
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
async fn run_keyword(
    query: &str,
    kind: Option<&str>,
    json: bool,
    read: Option<&WorkspaceRead>,
) -> anyhow::Result<()> {
    let hits = match read {
        Some(read) => read.search_body(query, kind),
        None => common::store().await?.search_body(query, kind).await?,
    };

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
        use forgeplan_core::constellation::jobs::DEFAULT_MODEL_NAME;
        use forgeplan_core::embed::CachedSnapshot;

        let store = common::store().await?;

        // Cached-only, verified snapshot. `Embedder::new()` here was a query
        // that could start a 2.1 GB download; the absence of a model is now an
        // error that says so, which is what `--semantic` should have always
        // meant.
        let mut model = CachedSnapshot::open(DEFAULT_MODEL_NAME).map_err(|e| {
            anyhow::anyhow!(
                "{e}\nFor automatic fallback, use smart search (default) without --semantic."
            )
        })?;
        let query_vec = model.embedder.embed(query)?;
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
    read: Option<&WorkspaceRead>,
) -> anyhow::Result<()> {
    use forgeplan_core::graph::knowledge::KnowledgeGraph;
    use forgeplan_core::search::smart;

    let store = if read.is_none() {
        Some(common::store().await?)
    } else {
        None
    };
    let legacy_records;
    let records = match read {
        Some(read) => read.records(),
        None => {
            legacy_records = store
                .as_ref()
                .expect("legacy source was opened")
                .list_records(None)
                .await?;
            &legacy_records
        }
    };
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
    let (semantic_scores, has_embeddings) = match read {
        Some(read) => {
            forgeplan_core::search::semantic::workspace_semantic_scores(read, query).await
        }
        None => get_semantic_scores(store.as_ref().expect("legacy source was opened"), query).await,
    };

    // Build knowledge graph for centrality booster + neighbor expansion.
    // --no-expand disables expansion (graph passed as None) AND drops the
    // centrality booster — acceptable trade-off for "strict matches only".
    let graph = if no_expand {
        None
    } else if let Some(read) = read {
        Some(read.knowledge_graph())
    } else {
        KnowledgeGraph::from_store(store.as_ref().expect("legacy source was opened"))
            .await
            .ok()
    };

    // Run smart search with composite filter pushed down into core.
    let results = smart::smart_search(
        records,
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
                    "bm25_score": (r.bm25_score * 100.0).round() / 100.0,
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
            "semantic_available": has_embeddings,
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

#[cfg(all(test, feature = "semantic-search"))]
mod tests {
    use super::expected_rows;

    use forgeplan_core::constellation::capture::CapturedSection;
    use std::collections::HashSet;
    use std::sync::Arc;

    fn section(parent: &str, ordinal: u32) -> CapturedSection {
        let start = ordinal as usize * 10;
        CapturedSection {
            canonical_key: format!("{parent}#s{ordinal}"),
            parent_key: parent.to_string(),
            ordinal,
            byte_start: start,
            byte_end: start + 10,
            heading: None,
            level: None,
            content: Arc::from("body"),
            input_digest: String::new(),
            parent_input_digest: String::new(),
        }
    }

    fn keys(ids: &[&str]) -> HashSet<&str> {
        ids.iter().copied().collect()
    }

    /// The whole point of the change: artifacts with NO published sections must
    /// not have section rows counted for them. A `semantic_unavailable`
    /// capture is exactly this shape, and the old table-version check counted
    /// nothing extra only by luck.
    #[test]
    fn a_capture_with_no_sections_yields_one_row_per_artifact() {
        assert_eq!(expected_rows(&[], &keys(&["prd-auth", "rfc-wire"])), 2);
    }

    /// Sections under the filter are counted: two artifacts, two sections
    /// each.
    #[test]
    fn sections_of_filtered_artifacts_are_counted() {
        let sections = vec![
            section("prd-auth", 0),
            section("prd-auth", 1),
            section("rfc-wire", 0),
            section("rfc-wire", 1),
        ];
        assert_eq!(expected_rows(&sections, &keys(&["prd-auth", "rfc-wire"])), 6);
    }

    /// Sections of an artifact OUTSIDE the filter are not counted — the
    /// denominator has to describe the filter, not the corpus.
    #[test]
    fn sections_outside_the_filter_are_not_counted() {
        let sections = vec![
            section("prd-auth", 0),
            section("prd-auth", 1),
            section("prd-other", 0),
            section("prd-other", 1),
        ];
        assert_eq!(expected_rows(&sections, &keys(&["prd-auth"])), 3);
    }

    /// An empty filter matches no artifact, so nothing is expected — including
    /// no section rows, even for artifacts that have them.
    #[test]
    fn an_empty_key_set_expects_nothing() {
        let sections = vec![section("prd-auth", 0), section("prd-auth", 1)];
        assert_eq!(expected_rows(&sections, &keys(&[])), 0);
    }
}
