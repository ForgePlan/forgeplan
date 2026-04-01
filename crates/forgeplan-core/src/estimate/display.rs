use super::types::{EstimateResult, Grade};

/// Format estimate result as a terminal-friendly table.
pub fn format_table(result: &EstimateResult, highlight_grade: Option<Grade>) -> String {
    let mut out = String::new();

    // Header
    out.push_str(&format!(
        "Estimate for {}: {}\n",
        result.artifact_id, result.artifact_title
    ));
    if let Some(grade) = highlight_grade {
        out.push_str(&format!(
            "Grade: {} | Confidence: {:.0}%\n\n",
            grade,
            result.confidence * 100.0
        ));
    } else {
        out.push_str(&format!(
            "Confidence: {:.0}%\n\n",
            result.confidence * 100.0
        ));
    }

    if result.items.is_empty() {
        out.push_str("  No work items found. Add FR to PRD or Phase items to RFC.\n");
        return out;
    }

    // Column widths
    let id_width = result
        .items
        .iter()
        .map(|i| i.id.len())
        .max()
        .unwrap_or(6)
        .max(6);
    let desc_width = result
        .items
        .iter()
        .map(|i| i.description.chars().count().min(35))
        .max()
        .unwrap_or(20)
        .max(20);

    // Table header
    out.push_str(&format!(
        "  {:<id_w$}  {:<desc_w$}  {:>4}  {:>6}  {:>6}  {:>6}  {:>6}  {:>6}\n",
        "ID", "Description", "Cmpl", "Jun", "Mid", "Senior", "PS", "AI",
        id_w = id_width, desc_w = desc_width,
    ));
    let line_width = id_width + desc_width + 50;
    out.push_str(&format!("  {}\n", "-".repeat(line_width)));

    // Rows
    for item in &result.items {
        let desc_truncated: String = item.description.chars().take(desc_width).collect();

        out.push_str(&format!(
            "  {:<id_w$}  {:<desc_w$}  {:>4}  {:>5}h  {:>5}h  {:>5}h  {:>5}h  {:>5}h\n",
            item.id,
            desc_truncated,
            item.complexity.value(),
            format_hours(item.hours.get(&Grade::Junior).copied().unwrap_or(0.0)),
            format_hours(item.hours.get(&Grade::Middle).copied().unwrap_or(0.0)),
            format_hours(item.hours.get(&Grade::Senior).copied().unwrap_or(0.0)),
            format_hours(item.hours.get(&Grade::Principal).copied().unwrap_or(0.0)),
            format_hours(item.hours.get(&Grade::Ai).copied().unwrap_or(0.0)),
            id_w = id_width, desc_w = desc_width,
        ));
    }

    // Separator
    out.push_str(&format!("  {}\n", "-".repeat(line_width)));

    // Totals
    out.push_str(&format!(
        "  {:<id_w$}  {:<desc_w$}  {:>4}  {:>5}h  {:>5}h  {:>5}h  {:>5}h  {:>5}h\n",
        "TOTAL", "",
        format!("{}", result.items.iter().map(|i| i.complexity.value()).sum::<u32>()),
        format_hours(result.totals.get(&Grade::Junior).copied().unwrap_or(0.0)),
        format_hours(result.totals.get(&Grade::Middle).copied().unwrap_or(0.0)),
        format_hours(result.totals.get(&Grade::Senior).copied().unwrap_or(0.0)),
        format_hours(result.totals.get(&Grade::Principal).copied().unwrap_or(0.0)),
        format_hours(result.totals.get(&Grade::Ai).copied().unwrap_or(0.0)),
        id_w = id_width, desc_w = desc_width,
    ));

    // Days row
    out.push_str(&format!(
        "  {:<id_w$}  {:<desc_w$}  {:>4}  {:>5}d  {:>5}d  {:>5}d  {:>5}d  {:>5}d\n",
        "", "", "",
        format_days(result.totals.get(&Grade::Junior).copied().unwrap_or(0.0)),
        format_days(result.totals.get(&Grade::Middle).copied().unwrap_or(0.0)),
        format_days(result.totals.get(&Grade::Senior).copied().unwrap_or(0.0)),
        format_days(result.totals.get(&Grade::Principal).copied().unwrap_or(0.0)),
        format_days(result.totals.get(&Grade::Ai).copied().unwrap_or(0.0)),
        id_w = id_width, desc_w = desc_width,
    ));

    // Confidence footer
    out.push('\n');
    if !result.confidence_reasons.is_empty() {
        out.push_str(&format!(
            "  Confidence: {:.0}% — {}\n",
            result.confidence * 100.0,
            result.confidence_reasons.join(", ")
        ));
    }

    // Hints
    if !result.hints.is_empty() {
        out.push('\n');
        for hint in &result.hints {
            let prefix = match hint.level {
                super::types::HintLevel::Warning => "!",
                super::types::HintLevel::Info => "i",
                super::types::HintLevel::Suggestion => "*",
            };
            out.push_str(&format!("  {} {}\n", prefix, hint.message));
            if let Some(ref action) = hint.action {
                out.push_str(&format!("    -> {}\n", action));
            }
        }
    }

    out
}

