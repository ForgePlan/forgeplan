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
    hints: Vec<super::types::EstimateHint>,
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
        hints,
    }
}

fn calculate_item(scored: &ScoredItem, config: &EstimateConfig) -> EstimateItem {
    let base_hours = scored.complexity.base_senior_hours();
    let mut hours = HashMap::new();

    for grade in Grade::all() {
        let grade_mult = config
            .grade_multipliers
            .get(grade)
            .copied()
            .unwrap_or(grade.default_multiplier());

        let effective_hours = if *grade == Grade::Ai {
            // AI uses task-type-specific multiplier (not grade_multiplier) because
            // AI effort depends on task nature (coding vs infra), not experience level.
            // The `ai` key in grade_multipliers is intentionally unused here.
            let task_mult = config
                .ai_task_multipliers
                .get(&scored.task_type)
                .copied()
                .unwrap_or(scored.task_type.ai_multiplier());
            let ai_hours = base_hours * task_mult;
            // Add review overhead (human reviewing AI output)
            ai_hours * (1.0 + config.review_overhead)
        } else {
            base_hours * grade_mult
        };

        hours.insert(*grade, effective_hours);
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
    use crate::estimate::types::{Complexity, ScoredItem, TaskType};

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
        let result = calculate("PRD-001", "Test", &items, &default_config(), 0.75, vec![], vec![]);

        let senior_hours = result.items[0].hours[&Grade::Senior];
        assert_eq!(senior_hours, 8.0); // Medium = 8h Senior
    }

    #[test]
    fn grade_multipliers_applied() {
        let items = vec![scored("FR-001", Complexity::Medium)];
        let result = calculate("PRD-001", "Test", &items, &default_config(), 0.75, vec![], vec![]);

        let item = &result.items[0];
        assert_eq!(item.hours[&Grade::Junior], 16.0);  // 8 × 2.0
        assert_eq!(item.hours[&Grade::Middle], 12.0);   // 8 × 1.5
        assert_eq!(item.hours[&Grade::Senior], 8.0);    // 8 × 1.0
        assert!((item.hours[&Grade::Principal] - 5.6).abs() < 0.01); // 8 × 0.7
        // AI: base_hours(8) × task_ai_mult(0.10 for PureCoding) × (1 + review_overhead(0.30))
        // = 8 × 0.10 × 1.30 = 1.04
        assert!((item.hours[&Grade::Ai] - 1.04).abs() < 0.01);
    }

    #[test]
    fn ai_uses_task_type_multiplier() {
        let items = vec![ScoredItem {
            id: "FR-001".to_string(),
            description: "Infra task".to_string(),
            complexity: Complexity::Medium, // 8h base
            task_type: TaskType::PureInfra, // AI ×0.50
        }];
        let result = calculate("PRD-001", "Test", &items, &default_config(), 0.75, vec![], vec![]);

        // AI: 8 × 0.50 × 1.30 = 5.2
        assert!((result.items[0].hours[&Grade::Ai] - 5.2).abs() < 0.01);
        // Senior unchanged: 8 × 1.0
        assert_eq!(result.items[0].hours[&Grade::Senior], 8.0);
    }

    #[test]
    fn totals_sum_correctly() {
        let items = vec![
            scored("FR-001", Complexity::Medium),   // 8h Senior
            scored("FR-002", Complexity::Complex),   // 13h Senior
        ];
        let result = calculate("PRD-001", "Test", &items, &default_config(), 0.75, vec![], vec![]);

        assert_eq!(result.totals[&Grade::Senior], 21.0); // 8 + 13
        assert_eq!(result.totals[&Grade::Junior], 42.0);  // 21 × 2.0
    }

    #[test]
    fn score_calculation() {
        let items = vec![scored("FR-001", Complexity::Complex)];
        let result = calculate("PRD-001", "Test", &items, &default_config(), 0.75, vec![], vec![]);

        // Score = complexity_value × senior_hours = 5 × 13 = 65
        assert_eq!(result.items[0].score, 65.0);
        assert_eq!(result.total_score, 65.0);
    }

    #[test]
    fn custom_multipliers() {
        let mut config = default_config();
        config.grade_multipliers.insert(Grade::Junior, 3.0); // override

        let items = vec![scored("FR-001", Complexity::Trivial)]; // 3h Senior
        let result = calculate("PRD-001", "Test", &items, &config, 0.5, vec![], vec![]);

        assert_eq!(result.items[0].hours[&Grade::Junior], 9.0); // 3 × 3.0
        assert_eq!(result.items[0].hours[&Grade::Senior], 3.0); // unchanged
    }

    #[test]
    fn empty_items() {
        let result = calculate("PRD-001", "Test", &[], &default_config(), 0.0, vec![], vec![]);
        assert!(result.items.is_empty());
        assert!(result.totals.is_empty());
        assert_eq!(result.total_score, 0.0);
    }

    #[test]
    fn confidence_passed_through() {
        let result = calculate(
            "PRD-001", "Test", &[], &default_config(),
            0.65, vec!["has FR".to_string(), "no RFC phases".to_string()], vec![],
        );
        assert_eq!(result.confidence, 0.65);
        assert_eq!(result.confidence_reasons.len(), 2);
    }

    #[test]
    fn missing_grade_in_config_uses_default() {
        let mut config = default_config();
        config.grade_multipliers.remove(&Grade::Junior); // remove Junior

        let items = vec![scored("FR-001", Complexity::Trivial)]; // 3h Senior
        let result = calculate("PRD-001", "Test", &items, &config, 0.5, vec![], vec![]);

        // Junior falls back to default multiplier 2.0
        assert_eq!(result.items[0].hours[&Grade::Junior], 6.0); // 3 × 2.0
    }
}
