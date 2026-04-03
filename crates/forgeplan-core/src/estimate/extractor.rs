use std::sync::OnceLock;

use regex::Regex;

use super::types::{EstimateHint, HintLevel, ItemSource, WorkItem};

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

/// Collect hints about artifact quality issues affecting estimates.
pub fn collect_hints(body: &str, extracted_count: usize, kind: &str) -> Vec<EstimateHint> {
    let mut hints = Vec::new();

    // Detect template placeholder FRs that were filtered out
    let total_fr_rows = count_fr_rows(body);
    let template_count = total_fr_rows.saturating_sub(extracted_count);
    if template_count > 0 {
        hints.push(EstimateHint {
            level: HintLevel::Warning,
            message: format!(
                "{} FR row(s) contain template placeholders (\"[Actor] can [capability]\") and were skipped",
                template_count
            ),
            action: Some(format!(
                "Fill in FR descriptions in the PRD with real requirements"
            )),
        });
    }

    // No items at all — but only if there weren't template placeholders (which have their own hint)
    if extracted_count == 0 && template_count == 0 {
        let suggestion = match kind {
            "prd" => "Add FR table: | FR-001 | Core | Must | User can ... | Journey 1 |",
            "rfc" => "Add Phase checklist: - [ ] **1.1** Description",
            _ => "Add FR table to PRD or Phase checklist to RFC",
        };
        hints.push(EstimateHint {
            level: HintLevel::Warning,
            message: "No estimable work items found".to_string(),
            action: Some(suggestion.to_string()),
        });
    }

    // Low item count
    if extracted_count > 0 && extracted_count <= 2 {
        hints.push(EstimateHint {
            level: HintLevel::Info,
            message: format!(
                "Only {} item(s) — estimate may be incomplete",
                extracted_count
            ),
            action: Some("Consider breaking down into more granular FR/Phase items".to_string()),
        });
    }

    // Confidence boost suggestions
    let has_fr = body.contains("| FR-");
    let has_phases = body.contains("- [ ] **") || body.contains("- [x] **");
    if has_fr && !has_phases {
        hints.push(EstimateHint {
            level: HintLevel::Suggestion,
            message: "Create an RFC with Implementation Phases for +25% confidence".to_string(),
            action: Some("forgeplan new rfc \"<title>\"".to_string()),
        });
    }

    hints
}

/// Count total FR table rows (including template placeholders).
fn count_fr_rows(body: &str) -> usize {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"(?m)^\|\s*FR-\d+\s*\|").expect("valid regex"));
    re.find_iter(body).count()
}

/// Extract FR rows from a markdown table in PRD.
/// Expected format: | FR-001 | Core | Must | [Actor] can [capability] | Journey 1 |
fn extract_fr_table(body: &str) -> Vec<WorkItem> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(
        r"(?m)^\|\s*(FR-\d+)\s*\|\s*([\w\-][\w\s\-]*?)\s*\|\s*(Must|Should|Could|Won't)\s*\|\s*(.+?)\s*\|\s*(.+?)\s*\|"
    ).expect("valid regex"));

    re.captures_iter(body)
        .filter_map(|cap| {
            let desc = cap[4].trim().to_string();
            // Skip template placeholders — unfilled FR are noise
            if is_template_placeholder(&desc) {
                return None;
            }
            Some(WorkItem {
                id: cap[1].to_string(),
                description: desc,
                category: cap[2].trim().to_string(),
                priority: cap[3].trim().to_string(),
                source: ItemSource::Fr,
            })
        })
        .collect()
}

/// Detect template placeholder descriptions that were never filled in.
fn is_template_placeholder(desc: &str) -> bool {
    let d = desc.trim();
    d == "[Actor] can [capability]"
        || d.starts_with("{")
        || d.starts_with("[Actor]")
        || d == "..."
        || d == "TBD"
        || d.is_empty()
}

