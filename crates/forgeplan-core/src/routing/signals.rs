//! Signal extraction from text — keyword triggers, complexity metrics, blast radius.

use crate::artifact::types::Mode;
use crate::routing::Signal;

/// Keyword trigger: pattern → minimum depth.
struct KeywordTrigger {
    keywords: &'static [&'static str],
    id: &'static str,
    description: &'static str,
    min_depth: Mode,
    weight: f64,
}

/// Hardcoded safety-net rules. These cannot be overridden.
const KEYWORD_TRIGGERS: &[KeywordTrigger] = &[
    // Security / Compliance → Deep+
    KeywordTrigger {
        keywords: &["security", "auth", "authentication", "authorization", "oauth", "jwt", "encryption", "compliance", "gdpr", "hipaa", "soc2"],
        id: "keyword:security",
        description: "Security or compliance topic detected",
        min_depth: Mode::Deep,
        weight: 0.9,
    },
    // Breaking changes → Deep+
    KeywordTrigger {
        keywords: &["breaking change", "backwards compatibility", "migration", "deprecat"],
        id: "keyword:breaking",
        description: "Breaking change or migration detected",
        min_depth: Mode::Deep,
        weight: 0.8,
    },
    // Cross-team / Multi-team → Standard+
    KeywordTrigger {
        keywords: &["cross-team", "multi-team", "multiple teams", "cross-service", "inter-service"],
        id: "keyword:cross_team",
        description: "Cross-team coordination needed",
        min_depth: Mode::Standard,
        weight: 0.7,
    },
    // Public API → Deep+
    KeywordTrigger {
        keywords: &["public api", "external api", "api contract", "api versioning", "openapi", "graphql schema"],
        id: "keyword:public_api",
        description: "Public/external API changes",
        min_depth: Mode::Deep,
        weight: 0.8,
    },
    // Data model / Schema → Standard+
    KeywordTrigger {
        keywords: &["data model", "schema change", "database migration", "table alter"],
        id: "keyword:data_model",
        description: "Data model or schema changes",
        min_depth: Mode::Standard,
        weight: 0.6,
    },
    // Infrastructure → Standard+
    KeywordTrigger {
        keywords: &["infrastructure", "deployment", "ci/cd", "kubernetes", "docker", "terraform"],
        id: "keyword:infra",
        description: "Infrastructure changes",
        min_depth: Mode::Standard,
        weight: 0.5,
    },
    // Strategy / Roadmap → Deep+
    KeywordTrigger {
        keywords: &["strategy", "roadmap", "quarterly", "okr", "initiative"],
        id: "keyword:strategy",
        description: "Strategic initiative",
        min_depth: Mode::Deep,
        weight: 0.7,
    },
    // New subsystem → Standard+
    KeywordTrigger {
        keywords: &["new module", "new service", "new subsystem", "new crate", "new package"],
        id: "keyword:new_subsystem",
        description: "New subsystem or module",
        min_depth: Mode::Standard,
        weight: 0.6,
    },
    // Redesign / Overhaul → Standard+
    KeywordTrigger {
        keywords: &["redesign", "overhaul", "rewrite", "refactor all", "rework", "revamp"],
        id: "keyword:redesign",
        description: "Major redesign or overhaul detected",
        min_depth: Mode::Standard,
        weight: 0.6,
    },
    // Bug / Defect patterns → Standard+
    KeywordTrigger {
        keywords: &["bug", "bugfix", "defect", "broken", "fix bug", "regression"],
        id: "keyword:bug_fix",
        description: "Bug or defect fix detected",
        min_depth: Mode::Standard,
        weight: 0.6,
    },
    // Severity / Priority patterns → Deep+
    KeywordTrigger {
        keywords: &["p0", "critical", "urgent", "high priority", "severity", "hotfix"],
        id: "keyword:severity",
        description: "High severity or priority issue detected",
        min_depth: Mode::Deep,
        weight: 0.8,
    },
    // Integrity / Consistency patterns → Standard+
    KeywordTrigger {
        keywords: &["inconsistency", "divergence", "integrity", "mismatch", "out of sync", "data loss"],
        id: "keyword:integrity",
        description: "Data integrity or consistency issue detected",
        min_depth: Mode::Standard,
        weight: 0.7,
    },
    // Multi-issue patterns → Standard+
    KeywordTrigger {
        keywords: &["multiple issues", "several bugs", "batch fix", "remediation", "audit findings"],
        id: "keyword:multi_issue",
        description: "Multiple issues or batch remediation detected",
        min_depth: Mode::Standard,
        weight: 0.7,
    },
];

