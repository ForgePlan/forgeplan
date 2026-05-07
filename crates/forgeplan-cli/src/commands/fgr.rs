use forgeplan_core::artifact::frontmatter;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::scoring::fgr;

use crate::commands::common;

/// PROB-060 Phase 2 audit closure (MED-10) — single date-parsing helper
/// shared between the JSON and text branches of `fgr`.
///
/// Round 1 audit caught a divergence: the JSON path (records iteration)
/// tried both the full datetime format `%Y-%m-%dT%H:%M:%S` *and* the
/// date-only fallback `%Y-%m-%d`, while the text path only tried the
/// datetime format. Artifacts using `valid_until: 2099-12-31` (which is
/// the canonical `forgeplan renew --until` shape) were therefore
/// correctly recognised as fresh in JSON output but incorrectly treated
/// as never-expiring in text output. This helper removes the divergence.
///
/// Returns `Some(true)` if the artifact's `valid_until` has passed,
/// `Some(false)` if it is still fresh, and `None` if the string cannot
/// be parsed by any supported format (treated as fresh by callers via
/// `unwrap_or(false)`).
fn is_valid_until_expired(s: &str) -> Option<bool> {
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Some(chrono::Utc::now().naive_utc() > dt);
    }
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(chrono::Utc::now().date_naive() > d);
    }
    None
}

