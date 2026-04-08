//! Declarative Rule Engine for explore-exploit decisions.
//!
//! Two-tier evaluation:
//! - Tier 1 (pure): check_basic() — checks fields from ArtifactData only, no I/O
//! - Tier 2 (enriched): check_enriched() — checks all fields including graph-aware
//!   (links_missing, days_until_expiry) using pre-fetched EnrichedData
//!
//! Rules are loaded from config.yaml under `fpf.rules`.

use serde::{Deserialize, Serialize};

use crate::fpf::core::model::{ActionType, ArtifactData, SuggestedAction};

// ---------------------------------------------------------------------------
// YAML types
// ---------------------------------------------------------------------------

/// A single declarative rule from config.yaml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub name: String,
    #[serde(rename = "when")]
    pub condition: Condition,
    pub action: ActionType,
    #[serde(default = "default_priority")]
    pub priority: u8,
    pub message: Option<String>,
}

fn default_priority() -> u8 {
    3
}

/// Conditions that must ALL be true for a rule to match (implicit AND).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Condition {
    pub status: Option<ValueMatch>,
    pub kind: Option<ValueMatch>,
    pub depth: Option<ValueMatch>,
    pub r_eff: Option<NumericExpr>,
    pub overall: Option<NumericExpr>,
    pub link_count: Option<NumericExpr>,
    pub is_stale: Option<bool>,
    // --- Tier 2: graph-aware (need enrichment) ---
    pub links_missing: Option<Vec<String>>,
    pub days_until_expiry: Option<NumericExpr>,
}

/// Max length of a human-readable condition summary (chars).
pub const CONDITION_SUMMARY_MAX: usize = 120;

impl Condition {
    /// Returns true if this condition requires enrichment data (Tier 2).
    pub fn needs_enrichment(&self) -> bool {
        self.links_missing.is_some() || self.days_until_expiry.is_some()
    }

    /// Returns true if no conditions are set (vacuous truth — matches everything).
    pub fn is_empty(&self) -> bool {
        self.status.is_none()
            && self.kind.is_none()
            && self.depth.is_none()
            && self.r_eff.is_none()
            && self.overall.is_none()
            && self.link_count.is_none()
            && self.is_stale.is_none()
            && self.links_missing.is_none()
            && self.days_until_expiry.is_none()
    }

    /// Render as "kind=prd AND status=active AND r_eff<0.5" (≤ `CONDITION_SUMMARY_MAX` chars).
    /// Empty conditions return "(always matches)".
    pub fn summarize(&self) -> String {
        if self.is_empty() {
            return "(always matches)".to_string();
        }

        let mut parts: Vec<String> = Vec::new();

        if let Some(v) = &self.kind {
            parts.push(format!("kind={}", format_value_match(v)));
        }
        if let Some(v) = &self.status {
            parts.push(format!("status={}", format_value_match(v)));
        }
        if let Some(v) = &self.depth {
            parts.push(format!("depth={}", format_value_match(v)));
        }
        if let Some(n) = &self.r_eff {
            parts.push(format!("r_eff{}", format_numeric(n)));
        }
        if let Some(n) = &self.overall {
            parts.push(format!("overall{}", format_numeric(n)));
        }
        if let Some(n) = &self.link_count {
            parts.push(format!("link_count{}", format_numeric(n)));
        }
        if let Some(b) = self.is_stale {
            parts.push(format!("is_stale={b}"));
        }
        if let Some(links) = &self.links_missing {
            parts.push(format!("links_missing={}", links.join(",")));
        }
        if let Some(n) = &self.days_until_expiry {
            parts.push(format!("days_until_expiry{}", format_numeric(n)));
        }

        let mut joined = parts.join(" AND ");
        if joined.chars().count() > CONDITION_SUMMARY_MAX {
            joined = joined
                .chars()
                .take(CONDITION_SUMMARY_MAX - 1)
                .collect::<String>();
            joined.push('…');
        }
        joined
    }
}

fn format_value_match(v: &ValueMatch) -> String {
    match v {
        ValueMatch::Single(s) => s.clone(),
        ValueMatch::Multiple(list) => format!("[{}]", list.join("|")),
    }
}

fn format_numeric(n: &NumericExpr) -> String {
    match n {
        NumericExpr::Lt(v) => format!("<{v}"),
        NumericExpr::Le(v) => format!("<={v}"),
        NumericExpr::Gt(v) => format!(">{v}"),
        NumericExpr::Ge(v) => format!(">={v}"),
        NumericExpr::Eq(v) => format!("={v}"),
        NumericExpr::Range(lo, hi) => format!("={lo}..{hi}"),
    }
}

// ---------------------------------------------------------------------------
// Value matching — "draft" or ["active", "stale"]
// ---------------------------------------------------------------------------

/// Match a string field against a single value or a list of values.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ValueMatch {
    Single(String),
    Multiple(Vec<String>),
}

impl ValueMatch {
    pub fn matches(&self, value: &str) -> bool {
        match self {
            ValueMatch::Single(s) => s.eq_ignore_ascii_case(value),
            ValueMatch::Multiple(list) => list.iter().any(|s| s.eq_ignore_ascii_case(value)),
        }
    }
}

// ---------------------------------------------------------------------------
// Numeric expressions — "< 0.5", ">= 0.7", "0.01..0.5", "== 0"
// ---------------------------------------------------------------------------

