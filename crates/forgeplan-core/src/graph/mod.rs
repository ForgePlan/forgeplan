use std::collections::BTreeMap;
use std::path::Path;

use crate::artifact::frontmatter;
use crate::artifact::store;
use crate::link;

/// Edge in the dependency graph.
#[derive(Debug, Clone)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub relation: String,
}

/// Build all edges by scanning all artifacts in the workspace.
pub fn build_edges(workspace: &Path) -> anyhow::Result<Vec<Edge>> {
    let artifacts = store::list_artifacts(workspace)?;
    let mut edges = Vec::new();

    for artifact in &artifacts {
        let content = std::fs::read_to_string(&artifact.path)?;
        if let Ok((fm, _)) = frontmatter::parse_frontmatter(&content) {
            let links = link::list_links(&fm);
            for (target, relation) in links {
                edges.push(Edge {
                    from: artifact.id.clone(),
                    to: target,
                    relation,
                });
            }
            // Also check parent_epic / epic / prd fields
            for field in &["epic", "prd", "parent_epic"] {
                if let Some(serde_yaml::Value::String(parent)) = fm.get(*field) {
                    if !parent.is_empty() {
                        edges.push(Edge {
                            from: artifact.id.clone(),
                            to: parent.clone(),
                            relation: "belongs_to".to_string(),
                        });
                    }
                }
            }
        }
    }

    edges.sort_by(|a, b| a.from.cmp(&b.from).then(a.to.cmp(&b.to)));
    Ok(edges)
}

/// Render edges as mermaid graph markup.
pub fn render_mermaid(edges: &[Edge]) -> String {
    if edges.is_empty() {
        return "graph LR\n    %% No links found between artifacts\n".to_string();
    }

    let mut lines = vec!["graph LR".to_string()];

    // Collect all node IDs for styling
    let mut nodes: BTreeMap<String, &str> = BTreeMap::new();
    for edge in edges {
        // Detect kind from ID prefix
        nodes.entry(edge.from.clone()).or_insert(kind_from_id(&edge.from));
        nodes.entry(edge.to.clone()).or_insert(kind_from_id(&edge.to));
    }

    // Render edges
    for edge in edges {
        let label = if edge.relation.is_empty() || edge.relation == "belongs_to" {
            format!("    {} --> {}", edge.from, edge.to)
        } else {
            format!("    {} -->|{}| {}", edge.from, edge.relation, edge.to)
        };
        lines.push(label);
    }

    // Style nodes by kind
    let mut style_groups: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for (id, kind) in &nodes {
        style_groups.entry(kind).or_default().push(id.clone());
    }

    if !style_groups.is_empty() {
        lines.push(String::new());
        let colors = [
            ("epic", "epicStyle", "fill:#e1bee7,stroke:#7b1fa2"),
            ("prd", "prdStyle", "fill:#bbdefb,stroke:#1565c0"),
            ("rfc", "rfcStyle", "fill:#c8e6c9,stroke:#2e7d32"),
            ("adr", "adrStyle", "fill:#fff9c4,stroke:#f9a825"),
            ("spec", "specStyle", "fill:#ffe0b2,stroke:#e65100"),
        ];
        for (kind, class_name, color) in &colors {
            if let Some(ids) = style_groups.get(kind) {
                lines.push(format!("    classDef {} {}", class_name, color));
                lines.push(format!("    class {} {}", ids.join(","), class_name));
            }
        }
    }

    lines.join("\n") + "\n"
}

fn kind_from_id(id: &str) -> &'static str {
    let upper = id.to_uppercase();
    if upper.starts_with("EPIC") { "epic" }
    else if upper.starts_with("PRD") { "prd" }
    else if upper.starts_with("RFC") { "rfc" }
    else if upper.starts_with("ADR") { "adr" }
    else if upper.starts_with("SPEC") { "spec" }
    else { "other" }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- kind_from_id ---

    #[test]
    fn kind_from_id_all_known_kinds() {
        assert_eq!(kind_from_id("EPIC-001"), "epic");
        assert_eq!(kind_from_id("epic-001"), "epic");
        assert_eq!(kind_from_id("PRD-001"), "prd");
        assert_eq!(kind_from_id("prd-001"), "prd");
        assert_eq!(kind_from_id("RFC-001"), "rfc");
        assert_eq!(kind_from_id("rfc-001"), "rfc");
        assert_eq!(kind_from_id("ADR-001"), "adr");
        assert_eq!(kind_from_id("adr-001"), "adr");
        assert_eq!(kind_from_id("SPEC-001"), "spec");
        assert_eq!(kind_from_id("spec-001"), "spec");
    }

    #[test]
    fn kind_from_id_unknown_returns_other() {
        assert_eq!(kind_from_id("NOTE-001"), "other");
        assert_eq!(kind_from_id("PROB-001"), "other");
        assert_eq!(kind_from_id(""), "other");
    }

    // --- render_mermaid ---

    #[test]
    fn render_mermaid_empty_edges() {
        let output = render_mermaid(&[]);
        assert_eq!(
            output,
            "graph LR\n    %% No links found between artifacts\n"
        );
    }

    #[test]
    fn render_mermaid_single_edge_with_relation() {
        let edges = vec![Edge {
            from: "PRD-001".to_string(),
            to: "RFC-001".to_string(),
            relation: "informs".to_string(),
        }];
        let output = render_mermaid(&edges);
        assert!(output.starts_with("graph LR\n"));
        assert!(output.contains("PRD-001 -->|informs| RFC-001"));
        // Uses classDef syntax, not old `style`
        assert!(output.contains("classDef"));
        assert!(output.contains("class "));
        assert!(!output.contains("\n    style "));
    }

    #[test]
    fn render_mermaid_multiple_edges_same_kind() {
        let edges = vec![
            Edge {
                from: "PRD-001".to_string(),
                to: "RFC-001".to_string(),
                relation: "informs".to_string(),
            },
            Edge {
                from: "PRD-002".to_string(),
                to: "RFC-001".to_string(),
                relation: "informs".to_string(),
            },
        ];
        let output = render_mermaid(&edges);
        // Both PRD nodes should appear in one `class` line for prdStyle
        assert!(output.contains("classDef prdStyle"));
        assert!(output.contains("classDef rfcStyle"));
    }

    #[test]
    fn render_mermaid_empty_relation_uses_plain_arrow() {
        let edges = vec![Edge {
            from: "PRD-001".to_string(),
            to: "EPIC-001".to_string(),
            relation: "belongs_to".to_string(),
        }];
        let output = render_mermaid(&edges);
        // belongs_to relation uses plain --> without label
        assert!(output.contains("PRD-001 --> EPIC-001"));
        assert!(!output.contains("|belongs_to|"));
    }

    #[test]
    fn render_mermaid_named_relation_uses_label_arrow() {
        let edges = vec![Edge {
            from: "RFC-001".to_string(),
            to: "PRD-001".to_string(),
            relation: "based_on".to_string(),
        }];
        let output = render_mermaid(&edges);
        assert!(output.contains("RFC-001 -->|based_on| PRD-001"));
    }
}
