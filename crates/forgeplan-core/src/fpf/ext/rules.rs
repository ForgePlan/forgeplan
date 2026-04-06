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

impl Condition {
    /// Returns true if this condition requires enrichment data (Tier 2).
    pub fn needs_enrichment(&self) -> bool {
        self.links_missing.is_some() || self.days_until_expiry.is_some()
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
            NumericExpr::Eq(n) => (value - *n).abs() < f64::EPSILON,
            NumericExpr::Range(lo, hi) => value >= *lo && value < *hi,
        }
    }

    /// Parse from string: "< 0.5", ">= 0.7", "0.01..0.5", "== 0"
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();

        // Range: "0.01..0.5"
        if let Some((lo, hi)) = s.split_once("..") {
            let lo = lo.trim().parse::<f64>().ok()?;
            let hi = hi.trim().parse::<f64>().ok()?;
            return Some(NumericExpr::Range(lo, hi));
        }

        // Operators: "<=", ">=", "<", ">", "=="
        if let Some(rest) = s.strip_prefix("<=") {
            return rest.trim().parse().ok().map(NumericExpr::Le);
        }
        if let Some(rest) = s.strip_prefix(">=") {
            return rest.trim().parse().ok().map(NumericExpr::Ge);
        }
        if let Some(rest) = s.strip_prefix("==") {
            return rest.trim().parse().ok().map(NumericExpr::Eq);
        }
        if let Some(rest) = s.strip_prefix('<') {
            return rest.trim().parse().ok().map(NumericExpr::Lt);
        }
        if let Some(rest) = s.strip_prefix('>') {
            return rest.trim().parse().ok().map(NumericExpr::Gt);
        }

        // Bare number = exact match
        s.parse().ok().map(NumericExpr::Eq)
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

    if let Some(ref v) = c.status {
        if !v.matches(&data.status) {
            return false;
        }
    }
    if let Some(ref v) = c.kind {
        if !v.matches(&data.kind) {
            return false;
        }
    }
    if let Some(ref v) = c.depth {
        if !v.matches(&data.depth) {
            return false;
        }
    }
    if let Some(ref expr) = c.r_eff {
        if !expr.check(data.trust.r_eff) {
            return false;
        }
    }
    if let Some(ref expr) = c.overall {
        if !expr.check(data.trust.overall) {
            return false;
        }
    }
    if let Some(ref expr) = c.link_count {
        if !expr.check(data.link_count as f64) {
            return false;
        }
    }
    if let Some(stale) = c.is_stale {
        if data.is_stale != stale {
            return false;
        }
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
}
