use crate::validation::{Finding, Severity};

/// Additional adversarial checks — content quality, not just structure.
/// Implements devil's advocate review: MUST find at least 1 issue.
pub fn adversarial_checks(body: &str, kind: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let lower = body.to_lowercase();

    // Check 1: Vague words without measurable metrics
    let vague = [
        "улучшить",
        "ускорить",
        "повысить",
        "оптимизировать",
        "improve",
        "enhance",
        "optimize",
        "better",
        "faster",
    ];
    for word in &vague {
        if let Some(pos) = lower.find(word) {
            // Check if there's a number nearby (within 80 chars)
            let start = pos.saturating_sub(40);
            let end = (pos + word.len() + 40).min(lower.len());
            let nearby = &lower[start..end];
            let has_metric = nearby.chars().any(|c| c.is_ascii_digit());
            if !has_metric {
                findings.push(Finding {
                    rule_id: "adversarial-vague".into(),
                    severity: Severity::Should,
                    message: format!(
                        "Vague word '{}' without measurable metric nearby",
                        word
                    ),
                    section: None,
                });
                break; // one finding per vague category
            }
        }
    }

    // Check 2: RFC without alternatives/options considered
    if kind == "rfc"
        && !lower.contains("option")
        && !lower.contains("alternative")
        && !lower.contains("variant")
        && !lower.contains("вариант")
    {
        findings.push(Finding {
            rule_id: "adversarial-no-alternatives".into(),
            severity: Severity::Should,
            message: "RFC has no alternatives/options section — single-option proposals lack rigor"
                .into(),
            section: Some("Options".into()),
        });
    }

    // Check 3: PRD without risk assessment
    if kind == "prd" && !lower.contains("risk") && !lower.contains("риск") {
        findings.push(Finding {
            rule_id: "adversarial-no-risks".into(),
            severity: Severity::Should,
            message: "PRD has no risk assessment — what could go wrong?".into(),
            section: Some("Risks".into()),
        });
    }

    // Check 4: ADR without trade-offs or downsides
    if kind == "adr"
        && !lower.contains("trade")
        && !lower.contains("downside")
        && !lower.contains("disadvantage")
        && !lower.contains("недостат")
        && !lower.contains("компромисс")
    {
        findings.push(Finding {
            rule_id: "adversarial-no-tradeoffs".into(),
            severity: Severity::Should,
            message: "ADR has no trade-offs or downsides — every decision has costs".into(),
            section: Some("Trade-offs".into()),
        });
    }

    // Check 5: Adversarial MUST find >= 1 issue (BMAD rule)
    if findings.is_empty() {
        findings.push(Finding {
            rule_id: "adversarial-too-clean".into(),
            severity: Severity::Could,
            message:
                "Adversarial found 0 issues — either perfect or review was insufficient".into(),
            section: None,
        });
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn always_finds_at_least_one_issue() {
        let findings = adversarial_checks("Perfect document with no issues.", "note");
        assert!(!findings.is_empty());
        assert_eq!(findings[0].rule_id, "adversarial-too-clean");
    }

    #[test]
    fn detects_vague_words() {
        let findings = adversarial_checks("We need to improve the system.", "prd");
        assert!(findings.iter().any(|f| f.rule_id == "adversarial-vague"));
    }

    #[test]
    fn vague_word_with_metric_ok() {
        let findings =
            adversarial_checks("We need to improve latency by 50ms to meet SLA.", "note");
        assert!(!findings.iter().any(|f| f.rule_id == "adversarial-vague"));
    }

    #[test]
    fn rfc_without_alternatives() {
        let findings = adversarial_checks("## Proposal\nUse Redis for caching.", "rfc");
        assert!(findings
            .iter()
            .any(|f| f.rule_id == "adversarial-no-alternatives"));
    }

    #[test]
    fn rfc_with_alternatives_ok() {
        let body = "## Proposal\nUse Redis.\n## Alternative\nUse Memcached.";
        let findings = adversarial_checks(body, "rfc");
        assert!(!findings
            .iter()
            .any(|f| f.rule_id == "adversarial-no-alternatives"));
    }

    #[test]
    fn prd_without_risks() {
        let findings = adversarial_checks("## Goals\nBuild auth system.", "prd");
        assert!(findings
            .iter()
            .any(|f| f.rule_id == "adversarial-no-risks"));
    }

    #[test]
    fn adr_without_tradeoffs() {
        let findings = adversarial_checks("## Decision\nUse Rust.", "adr");
        assert!(findings
            .iter()
            .any(|f| f.rule_id == "adversarial-no-tradeoffs"));
    }
}