fn format_hours(h: f64) -> String {
    if h < 10.0 {
        format!("{:.1}", h)
    } else {
        format!("{:.0}", h)
    }
}

fn format_days(h: f64) -> String {
    let days = h / 8.0;
    if days < 10.0 {
        format!("{:.1}", days)
    } else {
        format!("{:.0}", days)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::estimate::types::{Complexity, EstimateItem, TaskType};
    use std::collections::HashMap;

    fn make_result() -> EstimateResult {
        let mut hours1 = HashMap::new();
        hours1.insert(Grade::Junior, 16.0);
        hours1.insert(Grade::Middle, 12.0);
        hours1.insert(Grade::Senior, 8.0);
        hours1.insert(Grade::Principal, 5.6);
        hours1.insert(Grade::Ai, 3.2);

        let mut totals = HashMap::new();
        totals.insert(Grade::Junior, 16.0);
        totals.insert(Grade::Middle, 12.0);
        totals.insert(Grade::Senior, 8.0);
        totals.insert(Grade::Principal, 5.6);
        totals.insert(Grade::Ai, 3.2);

        EstimateResult {
            artifact_id: "PRD-022".to_string(),
            artifact_title: "AI Estimation Engine".to_string(),
            items: vec![EstimateItem {
                id: "FR-001".to_string(),
                description: "User can run estimate".to_string(),
                complexity: Complexity::Medium,
                task_type: TaskType::PureCoding,
                hours: hours1,
                score: 24.0,
            }],
            totals,
            total_score: 24.0,
            confidence: 0.75,
            confidence_reasons: vec!["has FR".to_string(), "no RFC phases".to_string()],
            hints: vec![],
        }
    }

    #[test]
    fn table_contains_artifact_id() {
        let output = format_table(&make_result(), None);
        assert!(output.contains("PRD-022"));
        assert!(output.contains("AI Estimation Engine"));
    }

    #[test]
    fn table_contains_fr_id() {
        let output = format_table(&make_result(), None);
        assert!(output.contains("FR-001"));
    }

    #[test]
    fn table_contains_grade_headers() {
        let output = format_table(&make_result(), None);
        assert!(output.contains("Jun"));
        assert!(output.contains("Mid"));
        assert!(output.contains("Senior"));
        assert!(output.contains("AI"));
    }

    #[test]
    fn table_contains_confidence() {
        let output = format_table(&make_result(), None);
        assert!(output.contains("75%"));
        assert!(output.contains("has FR"));
    }

    #[test]
    fn table_contains_total() {
        let output = format_table(&make_result(), None);
        assert!(output.contains("TOTAL"));
    }

    #[test]
    fn table_with_highlight_grade() {
        let output = format_table(&make_result(), Some(Grade::Middle));
        assert!(output.contains("Grade: Middle"));
    }

    #[test]
    fn empty_result_shows_message() {
        let result = EstimateResult {
            artifact_id: "PRD-001".to_string(),
            artifact_title: "Empty".to_string(),
            items: vec![],
            totals: HashMap::new(),
            total_score: 0.0,
            confidence: 0.0,
            confidence_reasons: vec![],
            hints: vec![],
        };
        let output = format_table(&result, None);
        assert!(output.contains("No work items found"));
    }
}
