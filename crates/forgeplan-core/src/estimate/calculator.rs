use std::collections::HashMap;

use super::types::{EstimateConfig, EstimateItem, EstimateResult, Grade, ScoredItem};

/// Calculate hours for each grade from scored items.
pub fn calculate(
    artifact_id: &str,
    artifact_title: &str,
    scored_items: &[ScoredItem],
    config: &EstimateConfig,
    confidence: f64,
    confidence_reasons: Vec<String>,
) -> EstimateResult {
    let items: Vec<EstimateItem> = scored_items
        .iter()
        .map(|si| calculate_item(si, config))
        .collect();

    let mut totals: HashMap<Grade, f64> = HashMap::new();
    let mut total_score = 0.0;

    for item in &items {
        for (grade, hours) in &item.hours {
            *totals.entry(*grade).or_default() += hours;
        }
        total_score += item.score;
    }

    EstimateResult {
        artifact_id: artifact_id.to_string(),
        artifact_title: artifact_title.to_string(),
        items,
        totals,
        total_score,
        confidence,
        confidence_reasons,
    }
}

fn calculate_item(scored: &ScoredItem, config: &EstimateConfig) -> EstimateItem {
    let base_hours = scored.complexity.base_senior_hours();
    let mut hours = HashMap::new();

    for grade in Grade::all() {
        let multiplier = config
            .grade_multipliers
            .get(grade)
            .copied()
            .unwrap_or(grade.default_multiplier());
        hours.insert(*grade, base_hours * multiplier);
    }

    let senior_hours = base_hours;
    let score = scored.complexity.value() as f64 * senior_hours;

    EstimateItem {
        id: scored.id.clone(),
        description: scored.description.clone(),
        complexity: scored.complexity,
        task_type: scored.task_type,
        hours,
        score,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::estimate::types::{Complexity, TaskType};

    fn default_config() -> EstimateConfig {
        EstimateConfig::default()
    }

    fn scored(id: &str, complexity: Complexity) -> ScoredItem {
        ScoredItem {
            id: id.to_string(),
            description: format!("Task {}", id),
            complexity,
            task_type: TaskType::PureCoding,
        }
    }

    #[test]
    fn single_item_senior_baseline() {
        let items = vec![scored("FR-001", Complexity::Medium)];
        let result = calculate("PRD-001", "Test", &items, &default_config(), 0.75, vec![]);

        let senior_hours = result.items[0].hours[&Grade::Senior];
        assert_eq!(senior_hours, 8.0); // Medium = 8h Senior
    }

    #[test]
    fn grade_multipliers_applied() {
        let items = vec![scored("FR-001", Complexity::Medium)];
        let result = calculate("PRD-001", "Test", &items, &default_config(), 0.75, vec![]);

        let item = &result.items[0];
        assert_eq!(item.hours[&Grade::Junior], 16.0);  // 8 × 2.0
        assert_eq!(item.hours[&Grade::Middle], 12.0);   // 8 × 1.5
        assert_eq!(item.hours[&Grade::Senior], 8.0);    // 8 × 1.0
        assert!((item.hours[&Grade::Principal] - 5.6).abs() < 0.01); // 8 × 0.7
        assert!((item.hours[&Grade::Ai] - 3.2).abs() < 0.01);       // 8 × 0.4
    }

    #[test]
    fn totals_sum_correctly() {
        let items = vec![
            scored("FR-001", Complexity::Medium),   // 8h Senior
            scored("FR-002", Complexity::Complex),   // 13h Senior
        ];
        let result = calculate("PRD-001", "Test", &items, &default_config(), 0.75, vec![]);

        assert_eq!(result.totals[&Grade::Senior], 21.0); // 8 + 13
        assert_eq!(result.totals[&Grade::Junior], 42.0);  // 21 × 2.0
    }

    #[test]
    fn score_calculation() {
        let items = vec![scored("FR-001", Complexity::Complex)];
        let result = calculate("PRD-001", "Test", &items, &default_config(), 0.75, vec![]);

        // Score = complexity_value × senior_hours = 5 × 13 = 65
        assert_eq!(result.items[0].score, 65.0);
        assert_eq!(result.total_score, 65.0);
    }

    #[test]
    fn custom_multipliers() {
        let mut config = default_config();
        config.grade_multipliers.insert(Grade::Junior, 3.0); // override

        let items = vec![scored("FR-001", Complexity::Trivial)]; // 3h Senior
        let result = calculate("PRD-001", "Test", &items, &config, 0.5, vec![]);

        assert_eq!(result.items[0].hours[&Grade::Junior], 9.0); // 3 × 3.0
        assert_eq!(result.items[0].hours[&Grade::Senior], 3.0); // unchanged
    }

    #[test]
    fn empty_items() {
        let result = calculate("PRD-001", "Test", &[], &default_config(), 0.0, vec![]);
        assert!(result.items.is_empty());
        assert!(result.totals.is_empty());
        assert_eq!(result.total_score, 0.0);
    }

    #[test]
    fn confidence_passed_through() {
        let result = calculate(
            "PRD-001", "Test", &[], &default_config(),
            0.65, vec!["has FR".to_string(), "no RFC phases".to_string()],
        );
        assert_eq!(result.confidence, 0.65);
        assert_eq!(result.confidence_reasons.len(), 2);
    }
}
