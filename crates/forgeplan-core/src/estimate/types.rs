use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

/// Developer grade — determines the time multiplier for effort estimation.
/// Senior is the baseline (×1.0). All other grades are relative to Senior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Grade {
    Junior,
    Middle,
    #[default]
    Senior,
    Principal,
    Ai,
}

impl Grade {
    /// Default multiplier relative to Senior baseline.
    /// Source: user's Excel model (Fibonacci × multiplier = hours).
    pub fn default_multiplier(&self) -> f64 {
        match self {
            Grade::Junior => 2.0,
            Grade::Middle => 1.5,
            Grade::Senior => 1.0,
            Grade::Principal => 0.7,
            Grade::Ai => 0.4,
        }
    }

    /// All grades in display order.
    pub fn all() -> &'static [Grade] {
        &[
            Grade::Junior,
            Grade::Middle,
            Grade::Senior,
            Grade::Principal,
            Grade::Ai,
        ]
    }
}

impl fmt::Display for Grade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Grade::Junior => write!(f, "Junior"),
            Grade::Middle => write!(f, "Middle"),
            Grade::Senior => write!(f, "Senior"),
            Grade::Principal => write!(f, "Principal"),
            Grade::Ai => write!(f, "AI"),
        }
    }
}

impl std::str::FromStr for Grade {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "junior" | "jun" => Ok(Grade::Junior),
            "middle" | "mid" => Ok(Grade::Middle),
            "senior" | "sen" => Ok(Grade::Senior),
            "principal" | "ps" | "principal_senior" => Ok(Grade::Principal),
            "ai" => Ok(Grade::Ai),
            other => Err(format!("Unknown grade: '{}'. Valid: junior, middle, senior, principal, ai", other)),
        }
    }
}

/// Fibonacci complexity scale — maps to base Senior hours.
/// Each level roughly doubles the previous in effort.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Complexity {
    Trivial = 1,
    Simple = 2,
    Medium = 3,
    Complex = 5,
    Hard = 8,
    Epic = 13,
}

impl Complexity {
    /// Base Senior hours for this complexity level.
    /// Derived from user's empirical data: Complexity × ~2.6 ≈ Senior hours.
    pub fn base_senior_hours(&self) -> f64 {
        match self {
            Complexity::Trivial => 3.0,
            Complexity::Simple => 5.0,
            Complexity::Medium => 8.0,
            Complexity::Complex => 13.0,
            Complexity::Hard => 21.0,
            Complexity::Epic => 34.0,
        }
    }

    /// Fibonacci value for scoring (Complexity × Senior = Score).
    pub fn value(&self) -> u32 {
        *self as u32
    }

    /// Parse from numeric Fibonacci value.
    pub fn from_value(v: u32) -> Option<Self> {
        match v {
            1 => Some(Complexity::Trivial),
            2 => Some(Complexity::Simple),
            3 => Some(Complexity::Medium),
            5 => Some(Complexity::Complex),
            8 => Some(Complexity::Hard),
            13 => Some(Complexity::Epic),
            _ => None,
        }
    }
}

impl fmt::Display for Complexity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value())
    }
}

/// Task type — determines the AI-specific multiplier.
/// Coding tasks benefit most from AI; coordination tasks don't.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    PureCoding,
    CodingInfra,
    DesignCoding,
    PureInfra,
    Coordination,
}

impl TaskType {
    /// AI-specific multiplier (applied on top of Grade::Ai multiplier).
    /// Represents how much faster AI is vs human for this task type.
    pub fn ai_multiplier(&self) -> f64 {
        match self {
            TaskType::PureCoding => 0.10,
            TaskType::CodingInfra => 0.25,
            TaskType::DesignCoding => 0.30,
            TaskType::PureInfra => 0.50,
            TaskType::Coordination => 1.00,
        }
    }
}

impl fmt::Display for TaskType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskType::PureCoding => write!(f, "coding"),
            TaskType::CodingInfra => write!(f, "coding+infra"),
            TaskType::DesignCoding => write!(f, "design+coding"),
            TaskType::PureInfra => write!(f, "infra"),
            TaskType::Coordination => write!(f, "coordination"),
        }
    }
}