/// Extract signals from a text description (task description or artifact body).
pub fn extract(text: &str) -> Vec<Signal> {
    let lower = text.to_lowercase();
    let mut signals = Vec::new();

    for trigger in KEYWORD_TRIGGERS {
        if trigger.keywords.iter().any(|kw| lower.contains(kw)) {
            signals.push(Signal {
                id: trigger.id.to_string(),
                description: trigger.description.to_string(),
                minimum_depth: trigger.min_depth.clone(),
                weight: trigger.weight,
            });
        }
    }

    // Complexity signal: estimated scope from text
    let word_count = text.split_whitespace().count();
    if word_count > 500 {
        signals.push(Signal {
            id: "complexity:length".into(),
            description: format!("{word_count} words — complex description"),
            minimum_depth: Mode::Standard,
            weight: 0.4,
        });
    }

    // Bug density heuristic: multiple bug-related words → at least Standard
    // NOTE: may double-count with keyword:bug_fix — this inflates confidence, not depth.
    // Acceptable: compute_depth takes max(signal.depth), not sum(weights).
    let bug_words = ["bug", "fix", "broken", "issue", "error", "fail"];
    let bug_count = bug_words.iter().filter(|w| lower.contains(*w)).count();
    if bug_count >= 3 {
        signals.push(Signal {
            id: "heuristic:bug_density".into(),
            description: format!("{bug_count} bug-related words found"),
            minimum_depth: Mode::Standard,
            weight: 0.7,
        });
    }

    // Detect "N issues/bugs/problems" pattern where N > 2
    // The number may not be immediately before the target word (e.g. "5 P0 integrity issues")
    // Only emit ONE signal to avoid duplicate weight inflation
    // Uses word-boundary check to avoid substring matches ("tissues" ≠ "issues")
    'issue_count: for word in ["issues", "bugs", "problems", "fixes", "errors"] {
        if let Some(pos) = lower.find(word) {
            // Word boundary: char before must be whitespace or start of string
            let at_boundary = pos == 0 || lower.as_bytes().get(pos - 1).map_or(true, |b| !b.is_ascii_alphanumeric());
            if !at_boundary {
                continue;
            }
            let prefix = lower.get(..pos).unwrap_or("");
            for token in prefix.split_whitespace() {
                if let Ok(n) = token.parse::<u32>() {
                    if n > 2 {
                        signals.push(Signal {
                            id: "heuristic:issue_count".into(),
                            description: format!("{n} {word}"),
                            minimum_depth: Mode::Standard,
                            weight: 0.7,
                        });
                        break 'issue_count;
                    }
                }
            }
        }
    }

    // Reversibility signal: explicit mentions
    if lower.contains("irreversible") || lower.contains("cannot undo") || lower.contains("one-way") {
        signals.push(Signal {
            id: "reversibility:low".into(),
            description: "Explicitly irreversible".into(),
            minimum_depth: Mode::Deep,
            weight: 0.9,
        });
    }

    signals
}