/// Extract Phase checklist items from RFC.
/// Supports multiple formats:
///   - [ ] **1.1** Description        (standard forgeplan format)
///   - [ ] Step 1: Description         (simple checklist)
///   - [ ] Description                 (plain checklist, auto-numbered)
fn extract_phase_items(body: &str) -> Vec<WorkItem> {
    // Format 1: - [ ] **1.1** Description (standard)
    static RE_STANDARD: OnceLock<Regex> = OnceLock::new();
    let re_standard = RE_STANDARD.get_or_init(|| {
        Regex::new(r"(?m)^- \[[ x]\] \*\*(\d+\.\d+)\*\*\s+(.+)$").expect("valid regex")
    });

    let mut items: Vec<WorkItem> = re_standard
        .captures_iter(body)
        .map(|cap| WorkItem {
            id: format!("P{}", cap[1].to_string()),
            description: cap[2].trim().to_string(),
            category: "Implementation".to_string(),
            priority: "Must".to_string(),
            source: ItemSource::Phase,
        })
        .collect();

    // If standard format found items, return them
    if !items.is_empty() {
        return items;
    }

    // Format 2: - [ ] Description (plain checklist, under ## Implementation / ## Phase headers)
    static RE_PLAIN: OnceLock<Regex> = OnceLock::new();
    let re_plain =
        RE_PLAIN.get_or_init(|| Regex::new(r"(?m)^- \[[ x]\]\s+(.+)$").expect("valid regex"));

    let mut counter = 0u32;
    for cap in re_plain.captures_iter(body) {
        let desc = cap[1].trim().to_string();
        if is_template_placeholder(&desc) {
            continue;
        }
        counter += 1;
        items.push(WorkItem {
            id: format!("P0.{}", counter),
            description: desc,
            category: "Implementation".to_string(),
            priority: "Must".to_string(),
            source: ItemSource::Phase,
        });
    }

    items
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

    #[test]
    fn template_fr_rows_are_filtered_out() {
        let body = r#"
| ID | Category | Priority | Requirement | Journey |
|----|----------|----------|-------------|---------|
| FR-001 | Core | Must | [Actor] can [capability] | Journey 1 |
| FR-002 | Core | Must | User can search artifacts by keyword | Journey 1 |
| FR-003 | UX | Should | [Actor] can [capability] | Journey 2 |
"#;
        let items = extract_fr_table(body);
        assert_eq!(items.len(), 1, "Only filled FR should be extracted");
        assert_eq!(items[0].id, "FR-002");
        assert!(items[0].description.contains("search"));
    }

    #[test]
    fn plain_checklist_fallback() {
        let body = r#"
## Implementation Phases

### Phase 1: Core
- [ ] Set up project structure
- [x] Define data model
- [ ] Implement CRUD operations

### Phase 2: Polish
- [ ] Add error handling
"#;
        let items = extract_phase_items(body);
        assert_eq!(items.len(), 4);
        assert_eq!(items[0].id, "P0.1");
        assert!(items[0].description.contains("project structure"));
        assert_eq!(items[2].id, "P0.3");
        assert!(items[2].description.contains("CRUD"));
    }

    #[test]
    fn collect_hints_no_items_no_templates() {
        let body = "# Just a title\n\nSome text.";
        let hints = collect_hints(body, 0, "prd");
        // Should get "no estimable items" hint but NOT template warning
        assert_eq!(hints.len(), 1);
        assert!(hints[0].message.contains("No estimable"));
        assert!(hints[0].action.as_ref().unwrap().contains("FR table"));
    }

    #[test]
    fn collect_hints_all_templates_no_double_hint() {
        let body = r#"
| ID | Category | Priority | Requirement | Journey |
|----|----------|----------|-------------|---------|
| FR-001 | Core | Must | [Actor] can [capability] | Journey 1 |
| FR-002 | Core | Must | [Actor] can [capability] | Journey 2 |
"#;
        let hints = collect_hints(body, 0, "prd");
        // Should get template warning + RFC suggestion, but NOT "no items found" (F3 fix)
        assert!(
            hints
                .iter()
                .any(|h| h.message.contains("template placeholders"))
        );
        assert!(
            !hints.iter().any(|h| h.message.contains("No estimable")),
            "Should NOT show 'no items' when templates exist — user already has FR table"
        );
    }

    #[test]
    fn collect_hints_low_item_count() {
        let body = r#"
| ID | Category | Priority | Requirement | Journey |
|----|----------|----------|-------------|---------|
| FR-001 | Core | Must | User can do X | Journey 1 |
"#;
        let hints = collect_hints(body, 1, "prd");
        // Should get low count info + RFC suggestion
        assert!(hints.iter().any(|h| h.message.contains("Only 1 item")));
        assert!(hints.iter().any(|h| h.message.contains("RFC")));
    }

    #[test]
    fn collect_hints_rfc_kind_suggestion() {
        let body = "# No FR or phases";
        let hints = collect_hints(body, 0, "rfc");
        assert!(
            hints[0]
                .action
                .as_ref()
                .unwrap()
                .contains("Phase checklist")
        );
    }

    #[test]
    fn fr_regex_accepts_hyphenated_category() {
        let body = r#"
| ID | Category | Priority | Requirement | Journey |
|----|----------|----------|-------------|---------|
| FR-001 | Non-Core | Must | User can do X | Journey 1 |
| FR-002 | API-Design | Should | System can do Y | Journey 2 |
"#;
        let items = extract_fr_table(body);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].category, "Non-Core");
        assert_eq!(items[1].category, "API-Design");
    }

    #[test]
    fn standard_format_takes_priority_over_plain() {
        let body = r#"
- [ ] **1.1** Create types.rs
- [ ] **1.2** Create parser.rs
- [ ] Some plain checklist item
"#;
        let items = extract_phase_items(body);
        // Standard format found — plain items ignored
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id, "P1.1");
    }
}
