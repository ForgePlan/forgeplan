//! Query layer on top of the append-only JSONL log.
//!
//! Streams through daily log files, filters by time/tool/status, and
//! returns parsed entries. Corrupted lines are skipped with a warning
//! so a malformed single entry never breaks the query.
//!
//! All reads are sequential — the log is small relative to LanceDB
//! (10k lines = ~5 MB) and we'd rather keep the format grep-friendly
//! than build an index.

use super::{ActivityEntry, log_file_for};
use chrono::{DateTime, Duration, Utc};
use std::path::Path;
use tokio::io::AsyncBufReadExt;

/// Filters applied to an activity query.
#[derive(Debug, Clone, Default)]
pub struct QueryFilter {
    /// Only return entries with `ts` within this window from now.
    pub since: Option<Duration>,

    /// Only entries whose `tool` field exactly matches any of these names.
    /// Empty = no filter.
    pub tools: Vec<String>,

    /// Only entries whose `status` matches any of these. Empty = no filter.
    pub statuses: Vec<String>,

    /// Max number of entries to return (from the end — most recent).
    /// None = no cap.
    pub limit: Option<usize>,
}

/// Result of a query: parsed entries plus warnings about corrupted lines.
#[derive(Debug)]
pub struct QueryResult {
    pub entries: Vec<ActivityEntry>,
    pub total_scanned: usize,
    pub warnings: Vec<String>,
}

/// Run a query against the activity log. Reads today's file plus prior
/// days' files if `since` extends into them.
pub async fn query(workspace: &Path, filter: &QueryFilter) -> anyhow::Result<QueryResult> {
    let now = Utc::now();
    let threshold: Option<DateTime<Utc>> = filter.since.map(|d| now - d);

    // Determine which date-bucket files to scan. If `since` is 1 hour,
    // only today. If 48 hours, today + yesterday. Etc.
    let days_back: i64 = match filter.since {
        Some(d) => (d.num_seconds() / 86_400) + 1,
        None => 0, // only today's file if no since
    };

    let mut entries: Vec<ActivityEntry> = Vec::new();
    let mut total_scanned = 0usize;
    let mut warnings: Vec<String> = Vec::new();

    // Scan from oldest to newest so output is naturally chronological.
    let start_day = days_back.max(0);
    for days_ago in (0..=start_day).rev() {
        let ts = now - Duration::days(days_ago);
        let path = log_file_for(workspace, ts);
        if !path.exists() {
            continue;
        }
        let file = tokio::fs::File::open(&path).await?;
        let reader = tokio::io::BufReader::new(file);
        let mut lines = reader.lines();
        let mut line_no = 0usize;
        while let Some(line) = lines.next_line().await? {
            line_no += 1;
            total_scanned += 1;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<ActivityEntry>(&line) {
                Ok(entry) => {
                    if matches(&entry, filter, threshold) {
                        entries.push(entry);
                    }
                }
                Err(e) => {
                    warnings.push(format!(
                        "parse error in {}:{}: {}",
                        path.display(),
                        line_no,
                        e
                    ));
                }
            }
        }
    }

    // Apply limit — keep most recent.
    if let Some(limit) = filter.limit
        && entries.len() > limit
    {
        let drop = entries.len() - limit;
        entries.drain(..drop);
    }

    Ok(QueryResult {
        entries,
        total_scanned,
        warnings,
    })
}

fn matches(entry: &ActivityEntry, filter: &QueryFilter, threshold: Option<DateTime<Utc>>) -> bool {
    if let Some(threshold) = threshold {
        match DateTime::parse_from_rfc3339(&entry.ts) {
            Ok(ts) => {
                if ts.with_timezone(&Utc) < threshold {
                    return false;
                }
            }
            Err(_) => return false, // unparseable ts => skip
        }
    }
    if !filter.tools.is_empty() && !filter.tools.iter().any(|t| t == &entry.tool) {
        return false;
    }
    if !filter.statuses.is_empty() && !filter.statuses.iter().any(|s| s == &entry.status) {
        return false;
    }
    true
}

/// Per-tool statistics across the filtered entry set.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolStats {
    pub tool: String,
    pub count: usize,
    pub err_count: usize,
    pub p50_ms: u64,
    pub p95_ms: u64,
    pub total_ms: u64,
}

/// Compute aggregated per-tool statistics.
pub fn compute_stats(entries: &[ActivityEntry]) -> Vec<ToolStats> {
    use std::collections::BTreeMap;
    let mut by_tool: BTreeMap<String, Vec<u64>> = BTreeMap::new();
    let mut errors_by_tool: BTreeMap<String, usize> = BTreeMap::new();

    for e in entries {
        by_tool
            .entry(e.tool.clone())
            .or_default()
            .push(e.duration_ms);
        if e.status != super::status::OK {
            *errors_by_tool.entry(e.tool.clone()).or_default() += 1;
        }
    }

    let mut out: Vec<ToolStats> = Vec::with_capacity(by_tool.len());
    for (tool, mut durations) in by_tool {
        durations.sort_unstable();
        let count = durations.len();
        let total_ms: u64 = durations.iter().sum();
        let p50 = percentile(&durations, 50.0);
        let p95 = percentile(&durations, 95.0);
        out.push(ToolStats {
            tool: tool.clone(),
            count,
            err_count: *errors_by_tool.get(&tool).unwrap_or(&0),
            p50_ms: p50,
            p95_ms: p95,
            total_ms,
        });
    }
    // Sort by total time descending — puts costliest tools first.
    out.sort_by(|a, b| b.total_ms.cmp(&a.total_ms));
    out
}

