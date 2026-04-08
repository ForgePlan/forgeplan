//! Routing Skills Memory (PRD-040 FR-001).
//!
//! Adaptive routing — the router learns from past decisions. Successful
//! routing + activation flows are captured as `RoutingSkill` entries.
//! On a new `forgeplan route`, the engine checks skills before keyword
//! rules: if a pattern matches an established skill with high success
//! rate, the skill's recommendation wins.
//!
//! # Design
//!
//! - Skills stored as Memory artifacts (`.forgeplan/memory/`, kind=memory)
//!   — no new schema, reuses existing memory infrastructure.
//! - Skill pattern = substring tokens matched case-insensitively against
//!   the task description (simple but works for v1).
//! - Decay: skills with `last_used` older than 90 days get their
//!   `usage_count` halved on weight computation (soft decay).
//! - Max influence: a skill can override keyword routing only if its
//!   confidence (success_rate × min(usage_count/3, 1.0)) ≥ 0.6.
//!
//! Inspired by RuVector `agenticdb.rs::Skill` — adapted as simple
//! memory artifacts, not an RL pipeline.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::artifact::types::Mode;

/// A routing skill — a learned pattern from past routing decisions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoutingSkill {
    /// Normalized pattern string, e.g. "bugfix frontmatter parser".
    /// Whitespace-separated tokens matched case-insensitively.
    pub pattern: String,
    /// Recommended depth for this pattern.
    pub recommended_depth: Mode,
    /// How many times this skill was applied and confirmed successful.
    pub usage_count: u32,
    /// Success rate in [0.0, 1.0] — fraction of usages that led to
    /// activated artifacts or user confirmation.
    pub success_rate: f64,
    /// When the skill was last applied (for decay).
    pub last_used: DateTime<Utc>,
    /// When the skill was first created.
    pub created_at: DateTime<Utc>,
}

impl RoutingSkill {
    /// Create a new skill from an initial successful routing decision.
    pub fn new(pattern: impl Into<String>, depth: Mode) -> Self {
        let now = Utc::now();
        Self {
            pattern: normalize_pattern(&pattern.into()),
            recommended_depth: depth,
            usage_count: 1,
            success_rate: 1.0,
            last_used: now,
            created_at: now,
        }
    }

    /// Record a successful application of this skill.
    pub fn record_success(&mut self) {
        let total = self.usage_count as f64;
        let new_total = total + 1.0;
        // Running mean: (prev_rate * prev_total + 1.0) / new_total
        self.success_rate = (self.success_rate * total + 1.0) / new_total;
        self.usage_count += 1;
        self.last_used = Utc::now();
    }

    /// Record a failed application (user overrode the skill).
    pub fn record_failure(&mut self) {
        let total = self.usage_count as f64;
        let new_total = total + 1.0;
        self.success_rate = (self.success_rate * total) / new_total;
        self.usage_count += 1;
        self.last_used = Utc::now();
    }

    /// Confidence in [0.0, 1.0] — combined success rate and usage count.
    /// Applies decay for stale skills.
    pub fn confidence(&self) -> f64 {
        let decay = decay_factor(self.last_used);
        let usage_weight = (self.usage_count as f64 / 3.0).min(1.0);
        (self.success_rate * usage_weight * decay).clamp(0.0, 1.0)
    }

    /// Check if task description matches this skill's pattern.
    /// Matching rule: every token in `self.pattern` must appear as a
    /// substring (case-insensitive) in the task description.
    pub fn matches(&self, task_description: &str) -> bool {
        let task_lower = task_description.to_lowercase();
        self.pattern
            .split_whitespace()
            .all(|token| task_lower.contains(token))
    }

    /// Whether this skill has enough confidence to override keyword rules.
    pub fn should_override(&self) -> bool {
        self.confidence() >= 0.6
    }

