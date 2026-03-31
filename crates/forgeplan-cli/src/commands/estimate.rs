use std::collections::HashMap;

use anyhow::Result;

use forgeplan_core::estimate::{calculator, confidence, display, extractor, scorer};
use forgeplan_core::estimate::types::{Complexity, EstimateConfig, Grade, ItemSource, ScoredItem};

use crate::commands::common;

pub async fn run(
    id: &str,
    grade: Option<&str>,
    my_grade: bool,
    llm_score: bool,
    complexity_overrides: Option<&str>,
    json: bool,
) -> Result<()> {
    let store = common::store().await?;

    // Fetch artifact
    let record = store
        .get_record(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact '{}' not found", id))?;

    // Extract work items from artifact body
    let work_items = extractor::extract_work_items(&record.body);

    // Collect hints about artifact quality
    let hints = extractor::collect_hints(&record.body, work_items.len(), &record.kind);

    if work_items.is_empty() {
        if json {
            // In JSON mode, return empty result with hints
            let result = forgeplan_core::estimate::types::EstimateResult {
                artifact_id: record.id.clone(),
                artifact_title: record.title.clone(),
                items: vec![],
                totals: std::collections::HashMap::new(),
                total_score: 0.0,
                confidence: 0.0,
                confidence_reasons: vec![],
                hints,
            };
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            println!("  No estimable items found in {}.", id);
            for hint in &hints {
                let prefix = match hint.level {
                    forgeplan_core::estimate::types::HintLevel::Warning => "!",
                    forgeplan_core::estimate::types::HintLevel::Info => "i",
                    forgeplan_core::estimate::types::HintLevel::Suggestion => "*",
                };
                println!("  {} {}", prefix, hint.message);
                if let Some(ref action) = hint.action {
                    println!("    -> {}", action);
                }
            }
        }
        return Ok(());
    }

    // Score complexity: LLM L1 (opt-in) or rule-based L0 (default)
    let mut scored_items = if llm_score {
        let llm_config = common::require_llm_config()?;
        if !json {
            eprintln!(
                "  Using LLM scorer ({}/{}). Fallback to rules if LLM fails.",
                llm_config.provider, llm_config.model
            );
        }
        scorer::score_items_with_llm(&work_items, &llm_config).await
    } else {
        scorer::score_items(&work_items)
    };

    // Apply manual complexity overrides (highest priority per ADR-004)
    if let Some(overrides) = complexity_overrides {
        let map = parse_complexity_overrides(overrides)?;
        apply_overrides(&mut scored_items, &map);
        if !json {
            eprintln!("  Applied {} manual complexity override(s)", map.len());
        }
    }

    // Calculate confidence
    let fr_items: Vec<_> = work_items.iter().filter(|w| w.source == ItemSource::Fr).collect();
    let phase_items: Vec<_> = work_items.iter().filter(|w| w.source == ItemSource::Phase).collect();

    // Check linked Spec and Evidence for confidence boost
    let relations = store.get_relations(&record.id).await.unwrap_or_default();
    let incoming = store.get_incoming_relations(&record.id).await.unwrap_or_default();
    let all_rels: Vec<_> = relations.iter().chain(incoming.iter()).collect();

    let has_spec = all_rels.iter().any(|(target, _)| target.to_uppercase().starts_with("SPEC-"));
    let has_evidence = all_rels.iter().any(|(target, _)| target.to_uppercase().starts_with("EVID-"));

    let (conf, conf_reasons) = confidence::score_confidence(
        !fr_items.is_empty(),
        fr_items.len(),
        !phase_items.is_empty(),
        phase_items.len(),
        has_spec,
        has_evidence,
    );

    // Build config from .forgeplan/config.yaml or defaults
    let config = load_estimate_config();

    // Calculate hours
    let result = calculator::calculate(
        &record.id,
        &record.title,
        &scored_items,
        &config,
        conf,
        conf_reasons,
        hints,
    );

    // Determine highlight grade
    let highlight_grade = if my_grade {
        let domain = if llm_score {
            // LLM-assisted domain inference when --llm-score is active
            let llm_config = common::config().ok().and_then(|c| c.llm);
            infer_domain_with_llm(&record.title, &record.body, llm_config.as_ref()).await
        } else {
            infer_domain(&record.title, &record.body)
        };
        let resolved = config.resolve_grade(&domain);
        if !json {
            eprintln!(
                "  Using grade: {} (domain: {}, from config grade_profile)",
                resolved, domain
            );
        }
        Some(resolved)
    } else if let Some(g) = grade {
        Some(g.parse::<Grade>().map_err(|e| anyhow::anyhow!("{}", e))?)
    } else {
        None
    };

    // Output
    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        print!("{}", display::format_table(&result, highlight_grade));
    }

    Ok(())
}