/// Source of a work item — allows typed filtering instead of string prefix checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ItemSource {
    /// Functional Requirement from PRD table
    Fr,
    /// Phase checklist item from RFC
    Phase,
}

/// A single work item extracted from an artifact (FR from PRD or Phase step from RFC).
#[derive(Debug, Clone, Serialize)]
pub struct WorkItem {
    pub id: String,
    pub description: String,
    pub category: String,
    pub priority: String,
    pub source: ItemSource,
}

/// A work item with assigned complexity and task type.
#[derive(Debug, Clone, Serialize)]
pub struct ScoredItem {
    pub id: String,
    pub description: String,
    pub complexity: Complexity,
    pub task_type: TaskType,
}

/// Hours estimate for a single item across all grades.
#[derive(Debug, Clone, Serialize)]
pub struct EstimateItem {
    pub id: String,
    pub description: String,
    pub complexity: Complexity,
    pub task_type: TaskType,
    /// Hours per grade: grade → hours
    pub hours: HashMap<Grade, f64>,
    /// Score = complexity.value() × senior_hours (for prioritization)
    pub score: f64,
}

/// Complete estimate result for an artifact.
#[derive(Debug, Clone, Serialize)]
pub struct EstimateResult {
    pub artifact_id: String,
    pub artifact_title: String,
    pub items: Vec<EstimateItem>,
    pub totals: HashMap<Grade, f64>,
    pub total_score: f64,
    pub confidence: f64,
    pub confidence_reasons: Vec<String>,
}

/// Per-domain grade mapping for a user.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GradeProfile {
    /// domain → grade (e.g., "backend" → Middle)
    #[serde(default)]
    pub domains: HashMap<String, Grade>,
    /// default grade when domain not mapped
    #[serde(default)]
    pub default_grade: Grade,
}

/// Configuration for the estimate engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstimateConfig {
    #[serde(default = "default_grade_multipliers")]
    pub grade_multipliers: HashMap<Grade, f64>,
    #[serde(default = "default_ai_task_multipliers")]
    pub ai_task_multipliers: HashMap<TaskType, f64>,
    #[serde(default = "default_review_overhead")]
    pub review_overhead: f64,
    #[serde(default = "default_safety_margin")]
    pub safety_margin: f64,
    #[serde(default)]
    pub grade_profile: GradeProfile,
}

impl Default for EstimateConfig {
    fn default() -> Self {
        Self {
            grade_multipliers: default_grade_multipliers(),
            ai_task_multipliers: default_ai_task_multipliers(),
            review_overhead: default_review_overhead(),
            safety_margin: default_safety_margin(),
            grade_profile: GradeProfile::default(),
        }
    }
}

fn default_grade_multipliers() -> HashMap<Grade, f64> {
    Grade::all().iter().map(|g| (*g, g.default_multiplier())).collect()
}

fn default_ai_task_multipliers() -> HashMap<TaskType, f64> {
    [
        (TaskType::PureCoding, 0.10),
        (TaskType::CodingInfra, 0.25),
        (TaskType::DesignCoding, 0.30),
        (TaskType::PureInfra, 0.50),
        (TaskType::Coordination, 1.00),
    ]
    .into_iter()
    .collect()
}

fn default_review_overhead() -> f64 {
    0.30
}

