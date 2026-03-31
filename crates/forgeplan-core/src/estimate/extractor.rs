use regex::Regex;

use super::types::WorkItem;

/// Extract work items from an artifact's markdown body.
/// Supports: FR table rows from PRD, Phase checklist items from RFC.
pub fn extract_work_items(body: &str) -> Vec<WorkItem> {
    let mut items = Vec::new();

    // Try FR table extraction (PRD format)
    items.extend(extract_fr_table(body));

    // Try Phase checklist extraction (RFC format)
    items.extend(extract_phase_items(body));

    items
}

/// Extract FR rows from a markdown table in PRD.
/// Expected format: | FR-001 | Core | Must | [Actor] can [capability] | Journey 1 |
fn extract_fr_table(body: &str) -> Vec<WorkItem> {
    let re = Regex::new(
        r"(?m)^\|\s*(FR-\d+)\s*\|\s*(\w[\w\s]*?)\s*\|\s*(Must|Should|Could|Won't)\s*\|\s*(.+?)\s*\|\s*(.+?)\s*\|"
    ).expect("valid regex");

    re.captures_iter(body)
        .map(|cap| WorkItem {
            id: cap[1].to_string(),
            description: cap[4].trim().to_string(),
            category: cap[2].trim().to_string(),
            priority: cap[3].trim().to_string(),
        })
        .collect()
}

/// Extract Phase checklist items from RFC.
/// Expected format: - [ ] **1.1** Description here
///                  - [x] **2.3** Already done
fn extract_phase_items(body: &str) -> Vec<WorkItem> {
    let re = Regex::new(
        r"(?m)^- \[[ x]\] \*\*(\d+\.\d+)\*\*\s+(.+)$"
    ).expect("valid regex");

    re.captures_iter(body)
        .map(|cap| WorkItem {
            id: format!("P{}", cap[1].to_string()),
            description: cap[2].trim().to_string(),
            category: "Implementation".to_string(),
            priority: "Must".to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_fr_from_prd_table() {
        let body = r#"
## Functional Requirements

| ID | Category | Priority | Requirement | Journey |
|----|----------|----------|-------------|---------|
| FR-001 | Core | Must | User can run estimate on any artifact | Journey 1 |
| FR-002 | Core | Must | System can extract work items from FR table | Journey 1 |
| FR-003 | UX | Should | User can specify target grade via --grade | Journey 2 |
"#;
        let items = extract_fr_table(body);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].id, "FR-001");
        assert_eq!(items[0].category, "Core");
        assert_eq!(items[0].priority, "Must");
        assert!(items[0].description.contains("estimate"));
        assert_eq!(items[2].priority, "Should");
    }

    #[test]
    fn extract_phases_from_rfc() {
        let body = r#"
### Phase 1: Core Types
- [ ] **1.1** Create types.rs — Grade, Complexity enums
- [ ] **1.2** Create extractor.rs — parse FR table
- [x] **1.3** Create scorer.rs — rule-based scoring

### Phase 2: CLI
- [ ] **2.1** Create estimate command
"#;
        let items = extract_phase_items(body);
        assert_eq!(items.len(), 4);
        assert_eq!(items[0].id, "P1.1");
        assert!(items[0].description.contains("types.rs"));
        assert_eq!(items[2].id, "P1.3");
        assert_eq!(items[3].id, "P2.1");
    }

    #[test]
    fn extract_work_items_combines_both() {
        let body = r#"
| ID | Category | Priority | Requirement | Journey |
|----|----------|----------|-------------|---------|
| FR-001 | Core | Must | User can do X | Journey 1 |

### Phase 1: MVP
- [ ] **1.1** Implement feature X
"#;
        let items = extract_work_items(body);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id, "FR-001");
        assert_eq!(items[1].id, "P1.1");
    }

    #[test]
    fn empty_body_returns_empty() {
        let items = extract_work_items("");
        assert!(items.is_empty());
    }

    #[test]
    fn no_matching_patterns() {
        let body = "# Just a title\n\nSome regular text without FR or Phase items.";
        let items = extract_work_items(body);
        assert!(items.is_empty());
    }
}
