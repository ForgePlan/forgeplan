use crate::db::store::ArtifactRecord;
use crate::scoring::reff::{EvidenceItem, EvidenceType, Verdict};
use serde::{Deserialize, Serialize};

/// Source tier for discovery findings.
/// Maps to congruence level when evidence is created from tiered sources.
///
/// - T1 (authoritative): code files, git log, package manifests → CL3 (no penalty)
/// - T2 (extracted): tests, JSDoc, CI configs → CL2 (0.1 penalty)
/// - T3 (supplementary): docs/, README, legacy documentation → CL1 (0.4 penalty)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceTier {
    /// Tier 1: authoritative — code files, git log, package manifests.
    T1,
    /// Tier 2: extracted — tests, JSDoc, CI configs.
    T2,
    /// Tier 3: supplementary — docs/, README, legacy (may be stale).
    T3,
}

impl SourceTier {
    /// Map tier to congruence level for R_eff computation.
    /// T1 → CL3 (no penalty), T2 → CL2 (0.1 penalty), T3 → CL1 (0.4 penalty).
    pub fn to_congruence_level(&self) -> u8 {
        match self {
            Self::T1 => 3,
            Self::T2 => 2,
            Self::T3 => 1,
        }
    }

    /// Parse from string — accepts "t1"/"tier1"/"tier-1"/"1" and case variants.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().trim() {
            "t1" | "tier1" | "tier-1" | "1" => Some(Self::T1),
            "t2" | "tier2" | "tier-2" | "2" => Some(Self::T2),
            "t3" | "tier3" | "tier-3" | "3" => Some(Self::T3),
            _ => None,
        }
    }
}

/// Parse evidence metadata from an ArtifactRecord's body fields.
///
/// Evidence CL precedence rules:
/// - If only `source_tier`: use tier_cl (T1→CL3, T2→CL2, T3→CL1)
/// - If only explicit `congruence_level`: use it directly
/// - If BOTH present: take the MINIMUM (more conservative) — explicit user
///   downgrade always wins over automatic tier upgrade
/// - If neither: default to CL3
///
/// This prevents self-signed T1 evidence from overriding an operator's explicit
/// downgrade, closing the trust-amplification attack surface identified in
/// PRD-035 Sprint 13.3 security audit H2 (a malicious contributor cannot
/// inflate `R_eff` by tagging weak evidence as `source_tier: t1`).
pub fn parse_evidence_from_record(record: &ArtifactRecord) -> EvidenceItem {
    let verdict = extract_field(&record.body, "verdict")
        .map(|s| match s.to_lowercase().as_str() {
            "supports" => Verdict::Supports,
            "weakens" => Verdict::Weakens,
            "refutes" => Verdict::Refutes,
            _ => Verdict::Supports,
        })
        .unwrap_or(Verdict::Supports);

    let tier_cl = extract_field(&record.body, "source_tier")
        .and_then(|s| SourceTier::parse(&s))
        .map(|t| t.to_congruence_level());

    // PROB-034 follow-up (F2, audit C): distinguish "field absent" from
    // "field present but garbage". Absent → trust-local CL3 default (policy
    // for workspace-created evidence). Garbage → fail-closed to CL0 + warn
    // (user declared something, we refuse to guess what).
    let explicit_cl_raw = extract_field(&record.body, "congruence_level");
    let explicit_cl: Option<u8> = match &explicit_cl_raw {
        Some(raw) => match raw.parse::<u8>() {
            Ok(n) if n <= 3 => Some(n),
            _ => {
                eprintln!(
                    "warn: evidence {} has unparseable congruence_level='{}' — fail-closed to CL0 (penalty 0.9) to prevent silent trust inflation",
                    record.id, raw
                );
                Some(0)
            }
        },
        None => None,
    };

    // PROB-034 follow-up (F1, audit C): unclosed multi-line HTML comment
    // swallows the entire remainder of the body → extract_field returns
    // None for every field → silent CL3 default. Same trust-inflation class
    // as PROB-034 itself. Detect and fail-closed to CL0.
    let unclosed_comment = body_has_unclosed_html_comment(&record.body);
    let explicit_cl = if unclosed_comment && explicit_cl.is_none() {
        eprintln!(
            "warn: evidence {} has an unclosed '<!--' comment block — fail-closed to CL0 (penalty 0.9) to prevent silent trust inflation",
            record.id
        );
        Some(0)
    } else {
        explicit_cl
    };

    if let (Some(tcl), Some(ecl)) = (tier_cl, explicit_cl)
        && tcl != ecl
    {
        eprintln!(
            "warn: evidence {} has divergent source_tier (CL{tcl}) and congruence_level (CL{ecl}); using MIN (more conservative) to prevent trust inflation",
            record.id,
        );
    }

    // Precedence: take MIN of (tier_cl, explicit_cl). Explicit operator
    // downgrade can never be silently overridden by an automatic tier mapping.
    // Default CL=3 (same context) — evidence created locally is same-context by default.
    let cl = match (tier_cl, explicit_cl) {
        (Some(t), Some(e)) => t.min(e),
        (Some(t), None) => t,
        (None, Some(e)) => e,
        (None, None) => 3,
    };

    let valid_until = record.valid_until.as_deref().and_then(|s| {
        chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
            .ok()
            .or_else(|| {
                chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                    .ok()
                    .and_then(|d| d.and_hms_opt(23, 59, 59))
            })
    });

    let evidence_type = extract_field(&record.body, "evidence_type")
        .or_else(|| extract_field(&record.body, "type"))
        .map(|s| match s.to_lowercase().as_str() {
            "test" => EvidenceType::Test,
            "measurement" => EvidenceType::Measurement,
            "benchmark" => EvidenceType::Benchmark,
            "audit" => EvidenceType::Audit,
            _ => EvidenceType::Measurement,
        })
        .unwrap_or(EvidenceType::Measurement);

    EvidenceItem {
        id: record.id.clone(),
        evidence_type,
        verdict,
        congruence_level: cl,
        valid_until,
    }
}