fn default_safety_margin() -> f64 {
    0.50
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grade_multipliers_ordered() {
        assert!(Grade::Junior.default_multiplier() > Grade::Middle.default_multiplier());
        assert!(Grade::Middle.default_multiplier() > Grade::Senior.default_multiplier());
        assert!(Grade::Senior.default_multiplier() > Grade::Principal.default_multiplier());
        assert!(Grade::Principal.default_multiplier() > Grade::Ai.default_multiplier());
    }

    #[test]
    fn grade_parse_roundtrip() {
        for g in Grade::all() {
            let s = g.to_string().to_lowercase();
            let parsed: Grade = s.parse().unwrap();
            assert_eq!(*g, parsed);
        }
    }

    #[test]
    fn grade_parse_aliases() {
        assert_eq!("jun".parse::<Grade>().unwrap(), Grade::Junior);
        assert_eq!("mid".parse::<Grade>().unwrap(), Grade::Middle);
        assert_eq!("sen".parse::<Grade>().unwrap(), Grade::Senior);
        assert_eq!("ps".parse::<Grade>().unwrap(), Grade::Principal);
    }

    #[test]
    fn grade_parse_error() {
        let err = "expert".parse::<Grade>();
        assert!(err.is_err());
        let msg = "expert".parse::<Grade>().unwrap_err();
        assert!(msg.contains("Unknown grade"), "Error should mention 'Unknown grade': {}", msg);

        assert!("".parse::<Grade>().is_err());
        assert!("INVALID".parse::<Grade>().is_err());
    }

    #[test]
    fn grade_profile_fallback_to_default() {
        let profile = GradeProfile::default();
        let grade = profile.domains.get("unknown_domain")
            .copied()
            .unwrap_or(profile.default_grade);
        assert_eq!(grade, Grade::Senior); // default fallback
    }

    #[test]
    fn complexity_fibonacci_values() {
        assert_eq!(Complexity::Trivial.value(), 1);
        assert_eq!(Complexity::Simple.value(), 2);
        assert_eq!(Complexity::Medium.value(), 3);
        assert_eq!(Complexity::Complex.value(), 5);
        assert_eq!(Complexity::Hard.value(), 8);
        assert_eq!(Complexity::Epic.value(), 13);
    }

    #[test]
    fn complexity_from_value_roundtrip() {
        for v in [1, 2, 3, 5, 8, 13] {
            let c = Complexity::from_value(v).unwrap();
            assert_eq!(c.value(), v);
        }
        assert!(Complexity::from_value(4).is_none());
        assert!(Complexity::from_value(0).is_none());
    }

    #[test]
    fn complexity_base_hours_increasing() {
        let complexities = [
            Complexity::Trivial,
            Complexity::Simple,
            Complexity::Medium,
            Complexity::Complex,
            Complexity::Hard,
            Complexity::Epic,
        ];
        for pair in complexities.windows(2) {
            assert!(pair[0].base_senior_hours() < pair[1].base_senior_hours());
        }
    }

    #[test]
    fn task_type_ai_multipliers_ordered() {
        assert!(TaskType::PureCoding.ai_multiplier() < TaskType::CodingInfra.ai_multiplier());
        assert!(TaskType::CodingInfra.ai_multiplier() < TaskType::DesignCoding.ai_multiplier());
        assert!(TaskType::DesignCoding.ai_multiplier() < TaskType::PureInfra.ai_multiplier());
        assert!(TaskType::PureInfra.ai_multiplier() < TaskType::Coordination.ai_multiplier());
    }

    #[test]
    fn estimate_config_defaults() {
        let config = EstimateConfig::default();
        assert_eq!(config.grade_multipliers[&Grade::Senior], 1.0);
        assert_eq!(config.grade_multipliers[&Grade::Junior], 2.0);
        assert_eq!(config.ai_task_multipliers[&TaskType::PureCoding], 0.10);
        assert_eq!(config.review_overhead, 0.30);
        assert_eq!(config.safety_margin, 0.50);
        assert_eq!(config.grade_profile.default_grade, Grade::Senior);
    }

    #[test]
    fn grade_profile_domain_lookup() {
        let mut profile = GradeProfile::default();
        profile.domains.insert("backend".to_string(), Grade::Middle);
        profile.domains.insert("devops".to_string(), Grade::Senior);

        assert_eq!(profile.domains.get("backend"), Some(&Grade::Middle));
        assert_eq!(profile.domains.get("devops"), Some(&Grade::Senior));
        assert_eq!(profile.domains.get("frontend"), None);
    }
}