/// Extract structural signals from an existing artifact body + metadata.
pub fn extract_structural(body: &str, link_count: usize, has_epic: bool) -> Vec<Signal> {
    let mut signals = Vec::new();

    // FR count
    let fr_count = body
        .lines()
        .filter(|l| {
            let t = l.trim();
            t.starts_with("- [") || t.starts_with("* [")
        })
        .count();

    if fr_count > 10 {
        signals.push(Signal {
            id: "structure:fr_count".into(),
            description: format!("{fr_count} functional requirements"),
            minimum_depth: Mode::Deep,
            weight: 0.7,
        });
    } else if fr_count > 3 {
        signals.push(Signal {
            id: "structure:fr_count".into(),
            description: format!("{fr_count} functional requirements"),
            minimum_depth: Mode::Standard,
            weight: 0.5,
        });
    }

    // Link count (dependency proxy)
    if link_count > 5 {
        signals.push(Signal {
            id: "structure:links".into(),
            description: format!("{link_count} dependency links"),
            minimum_depth: Mode::Deep,
            weight: 0.6,
        });
    } else if link_count > 2 {
        signals.push(Signal {
            id: "structure:links".into(),
            description: format!("{link_count} dependency links"),
            minimum_depth: Mode::Standard,
            weight: 0.4,
        });
    }

    // Part of epic → at least Standard
    if has_epic {
        signals.push(Signal {
            id: "structure:parent_epic".into(),
            description: "Part of an epic (strategic initiative)".into(),
            minimum_depth: Mode::Standard,
            weight: 0.5,
        });
    }

    // Section count
    let section_count = body.lines().filter(|l| l.starts_with("## ")).count();
    if section_count > 8 {
        signals.push(Signal {
            id: "structure:sections".into(),
            description: format!("{section_count} sections — complex artifact"),
            minimum_depth: Mode::Deep,
            weight: 0.5,
        });
    } else if section_count > 4 {
        signals.push(Signal {
            id: "structure:sections".into(),
            description: format!("{section_count} sections"),
            minimum_depth: Mode::Standard,
            weight: 0.3,
        });
    }

    signals
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn security_keyword_triggers_deep() {
        let signals = extract("We need to implement OAuth2 authentication for the API");
        assert!(signals.iter().any(|s| s.id == "keyword:security"));
        assert!(signals.iter().any(|s| matches!(s.minimum_depth, Mode::Deep)));
    }

    #[test]
    fn breaking_change_triggers_deep() {
        let signals = extract("This is a breaking change to the API contract");
        assert!(signals.iter().any(|s| s.id == "keyword:breaking"));
    }

    #[test]
    fn cross_team_triggers_standard() {
        let signals = extract("Cross-team effort involving backend and mobile");
        assert!(signals.iter().any(|s| s.id == "keyword:cross_team"));
    }

    #[test]
    fn simple_task_no_triggers() {
        let signals = extract("Fix the typo in the readme");
        assert!(signals.is_empty());
    }

    #[test]
    fn irreversible_triggers_deep() {
        let signals = extract("This is an irreversible database migration");
        assert!(signals.iter().any(|s| s.id == "reversibility:low"));
    }

    #[test]
    fn redesign_triggers_standard() {
        let signals = extract("Redesign the entire CLI with new UI framework");
        assert!(signals.iter().any(|s| s.id == "keyword:redesign"));
    }

    #[test]
    fn structural_fr_count() {
        let body = "## FR\n- [ ] FR-001\n- [ ] FR-002\n- [ ] FR-003\n- [ ] FR-004\n";
        let signals = extract_structural(body, 0, false);
        assert!(signals.iter().any(|s| s.id == "structure:fr_count"));
    }

    #[test]
    fn structural_links() {
        let signals = extract_structural("body", 6, false);
        assert!(signals.iter().any(|s| s.id == "structure:links"));
    }

    #[test]
    fn structural_epic() {
        let signals = extract_structural("body", 0, true);
        assert!(signals.iter().any(|s| s.id == "structure:parent_epic"));
    }

    #[test]
    fn p0_integrity_issues_not_tactical() {
        let signals = extract("Fix 5 P0 integrity issues");
        // Should match severity (P0), integrity, and issue_count (5 issues)
        assert!(!signals.is_empty(), "should not be empty for P0 integrity issues");
        assert!(signals.iter().any(|s| s.id == "keyword:severity"));
        assert!(signals.iter().any(|s| s.id == "keyword:integrity"));
        assert!(signals.iter().any(|s| s.id == "heuristic:issue_count"));
    }

    #[test]
    fn simple_typo_fix_remains_tactical() {
        let signals = extract("Fix a typo in the readme");
        // "fix" alone triggers keyword:bug_fix, but no severity/integrity/density
        let non_bug = signals.iter().filter(|s| s.id != "keyword:bug_fix").count();
        assert_eq!(non_bug, 0, "typo fix should not trigger severity or integrity");
    }

    #[test]
    fn critical_security_bug_triggers_deep() {
        let signals = extract("Critical security bug in auth system");
        assert!(signals.iter().any(|s| s.id == "keyword:severity"));
        assert!(signals.iter().any(|s| s.id == "keyword:security"));
    }

    #[test]
    fn bug_density_heuristic() {
        let signals = extract("This bug causes error when fix is applied and the issue persists");
        assert!(signals.iter().any(|s| s.id == "heuristic:bug_density"));
    }

    #[test]
    fn issue_count_heuristic() {
        let signals = extract("We found 5 issues in the codebase");
        assert!(signals.iter().any(|s| s.id == "heuristic:issue_count" && s.description == "5 issues"));
    }

    #[test]
    fn issue_count_below_threshold() {
        let signals = extract("We found 2 issues in the codebase");
        assert!(!signals.iter().any(|s| s.id == "heuristic:issue_count"), "2 issues should not trigger");
    }

    #[test]
    fn word_boundary_prevents_substring_match() {
        // "tissues" contains "issues" but should NOT trigger issue_count
        let signals = extract("We have 5 tissues on the table");
        assert!(
            !signals.iter().any(|s| s.id == "heuristic:issue_count"),
            "tissues should not match issues"
        );
    }

    #[test]
    fn word_boundary_allows_real_match() {
        let signals = extract("Found 4 issues in production");
        assert!(signals.iter().any(|s| s.id == "heuristic:issue_count"));
    }

    // ─── Corner Cases ────────────────────────────────────

    #[test]
    fn empty_string_returns_no_signals() {
        let signals = extract("");
        assert!(signals.is_empty());
    }

    #[test]
    fn zero_issues_below_threshold() {
        let signals = extract("We found 0 issues");
        assert!(!signals.iter().any(|s| s.id == "heuristic:issue_count"), "0 issues should not trigger");
    }

    #[test]
    fn number_after_word_not_matched() {
        // "issues 5" — number AFTER the word, not before
        let signals = extract("There are issues 5 of them critical");
        assert!(!signals.iter().any(|s| s.id == "heuristic:issue_count"),
            "Number after word should not trigger issue_count");
    }

    #[test]
    fn mixed_case_p0_triggers_severity() {
        let signals = extract("This is a P0 incident");
        assert!(signals.iter().any(|s| s.id == "keyword:severity"), "P0 should trigger severity");
        // Also lowercase
        let signals2 = extract("this is a p0 incident");
        assert!(signals2.iter().any(|s| s.id == "keyword:severity"), "p0 should trigger severity");
    }

    #[test]
    fn no_duplicate_issue_count_signals() {
        // "5 issues and 3 bugs" — should only emit ONE issue_count signal
        let signals = extract("Found 5 issues and 3 bugs in the system");
        let count = signals.iter().filter(|s| s.id == "heuristic:issue_count").count();
        assert_eq!(count, 1, "Should emit exactly one issue_count signal, got {count}");
    }
}