/// Extract a simple "key: value" from body text.
///
/// Skips markdown table rows (lines starting with `|`) and HTML comments
/// (both single-line `<!-- ... -->` and MULTI-LINE blocks) to avoid matching
/// template placeholder values like:
///
/// ```text
/// <!--
///      congruence_level: 0 | 1 | 2 | 3 (CL3=..., CL0=...)
/// -->
/// ```
///
/// PROB-034: previously only lines literally starting with `<!--` were
/// skipped, so inner lines of multi-line comments leaked into the parser
/// and shadowed real structured fields below (the first match wins). Every
/// evidence created via `forgeplan new evidence` had its congruence_level
/// silently reset to the CL3 default because the template comment's
/// placeholder value `"0 | 1 | 2 | 3 (CL3=...)"` matched first, then
/// `parse::<u8>()` failed, and the real `congruence_level: X` below was
/// never inspected.
///
/// Only matches standalone `key: value` lines (structured fields).
/// Returns true if `body` contains an unclosed multi-line HTML comment
/// (a `<!--` opening that never sees a matching `-->` before EOF).
///
/// Used by `parse_evidence_from_record` to fail-closed on malformed bodies:
/// an unclosed comment swallows every subsequent line in `extract_field`,
/// which would silently make all structured fields default to CL3 — the
/// same trust-inflation class as PROB-034.
pub fn body_has_unclosed_html_comment(body: &str) -> bool {
    let mut in_comment = false;
    for line in body.lines() {
        let trimmed = line.trim();
        if in_comment {
            if trimmed.contains("-->") {
                in_comment = false;
            }
            continue;
        }
        if trimmed.starts_with("<!--") && !trimmed.contains("-->") {
            in_comment = true;
        }
    }
    in_comment
}

