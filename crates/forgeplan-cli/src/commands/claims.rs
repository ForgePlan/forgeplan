//! `forgeplan claims` — list active claims sorted by expiry (PRD-057 Inc 3
//! + PRD-070 CLI parity).
//!
//! Mirrors `forgeplan_claims` MCP tool: read-only listing, expired claims
//! filtered out, malformed files surfaced as a `skipped` count rather than
//! silently dropped.

use chrono::Utc;
use forgeplan_core::claim::ClaimStore;
use forgeplan_core::workspace;

pub async fn run(json: bool) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let store = ClaimStore::new(&ws);
    let (claims, skipped) = store
        .list_active_with_stats()
        .await
        .map_err(|e| anyhow::anyhow!("list_active failed: {e}"))?;
    let count = claims.len();

    if json {
        let claims_json: Vec<_> = claims
            .iter()
            .map(|c| {
                serde_json::json!({
                    "id": c.id,
                    "agent_id": c.agent_id,
                    "claimed_at": c.claimed_at.to_rfc3339(),
                    "expires_at": c.expires_at.to_rfc3339(),
                    "note": c.note,
                })
            })
            .collect();
        let body = serde_json::json!({
            "count": count,
            "skipped": skipped,
            "claims": claims_json,
        });
        println!("{}", serde_json::to_string_pretty(&body)?);
        return Ok(());
    }

    if count == 0 && skipped == 0 {
        println!("No active claims. Workspace is free for any agent to claim work.");
        return Ok(());
    }

    println!("{:<14}  {:<28}  {:<14}  NOTE", "ID", "AGENT", "EXPIRES IN");
    let separator = "-".repeat(80);
    println!("{separator}");
    let now = Utc::now();
    for c in &claims {
        let remaining = c.expires_at - now;
        let expires_in = humanize_remaining(remaining);
        let note = c.note.as_deref().unwrap_or("");
        println!(
            "{:<14}  {:<28}  {:<14}  {}",
            truncate(&c.id, 14),
            truncate(&c.agent_id, 28),
            expires_in,
            note
        );
    }

    println!();
    println!(
        "{count} active claim{} (sorted by expiry, soonest first).",
        if count == 1 { "" } else { "s" }
    );
    if skipped > 0 {
        println!(
            "  Warning: {skipped} claim file{} skipped (parse error or oversize). Run \
             `forgeplan health` to surface the offenders.",
            if skipped == 1 { "" } else { "s" }
        );
    }

    Ok(())
}

/// Render a chrono::Duration as a short human string ("12m", "2h 5m", "1d").
/// Negative or zero remaining is rendered as "expired" (defensive — we
/// only list live claims, but a clock skew between filter-time and
/// render-time could land here).
fn humanize_remaining(d: chrono::Duration) -> String {
    let secs = d.num_seconds();
    if secs <= 0 {
        return "expired".to_string();
    }
    let days = secs / 86_400;
    let hours = (secs % 86_400) / 3600;
    let minutes = (secs % 3600) / 60;
    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else if minutes > 0 {
        format!("{minutes}m")
    } else {
        format!("{secs}s")
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn humanize_seconds_minutes_hours_days() {
        use chrono::Duration;
        assert_eq!(humanize_remaining(Duration::seconds(0)), "expired");
        assert_eq!(humanize_remaining(Duration::seconds(-5)), "expired");
        assert_eq!(humanize_remaining(Duration::seconds(45)), "45s");
        assert_eq!(humanize_remaining(Duration::minutes(12)), "12m");
        assert_eq!(humanize_remaining(Duration::minutes(125)), "2h 5m");
        assert_eq!(humanize_remaining(Duration::hours(50)), "2d 2h");
    }

    #[test]
    fn truncate_keeps_short_strings() {
        assert_eq!(truncate("PRD-001", 14), "PRD-001");
    }

    #[test]
    fn truncate_appends_ellipsis_for_long_strings() {
        let out = truncate("really-long-agent-name-1234567890", 10);
        assert_eq!(out.chars().count(), 10);
        assert!(out.ends_with('…'));
    }
}