fn percentile(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let rank = (p / 100.0 * sorted.len() as f64).ceil() as usize;
    let idx = rank.saturating_sub(1).min(sorted.len() - 1);
    sorted[idx]
}

#[cfg(test)]
mod tests {
    use super::super::{append, make_entry, status};
    use super::*;
    use tempfile::TempDir;

    async fn seed(ws: &Path, n: usize, tool: &str, status_str: &str, duration: u64) {
        for _ in 0..n {
            let entry = make_entry(
                tool,
                &serde_json::json!({}),
                duration,
                status_str,
                ws,
                None,
                false,
            );
            append(ws, &entry).await.unwrap();
        }
    }

    #[tokio::test]
    async fn empty_workspace_returns_empty_result() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let r = query(&ws, &QueryFilter::default()).await.unwrap();
        assert_eq!(r.entries.len(), 0);
        assert_eq!(r.total_scanned, 0);
    }

    #[tokio::test]
    async fn since_filter_scopes_time_window() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        seed(&ws, 3, "forgeplan_health", status::OK, 10).await;

        let r = query(
            &ws,
            &QueryFilter {
                since: Some(Duration::hours(1)),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(r.entries.len(), 3);
    }

    #[tokio::test]
    async fn tool_filter_matches_exact_name() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        seed(&ws, 2, "forgeplan_health", status::OK, 10).await;
        seed(&ws, 3, "forgeplan_score", status::OK, 20).await;

        let r = query(
            &ws,
            &QueryFilter {
                tools: vec!["forgeplan_score".into()],
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(r.entries.len(), 3);
        assert!(r.entries.iter().all(|e| e.tool == "forgeplan_score"));
    }

    #[tokio::test]
    async fn status_filter_selects_errors() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        seed(&ws, 2, "forgeplan_health", status::OK, 10).await;
        seed(&ws, 1, "forgeplan_reason", status::TOOL_ERR, 5000).await;

        let r = query(
            &ws,
            &QueryFilter {
                statuses: vec![status::TOOL_ERR.into()],
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(r.entries.len(), 1);
        assert_eq!(r.entries[0].status, "tool_err");
    }

    #[tokio::test]
    async fn limit_keeps_most_recent() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        for i in 0..5u64 {
            let entry = make_entry(
                "forgeplan_health",
                &serde_json::json!({"n": i}),
                i,
                status::OK,
                &ws,
                None,
                false,
            );
            append(&ws, &entry).await.unwrap();
        }
        let r = query(
            &ws,
            &QueryFilter {
                limit: Some(2),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(r.entries.len(), 2);
        // Two most recent = durations 3 and 4.
        assert_eq!(r.entries[0].duration_ms, 3);
        assert_eq!(r.entries[1].duration_ms, 4);
    }

    #[tokio::test]
    async fn corrupted_line_is_reported_in_warnings() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        // Write one good + one corrupt + one good line directly.
        let path = log_file_for(&ws, Utc::now());
        tokio::fs::create_dir_all(path.parent().unwrap())
            .await
            .unwrap();
        let good = make_entry(
            "forgeplan_health",
            &serde_json::json!({}),
            1,
            status::OK,
            &ws,
            None,
            false,
        );
        let good_line = serde_json::to_string(&good).unwrap();
        let mixed = format!("{good_line}\n{{truncated json without closing\n{good_line}\n");
        tokio::fs::write(&path, mixed).await.unwrap();

        let r = query(&ws, &QueryFilter::default()).await.unwrap();
        assert_eq!(r.entries.len(), 2, "two good lines recovered");
        assert_eq!(r.warnings.len(), 1);
        assert!(r.warnings[0].contains("parse error"));
    }

    #[test]
    fn stats_computes_percentiles() {
        let entries: Vec<ActivityEntry> = (1..=100)
            .map(|i| ActivityEntry {
                ts: "2026-04-18T00:00:00.000Z".into(),
                tool: "forgeplan_health".into(),
                args_hash: "x".repeat(12),
                duration_ms: i as u64,
                status: "ok".into(),
                workspace: "/tmp".into(),
                client_info: None,
                args: None,
            })
            .collect();
        let s = compute_stats(&entries);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].count, 100);
        assert_eq!(s[0].p50_ms, 50);
        assert_eq!(s[0].p95_ms, 95);
    }

    #[test]
    fn stats_counts_errors_separately() {
        let entries = vec![
            ActivityEntry {
                ts: "2026-04-18T00:00:00.000Z".into(),
                tool: "forgeplan_reason".into(),
                args_hash: "x".repeat(12),
                duration_ms: 5000,
                status: "ok".into(),
                workspace: "/tmp".into(),
                client_info: None,
                args: None,
            },
            ActivityEntry {
                ts: "2026-04-18T00:00:01.000Z".into(),
                tool: "forgeplan_reason".into(),
                args_hash: "x".repeat(12),
                duration_ms: 100,
                status: "tool_err".into(),
                workspace: "/tmp".into(),
                client_info: None,
                args: None,
            },
        ];
        let s = compute_stats(&entries);
        assert_eq!(s[0].count, 2);
        assert_eq!(s[0].err_count, 1);
    }
}