pub fn extract_field(body: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");
    let mut in_multiline_comment = false;
    for line in body.lines() {
        let trimmed = line.trim();

        // Multi-line HTML comment state machine. Handles blocks that span
        // multiple lines (`<!--` on one line, `-->` several lines later).
        // Single-line comments `<!-- ... -->` open and close on the same
        // line, so `in_multiline_comment` stays false after processing.
        if in_multiline_comment {
            if trimmed.contains("-->") {
                in_multiline_comment = false;
            }
            continue;
        }
        if trimmed.starts_with("<!--") {
            if !trimmed.contains("-->") {
                in_multiline_comment = true;
            }
            continue;
        }

        // Skip markdown table rows — template placeholders like
        // `| CL | 0 / 1 / 2 / 3 |`.
        if trimmed.starts_with('|') {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix(&prefix) {
            let val = rest.trim();
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_field_basic() {
        let body = "verdict: supports\ncongruence_level: 2\n";
        assert_eq!(extract_field(body, "verdict"), Some("supports".into()));
        assert_eq!(extract_field(body, "congruence_level"), Some("2".into()));
        assert_eq!(extract_field(body, "missing"), None);
    }

    #[test]
    fn extract_field_with_whitespace() {
        let body = "  verdict:   Weakens  \n";
        assert_eq!(extract_field(body, "verdict"), Some("Weakens".into()));
    }

    #[test]
    fn extract_field_ignores_table_rows() {
        let body = "| Type | measurement / test / benchmark / audit |\n| Verdict | supports / weakens / refutes |\n| CL | 0 / 1 / 2 / 3 |\n";
        // Table rows should NOT match any field
        assert_eq!(extract_field(body, "type"), None);
        assert_eq!(extract_field(body, "verdict"), None);
        assert_eq!(extract_field(body, "congruence_level"), None);
    }

    #[test]
    fn extract_field_ignores_html_comments() {
        let body = "<!-- congruence_level: 0 | 1 | 2 | 3 -->\ncongruence_level: 3\n";
        assert_eq!(extract_field(body, "congruence_level"), Some("3".into()));
    }

    #[test]
    fn extract_field_ignores_multiline_html_comments() {
        // PROB-034: the evidence template ships with a MULTI-LINE help
        // comment explaining valid values. Previously only lines literally
        // starting with `<!--` were skipped, so the placeholder leaked
        // into the parser and shadowed the real field below.
        let body = r#"<!-- Fill in the Structured Fields section below.

     evidence_type: measurement | test | benchmark | audit
     verdict: supports | weakens | refutes
     congruence_level: 0 | 1 | 2 | 3 (CL3=same context, CL0=opposed)
-->

## Structured Fields

evidence_type: measurement
verdict: supports
congruence_level: 0
"#;
        assert_eq!(
            extract_field(body, "congruence_level"),
            Some("0".into()),
            "multi-line comment must not shadow real structured field"
        );
        assert_eq!(extract_field(body, "verdict"), Some("supports".into()));
        assert_eq!(
            extract_field(body, "evidence_type"),
            Some("measurement".into())
        );
    }

    #[test]
    fn body_has_unclosed_html_comment_detects_dangling_open() {
        // PROB-034 follow-up (F1): unclosed `<!--` should be flagged so
        // parse_evidence_from_record can fail-closed to CL0.
        assert!(body_has_unclosed_html_comment("<!-- unclosed"));
        assert!(body_has_unclosed_html_comment(
            "before\n<!-- starts here\nnever closes\n"
        ));
        // Closed single-line should NOT flag.
        assert!(!body_has_unclosed_html_comment("<!-- closed -->"));
        // Closed multi-line should NOT flag.
        assert!(!body_has_unclosed_html_comment(
            "<!--\nstill inside\n-->\nafter"
        ));
        // No comments should NOT flag.
        assert!(!body_has_unclosed_html_comment("plain text\nno html"));
    }

    #[test]
    fn unclosed_comment_forces_cl0_fail_closed() {
        // PROB-034 follow-up (F1, audit C MEDIUM): body with unclosed
        // `<!--` that hides the real congruence_level below → parser
        // must fail-closed to CL0, not silently default to CL3.
        let body = "<!-- oops forgot to close this\ncongruence_level: 3\nverdict: supports\n";
        let record = mk_record(body);
        let item = parse_evidence_from_record(&record);
        assert_eq!(
            item.congruence_level, 0,
            "unclosed comment must fail-closed to CL0, got CL{}",
            item.congruence_level
        );
    }

    #[test]
    fn unparseable_congruence_level_forces_cl0_fail_closed() {
        // PROB-034 follow-up (F2, audit C MEDIUM): when congruence_level
        // field is present but cannot be parsed as u8 0..=3, fail-closed
        // to CL0 instead of silently defaulting to CL3.
        let body = "## Structured Fields\n\ncongruence_level: high\nverdict: supports\n";
        let record = mk_record(body);
        let item = parse_evidence_from_record(&record);
        assert_eq!(
            item.congruence_level, 0,
            "unparseable 'high' must fail-closed to CL0, got CL{}",
            item.congruence_level
        );
    }

    #[test]
    fn parse_evidence_on_verbatim_template_returns_cl3_default() {
        // PROB-034 verbatim template guard: the shipped template has
        // multi-line comments + default Structured Fields with CL3.
        // When loaded AS-IS without any user edit, parser should return
        // CL3 (trust-local default for freshly created evidence).
        let template = include_str!("../../../../templates/evidence/_TEMPLATE.md");
        let record = mk_record(template);
        let item = parse_evidence_from_record(&record);
        assert_eq!(
            item.congruence_level, 3,
            "verbatim template must parse to default CL3, got CL{} — parser may have regressed on multi-line comment handling",
            item.congruence_level
        );
        // evidence_type should be measurement (template default).
        matches!(item.evidence_type, EvidenceType::Measurement);
    }

    #[test]
    fn extract_field_multiline_comment_nested_fields_all_ignored() {
        // Guard against a variant where the comment contains multiple
        // lines each mentioning the key — none should leak.
        let body = r#"<!--
congruence_level: 3
congruence_level: 0
-->
congruence_level: 2
"#;
        assert_eq!(extract_field(body, "congruence_level"), Some("2".into()));
    }

    #[test]
    fn extract_field_structured_field_wins_over_template_placeholders() {
        // Simulates body with template table AND user-added structured fields
        let body = r#"| Field | Value |
|-------|-------|
| Status | Draft |
| Type | measurement / test / benchmark / audit |
| Verdict | supports / weakens / refutes |
| CL | 0 / 1 / 2 / 3 |

## Structured Fields

evidence_type: test
verdict: supports
congruence_level: 3
"#;
        assert_eq!(extract_field(body, "verdict"), Some("supports".into()));
        assert_eq!(extract_field(body, "congruence_level"), Some("3".into()));
        assert_eq!(extract_field(body, "evidence_type"), Some("test".into()));
    }

    fn mk_record(body: &str) -> ArtifactRecord {
        ArtifactRecord {
            id: "EVID-999".into(),
            kind: "evidence".into(),
            status: "draft".into(),
            title: "t".into(),
            body: body.into(),
            depth: "tactical".into(),
            author: None,
            parent_epic: None,
            r_eff_score: 0.0,
            valid_until: None,
            created_at: String::new(),
            updated_at: String::new(),
            tags: Vec::new(),
            body_hash: None,
            embedding: None,
        }
    }

    #[test]
    fn source_tier_to_cl_mapping() {
        assert_eq!(SourceTier::T1.to_congruence_level(), 3);
        assert_eq!(SourceTier::T2.to_congruence_level(), 2);
        assert_eq!(SourceTier::T3.to_congruence_level(), 1);
    }

    #[test]
    fn source_tier_parse_variants() {
        assert_eq!(SourceTier::parse("t1"), Some(SourceTier::T1));
        assert_eq!(SourceTier::parse("T1"), Some(SourceTier::T1));
        assert_eq!(SourceTier::parse("tier1"), Some(SourceTier::T1));
        assert_eq!(SourceTier::parse("TIER-2"), Some(SourceTier::T2));
        assert_eq!(SourceTier::parse("  tier-3 "), Some(SourceTier::T3));
        assert_eq!(SourceTier::parse("3"), Some(SourceTier::T3));
        assert_eq!(SourceTier::parse("bogus"), None);
        assert_eq!(SourceTier::parse("t4"), None);
    }

    #[test]
    fn evidence_body_with_source_tier_maps_to_cl() {
        let body = "source_tier: t2\nevidence_type: test\n";
        let item = parse_evidence_from_record(&mk_record(body));
        assert_eq!(item.congruence_level, 2);
    }

    #[test]
    fn evidence_body_source_tier_t1_maps_to_cl3() {
        let body = "source_tier: tier1\n";
        let item = parse_evidence_from_record(&mk_record(body));
        assert_eq!(item.congruence_level, 3);
    }

    #[test]
    fn evidence_body_source_tier_t3_maps_to_cl1() {
        let body = "source_tier: 3\n";
        let item = parse_evidence_from_record(&mk_record(body));
        assert_eq!(item.congruence_level, 1);
    }

    #[test]
    fn source_tier_does_not_override_explicit_downgrade() {
        // H2 fix: explicit operator downgrade must win over automatic tier upgrade.
        let body = "source_tier: t1\ncongruence_level: 0\n";
        let item = parse_evidence_from_record(&mk_record(body));
        assert_eq!(
            item.congruence_level, 0,
            "explicit CL=0 must win over source_tier=t1 (min wins, prevents trust inflation)"
        );
    }

    #[test]
    fn explicit_cl_does_not_inflate_above_source_tier() {
        // source_tier=t3 (CL1) + congruence_level=3 → min = 1
        let body = "source_tier: t3\ncongruence_level: 3\n";
        let item = parse_evidence_from_record(&mk_record(body));
        assert_eq!(item.congruence_level, 1);
    }

    #[test]
    fn source_tier_used_when_no_explicit_cl() {
        let body = "source_tier: t2\n";
        let item = parse_evidence_from_record(&mk_record(body));
        assert_eq!(item.congruence_level, 2);
    }

    #[test]
    fn explicit_cl_used_when_no_source_tier() {
        let body = "congruence_level: 2\n";
        let item = parse_evidence_from_record(&mk_record(body));
        assert_eq!(item.congruence_level, 2);
    }

    #[test]
    fn neither_field_defaults_to_cl3() {
        let body = "verdict: supports\n";
        let item = parse_evidence_from_record(&mk_record(body));
        assert_eq!(item.congruence_level, 3);
    }

    #[test]
    fn evidence_body_invalid_source_tier_falls_back_to_cl() {
        let body = "source_tier: bogus\ncongruence_level: 2\n";
        let item = parse_evidence_from_record(&mk_record(body));
        assert_eq!(item.congruence_level, 2);
    }

    #[test]
    fn parse_evidence_with_template_placeholders_returns_cl3() {
        // Body has both template table placeholders AND structured fields
        let body = r#"| Type | measurement / test / benchmark / audit |
| Verdict | supports / weakens / refutes |
| CL | 0 / 1 / 2 / 3 |

## Structured Fields

evidence_type: test
verdict: supports
congruence_level: 3
"#;
        let record = ArtifactRecord {
            id: "EVID-001".into(),
            kind: "evidence".into(),
            status: "draft".into(),
            title: "Test evidence".into(),
            body: body.into(),
            depth: "tactical".into(),
            author: None,
            parent_epic: None,
            r_eff_score: 0.0,
            valid_until: None,
            created_at: String::new(),
            updated_at: String::new(),
            tags: Vec::new(),
            body_hash: None,
            embedding: None,
        };
        let item = parse_evidence_from_record(&record);
        assert_eq!(item.congruence_level, 3, "Should be CL3, not CL0");
        assert!(matches!(item.verdict, Verdict::Supports));
        assert!(matches!(item.evidence_type, EvidenceType::Test));
    }
}
