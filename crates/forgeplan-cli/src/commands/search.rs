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

    let ws = common::find_workspace_path()?;
    let permit = forgeplan_core::workspace::lock::acquire_workspace_read_lock(&ws).await?;
    let read = if common::index_migrated(&ws) {
        Some(forgeplan_core::constellation::facade::open_workspace_read_with_permit(&ws, &permit).await?)
    } else {
        None
    };

    // Build composite filter from CLI flags.
    let filter = build_filter(kind, status, depth, with_evidence, no_evidence, since_dt);

    match mode {
        SearchMode::Keyword => run_keyword(query, kind, filter.as_ref(), limit, json, read.as_ref(), &permit).await,
        SearchMode::Semantic => match read.as_ref() {
            Some(read) => run_constellation(read, query, filter.as_ref(), limit, json).await,
            None => run_semantic_only(query, filter.as_ref(), limit, json, &permit).await,
        },
        SearchMode::Smart => run_smart(query, filter, no_expand, limit, json, read.as_ref(), &permit).await,
    }
}


async fn run_constellation(
    read: &WorkspaceRead,
    query: &str,
    filter: Option<&forgeplan_core::search::filter::ArtifactFilter>,
    limit: usize,
    json: bool,
) -> anyhow::Result<()> {
    #[cfg(not(feature = "semantic-search"))]
    {
        let _ = (read, query, filter, limit, json);
        anyhow::bail!("Semantic search requires a binary built with semantic-search");
    }
    #[cfg(feature = "semantic-search")]
    {
        use forgeplan_core::constellation::{facade, jobs, runtime};
        let profile = read.view().semantic_profile_id().ok_or_else(|| {
            anyhow::anyhow!("No embedding profile is published; run forgeplan embed first")
        })?;
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
            read.records().len().saturating_add(read.captured().sections.len()).max(1),
            &facade::PinnedFilter::default(),
        )
        .await?;
        let expected_keys: std::collections::HashSet<_> = read
            .records()
            .iter()
            .filter(|record| semantic_matches(filter, record))
            .map(|record| record.canonical_key.as_str())
            .collect();
        // Expected rows are the CURRENT source set under this filter: one per
        // artifact, plus one per section the capture actually holds for those
        // artifacts. See [`expected_rows_for_capture`] for why that is a
        // membership count and not a table-version check.
        let expected_rows = expected_rows_for_capture(read, &expected_keys);
        let mut scores = std::collections::HashMap::<&str, f64>::new();
        let mut eligible_rows = 0;
        for hit in &outcome.hits {
            let key = hit.parent_key.as_deref().unwrap_or(&hit.canonical_key);
            let record = read.get_record_by_key(key)
                .ok_or_else(|| anyhow::anyhow!("Search hit {key} has no captured artifact"))?;
            if !semantic_matches(filter, record) || !hit.similarity.is_finite() {
                continue;
            }
            eligible_rows += 1;
            scores.entry(key).and_modify(|score| *score = score.max(hit.similarity)).or_insert(hit.similarity);
        }
        let mut ranked: Vec<_> = scores.into_iter().collect();
        ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(b.0)));
        ranked.truncate(limit);
        let mut rows = Vec::with_capacity(ranked.len());
        for (key, similarity) in ranked {
            let record = read.get_record_by_key(key)
                .ok_or_else(|| anyhow::anyhow!("Search hit {key} has no captured artifact"))?;
            rows.push(serde_json::json!({
                "id": record.id, "title": record.title, "kind": record.kind,
                "status": record.status, "similarity": similarity,
                "score": similarity, "distance": 1.0 - similarity,
                "canonical_key": record.canonical_key, "document_key": key,
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
                    "eligible_rows": eligible_rows, "expected_rows": expected_rows,
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
            if eligible_rows < expected_rows {
                ui::info(&format!(
                    "Semantic coverage: {}/{} current input rows are eligible",
                    eligible_rows, expected_rows
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

#[cfg(any(test, feature = "semantic-search"))]
fn semantic_matches(
    filter: Option<&forgeplan_core::search::filter::ArtifactFilter>,
    record: &forgeplan_core::db::store::ArtifactRecord,
) -> bool {
    filter.is_none_or(|filter| filter.matches(record))
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
    filter: Option<&forgeplan_core::search::filter::ArtifactFilter>,
    limit: usize,
    json: bool,
    read: Option<&WorkspaceRead>,
    permit: &dyn forgeplan_core::workspace::lock::WorkspaceReadPermit,
) -> anyhow::Result<()> {
    let mut hits = match read {
        Some(read) => read.search_body(query, kind),
        None => common::store().await?.search_body(permit, query, kind).await?,
    };
    hits.retain(|record| filter.is_none_or(|filter| filter.matches(record)));
    hits.truncate(limit);

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

    let next_hints: Vec<Hint> =
        vec![Hint::info("Top result").with_action(format!("forgeplan get {}", hits[0].canonical_key))];

    if json {
        let json_data: Vec<_> = hits
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id,
                    "canonical_key": r.canonical_key,
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
async fn unindexed_count(store: &forgeplan_core::db::store::LanceStore, permit: &dyn forgeplan_core::workspace::lock::WorkspaceReadPermit) -> usize {
    match store.list_records(permit, None).await {
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
async fn run_semantic_only(query: &str, filter: Option<&forgeplan_core::search::filter::ArtifactFilter>, limit: usize, json: bool, permit: &dyn forgeplan_core::workspace::lock::WorkspaceReadPermit) -> anyhow::Result<()> {
    #[cfg(feature = "semantic-search")]
    {

        let store = common::store().await?;

        let count = store.list_records(permit, None).await?.len();
        let all_hits = store.vector_search(permit, query, count.max(1)).await?;
        let mut hits: Vec<_> = all_hits.into_iter()
            .filter(|hit| semantic_matches(filter, &hit.record))
            .collect();
        hits.sort_by(|a, b| b.similarity().total_cmp(&a.similarity())
            .then_with(|| a.record.canonical_key.cmp(&b.record.canonical_key)));
        hits.truncate(limit);

        // PROB-093: how much of the workspace this query could not reach.
        let unindexed = unindexed_count(&store, permit).await;

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
                        "canonical_key": h.record.canonical_key,
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
        let _ = (query, filter, limit, json, permit);
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
    permit: &dyn forgeplan_core::workspace::lock::WorkspaceReadPermit,
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
                .list_records(permit, None)
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
            forgeplan_core::search::semantic::workspace_semantic_scores(read, permit, query).await?
        }
        None => forgeplan_core::search::semantic::semantic_scores(store.as_ref().expect("legacy source was opened"), permit, query).await?,
    };


    // Build knowledge graph for centrality booster + neighbor expansion.
    // --no-expand disables expansion (graph passed as None) AND drops the
    // centrality booster — acceptable trade-off for "strict matches only".
    let graph = if no_expand {
        None
    } else if let Some(read) = read {
        Some(read.knowledge_graph())
    } else {
        KnowledgeGraph::from_store(store.as_ref().expect("legacy source was opened"), permit)
            .await
            .ok()
    };

    // The core scorer currently uses its `id` field as the result-map key.
    // Give it canonical identities so shared display aliases remain separate.
    let keyed_records: Vec<_> = records
        .iter()
        .cloned()
        .map(|mut record| {
            record.id.clone_from(&record.canonical_key);
            record
        })
        .collect();
    let results = smart::smart_search(
        &keyed_records,
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
                let display_id = records
                    .iter()
                    .find(|record| record.canonical_key == r.id)
                    .map(|record| record.id.as_str())
                    .unwrap_or(&r.id);
                serde_json::json!({
                    "id": display_id,
                    "canonical_key": r.id,
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
            let display_id = records
                .iter()
                .find(|record| record.canonical_key == r.id)
                .map(|record| record.id.as_str())
                .unwrap_or(&r.id);
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
                display_id,
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

    fn keys<'a>(ids: &[&'a str]) -> HashSet<&'a str> {
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
        assert_eq!(
            expected_rows(&sections, &keys(&["prd-auth", "rfc-wire"])),
            6
        );
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

#[cfg(test)]
mod semantic_filter_tests {
    use super::{build_filter, semantic_matches};
    use forgeplan_core::db::store::ArtifactRecord;

    fn record() -> ArtifactRecord {
        ArtifactRecord {
            canonical_key: "prd-alpha".into(), id: "PRD-001".into(),
            kind: "prd".into(), status: "active".into(), title: "Alpha".into(),
            body: String::new(), depth: "deep".into(), author: None,
            parent_epic: None, r_eff_score: 0.5, valid_until: None,
            created_at: "2026-02-01T00:00:00Z".into(),
            updated_at: "2026-02-01T00:00:00Z".into(),
            tags: Vec::new(), body_hash: None, embedding: None,
        }
    }

    #[test]
    fn semantic_composite_filters_apply_all_predicates() {
        let date = chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap();
        let filter = build_filter(Some("PRD"), Some("ACTIVE"), Some("deep"), true, false, Some(date));
        let mut hit = record();
        assert!(semantic_matches(filter.as_ref(), &hit));
        hit.kind = "rfc".into();
        assert!(!semantic_matches(filter.as_ref(), &hit));
        hit.kind = "prd".into();
        hit.status = "draft".into();
        assert!(!semantic_matches(filter.as_ref(), &hit));
        hit.status = "active".into();
        hit.depth = "standard".into();
        assert!(!semantic_matches(filter.as_ref(), &hit));
        hit.depth = "deep".into();
        hit.r_eff_score = 0.0;
        assert!(!semantic_matches(filter.as_ref(), &hit));
        hit.r_eff_score = 0.5;
        hit.created_at = "2025-01-01T00:00:00Z".into();
        assert!(!semantic_matches(filter.as_ref(), &hit));
        assert!(!semantic_matches(build_filter(None, None, None, false, true, None).as_ref(), &record()));
    }
}