pub async fn run(id: Option<&str>, json: bool) -> anyhow::Result<()> {
    let store = common::store().await?;
    let fpf_weights = common::config().ok().and_then(|c| c.fpf.map(|f| f.weights));

    let records = if let Some(id) = id {
        // PROB-060 / SPEC-005 Phase 2.6 (CD-6) — accept slug or display id.
        let canonical = store
            .resolve_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Artifact '{id}' not found\nFix: forgeplan list"))?;
        let record = store
            .get_record(&canonical)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Artifact not found: {canonical}"))?;
        vec![record]
    } else {
        store.list_records(None).await?
    };

    if records.is_empty() {
        let hint_list = vec![
            Hint::info("Create your first artifact")
                .with_action("forgeplan new prd \"<title>\"".to_string()),
        ];
        if json {
            let payload = serde_json::json!({
                "results": [],
                "_next_action": hints::primary_action(&hint_list),
                "hints": hint_list,
            });
            println!("{}", serde_json::to_string_pretty(&payload)?);
        } else {
            println!("No artifacts to score.");
            print!("{}", hints::render_next_action_line(&hint_list));
        }
        return Ok(());
    }

    if json {
        let mut results = Vec::new();
        // PROB-060 (W1.B, CD-5) — track the lowest-grade record's ref_form
        // (slug pre-merge / display id post-merge) so the agent's next
        // command stays canonical for commit `Refs:`.
        let mut lowest: Option<(String, String, f64)> = None;
        for record in &records {
            let kind = record
                .kind
                .parse()
                .unwrap_or(forgeplan_core::artifact::types::ArtifactKind::Note);
            let depth = record
                .depth
                .parse()
                .unwrap_or(forgeplan_core::artifact::types::Mode::Standard);
            let fm = frontmatter::parse_frontmatter(&record.body)
                .map(|(fm, _)| fm)
                .unwrap_or_default();
            let relations = store.get_relations(&record.id).await.unwrap_or_default();
            // PROB-060 MED-10 — use the shared helper so JSON and text
            // branches treat date-only `valid_until` identically.
            let is_stale = record
                .valid_until
                .as_ref()
                .is_some_and(|v| is_valid_until_expired(v).unwrap_or(false));
            let score = fgr::compute(
                &record.id,
                &record.body,
                &fm,
                &kind,
                &depth,
                record.r_eff_score,
                relations.len(),
                is_stale,
                fpf_weights.as_ref(),
            );
            let overall = score.overall();
            let ref_form =
                forgeplan_core::artifact::frontmatter::refs_form(&fm, &record.id).to_string();
            results.push(serde_json::json!({
                "id": record.id, "title": record.title,
                "formality": score.formality, "granularity": score.granularity,
                "reliability": score.reliability, "overall": overall, "grade": score.grade(),
            }));
            if lowest.as_ref().is_none_or(|(_, _, v)| overall < *v) {
                lowest = Some((record.id.clone(), ref_form, overall));
            }
        }
        let hint_list = if let Some((_id, target_ref, _)) = lowest {
            vec![
                Hint::info(format!("Improve lowest-grade artifact {}", target_ref))
                    .with_action(format!("forgeplan get {}", target_ref)),
            ]
        } else {
            Vec::new()
        };
        let payload = serde_json::json!({
            "results": results,
            "_next_action": hints::primary_action(&hint_list),
            "hints": hint_list,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    println!(
        "{:<12} {:<30} {:>6} {:>6} {:>6} {:>5}",
        "ID", "Title", "F", "G", "R", "Grade"
    );
    println!("{}", "-".repeat(70));

    // PROB-060 (W1.B, CD-5) — track ref_form of lowest-grade record so the
    // emitted hint stays canonical (slug pre-merge / display id post-merge).
    let mut lowest: Option<(String, String, f64)> = None;
    for record in &records {
        let kind = record
            .kind
            .parse()
            .unwrap_or(forgeplan_core::artifact::types::ArtifactKind::Note);
        let depth = record
            .depth
            .parse()
            .unwrap_or(forgeplan_core::artifact::types::Mode::Standard);
        let fm = frontmatter::parse_frontmatter(&record.body)
            .map(|(fm, _)| fm)
            .unwrap_or_default();

        let relations = store.get_relations(&record.id).await.unwrap_or_default();
        // PROB-060 MED-10 — shared helper (text branch parity with JSON).
        let is_stale = record
            .valid_until
            .as_ref()
            .is_some_and(|v| is_valid_until_expired(v).unwrap_or(false));

        let score = fgr::compute(
            &record.id,
            &record.body,
            &fm,
            &kind,
            &depth,
            record.r_eff_score,
            relations.len(),
            is_stale,
            fpf_weights.as_ref(),
        );

        let title: String = record.title.chars().take(28).collect();
        println!(
            "{:<12} {:<30} {:>5.2} {:>5.2} {:>5.2} {:>5}",
            record.id,
            title,
            score.formality,
            score.granularity,
            score.reliability,
            score.grade()
        );

        let overall = score.overall();
        let ref_form =
            forgeplan_core::artifact::frontmatter::refs_form(&fm, &record.id).to_string();
        if lowest.as_ref().is_none_or(|(_, _, v)| overall < *v) {
            lowest = Some((record.id.clone(), ref_form, overall));
        }
    }

    let hint_list = if let Some((_id, target_ref, _)) = lowest {
        vec![
            Hint::info(format!("Improve lowest-grade artifact {}", target_ref))
                .with_action(format!("forgeplan get {}", target_ref)),
        ]
    } else {
        Vec::new()
    };
    print!("{}", hints::render_next_action_line(&hint_list));

    Ok(())
}

#[cfg(test)]
mod tests {
    //! PROB-060 Phase 2 audit closure (MED-10) — date parsing parity.
    //!
    //! These tests pin down the contract of [`is_valid_until_expired`]
    //! so the JSON and text branches of `fgr` cannot diverge again.
    //! Two formats are accepted: full datetime (`%Y-%m-%dT%H:%M:%S`)
    //! and date-only (`%Y-%m-%d`). Anything else returns `None`.

    use super::is_valid_until_expired;

    #[test]
    fn datetime_far_future_is_fresh() {
        assert_eq!(
            is_valid_until_expired("2099-12-31T23:59:59"),
            Some(false),
            "datetime in the far future must be reported as not expired"
        );
    }

    #[test]
    fn datetime_far_past_is_expired() {
        assert_eq!(
            is_valid_until_expired("1970-01-01T00:00:00"),
            Some(true),
            "datetime in the far past must be reported as expired"
        );
    }

    #[test]
    fn date_only_far_future_is_fresh() {
        // PROB-060 MED-10 regression guard: text branch used to skip
        // this format, treating fresh `forgeplan renew --until
        // 2099-12-31` artifacts as never-expiring (always Some(false)
        // via the divergent fallback chain). After the helper unification
        // both branches must report `Some(false)` for a future date.
        assert_eq!(
            is_valid_until_expired("2099-12-31"),
            Some(false),
            "date-only future must be reported as not expired (text/JSON parity)"
        );
    }

    #[test]
    fn date_only_far_past_is_expired() {
        assert_eq!(
            is_valid_until_expired("1970-01-01"),
            Some(true),
            "date-only past must be reported as expired"
        );
    }

    #[test]
    fn malformed_string_returns_none() {
        // None falls through to `unwrap_or(false)` in the call sites,
        // i.e. unparseable strings are conservatively treated as fresh.
        assert!(is_valid_until_expired("not-a-date").is_none());
        assert!(is_valid_until_expired("").is_none());
    }
}
