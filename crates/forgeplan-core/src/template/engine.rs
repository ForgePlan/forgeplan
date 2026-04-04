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
        _ => None,
    }
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
