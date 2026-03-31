use super::types::{Complexity, ScoredItem, TaskType, WorkItem};

/// Score work items with rule-based heuristics.
/// Assigns Fibonacci complexity and task type based on description keywords.
pub fn score_items(items: &[WorkItem]) -> Vec<ScoredItem> {
    items.iter().map(|item| score_single(item)).collect()
}

fn score_single(item: &WorkItem) -> ScoredItem {
    let complexity = infer_complexity(item);
    let task_type = infer_task_type(item);
    ScoredItem {
        id: item.id.clone(),
        description: item.description.clone(),
        complexity,
        task_type,
    }
}

/// Infer Fibonacci complexity from description keywords and priority.
fn infer_complexity(item: &WorkItem) -> Complexity {
    let desc = item.description.to_lowercase();
    let words = desc.split_whitespace().count();

    // High complexity indicators
    let hard_keywords = [
        "migration", "redesign", "refactor", "integrate", "distributed",
        "concurrent", "security", "encryption", "authentication", "state machine",
    ];
    let complex_keywords = [
        "engine", "parser", "scoring", "search", "validation", "workflow",
        "pipeline", "orchestrat", "aggregate", "transform",
    ];
    let medium_keywords = [
        "extract", "convert", "calculate", "display", "format", "filter",
        "sort", "command", "handler", "endpoint",
    ];

    let hard_count = hard_keywords.iter().filter(|kw| desc.contains(*kw)).count();
    let complex_count = complex_keywords.iter().filter(|kw| desc.contains(*kw)).count();
    let medium_count = medium_keywords.iter().filter(|kw| desc.contains(*kw)).count();

    // Priority boost: Must items tend to be more complex than Could
    let priority_boost: i32 = match item.priority.to_lowercase().as_str() {
        "must" => 1,
        "should" => 0,
        "could" => -1,
        _ => 0,
    };

    // Description length contributes to complexity
    let length_score: i32 = if words > 20 { 2 } else if words > 10 { 1 } else { 0 };

    let raw_score = (hard_count as i32 * 3)
        + (complex_count as i32 * 2)
        + (medium_count as i32)
        + priority_boost
        + length_score;

    match raw_score {
        ..=0 => Complexity::Trivial,
        1 => Complexity::Simple,
        2..=3 => Complexity::Medium,
        4..=5 => Complexity::Complex,
        6..=8 => Complexity::Hard,
        _ => Complexity::Epic,
    }
}

/// Infer task type from description keywords.
fn infer_task_type(item: &WorkItem) -> TaskType {
    let desc = item.description.to_lowercase();

    let infra_keywords = ["deploy", "k8s", "docker", "ci/cd", "pipeline", "infrastructure",
        "kubernetes", "helm", "terraform", "vault", "registry", "runner", "namespace"];
    let design_keywords = ["design", "ux", "ui", "layout", "wireframe", "prototype", "mockup"];
    let coordination_keywords = ["meeting", "review", "discuss", "plan", "coordinate", "align"];
    let coding_keywords = ["implement", "create", "add", "build", "write", "develop", "parse",
        "extract", "calculate", "score", "validate", "convert", "format"];

    let infra_hits = infra_keywords.iter().filter(|kw| desc.contains(*kw)).count();
    let design_hits = design_keywords.iter().filter(|kw| desc.contains(*kw)).count();
    let coord_hits = coordination_keywords.iter().filter(|kw| desc.contains(*kw)).count();
    let coding_hits = coding_keywords.iter().filter(|kw| desc.contains(*kw)).count();

    if coord_hits > 0 && coding_hits == 0 {
        return TaskType::Coordination;
    }
    if infra_hits > 0 && coding_hits > 0 {
        return TaskType::CodingInfra;
    }
    if infra_hits > 0 {
        return TaskType::PureInfra;
    }
    if design_hits > 0 && coding_hits > 0 {
        return TaskType::DesignCoding;
    }
    if design_hits > 0 {
        return TaskType::DesignCoding;
    }
    TaskType::PureCoding
}

#[cfg(test)]
mod tests {
    use super::*;

    fn work_item(id: &str, desc: &str, priority: &str) -> WorkItem {
        WorkItem {
            id: id.to_string(),
            description: desc.to_string(),
            category: "Core".to_string(),
            priority: priority.to_string(),
        }
    }

    #[test]
    fn simple_coding_task() {
        let item = work_item("FR-001", "User can add a new item", "Should");
        let scored = score_single(&item);
        assert_eq!(scored.task_type, TaskType::PureCoding);
        // Simple task = low complexity
        assert!(scored.complexity.value() <= 3);
    }

    #[test]
    fn complex_integration_task() {
        let item = work_item("FR-002", "System can integrate distributed authentication with security encryption", "Must");
        let scored = score_single(&item);
        assert!(scored.complexity.value() >= 5, "Expected Complex+ for integration+security+encryption");
    }

    #[test]
    fn infra_task_detected() {
        let item = work_item("FR-003", "Deploy to kubernetes namespace with helm charts", "Must");
        let scored = score_single(&item);
        assert_eq!(scored.task_type, TaskType::PureInfra);
    }

    #[test]
    fn coding_infra_mixed() {
        let item = work_item("FR-004", "Implement CI/CD pipeline build step", "Must");
        let scored = score_single(&item);
        assert_eq!(scored.task_type, TaskType::CodingInfra);
    }

    #[test]
    fn design_task_detected() {
        let item = work_item("FR-005", "Design UI layout for dashboard", "Should");
        let scored = score_single(&item);
        assert_eq!(scored.task_type, TaskType::DesignCoding);
    }

    #[test]
    fn score_multiple_items() {
        let items = vec![
            work_item("FR-001", "Add button", "Could"),
            work_item("FR-002", "Implement distributed authentication engine with security", "Must"),
        ];
        let scored = score_items(&items);
        assert_eq!(scored.len(), 2);
        assert!(scored[0].complexity.value() < scored[1].complexity.value());
    }

    #[test]
    fn empty_items() {
        let scored = score_items(&[]);
        assert!(scored.is_empty());
    }
}
