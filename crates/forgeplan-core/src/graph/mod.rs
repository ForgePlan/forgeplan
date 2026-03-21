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
