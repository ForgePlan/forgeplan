use std::collections::HashMap;

/// Embedded minimal templates for each artifact type.
/// In the future, these will be loaded from a `templates/` directory.
pub fn get_embedded_template(kind: &str) -> Option<&'static str> {
    match kind {
        "prd" => Some(include_str!("../../../../templates/prd/_TEMPLATE.md")),
        "epic" => Some(include_str!("../../../../templates/epic/_TEMPLATE.md")),
        "spec" => Some(include_str!("../../../../templates/spec/_TEMPLATE.md")),
        "rfc" => Some(include_str!("../../../../templates/rfc/_TEMPLATE.md")),
        "adr" => Some(include_str!("../../../../templates/adr/_TEMPLATE.md")),
        "problem" => Some(include_str!("../../../../templates/problem/_TEMPLATE.md")),
        "solution" => Some(include_str!("../../../../templates/solution/_TEMPLATE.md")),
        "evidence" => Some(include_str!("../../../../templates/evidence/_TEMPLATE.md")),
        "note" => Some(include_str!("../../../../templates/note/_TEMPLATE.md")),
        "refresh" => Some(include_str!("../../../../templates/refresh/_TEMPLATE.md")),
        // Issue #287 Phase A — brownfield extraction kinds.
        "use_case" => Some(include_str!("../../../../templates/use_case/_TEMPLATE.md")),
        "glossary" => Some(include_str!("../../../../templates/glossary/_TEMPLATE.md")),
        "invariant" => Some(include_str!("../../../../templates/invariant/_TEMPLATE.md")),
        "scenario" => Some(include_str!("../../../../templates/scenario/_TEMPLATE.md")),
        "hypothesis" => Some(include_str!(
            "../../../../templates/hypothesis/_TEMPLATE.md"
        )),
        "domain_model" => Some(include_str!(
            "../../../../templates/domain_model/_TEMPLATE.md"
        )),
        _ => None,
    }
}

/// Drop the leading `---` metadata block of a template.
///
/// That block is guidance for whoever fills the template in — its values are
/// placeholders (`depth: tactical / standard / deep / critical`,
/// `priority: P0 / P1 / P2 / P3`). The frontmatter of a real document is
/// generated from the record, never copied from the template. Every caller of
/// [`render_template`] turns the result into a document body, so the sample
/// block is removed here; otherwise a created document opens with a second
/// frontmatter block full of placeholder values.
///
/// Returns `text` unchanged when it does not start with a fenced block.
fn strip_leading_frontmatter(text: &str) -> &str {
    let body = text.trim_start_matches(['\n', '\r']);
    let Some(after_open) = body.strip_prefix("---") else {
        return text;
    };
    // The opening fence must end its line: `---` alone, not `----` or `---x`.
    let Some(after_open) = after_open
        .strip_prefix("\r\n")
        .or_else(|| after_open.strip_prefix('\n'))
    else {
        return text;
    };
    let mut offset = 0usize;
    for line in after_open.split_inclusive('\n') {
        if line.trim_end_matches(['\n', '\r']) == "---" {
            let rest = &after_open[offset + line.len()..];
            return rest.trim_start_matches(['\n', '\r']);
        }
        offset += line.len();
    }
    // No closing fence: the leading `---` was content, not frontmatter.
    text
}

/// Render a template by replacing `{{key}}` and `{key}` placeholders with values.
///
/// This is a simple string replacement -- we don't use Tera yet to avoid the compile-time cost.
/// Tera will be added in Phase 3B when we need conditional sections based on depth.
pub fn render_template(template: &str, vars: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        // Replace template placeholders like {{NNN}}, {{title}}, etc.
        result = result.replace(&format!("{{{{{}}}}}", key), value);
        // Also replace patterns like {NNN} -> 001
        result = result.replace(&format!("{{{}}}", key), value);
    }
    result
}

#[cfg(test)]
mod frontmatter_strip_tests {
    use super::*;

    #[test]
    fn the_template_frontmatter_is_not_body_content() {
        let template = "---\nid: PRD-{NNN}\npriority: P0 / P1 / P2 / P3\n---\n\n# PRD-{NNN}: {title}\n";
        let mut vars = HashMap::new();
        vars.insert("NNN".to_string(), "001".to_string());
        vars.insert("title".to_string(), "Gamma".to_string());
        let rendered = render_template(template, &vars);
        assert_eq!(rendered, "# PRD-001: Gamma\n");
        assert!(!rendered.contains("priority:"), "the placeholder block leaked into the body");
    }

    #[test]
    fn a_template_without_frontmatter_is_untouched() {
        let mut vars = HashMap::new();
        vars.insert("title".to_string(), "X".to_string());
        assert_eq!(render_template("# {title}\n", &vars), "# X\n");
    }

    #[test]
    fn a_horizontal_rule_is_not_mistaken_for_a_fence() {
        let mut vars = HashMap::new();
        vars.insert("title".to_string(), "X".to_string());
        // Opens with a rule, never closes: the rule is content.
        assert_eq!(render_template("---\n\n{title}\n", &vars), "---\n\nX\n");
    }
}