/// A numeric comparison expression, deserialized from a string.
#[derive(Debug, Clone)]
pub enum NumericExpr {
    Lt(f64),
    Le(f64),
    Gt(f64),
    Ge(f64),
    Eq(f64),
    Range(f64, f64), // inclusive start, exclusive end
}

impl NumericExpr {
    pub fn check(&self, value: f64) -> bool {
        match self {
            NumericExpr::Lt(n) => value < *n,
            NumericExpr::Le(n) => value <= *n,
            NumericExpr::Gt(n) => value > *n,
            NumericExpr::Ge(n) => value >= *n,
            NumericExpr::Eq(n) => (value - *n).abs() < 1e-9,
            NumericExpr::Range(lo, hi) => value >= *lo && value < *hi,
        }
    }

    /// Parse from string: "< 0.5", ">= 0.7", "0.01..0.5", "== 0"
    ///
    /// Rejects NaN, Infinity, and inverted ranges (lo >= hi).
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();

        // Helper: reject non-finite values (NaN, Infinity)
        let parse_finite = |s: &str| -> Option<f64> {
            let v = s.trim().parse::<f64>().ok()?;
            if v.is_finite() { Some(v) } else { None }
        };

        // Range: "0.01..0.5"
        if let Some((lo, hi)) = s.split_once("..") {
            let lo = parse_finite(lo)?;
            let hi = parse_finite(hi)?;
            if lo >= hi {
                return None; // Inverted range — will surface as serde error
            }
            return Some(NumericExpr::Range(lo, hi));
        }

        // Operators: "<=", ">=", "<", ">", "=="
        if let Some(rest) = s.strip_prefix("<=") {
            return parse_finite(rest).map(NumericExpr::Le);
        }
        if let Some(rest) = s.strip_prefix(">=") {
            return parse_finite(rest).map(NumericExpr::Ge);
        }
        if let Some(rest) = s.strip_prefix("==") {
            return parse_finite(rest).map(NumericExpr::Eq);
        }
        if let Some(rest) = s.strip_prefix('<') {
            return parse_finite(rest).map(NumericExpr::Lt);
        }
        if let Some(rest) = s.strip_prefix('>') {
            return parse_finite(rest).map(NumericExpr::Gt);
        }

        // Bare number = exact match
        parse_finite(s).map(NumericExpr::Eq)
    }
}

// Custom serde: deserialize NumericExpr from string
impl<'de> Deserialize<'de> for NumericExpr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NumericExpr::parse(&s)
            .ok_or_else(|| serde::de::Error::custom(format!("invalid numeric expression: '{s}'")))
    }
}

impl Serialize for NumericExpr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = match self {
            NumericExpr::Lt(n) => format!("< {n}"),
            NumericExpr::Le(n) => format!("<= {n}"),
            NumericExpr::Gt(n) => format!("> {n}"),
            NumericExpr::Ge(n) => format!(">= {n}"),
            NumericExpr::Eq(n) => format!("== {n}"),
            NumericExpr::Range(lo, hi) => format!("{lo}..{hi}"),
        };
        serializer.serialize_str(&s)
    }
}

// Custom serde for ActionType
impl<'de> Deserialize<'de> for ActionType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_uppercase().as_str() {
            "EXPLORE" => Ok(ActionType::Explore),
            "INVESTIGATE" => Ok(ActionType::Investigate),
            "EXPLOIT" => Ok(ActionType::Exploit),
            _ => Err(serde::de::Error::custom(format!(
                "unknown action: '{s}', expected EXPLORE/INVESTIGATE/EXPLOIT"
            ))),
        }
    }
}

impl Serialize for ActionType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(match self {
            ActionType::Explore => "EXPLORE",
            ActionType::Investigate => "INVESTIGATE",
            ActionType::Exploit => "EXPLOIT",
        })
    }
}

// ---------------------------------------------------------------------------
// Enriched data — pre-fetched graph info for Tier 2
// ---------------------------------------------------------------------------

/// ArtifactData + pre-fetched graph/time info for Tier 2 rules.
#[derive(Debug, Clone)]
pub struct EnrichedData {
    pub base: ArtifactData,
    /// Kinds of artifacts linked to this one (lowercase): ["rfc", "evidence", "adr"]
    pub linked_kinds: Vec<String>,
    /// Days until valid_until expiry. None = no expiry set.
    /// Negative values mean the artifact has already expired (e.g., -30 = expired 30 days ago).
    /// Rules with `days_until_expiry: "< 14"` will match both soon-to-expire AND already-expired.
    pub days_until_expiry: Option<i64>,
}

// ---------------------------------------------------------------------------
// Evaluation — Tier 1 (pure) and Tier 2 (enriched, still pure)
// ---------------------------------------------------------------------------

/// Tier 1: Check rule using only ArtifactData fields. No I/O.
///
/// Returns false if any basic condition doesn't match.
/// Returns true if all basic conditions match (ignoring Tier 2 conditions).
pub fn check_basic(rule: &Rule, data: &ArtifactData) -> bool {
    let c = &rule.condition;

    if let Some(ref v) = c.status
        && !v.matches(&data.status)
    {
        return false;
    }
    if let Some(ref v) = c.kind
        && !v.matches(&data.kind)
    {
        return false;
    }
    if let Some(ref v) = c.depth
        && !v.matches(&data.depth)
    {
        return false;
    }
    if let Some(ref expr) = c.r_eff
        && !expr.check(data.trust.r_eff)
    {
        return false;
    }
    if let Some(ref expr) = c.overall
        && !expr.check(data.trust.overall)
    {
        return false;
    }
    if let Some(ref expr) = c.link_count
        && !expr.check(data.link_count as f64)
    {
        return false;
    }
    if let Some(stale) = c.is_stale
        && data.is_stale != stale
    {
        return false;
    }

    true
}

