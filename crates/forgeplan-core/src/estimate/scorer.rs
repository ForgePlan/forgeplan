use crate::config::LlmConfig;
use crate::llm::LlmClient;

use super::types::{Complexity, ScoredItem, TaskType, WorkItem};

/// Score work items with rule-based heuristics.
/// Assigns Fibonacci complexity and task type based on description keywords.
pub fn score_items(items: &[WorkItem]) -> Vec<ScoredItem> {
    items.iter().map(score_single).collect()
}

// ---------------------------------------------------------------------------
// LLM-based scoring (L1 opt-in)
// ---------------------------------------------------------------------------

/// System prompt for the LLM scorer.
const LLM_SCORER_SYSTEM: &str = "\
You are a senior engineering estimator. For each work item you receive, assign:
1. A Fibonacci complexity score: 1 (trivial), 2 (simple), 3 (medium), 5 (complex), 8 (hard), 13 (epic).
2. A task type: pure_coding, coding_infra, design_coding, pure_infra, coordination.

Respond ONLY with one line per item in exactly this format (no extra text):
ID|COMPLEXITY|TASK_TYPE

Example:
FR-001|3|pure_coding
FR-002|8|coding_infra
";

/// Build the user prompt listing all items for the LLM.
fn build_llm_prompt(items: &[WorkItem]) -> String {
    let mut prompt = String::from("Score the following work items:\n\n");
    for item in items {
        prompt.push_str(&format!(
            "- {} [{}] (priority: {}): {}\n",
            item.id, item.category, item.priority, item.description
        ));
    }
    prompt
}

/// Parse a single LLM response line like "FR-001|3|pure_coding" into (id, Complexity, TaskType).
/// Returns None if the line cannot be parsed.
pub fn parse_llm_line(line: &str) -> Option<(String, Complexity, TaskType)> {
    let parts: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
    if parts.len() != 3 {
        return None;
    }
    let id = parts[0].to_string();
    let complexity = parts[1]
        .parse::<u32>()
        .ok()
        .and_then(Complexity::from_value)?;
    let task_type = parse_task_type(parts[2])?;
    Some((id, complexity, task_type))
}

/// Parse a task type string from the LLM response.
fn parse_task_type(s: &str) -> Option<TaskType> {
    match s.to_lowercase().replace('-', "_").as_str() {
        "pure_coding" | "purecoding" | "coding" => Some(TaskType::PureCoding),
        "coding_infra" | "codinginfra" => Some(TaskType::CodingInfra),
        "design_coding" | "designcoding" => Some(TaskType::DesignCoding),
        "pure_infra" | "pureinfra" | "infra" => Some(TaskType::PureInfra),
        "coordination" | "coord" => Some(TaskType::Coordination),
        _ => None,
    }
}

/// Parse the full LLM response into a map of id -> (Complexity, TaskType).
fn parse_llm_response(response: &str, items: &[WorkItem]) -> Vec<ScoredItem> {
    use std::collections::HashMap;

    // Build a lookup from parsed LLM lines
    let mut llm_map: HashMap<String, (Complexity, TaskType)> = HashMap::new();
    for line in response.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some((id, complexity, task_type)) = parse_llm_line(line) {
            llm_map.insert(id, (complexity, task_type));
        }
    }

    // For each work item, use LLM result if available, else fall back to rule-based
    items
        .iter()
        .map(|item| {
            if let Some((complexity, task_type)) = llm_map.get(&item.id) {
                ScoredItem {
                    id: item.id.clone(),
                    description: item.description.clone(),
                    complexity: *complexity,
                    task_type: *task_type,
                }
            } else {
                // Fallback to rule-based for items the LLM missed
                score_single(item)
            }
        })
        .collect()
}

