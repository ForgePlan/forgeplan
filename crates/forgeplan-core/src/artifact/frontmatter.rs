use std::collections::BTreeMap;

/// Parsed frontmatter as key-value pairs (flexible, not tied to Meta struct).
pub type Frontmatter = BTreeMap<String, serde_yml::Value>;

/// Parse YAML frontmatter from markdown content.
/// Returns `(frontmatter, body)` where body is everything after the closing `---`.
pub fn parse_frontmatter(content: &str) -> anyhow::Result<(Frontmatter, String)> {
    let content = content.trim_start();
    if !content.starts_with("---") {
        anyhow::bail!("No YAML frontmatter found (missing opening ---)");
    }
    let after_first = &content[3..];
    let end = after_first
        .find("\n---")
        .ok_or_else(|| anyhow::anyhow!("No closing --- found for frontmatter"))?;
    let yaml_str = &after_first[..end];
    // Guard against YAML bomb / excessively large frontmatter (max 64 KB)
    if yaml_str.len() > 65536 {
        anyhow::bail!("Frontmatter too large ({} bytes, max 64KB)", yaml_str.len());
    }
    let fm: Frontmatter = serde_yml::from_str(yaml_str)?;
    // Body starts after closing --- and newline
    let body_start = 3 + end + 4; // "---" + yaml + "\n---"
    let body = if body_start < content.len() {
        content[body_start..].trim_start_matches('\n').to_string()
    } else {
        String::new()
    };
    Ok((fm, body))
}

/// Render frontmatter + body back to a markdown string.
pub fn render_frontmatter(fm: &Frontmatter, body: &str) -> anyhow::Result<String> {
    let yaml = serde_yml::to_string(fm)?;
    Ok(format!("---\n{}---\n\n{}", yaml, body))
}
