/// Infer work domain from artifact content for grade profile lookup.
/// Priority: frontmatter `domain:` field > keyword inference from title+body > "default".
pub fn infer_domain(title: &str, body: &str) -> String {
    // 1. Try frontmatter domain: field
    if let Some(domain) = extract_frontmatter_domain(body) {
        let d = domain.to_lowercase();
        if !d.contains('/') && !d.is_empty() && d != "general" {
            return d;
        }
    }

    // 2. Keyword inference from title + body (skip frontmatter, take 1000 chars of content)
    let content = body.split("---").skip(2).collect::<Vec<_>>().join(" ");
    let snippet: String = content.chars().take(1000).collect();
    let text = format!("{} {}", title, snippet).to_lowercase();

    let domains = [
        ("devops", &["k8s", "docker", "ci/cd", "deploy", "helm", "terraform", "kubernetes",
            "pipeline", "infrastructure", "namespace", "registry", "runner"][..]),
        ("frontend", &["react", "css", "ui", "component", "layout", "frontend", "tailwind",
            "responsive", "browser", "dom", "jsx", "tsx", "next.js"][..]),
        ("ai_ml", &["llm", "embedding", "model", "prompt", "ml", "ai", "vector",
            "semantic", "scoring", "neural", "training", "inference"][..]),
        ("backend", &["api", "database", "endpoint", "service", "backend", "crud",
            "rest", "graphql", "grpc", "migration", "schema", "query"][..]),
    ];

    let mut best_domain = "default";
    let mut best_score = 0usize;

    for (domain, keywords) in &domains {
        let score = keywords.iter().filter(|kw| text.contains(**kw)).count();
        if score > best_score {
            best_score = score;
            best_domain = domain;
        }
    }

    best_domain.to_string()
}

/// Extract `domain:` value from YAML frontmatter in body.
fn extract_frontmatter_domain(body: &str) -> Option<String> {
    let mut in_frontmatter = false;
    for line in body.lines().take(30) {
        let trimmed = line.trim();
        if trimmed == "---" {
            if !in_frontmatter {
                in_frontmatter = true;
                continue;
            } else {
                break;
            }
        }
        if in_frontmatter && trimmed.starts_with("domain:") {
            let value = trimmed[7..].trim().trim_matches('"').trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_backend_domain() {
        assert_eq!(infer_domain("API endpoint for users", "---\n---\nREST api database query"), "backend");
    }

    #[test]
    fn test_infer_frontend_domain() {
        assert_eq!(infer_domain("React component", "---\n---\nreact jsx tailwind ui component"), "frontend");
    }

    #[test]
    fn test_infer_from_frontmatter() {
        let body = "---\ndomain: devops\n---\nSome content";
        assert_eq!(infer_domain("Generic title", body), "devops");
    }

    #[test]
    fn test_infer_default() {
        assert_eq!(infer_domain("Something generic", "---\n---\nno keywords here"), "default");
    }
}
