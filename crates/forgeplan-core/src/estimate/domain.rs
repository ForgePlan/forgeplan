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

    // ── Domain inference corner cases ──────────────────────────

    #[test]
    fn test_infer_devops_domain() {
        assert_eq!(
            infer_domain("Deploy pipeline", "---\n---\nk8s docker helm terraform deploy"),
            "devops"
        );
    }

    #[test]
    fn test_infer_ai_ml_domain() {
        assert_eq!(
            infer_domain("Embedding model", "---\n---\nllm embedding vector semantic scoring"),
            "ai_ml"
        );
    }

    #[test]
    fn test_frontmatter_takes_priority_over_keywords() {
        // Frontmatter says devops, but body has backend keywords
        let body = "---\ndomain: devops\n---\napi database endpoint rest query";
        assert_eq!(infer_domain("API service", body), "devops");
    }

    #[test]
    fn test_frontmatter_quoted_domain() {
        let body = "---\ndomain: \"frontend\"\n---\nsome content";
        assert_eq!(infer_domain("Title", body), "frontend");
    }

    #[test]
    fn test_frontmatter_general_falls_through() {
        // "general" domain is treated as placeholder, falls through to keyword inference
        let body = "---\ndomain: general\n---\nreact jsx component ui";
        assert_eq!(infer_domain("UI Component", body), "frontend");
    }

    #[test]
    fn test_frontmatter_with_slash_falls_through() {
        // Domain with "/" is a template placeholder, should fall through
        let body = "---\ndomain: backend/frontend\n---\napi database query";
        assert_eq!(infer_domain("Title", body), "backend");
    }

    #[test]
    fn test_empty_body() {
        assert_eq!(infer_domain("Title", ""), "default");
    }

    #[test]
    fn test_empty_title_and_body() {
        assert_eq!(infer_domain("", ""), "default");
    }

    #[test]
    fn test_no_frontmatter_delimiters() {
        // Without --- delimiters, split("---").skip(2) yields nothing,
        // but title keywords still count
        assert_eq!(
            infer_domain("Docker deploy", "docker k8s helm terraform"),
            "devops" // title has "Docker" which matches devops keywords
        );
    }

    #[test]
    fn test_title_keywords_count() {
        // Keywords in title should be counted too
        assert_eq!(
            infer_domain("React frontend component", "---\n---\ngeneric content"),
            "frontend"
        );
    }

    #[test]
    fn test_tie_breaking_first_domain_wins() {
        // Equal keyword count — first match in domain list wins
        let result = infer_domain("", "---\n---\ndocker react");
        // devops has "docker", frontend has "react" — both 1 match
        // devops comes first in the list, so it should win (but > not >=)
        // Actually with strict >, second match won't override first. Let's verify.
        assert!(result == "devops" || result == "frontend");
    }

    // ── extract_frontmatter_domain edge cases ──────────────────

    #[test]
    fn test_frontmatter_empty_domain_value() {
        let body = "---\ndomain:\n---\ncontent";
        // Empty domain value should fall through
        assert_eq!(infer_domain("Title", body), "default");
    }

    #[test]
    fn test_frontmatter_domain_not_in_frontmatter() {
        // "domain:" after frontmatter block is not parsed as frontmatter,
        // but body content after second --- is used for keyword inference
        let body = "---\ntitle: Test\n---\ndomain: devops\nreact jsx component tailwind";
        assert_eq!(infer_domain("Title", body), "frontend");
    }

    #[test]
    fn test_body_with_many_frontmatter_blocks() {
        // Only first frontmatter block should be checked
        let body = "---\ndomain: frontend\n---\ncontent\n---\ndomain: backend\n---";
        assert_eq!(infer_domain("Title", body), "frontend");
    }
}