/// Score work items using an LLM (L1).
/// Falls back to rule-based scoring if the LLM call fails.
pub async fn score_items_with_llm(items: &[WorkItem], llm_config: &LlmConfig) -> Vec<ScoredItem> {
    if items.is_empty() {
        return Vec::new();
    }

    let client = LlmClient::new(llm_config.clone());
    let prompt = build_llm_prompt(items);

    match client.generate(&prompt, Some(LLM_SCORER_SYSTEM)).await {
        Ok(response) => parse_llm_response(&response, items),
        Err(_) => {
            // LLM failed — graceful fallback to rule-based
            score_items(items)
        }
    }
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
        "migration",
        "redesign",
        "refactor",
        "integrate",
        "distributed",
        "concurrent",
        "security",
        "encryption",
        "authentication",
        "state machine",
    ];
    let complex_keywords = [
        "engine",
        "parser",
        "scoring",
        "search",
        "validation",
        "workflow",
        "pipeline",
        "orchestrat",
        "aggregate",
        "transform",
    ];
    let medium_keywords = [
        "extract",
        "convert",
        "calculate",
        "display",
        "format",
        "filter",
        "sort",
        "command",
        "handler",
        "endpoint",
    ];

    let hard_count = hard_keywords.iter().filter(|kw| desc.contains(*kw)).count();
    let complex_count = complex_keywords
        .iter()
        .filter(|kw| desc.contains(*kw))
        .count();
    let medium_count = medium_keywords
        .iter()
        .filter(|kw| desc.contains(*kw))
        .count();

    // Priority boost: Must items tend to be more complex than Could
    let priority_boost: i32 = match item.priority.to_lowercase().as_str() {
        "must" => 1,
        "should" => 0,
        "could" => -1,
        _ => 0,
    };

    // Description length contributes to complexity
    let length_score: i32 = if words > 20 {
        2
    } else if words > 10 {
        1
    } else {
        0
    };

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

    let infra_keywords = [
        "deploy",
        "k8s",
        "docker",
        "ci/cd",
        "pipeline",
        "infrastructure",
        "kubernetes",
        "helm",
        "terraform",
        "vault",
        "registry",
        "runner",
        "namespace",
    ];
    let design_keywords = [
        "wireframe",
        "prototype",
        "mockup",
        "figma",
        "sketch",
        "visual design",
        "user interface design",
        "responsive layout",
    ];
    let coordination_keywords = [
        "meeting",
        "discuss",
        "coordinate",
        "align",
        "stakeholder",
        "handoff",
        "onboard",
        "workshop",
    ];
    let coding_keywords = [
        "implement",
        "create",
        "add",
        "build",
        "write",
        "develop",
        "parse",
        "extract",
        "calculate",
        "score",
        "validate",
        "convert",
        "format",
        "configure",
        "customize",
        "specify",
        "run",
        "show",
        "display",
        "list",
        "update",
        "delete",
    ];

    let infra_hits = infra_keywords
        .iter()
        .filter(|kw| desc.contains(*kw))
        .count();
    let design_hits = design_keywords
        .iter()
        .filter(|kw| desc.contains(*kw))
        .count();
    let coord_hits = coordination_keywords
        .iter()
        .filter(|kw| desc.contains(*kw))
        .count();
    let coding_hits = coding_keywords
        .iter()
        .filter(|kw| desc.contains(*kw))
        .count();

    if coord_hits > 0 && coding_hits == 0 {
        return TaskType::Coordination;
    }
    if infra_hits > 0 && coding_hits > 0 {
        return TaskType::CodingInfra;
    }
    if infra_hits > 0 {
        return TaskType::PureInfra;
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
            source: crate::estimate::types::ItemSource::Fr,
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
        let item = work_item(
            "FR-002",
            "System can integrate distributed authentication with security encryption",
            "Must",
        );
        let scored = score_single(&item);
        assert!(
            scored.complexity.value() >= 5,
            "Expected Complex+ for integration+security+encryption"
        );
    }

    #[test]
    fn infra_task_detected() {
        let item = work_item(
            "FR-003",
            "Deploy to kubernetes namespace with helm charts",
            "Must",
        );
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
        let item = work_item(
            "FR-005",
            "Create wireframe prototype for dashboard mockup",
            "Should",
        );
        let scored = score_single(&item);
        assert_eq!(scored.task_type, TaskType::DesignCoding);
    }

    #[test]
    fn customize_is_coding_not_design() {
        // "customize" was incorrectly classified as design in old keyword list
        let item = work_item(
            "FR-006",
            "User can customize Level 1 prompt template",
            "Should",
        );
        let scored = score_single(&item);
        assert_eq!(scored.task_type, TaskType::PureCoding);
    }

    #[test]
    fn show_capabilities_is_coding() {
        let item = work_item("FR-007", "System can show routing capabilities", "Should");
        let scored = score_single(&item);
        assert_eq!(scored.task_type, TaskType::PureCoding);
    }

    #[test]
    fn score_multiple_items() {
        let items = vec![
            work_item("FR-001", "Add button", "Could"),
            work_item(
                "FR-002",
                "Implement distributed authentication engine with security",
                "Must",
            ),
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

    // -----------------------------------------------------------------------
    // LLM scorer tests
    // -----------------------------------------------------------------------

    #[test]
    fn parse_llm_line_valid() {
        let result = parse_llm_line("FR-001|3|pure_coding");
        assert!(result.is_some());
        let (id, complexity, task_type) = result.unwrap();
        assert_eq!(id, "FR-001");
        assert_eq!(complexity, Complexity::Medium);
        assert_eq!(task_type, TaskType::PureCoding);
    }

    #[test]
    fn parse_llm_line_all_task_types() {
        assert_eq!(
            parse_llm_line("A|1|pure_coding").unwrap().2,
            TaskType::PureCoding
        );
        assert_eq!(
            parse_llm_line("B|2|coding_infra").unwrap().2,
            TaskType::CodingInfra
        );
        assert_eq!(
            parse_llm_line("C|3|design_coding").unwrap().2,
            TaskType::DesignCoding
        );
        assert_eq!(
            parse_llm_line("D|5|pure_infra").unwrap().2,
            TaskType::PureInfra
        );
        assert_eq!(
            parse_llm_line("E|8|coordination").unwrap().2,
            TaskType::Coordination
        );
    }

    #[test]
    fn parse_llm_line_invalid_complexity() {
        assert!(parse_llm_line("FR-001|4|pure_coding").is_none()); // 4 is not Fibonacci
    }

    #[test]
    fn parse_llm_line_invalid_task_type() {
        assert!(parse_llm_line("FR-001|3|unknown_type").is_none());
    }

    #[test]
    fn parse_llm_line_wrong_format() {
        assert!(parse_llm_line("just some text").is_none());
        assert!(parse_llm_line("FR-001|3").is_none()); // missing task type
        assert!(parse_llm_line("").is_none());
    }

    #[test]
    fn parse_llm_response_with_fallback() {
        // Simulate an LLM response where only FR-001 is scored; FR-002 is missing
        let items = vec![
            work_item(
                "FR-001",
                "Implement auth system with security encryption",
                "Must",
            ),
            work_item("FR-002", "Add button", "Could"),
        ];
        let llm_response = "FR-001|8|coding_infra\n";

        let scored = parse_llm_response(llm_response, &items);
        assert_eq!(scored.len(), 2);

        // FR-001 should use LLM values
        assert_eq!(scored[0].id, "FR-001");
        assert_eq!(scored[0].complexity, Complexity::Hard);
        assert_eq!(scored[0].task_type, TaskType::CodingInfra);

        // FR-002 should fall back to rule-based
        assert_eq!(scored[1].id, "FR-002");
        // Rule-based would assign low complexity for "Add button"
        assert!(scored[1].complexity.value() <= 3);
    }

    #[test]
    fn parse_llm_response_all_garbage_falls_back() {
        let items = vec![work_item(
            "FR-001",
            "Build search engine with validation pipeline",
            "Must",
        )];
        let llm_response = "This is not valid output at all\nNeither is this";

        let scored = parse_llm_response(llm_response, &items);
        assert_eq!(scored.len(), 1);
        // Should fall back to rule-based scoring
        assert_eq!(scored[0].id, "FR-001");
        assert!(scored[0].complexity.value() >= 1);
    }

    #[test]
    fn parse_llm_line_with_whitespace() {
        let result = parse_llm_line("  FR-001 | 5 | pure_coding  ");
        assert!(result.is_some());
        let (id, complexity, task_type) = result.unwrap();
        assert_eq!(id, "FR-001");
        assert_eq!(complexity, Complexity::Complex);
        assert_eq!(task_type, TaskType::PureCoding);
    }
}