    /// Serialize skill to JSON (for storage in memory artifact body).
    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Deserialize from JSON (from memory artifact body).
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(json)?)
    }
}

/// Normalize a pattern — lowercase, trim, collapse whitespace.
fn normalize_pattern(raw: &str) -> String {
    raw.to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Decay factor: 1.0 if recent, linearly decreases to 0.5 over 90 days.
/// After 180 days → 0.25. Never exactly zero (keeps historical signal).
fn decay_factor(last_used: DateTime<Utc>) -> f64 {
    let age = Utc::now() - last_used;
    let days = age.num_days() as f64;
    if days <= 0.0 {
        return 1.0;
    }
    // Exponential decay with half-life of 90 days.
    (0.5_f64).powf(days / 90.0).max(0.1)
}

/// Find the best matching skill for a task description.
/// Returns None if no skill matches or confidence is insufficient.
pub fn best_matching_skill<'a>(
    skills: &'a [RoutingSkill],
    task_description: &str,
) -> Option<&'a RoutingSkill> {
    skills
        .iter()
        .filter(|s| s.matches(task_description))
        .filter(|s| s.should_override())
        .max_by(|a, b| {
            a.confidence()
                .partial_cmp(&b.confidence())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

/// How old a skill can be before it's considered dormant (used for reporting).
pub fn dormant_threshold() -> Duration {
    Duration::days(180)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk(pattern: &str, depth: Mode, uses: u32, rate: f64) -> RoutingSkill {
        let now = Utc::now();
        RoutingSkill {
            pattern: normalize_pattern(pattern),
            recommended_depth: depth,
            usage_count: uses,
            success_rate: rate,
            last_used: now,
            created_at: now,
        }
    }

    #[test]
    fn normalize_pattern_lowercase_collapse() {
        assert_eq!(normalize_pattern("  BugFix  Parser "), "bugfix parser");
        assert_eq!(normalize_pattern("Fix\tthe   thing"), "fix the thing");
    }

    #[test]
    fn new_skill_has_defaults() {
        let s = RoutingSkill::new("bugfix parser", Mode::Tactical);
        assert_eq!(s.pattern, "bugfix parser");
        assert_eq!(s.usage_count, 1);
        assert_eq!(s.success_rate, 1.0);
    }

    #[test]
    fn matches_all_tokens_required() {
        let s = mk("bugfix parser", Mode::Tactical, 3, 1.0);
        // All tokens must appear — this has both "bugfix" and "parser"
        assert!(s.matches("BUGFIX in parser code"));
        assert!(s.matches("fix a bugfix in the parser"));
        // Missing "parser"
        assert!(!s.matches("bugfix only"));
        // Missing "bugfix"
        assert!(!s.matches("parser updates only"));
        // Empty description fails non-empty pattern
        assert!(!s.matches(""));
    }

    #[test]
    fn record_success_updates_stats() {
        let mut s = RoutingSkill::new("bugfix", Mode::Tactical);
        s.success_rate = 0.5;
        s.usage_count = 2;
        s.record_success();
        assert_eq!(s.usage_count, 3);
        // (0.5 * 2 + 1.0) / 3 = 0.666...
        assert!((s.success_rate - 0.6666).abs() < 0.01);
    }

    #[test]
    fn record_failure_lowers_rate() {
        let mut s = RoutingSkill::new("bugfix", Mode::Tactical);
        s.success_rate = 1.0;
        s.usage_count = 2;
        s.record_failure();
        // (1.0 * 2 + 0) / 3 = 0.666...
        assert!((s.success_rate - 0.6666).abs() < 0.01);
    }

    #[test]
    fn confidence_requires_usage_count() {
        // 1 use × 1.0 success × no decay = 1.0 / 3.0 = 0.333
        let s1 = mk("x", Mode::Tactical, 1, 1.0);
        assert!(s1.confidence() < 0.5);
        assert!(!s1.should_override());

        // 3 uses × 1.0 success = 1.0
        let s3 = mk("x", Mode::Tactical, 3, 1.0);
        assert!((s3.confidence() - 1.0).abs() < 0.01);
        assert!(s3.should_override());
    }

    #[test]
    fn confidence_low_success_rate_insufficient() {
        // 5 uses × 0.5 success = 0.5 → below 0.6 threshold
        let s = mk("x", Mode::Tactical, 5, 0.5);
        assert!(!s.should_override());
    }

    #[test]
    fn best_matching_picks_highest_confidence() {
        let skills = vec![
            mk("bugfix", Mode::Tactical, 5, 0.8),        // conf 0.8
            mk("bugfix parser", Mode::Tactical, 5, 1.0), // conf 1.0
        ];
        let best = best_matching_skill(&skills, "bugfix in parser").unwrap();
        assert_eq!(best.pattern, "bugfix parser");
    }

    #[test]
    fn no_match_returns_none() {
        let skills = vec![mk("bugfix", Mode::Tactical, 5, 1.0)];
        assert!(best_matching_skill(&skills, "add new feature").is_none());
    }

    #[test]
    fn low_confidence_skill_does_not_override() {
        let skills = vec![mk("bugfix", Mode::Tactical, 1, 0.5)];
        // usage_weight = 1/3 = 0.333, success = 0.5 → conf ~0.166
        assert!(best_matching_skill(&skills, "bugfix").is_none());
    }

    #[test]
    fn decay_factor_recent_is_one() {
        let now = Utc::now();
        assert!((decay_factor(now) - 1.0).abs() < 0.01);
    }

    #[test]
    fn decay_factor_stale_skill() {
        let old = Utc::now() - Duration::days(90);
        let factor = decay_factor(old);
        // Half-life = 90 days → factor ≈ 0.5
        assert!((factor - 0.5).abs() < 0.05);
    }

    #[test]
    fn to_json_from_json_roundtrip() {
        let original = mk("bugfix parser", Mode::Tactical, 5, 0.8);
        let json = original.to_json().unwrap();
        let restored = RoutingSkill::from_json(&json).unwrap();
        assert_eq!(original.pattern, restored.pattern);
        assert_eq!(original.recommended_depth, restored.recommended_depth);
        assert_eq!(original.usage_count, restored.usage_count);
        assert_eq!(original.success_rate, restored.success_rate);
    }

    // ── Corner cases ───────────────────────────────────────────

    #[test]
    fn empty_pattern_matches_everything() {
        let s = mk("", Mode::Tactical, 3, 1.0);
        // Empty split_whitespace().all() is vacuously true
        assert!(s.matches("anything"));
    }

    #[test]
    fn empty_description_matches_empty_pattern() {
        let s = mk("", Mode::Tactical, 3, 1.0);
        assert!(s.matches(""));
    }

    #[test]
    fn empty_description_fails_non_empty_pattern() {
        let s = mk("bugfix", Mode::Tactical, 3, 1.0);
        assert!(!s.matches(""));
    }

    #[test]
    fn very_stale_skill_decayed_below_override() {
        let mut s = RoutingSkill::new("bugfix", Mode::Tactical);
        s.usage_count = 10;
        s.success_rate = 1.0;
        s.last_used = Utc::now() - Duration::days(360); // ~0.063 factor
        // conf = 1.0 × 1.0 × 0.063 ≈ 0.063 → below 0.6
        assert!(!s.should_override());
    }

    #[test]
    fn confidence_clamps_to_one() {
        // Pathological input — already clamped by construction but test it
        let s = mk("x", Mode::Tactical, 1000, 10.0); // invalid success_rate
        assert!(s.confidence() <= 1.0);
    }

    #[test]
    fn malformed_json_returns_error() {
        assert!(RoutingSkill::from_json("not json").is_err());
        assert!(RoutingSkill::from_json("{}").is_err()); // missing fields
    }
}
