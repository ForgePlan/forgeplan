use crate::artifact::frontmatter::Frontmatter;
use regex::Regex;
use std::sync::LazyLock;

/// Check that a frontmatter key exists and is non-empty.
pub fn frontmatter_has(fm: &Frontmatter, key: &str) -> bool {
    fm.get(key)
        .map(|v| match v {
            serde_yaml::Value::Null => false,
            serde_yaml::Value::String(s) => !s.trim().is_empty(),
            _ => true,
        })
        .unwrap_or(false)
}

/// Check that a markdown section with given heading text exists (any heading level).
pub fn section_exists(body: &str, heading: &str) -> bool {
    let pattern = format!(r"(?m)^#+\s+{}", regex::escape(heading));
    Regex::new(&pattern).map(|re| re.is_match(body)).unwrap_or(false)
}

/// Count words in a section (from heading to next heading of same or higher level).
pub fn section_word_count(body: &str, heading: &str) -> usize {
    if let Some(content) = extract_section(body, heading) {
        content.split_whitespace().count()
    } else {
        0
    }
}

/// Count list items (lines starting with - or *) or table rows (lines with |) in a section.
pub fn section_item_count(body: &str, heading: &str) -> usize {
    if let Some(content) = extract_section(body, heading) {
        content
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                trimmed.starts_with("- ")
                    || trimmed.starts_with("* ")
                    || (trimmed.starts_with("| ")
                        && !trimmed.starts_with("|---")
                        && !trimmed.contains("| --- |")
                        && !trimmed.chars().skip(1).all(|c| c == '-' || c == '|' || c == ' '))
            })
            .count()
    } else {
        0
    }
}

/// Check for template placeholders like {{...}} or TODO/FIXME markers.
pub fn find_placeholders(body: &str) -> Vec<(usize, String)> {
    static PLACEHOLDER_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\{\{[^}]+\}\}").unwrap());
    static TODO_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)\bTODO\b|\bFIXME\b|\bXXX\b").unwrap());

    let mut results = Vec::new();
    let mut in_code_fence = false;
    let mut in_comment = false;
    for (i, line) in body.lines().enumerate() {
        let trimmed = line.trim();
        // Track fenced code blocks
        if trimmed.starts_with("```") {
            in_code_fence = !in_code_fence;
            continue;
        }
        if in_code_fence {
            continue;
        }
        // Track HTML comments (BMAD reminders)
        if trimmed.starts_with("<!--") {
            in_comment = true;
        }
        if in_comment {
            if trimmed.contains("-->") {
                in_comment = false;
            }
            continue;
        }
        for m in PLACEHOLDER_RE.find_iter(line) {
            results.push((i + 1, m.as_str().to_string()));
        }
        // Flag TODO outside of comments
        if !trimmed.starts_with("//") {
            for m in TODO_RE.find_iter(line) {
                results.push((i + 1, m.as_str().to_string()));
            }
        }
    }
    results
}

/// Tech names blocklist for implementation leakage detection.
static TECH_BLOCKLIST: &[&str] = &[
    "React", "Angular", "Vue", "Svelte", "Django", "Flask", "Rails",
    "Express", "Next.js", "Nuxt", "PostgreSQL", "MySQL", "MongoDB",
    "Redis", "Kafka", "RabbitMQ", "Docker", "Kubernetes", "AWS",
    "Azure", "GCP", "Lambda", "S3", "EC2", "Terraform",
];

/// Check for technology names in text (implementation leakage).
/// Returns list of (line_number, tech_name).
pub fn find_tech_leakage(text: &str) -> Vec<(usize, String)> {
    let mut results = Vec::new();
    for (i, line) in text.lines().enumerate() {
        for tech in TECH_BLOCKLIST {
            if line.contains(tech) {
                results.push((i + 1, tech.to_string()));
            }
        }
    }
    results
}

/// Check if text contains numeric targets (numbers with units or comparison).
pub fn has_numeric_targets(text: &str) -> bool {
    static NUMERIC_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"[<>≤≥=]\s*\d+|(\d+[%ms]|\d+\.\d+)").unwrap());
    NUMERIC_RE.is_match(text)
}

/// Extract section content between a heading and the next heading of same/higher level.
fn extract_section(body: &str, heading: &str) -> Option<String> {
    let heading_pattern = format!(r"(?m)^(#+)\s+{}", regex::escape(heading));
    let re = Regex::new(&heading_pattern).ok()?;

    let m = re.find(body)?;
    let start = m.end();
    let heading_level = body[m.start()..m.end()]
        .chars()
        .take_while(|c| *c == '#')
        .count();

    // Find next heading of same or higher level
    let next_heading = Regex::new(&format!(r"(?m)^#{{1,{}}}\s+", heading_level)).ok()?;
    let end = next_heading
        .find(&body[start..])
        .map(|m| start + m.start())
        .unwrap_or(body.len());

    Some(body[start..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_section_exists() {
        let body = "## Goals\n\nSome goals here\n\n## Non-Goals\n\nNope";
        assert!(section_exists(body, "Goals"));
        assert!(section_exists(body, "Non-Goals"));
        assert!(!section_exists(body, "Missing"));
    }

    #[test]
    fn test_section_word_count() {
        let body = "## Problem\n\nThis is a problem with five words here and more.\n\n## Goals\n\nGoal 1";
        assert!(section_word_count(body, "Problem") >= 5);
    }

    #[test]
    fn test_find_placeholders() {
        let body = "Title: {{project_name}}\nDescription here\nTODO: fill this";
        let results = find_placeholders(body);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_has_numeric_targets() {
        assert!(has_numeric_targets("< 100ms"));
        assert!(has_numeric_targets("> 80%"));
        assert!(has_numeric_targets("achieve 99.9% uptime"));
        assert!(!has_numeric_targets("improve performance"));
    }

    #[test]
    fn test_find_tech_leakage() {
        let text = "User can login via React component";
        let leaks = find_tech_leakage(text);
        assert_eq!(leaks.len(), 1);
        assert_eq!(leaks[0].1, "React");
    }
}