/// Parse "FR-001=5,FR-002=3" into HashMap<String, Complexity>.
fn parse_complexity_overrides(input: &str) -> Result<HashMap<String, Complexity>> {
    let mut map = HashMap::new();
    for pair in input.split(',') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        let parts: Vec<&str> = pair.splitn(2, '=').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid complexity override '{}'. Format: FR-001=5", pair);
        }
        let id = parts[0].trim().to_string();
        let value: u32 = parts[1].trim().parse()
            .map_err(|_| anyhow::anyhow!("Invalid number '{}' in complexity override", parts[1].trim()))?;
        let complexity = Complexity::from_value(value)
            .ok_or_else(|| anyhow::anyhow!(
                "Invalid Fibonacci value {}. Valid: 1, 2, 3, 5, 8, 13", value
            ))?;
        map.insert(id, complexity);
    }
    Ok(map)
}

/// Apply manual overrides to scored items — override has highest priority.
fn apply_overrides(items: &mut [ScoredItem], overrides: &HashMap<String, Complexity>) {
    for item in items.iter_mut() {
        if let Some(complexity) = overrides.get(&item.id) {
            item.complexity = *complexity;
        }
    }
}

/// Load EstimateConfig from .forgeplan/config.yaml, falling back to defaults.
fn load_estimate_config() -> EstimateConfig {
    match common::config() {
        Ok(cfg) => {
            if let Some(ref yaml) = cfg.estimate {
                EstimateConfig::from_yaml(yaml)
            } else {
                EstimateConfig::default()
            }
        }
        Err(_) => EstimateConfig::default(),
    }
}

/// LLM-assisted domain inference. Falls back to keyword-based if LLM unavailable.
async fn infer_domain_with_llm(
    title: &str,
    body: &str,
    llm_config: Option<&forgeplan_core::config::types::LlmConfig>,
) -> String {
    // Try frontmatter first (same as non-LLM path)
    if let Some(domain) = extract_frontmatter_domain(body) {
        let d = domain.to_lowercase();
        if !d.contains('/') && !d.is_empty() && d != "general" {
            return d;
        }
    }

    // Try LLM classification
    if let Some(config) = llm_config {
        let snippet: String = body.chars().take(300).collect();
        let prompt = format!(
            "Classify this artifact into exactly ONE domain. Reply with ONLY the domain name, nothing else.\n\
             Domains: backend, frontend, devops, ai_ml, default\n\n\
             Title: {}\nBody: {}",
            title, snippet
        );
        {
            let client = forgeplan_core::llm::LlmClient::new(config.clone());
            if let Ok(response) = client.generate(&prompt, Some("You are a domain classifier. Reply with exactly one word.")).await {
                let domain = response.trim().to_lowercase().replace(' ', "_");
                let valid = ["backend", "frontend", "devops", "ai_ml"];
                if valid.contains(&domain.as_str()) {
                    return domain;
                }
            }
        }
    }

    // Fallback to keyword-based
    infer_domain(title, body)
}

/// Infer work domain from artifact content for grade profile lookup.
/// Priority: frontmatter `domain:` field > keyword inference from title+body > "default".
fn infer_domain(title: &str, body: &str) -> String {
    // 1. Try frontmatter domain: field
    if let Some(domain) = extract_frontmatter_domain(body) {
        let d = domain.to_lowercase();
        // Skip template placeholders
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
    for line in body.lines().take(30) {
        let trimmed = line.trim();
        if trimmed.starts_with("domain:") {
            let value = trimmed[7..].trim().trim_matches('"').trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}