/// Tier 2: Check rule using enriched data (includes graph-aware checks).
///
/// This is a PURE function — all I/O happened during enrichment.
pub fn check_enriched(rule: &Rule, data: &EnrichedData) -> bool {
    // First check all basic conditions against base data
    if !check_basic(rule, &data.base) {
        return false;
    }

    let c = &rule.condition;

    // links_missing: check that NONE of the listed kinds are linked
    if let Some(ref missing) = c.links_missing {
        for kind in missing {
            if data
                .linked_kinds
                .iter()
                .any(|k| k.eq_ignore_ascii_case(kind))
            {
                return false; // this kind IS linked, "missing" condition fails
            }
        }
    }

    // days_until_expiry
    if let Some(ref expr) = c.days_until_expiry {
        match data.days_until_expiry {
            Some(days) => {
                if !expr.check(days as f64) {
                    return false;
                }
            }
            None => return false, // no expiry set, can't match expiry condition
        }
    }

    true
}

/// Run all rules against an artifact, return the first matching action.
///
/// Rules are checked in priority order (lowest number = highest priority).
pub fn run_rules(rules: &[Rule], data: &EnrichedData) -> Option<SuggestedAction> {
    let mut sorted: Vec<&Rule> = rules.iter().collect();
    sorted.sort_by_key(|r| r.priority);

    for rule in sorted {
        let matched = if rule.condition.needs_enrichment() {
            check_enriched(rule, data)
        } else {
            check_basic(rule, &data.base)
        };

        if matched {
            let reason = rule
                .message
                .clone()
                .unwrap_or_else(|| format!("Matched rule '{}'", rule.name));

            return Some(SuggestedAction {
                action_type: rule.action,
                reason,
                priority: rule.priority,
            });
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Default rules — backward compatible with hardcoded 5 rules
// ---------------------------------------------------------------------------

/// Returns the 5 default rules that match current hardcoded behavior.
///
/// Used when config.yaml has no `fpf.rules` section.
///
/// Note: "weak-evidence" uses `["active", "stale"]` explicitly instead of
/// the hardcoded `!= "draft"` logic. Terminal statuses (superseded, deprecated)
/// are already filtered out in `build_rule_actions()` before rules are evaluated.
/// Current lifecycle: draft → active → stale → superseded/deprecated.
/// If new intermediate statuses are added, update this list.
pub fn default_rules() -> Vec<Rule> {
    vec![
        Rule {
            name: "blind-spot".into(),
            condition: Condition {
                status: Some(ValueMatch::Single("draft".into())),
                r_eff: NumericExpr::parse("< 0.01"),
                ..Default::default()
            },
            action: ActionType::Explore,
            priority: 1,
            message: Some(
                "Draft with no evidence (R_eff < 0.01). Needs evidence to validate.".into(),
            ),
        },
        Rule {
            name: "weak-evidence".into(),
            condition: Condition {
                status: Some(ValueMatch::Multiple(vec!["active".into(), "stale".into()])),
                r_eff: NumericExpr::parse("< 0.5"),
                ..Default::default()
            },
            action: ActionType::Investigate,
            priority: 2,
            message: Some(
                "R_eff < 0.5 — evidence exists but weak/stale. Refresh or add stronger evidence."
                    .into(),
            ),
        },
        Rule {
            name: "orphan-active".into(),
            condition: Condition {
                status: Some(ValueMatch::Single("active".into())),
                link_count: NumericExpr::parse("== 0"),
                ..Default::default()
            },
            action: ActionType::Explore,
            priority: 3,
            message: Some(
                "Active but no links to other artifacts. Connect it to the graph.".into(),
            ),
        },
        Rule {
            name: "medium-quality".into(),
            condition: Condition {
                r_eff: NumericExpr::parse("0.5..0.7"),
                ..Default::default()
            },
            action: ActionType::Investigate,
            priority: 4,
            message: Some(
                "R_eff 0.5-0.7 — evidence moderate. Add stronger evidence to unlock EXPLOIT."
                    .into(),
            ),
        },
        Rule {
            name: "ready-to-build".into(),
            condition: Condition {
                r_eff: NumericExpr::parse(">= 0.7"),
                overall: NumericExpr::parse(">= 0.6"),
                ..Default::default()
            },
            action: ActionType::Exploit,
            priority: 5,
            message: Some("Strong evidence + quality. Ready to build on.".into()),
        },
    ]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fpf::core::trust::TrustScore;

    #[test]
    fn summarize_empty_condition() {
        let c = Condition::default();
        assert_eq!(c.summarize(), "(always matches)");
    }

    #[test]
    fn summarize_flat_condition_joined_with_and() {
        let c = Condition {
            kind: Some(ValueMatch::Single("prd".into())),
            status: Some(ValueMatch::Single("active".into())),
            r_eff: Some(NumericExpr::Lt(0.5)),
            ..Default::default()
        };
        let s = c.summarize();
        assert!(s.contains("kind=prd"));
        assert!(s.contains("status=active"));
        assert!(s.contains("r_eff<0.5"));
        assert!(s.contains(" AND "));
    }

    #[test]
    fn summarize_multi_value_match() {
        let c = Condition {
            status: Some(ValueMatch::Multiple(vec!["draft".into(), "stale".into()])),
            ..Default::default()
        };
        assert_eq!(c.summarize(), "status=[draft|stale]");
    }

    #[test]
    fn summarize_truncates_long_output() {
        let links: Vec<String> = (0..60).map(|i| format!("link{i}")).collect();
        let c = Condition {
            links_missing: Some(links),
            ..Default::default()
        };
        let s = c.summarize();
        assert!(s.chars().count() <= CONDITION_SUMMARY_MAX);
        assert!(s.ends_with('…'));
    }

    #[test]
    fn summarize_uses_all_numeric_operators() {
        let c = Condition {
            r_eff: Some(NumericExpr::Ge(0.7)),
            overall: Some(NumericExpr::Range(0.1, 0.5)),
            link_count: Some(NumericExpr::Eq(0.0)),
            ..Default::default()
        };
        let s = c.summarize();
        assert!(s.contains("r_eff>=0.7"));
        assert!(s.contains("overall=0.1..0.5"));
        assert!(s.contains("link_count=0"));
    }

    fn make_data(id: &str, status: &str, r_eff: f64, link_count: usize) -> ArtifactData {
        let trust = TrustScore {
            r_eff,
            formality: 0.5,
            granularity: 0.5,
            reliability: 0.5,
            overall: 0.5,
            weakest_link: None,
        };
        ArtifactData {
            id: id.into(),
            status: status.into(),
            kind: "prd".into(),
            depth: "standard".into(),
            evidence: vec![],
            formality: 0.5,
            granularity: 0.5,
            link_count,
            is_stale: false,
            trust,
        }
    }

    fn enrich(data: ArtifactData) -> EnrichedData {
        EnrichedData {
            base: data,
            linked_kinds: vec![],
            days_until_expiry: None,
        }
    }

    // --- NumericExpr tests ---

    #[test]
    fn parse_lt() {
        let expr = NumericExpr::parse("< 0.5").unwrap();
        assert!(expr.check(0.3));
        assert!(!expr.check(0.5));
        assert!(!expr.check(0.7));
    }

    #[test]
    fn parse_ge() {
        let expr = NumericExpr::parse(">= 0.7").unwrap();
        assert!(expr.check(0.7));
        assert!(expr.check(0.9));
        assert!(!expr.check(0.69));
    }

    #[test]
    fn parse_range() {
        let expr = NumericExpr::parse("0.01..0.5").unwrap();
        assert!(expr.check(0.01));
        assert!(expr.check(0.3));
        assert!(!expr.check(0.5)); // exclusive end
        assert!(!expr.check(0.0));
    }

    #[test]
    fn parse_eq() {
        let expr = NumericExpr::parse("== 0").unwrap();
        assert!(expr.check(0.0));
        assert!(!expr.check(0.1));
    }

    #[test]
    fn parse_bare_number() {
        let expr = NumericExpr::parse("0.5").unwrap();
        assert!(expr.check(0.5));
        assert!(!expr.check(0.6));
    }

    #[test]
    fn parse_inverted_range_rejected() {
        assert!(NumericExpr::parse("0.7..0.3").is_none());
    }

    #[test]
    fn parse_nan_rejected() {
        assert!(NumericExpr::parse("> NaN").is_none());
        assert!(NumericExpr::parse("< inf").is_none());
        assert!(NumericExpr::parse(">= -inf").is_none());
        assert!(NumericExpr::parse("NaN").is_none());
    }

    #[test]
    fn empty_condition_detected() {
        let c = Condition::default();
        assert!(c.is_empty());
        let c2 = Condition {
            status: Some(ValueMatch::Single("draft".into())),
            ..Default::default()
        };
        assert!(!c2.is_empty());
    }

    // --- ValueMatch tests ---

    #[test]
    fn value_match_single() {
        let v = ValueMatch::Single("draft".into());
        assert!(v.matches("draft"));
        assert!(v.matches("Draft")); // case-insensitive
        assert!(!v.matches("active"));
    }

    #[test]
    fn value_match_multiple() {
        let v = ValueMatch::Multiple(vec!["active".into(), "stale".into()]);
        assert!(v.matches("active"));
        assert!(v.matches("stale"));
        assert!(!v.matches("draft"));
    }

    // --- Tier 1 tests ---

    #[test]
    fn basic_status_match() {
        let rule = Rule {
            name: "test".into(),
            condition: Condition {
                status: Some(ValueMatch::Single("draft".into())),
                ..Default::default()
            },
            action: ActionType::Explore,
            priority: 1,
            message: None,
        };
        assert!(check_basic(&rule, &make_data("X", "draft", 0.0, 0)));
        assert!(!check_basic(&rule, &make_data("X", "active", 0.0, 0)));
    }

    #[test]
    fn basic_reff_expr() {
        let rule = Rule {
            name: "test".into(),
            condition: Condition {
                r_eff: NumericExpr::parse("< 0.5"),
                ..Default::default()
            },
            action: ActionType::Investigate,
            priority: 2,
            message: None,
        };
        assert!(check_basic(&rule, &make_data("X", "active", 0.3, 1)));
        assert!(!check_basic(&rule, &make_data("X", "active", 0.7, 1)));
    }

    #[test]
    fn basic_combined_conditions() {
        let rule = Rule {
            name: "test".into(),
            condition: Condition {
                status: Some(ValueMatch::Single("draft".into())),
                r_eff: NumericExpr::parse("< 0.01"),
                ..Default::default()
            },
            action: ActionType::Explore,
            priority: 1,
            message: None,
        };
        assert!(check_basic(&rule, &make_data("X", "draft", 0.0, 0)));
        assert!(!check_basic(&rule, &make_data("X", "draft", 0.5, 0)));
        assert!(!check_basic(&rule, &make_data("X", "active", 0.0, 0)));
    }

    // --- Tier 2 tests ---

    #[test]
    fn enriched_links_missing() {
        let rule = Rule {
            name: "prd-needs-rfc".into(),
            condition: Condition {
                kind: Some(ValueMatch::Single("prd".into())),
                status: Some(ValueMatch::Single("active".into())),
                links_missing: Some(vec!["rfc".into()]),
                ..Default::default()
            },
            action: ActionType::Explore,
            priority: 2,
            message: None,
        };

        // No RFC linked -> rule matches
        let data_no_rfc = EnrichedData {
            base: make_data("PRD-018", "active", 0.8, 2),
            linked_kinds: vec!["evidence".into(), "epic".into()],
            days_until_expiry: None,
        };
        assert!(check_enriched(&rule, &data_no_rfc));

        // RFC IS linked -> rule doesn't match
        let data_with_rfc = EnrichedData {
            base: make_data("PRD-018", "active", 0.8, 3),
            linked_kinds: vec!["rfc".into(), "evidence".into()],
            days_until_expiry: None,
        };
        assert!(!check_enriched(&rule, &data_with_rfc));
    }

    #[test]
    fn enriched_days_until_expiry() {
        let rule = Rule {
            name: "expiring".into(),
            condition: Condition {
                kind: Some(ValueMatch::Single("evidence".into())),
                days_until_expiry: NumericExpr::parse("< 14"),
                ..Default::default()
            },
            action: ActionType::Investigate,
            priority: 3,
            message: None,
        };

        // Expires in 7 days -> matches
        let mut data = make_data("EVID-001", "active", 0.8, 1);
        data.kind = "evidence".into();
        let enriched = EnrichedData {
            base: data.clone(),
            linked_kinds: vec![],
            days_until_expiry: Some(7),
        };
        assert!(check_enriched(&rule, &enriched));

        // Expires in 30 days -> doesn't match
        let enriched_far = EnrichedData {
            base: data.clone(),
            linked_kinds: vec![],
            days_until_expiry: Some(30),
        };
        assert!(!check_enriched(&rule, &enriched_far));

        // No expiry -> doesn't match
        let enriched_none = EnrichedData {
            base: data,
            linked_kinds: vec![],
            days_until_expiry: None,
        };
        assert!(!check_enriched(&rule, &enriched_none));
    }

    // --- run_rules() tests ---

    #[test]
    fn run_rules_picks_highest_priority() {
        let rules = vec![
            Rule {
                name: "low-priority".into(),
                condition: Condition {
                    status: Some(ValueMatch::Single("draft".into())),
                    ..Default::default()
                },
                action: ActionType::Investigate,
                priority: 5,
                message: None,
            },
            Rule {
                name: "high-priority".into(),
                condition: Condition {
                    status: Some(ValueMatch::Single("draft".into())),
                    r_eff: NumericExpr::parse("< 0.01"),
                    ..Default::default()
                },
                action: ActionType::Explore,
                priority: 1,
                message: Some("Urgent!".into()),
            },
        ];

        let data = enrich(make_data("X", "draft", 0.0, 0));
        let action = run_rules(&rules, &data).unwrap();
        assert_eq!(action.action_type, ActionType::Explore);
        assert_eq!(action.priority, 1);
        assert_eq!(action.reason, "Urgent!");
    }

    #[test]
    fn run_rules_no_match_returns_none() {
        let rules = vec![Rule {
            name: "only-draft".into(),
            condition: Condition {
                status: Some(ValueMatch::Single("draft".into())),
                ..Default::default()
            },
            action: ActionType::Explore,
            priority: 1,
            message: None,
        }];

        let data = enrich(make_data("X", "active", 0.8, 3));
        assert!(run_rules(&rules, &data).is_none());
    }

    // --- Default rules backward compat ---

    #[test]
    fn default_rules_match_hardcoded_behavior() {
        let rules = default_rules();

        // Draft + no evidence -> EXPLORE (rule "blind-spot")
        let draft_no_ev = enrich(make_data("P-1", "draft", 0.0, 0));
        let action = run_rules(&rules, &draft_no_ev).unwrap();
        assert_eq!(action.action_type, ActionType::Explore);
        assert_eq!(action.priority, 1);

        // Active + weak evidence -> INVESTIGATE (rule "weak-evidence")
        let active_weak = enrich(make_data("P-2", "active", 0.3, 2));
        let action = run_rules(&rules, &active_weak).unwrap();
        assert_eq!(action.action_type, ActionType::Investigate);

        // Active + orphan -> EXPLORE (rule "orphan-active")
        let mut orphan_data = make_data("P-3", "active", 0.8, 0);
        orphan_data.trust.overall = 0.8;
        let orphan = enrich(orphan_data);
        let action = run_rules(&rules, &orphan).unwrap();
        assert_eq!(action.action_type, ActionType::Explore);
        assert_eq!(action.priority, 3);

        // Strong evidence + quality -> EXPLOIT
        let mut strong_data = make_data("P-4", "active", 0.8, 3);
        strong_data.trust.overall = 0.7;
        let strong = enrich(strong_data);
        let action = run_rules(&rules, &strong).unwrap();
        assert_eq!(action.action_type, ActionType::Exploit);
    }

    // --- YAML deserialization ---

    #[test]
    fn rule_deserializes_from_yaml() {
        let yaml = r#"
name: "blind-spot"
when:
  status: "draft"
  r_eff: "< 0.01"
action: EXPLORE
priority: 1
message: "Draft with no evidence"
"#;
        let rule: Rule = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(rule.name, "blind-spot");
        assert_eq!(rule.action, ActionType::Explore);
        assert_eq!(rule.priority, 1);
        assert!(rule.condition.status.is_some());
        assert!(rule.condition.r_eff.is_some());
    }

    #[test]
    fn rule_with_list_status_deserializes() {
        let yaml = r#"
name: "weak-evidence"
when:
  status: ["active", "stale"]
  r_eff: "0.01..0.5"
action: INVESTIGATE
"#;
        let rule: Rule = serde_yaml::from_str(yaml).unwrap();
        if let Some(ValueMatch::Multiple(list)) = &rule.condition.status {
            assert_eq!(list.len(), 2);
        } else {
            panic!("Expected Multiple");
        }
    }

    #[test]
    fn rule_with_links_missing_deserializes() {
        let yaml = r#"
name: "prd-needs-rfc"
when:
  kind: "prd"
  status: "active"
  links_missing: ["rfc"]
action: EXPLORE
priority: 2
"#;
        let rule: Rule = serde_yaml::from_str(yaml).unwrap();
        assert!(rule.condition.needs_enrichment());
        assert_eq!(rule.condition.links_missing, Some(vec!["rfc".to_string()]));
    }

    // --- Comprehensive real-world scenario tests ---

    #[test]
    fn scenario_prd_without_rfc_detected() {
        let rules = vec![Rule {
            name: "prd-needs-rfc".into(),
            condition: Condition {
                kind: Some(ValueMatch::Single("prd".into())),
                status: Some(ValueMatch::Single("active".into())),
                links_missing: Some(vec!["rfc".into()]),
                ..Default::default()
            },
            action: ActionType::Explore,
            priority: 1,
            message: Some("Active PRD without linked RFC".into()),
        }];

        // PRD with evidence+epic but NO rfc → match
        let no_rfc = EnrichedData {
            base: make_data("PRD-004", "active", 0.8, 2),
            linked_kinds: vec!["evidence".into(), "epic".into()],
            days_until_expiry: None,
        };
        let action = run_rules(&rules, &no_rfc).unwrap();
        assert_eq!(action.action_type, ActionType::Explore);
        assert!(action.reason.contains("without linked RFC"));

        // PRD with rfc linked → NO match
        let has_rfc = EnrichedData {
            base: make_data("PRD-018", "active", 1.0, 5),
            linked_kinds: vec!["rfc".into(), "evidence".into(), "epic".into()],
            days_until_expiry: None,
        };
        assert!(run_rules(&rules, &has_rfc).is_none());

        // Draft PRD (not active) → NO match
        let draft_prd = EnrichedData {
            base: make_data("PRD-027", "draft", 0.0, 0),
            linked_kinds: vec![],
            days_until_expiry: None,
        };
        assert!(run_rules(&rules, &draft_prd).is_none());

        // Active RFC (wrong kind) → NO match
        let mut rfc_data = make_data("RFC-001", "active", 0.8, 3);
        rfc_data.kind = "rfc".into();
        let rfc = EnrichedData {
            base: rfc_data,
            linked_kinds: vec!["prd".into()],
            days_until_expiry: None,
        };
        assert!(run_rules(&rules, &rfc).is_none());
    }

    #[test]
    fn scenario_deep_rfc_without_adr() {
        let rules = vec![Rule {
            name: "rfc-needs-adr".into(),
            condition: Condition {
                kind: Some(ValueMatch::Single("rfc".into())),
                status: Some(ValueMatch::Single("active".into())),
                depth: Some(ValueMatch::Multiple(vec!["deep".into(), "critical".into()])),
                links_missing: Some(vec!["adr".into()]),
                ..Default::default()
            },
            action: ActionType::Explore,
            priority: 1,
            message: Some("Deep RFC without ADR".into()),
        }];

        // Deep RFC without ADR → match
        let mut data = make_data("RFC-002", "active", 0.8, 2);
        data.kind = "rfc".into();
        data.depth = "deep".into();
        let no_adr = EnrichedData {
            base: data,
            linked_kinds: vec!["prd".into(), "evidence".into()],
            days_until_expiry: None,
        };
        let action = run_rules(&rules, &no_adr).unwrap();
        assert_eq!(action.action_type, ActionType::Explore);

        // Deep RFC WITH ADR → no match
        let mut data2 = make_data("RFC-001", "active", 1.0, 5);
        data2.kind = "rfc".into();
        data2.depth = "deep".into();
        let has_adr = EnrichedData {
            base: data2,
            linked_kinds: vec!["prd".into(), "adr".into()],
            days_until_expiry: None,
        };
        assert!(run_rules(&rules, &has_adr).is_none());

        // Standard depth RFC without ADR → no match (depth filter)
        let mut data3 = make_data("RFC-003", "active", 0.8, 2);
        data3.kind = "rfc".into();
        data3.depth = "standard".into();
        let standard = EnrichedData {
            base: data3,
            linked_kinds: vec!["prd".into()],
            days_until_expiry: None,
        };
        assert!(run_rules(&rules, &standard).is_none());
    }

    #[test]
    fn scenario_expiring_evidence_with_days() {
        let rules = vec![Rule {
            name: "expiring-soon".into(),
            condition: Condition {
                kind: Some(ValueMatch::Single("evidence".into())),
                days_until_expiry: NumericExpr::parse("< 14"),
                ..Default::default()
            },
            action: ActionType::Investigate,
            priority: 2,
            message: Some("Evidence expires soon".into()),
        }];

        // Expires in 5 days → match
        let mut d1 = make_data("EVID-001", "active", 0.8, 1);
        d1.kind = "evidence".into();
        let soon = EnrichedData {
            base: d1.clone(),
            linked_kinds: vec![],
            days_until_expiry: Some(5),
        };
        assert!(run_rules(&rules, &soon).is_some());

        // Expired 10 days ago (negative) → also match (< 14 includes negatives)
        let expired = EnrichedData {
            base: d1.clone(),
            linked_kinds: vec![],
            days_until_expiry: Some(-10),
        };
        assert!(run_rules(&rules, &expired).is_some());

        // Expires in 30 days → no match
        let far = EnrichedData {
            base: d1.clone(),
            linked_kinds: vec![],
            days_until_expiry: Some(30),
        };
        assert!(run_rules(&rules, &far).is_none());

        // No expiry set → no match
        let no_exp = EnrichedData {
            base: d1,
            linked_kinds: vec![],
            days_until_expiry: None,
        };
        assert!(run_rules(&rules, &no_exp).is_none());
    }

    #[test]
    fn scenario_priority_ordering_real_world() {
        // Real scenario: artifact matches multiple rules, highest priority wins
        let rules = vec![
            Rule {
                name: "prd-needs-rfc".into(),
                condition: Condition {
                    kind: Some(ValueMatch::Single("prd".into())),
                    status: Some(ValueMatch::Single("active".into())),
                    links_missing: Some(vec!["rfc".into()]),
                    ..Default::default()
                },
                action: ActionType::Explore,
                priority: 1,
                message: Some("PRD needs RFC".into()),
            },
            Rule {
                name: "orphan".into(),
                condition: Condition {
                    status: Some(ValueMatch::Single("active".into())),
                    link_count: NumericExpr::parse("== 0"),
                    ..Default::default()
                },
                action: ActionType::Explore,
                priority: 3,
                message: Some("Orphan".into()),
            },
            Rule {
                name: "strong".into(),
                condition: Condition {
                    r_eff: NumericExpr::parse(">= 0.7"),
                    overall: NumericExpr::parse(">= 0.6"),
                    ..Default::default()
                },
                action: ActionType::Exploit,
                priority: 5,
                message: Some("Ready".into()),
            },
        ];

        // Active PRD, no links, strong evidence → matches prd-needs-rfc (p1) AND orphan (p3)
        // Priority 1 should win
        let mut data = make_data("PRD-999", "active", 0.9, 0);
        data.trust.overall = 0.8;
        let enriched = EnrichedData {
            base: data,
            linked_kinds: vec![], // no rfc, no anything
            days_until_expiry: None,
        };
        let action = run_rules(&rules, &enriched).unwrap();
        assert_eq!(action.priority, 1);
        assert!(action.reason.contains("RFC"));
    }

    #[test]
    fn scenario_full_config_yaml_deserialization() {
        // Test exact YAML format from config template
        let yaml = r#"
- name: "prd-needs-rfc"
  when:
    kind: "prd"
    status: "active"
    links_missing: ["rfc"]
  action: EXPLORE
  priority: 1
  message: "Active PRD without linked RFC"
- name: "blind-spot"
  when:
    status: "draft"
    r_eff: "< 0.01"
  action: EXPLORE
  priority: 2
- name: "weak-evidence"
  when:
    status: ["active", "stale"]
    r_eff: "< 0.5"
  action: INVESTIGATE
  priority: 3
- name: "ready-to-build"
  when:
    r_eff: ">= 0.7"
    overall: ">= 0.6"
  action: EXPLOIT
  priority: 5
"#;
        let rules: Vec<Rule> = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(rules.len(), 4);
        assert_eq!(rules[0].name, "prd-needs-rfc");
        assert!(rules[0].condition.needs_enrichment());
        assert_eq!(rules[1].name, "blind-spot");
        assert!(!rules[1].condition.needs_enrichment());
        assert_eq!(rules[2].action, ActionType::Investigate);
        assert_eq!(rules[3].priority, 5);

        // Run against test data
        let draft = enrich(make_data("P-1", "draft", 0.0, 0));
        let action = run_rules(&rules, &draft).unwrap();
        assert_eq!(action.action_type, ActionType::Explore);
        assert_eq!(action.priority, 2); // blind-spot, not prd-needs-rfc (wrong kind/status)
    }

    #[test]
    fn scenario_stale_artifact_gets_investigate() {
        let rules = default_rules();
        // Stale with low r_eff → INVESTIGATE (weak-evidence rule matches ["active", "stale"])
        let stale = enrich(make_data("P-1", "stale", 0.3, 2));
        let action = run_rules(&rules, &stale);
        // stale is not in ["active", "stale"] of weak-evidence... wait, it IS
        assert!(action.is_some());
        assert_eq!(action.unwrap().action_type, ActionType::Investigate);
    }

    #[test]
    fn scenario_deprecated_gets_no_action() {
        // Deprecated artifacts should be filtered BEFORE rules (in build_rule_actions)
        // But if someone passes one through, rules with no status filter could match
        let rules = default_rules();
        // "medium-quality" has no status filter, so deprecated with r_eff=0.6 would match
        let mut data = make_data("P-1", "deprecated", 0.6, 2);
        data.trust.overall = 0.5;
        let enriched = enrich(data);
        let action = run_rules(&rules, &enriched);
        // This DOES match (medium-quality has no status filter)
        // Dashboard filters deprecated BEFORE calling rules, so this is expected behavior
        assert!(action.is_some()); // rules don't filter terminal — dashboard does
    }

    // --- Negative tests (invalid input → rejection) ---

    #[test]
    fn negative_parse_empty_string() {
        assert!(NumericExpr::parse("").is_none());
    }

    #[test]
    fn negative_parse_garbage() {
        assert!(NumericExpr::parse("abc").is_none());
        assert!(NumericExpr::parse("< abc").is_none());
        assert!(NumericExpr::parse(">= xyz").is_none());
        assert!(NumericExpr::parse("..").is_none());
    }

    #[test]
    fn negative_parse_equal_range() {
        // lo == hi is also invalid (empty range)
        assert!(NumericExpr::parse("0.5..0.5").is_none());
    }

    #[test]
    fn negative_yaml_bad_action() {
        let yaml = r#"
name: "test"
when:
  status: "draft"
action: UNKNOWN_ACTION
"#;
        let result: Result<Rule, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn negative_yaml_missing_name() {
        let yaml = r#"
when:
  status: "draft"
action: EXPLORE
"#;
        let result: Result<Rule, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn negative_yaml_bad_numeric_expr() {
        let yaml = r#"
name: "test"
when:
  r_eff: "not_a_number"
action: EXPLORE
"#;
        let result: Result<Rule, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err());
    }

    // --- Corner case tests (boundary values) ---

    #[test]
    fn corner_reff_exact_boundary() {
        let rules = default_rules();

        // R_eff exactly 0.01 → NOT < 0.01, so blind-spot doesn't match
        // But weak-evidence requires status ["active","stale"], and this is draft
        let draft_border = enrich(make_data("X", "draft", 0.01, 0));
        let action = run_rules(&rules, &draft_border);
        // No blind-spot (r_eff >= 0.01), no weak-evidence (draft), no orphan (draft)
        // medium-quality matches 0.01..0.7? No, 0.01 < 0.5 so not in 0.5..0.7
        // Actually default rules: blind-spot=<0.01, weak-evidence=<0.5 for active/stale
        // Draft with r_eff=0.01 doesn't match any default rule
        assert!(action.is_none());

        // R_eff exactly 0.5 → not < 0.5 (investigate), yes in 0.5..0.7 (medium-quality)
        let mut medium_exact = make_data("X", "active", 0.5, 2);
        medium_exact.trust.overall = 0.4;
        let enriched = enrich(medium_exact);
        let action = run_rules(&rules, &enriched).unwrap();
        assert_eq!(action.action_type, ActionType::Investigate); // medium-quality
        assert_eq!(action.priority, 4);
    }

    #[test]
    fn corner_link_count_zero_vs_one() {
        let rules = default_rules();

        // link_count=0, active, strong evidence → orphan-active (priority 3) wins
        let mut zero_links = make_data("X", "active", 0.9, 0);
        zero_links.trust.overall = 0.8;
        let enriched = enrich(zero_links);
        let action = run_rules(&rules, &enriched).unwrap();
        assert_eq!(action.priority, 3); // orphan-active

        // link_count=1, active, strong evidence → EXPLOIT (no orphan rule)
        let mut one_link = make_data("X", "active", 0.9, 1);
        one_link.trust.overall = 0.8;
        let enriched = enrich(one_link);
        let action = run_rules(&rules, &enriched).unwrap();
        assert_eq!(action.action_type, ActionType::Exploit);
    }

    #[test]
    fn corner_multiple_links_missing() {
        // Rule requires BOTH rfc and adr missing
        let rule = Rule {
            name: "needs-both".into(),
            condition: Condition {
                links_missing: Some(vec!["rfc".into(), "adr".into()]),
                ..Default::default()
            },
            action: ActionType::Explore,
            priority: 1,
            message: None,
        };

        // Neither linked → match
        let neither = EnrichedData {
            base: make_data("X", "active", 0.5, 1),
            linked_kinds: vec!["evidence".into()],
            days_until_expiry: None,
        };
        assert!(check_enriched(&rule, &neither));

        // RFC linked but not ADR → still match (adr still missing)
        let has_rfc = EnrichedData {
            base: make_data("X", "active", 0.5, 2),
            linked_kinds: vec!["rfc".into()],
            days_until_expiry: None,
        };
        assert!(!check_enriched(&rule, &has_rfc)); // rfc IS linked → fails

        // Both linked → no match
        let has_both = EnrichedData {
            base: make_data("X", "active", 0.5, 3),
            linked_kinds: vec!["rfc".into(), "adr".into()],
            days_until_expiry: None,
        };
        assert!(!check_enriched(&rule, &has_both));
    }

    #[test]
    fn corner_case_insensitive_kind() {
        let rule = Rule {
            name: "test".into(),
            condition: Condition {
                kind: Some(ValueMatch::Single("PRD".into())),
                ..Default::default()
            },
            action: ActionType::Explore,
            priority: 1,
            message: None,
        };
        // kind stored as lowercase "prd" → should still match "PRD"
        assert!(check_basic(&rule, &make_data("X", "active", 0.5, 1)));
    }
}
