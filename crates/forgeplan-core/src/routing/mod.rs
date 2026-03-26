//! Smart Routing v2 — rule-based depth calibration and pipeline suggestion.
//!
//! Level 0: deterministic keyword rules (offline, instant).
//! Level 1: LLM-classified with graceful fallback to Level 0.

pub mod pipeline;
pub mod rules;
pub mod signals;

use crate::artifact::types::{ArtifactKind, Mode};
use crate::config::LlmConfig;

/// Result of routing a task description through the rule engine.
#[derive(Debug, Clone)]
pub struct RoutingResult {
    /// Computed depth level.
    pub depth: Mode,
    /// Ordered pipeline of artifact types to create.
    pub pipeline: Vec<ArtifactKind>,
    /// Signals that contributed to the depth decision.
    pub triggers: Vec<Signal>,
    /// Confidence score (0.0-1.0). More matching signals = higher confidence.
    pub confidence: f64,
    /// Routing level: 0 = keywords (rule-based), 1 = LLM-classified.
    pub level: u8,
    /// LLM explanation of the routing decision (only present at level 1).
    pub explanation: Option<String>,
}

/// A signal extracted from input that influences depth.
#[derive(Debug, Clone)]
pub struct Signal {
    /// Signal identifier (e.g., "keyword:security", "complexity:fr_count").
    pub id: String,
    /// Human-readable description.
    pub description: String,
    /// Minimum depth this signal requires.
    pub minimum_depth: Mode,
    /// Signal weight for confidence calculation.
    pub weight: f64,
}

impl std::fmt::Display for RoutingResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level_label = match self.level {
            0 => "Level 0 (keywords)",
            1 => "Level 1 (LLM)",
            _ => "Unknown",
        };
        writeln!(f, "## Level: {}", level_label)?;
        writeln!(f)?;
        writeln!(f, "## Depth: {}", depth_display(&self.depth))?;
        writeln!(f)?;

        writeln!(f, "## Pipeline")?;
        if self.pipeline.is_empty() {
            writeln!(f, "None (tactical — just do it)")?;
        } else {
            let names: Vec<&str> = self.pipeline.iter().map(|k| kind_display(k)).collect();
            writeln!(f, "{}", names.join(" → "))?;
        }
        writeln!(f)?;

        writeln!(f, "## Triggers Matched")?;
        if self.triggers.is_empty() {
            writeln!(f, "No escalation triggers — defaults to Tactical")?;
        } else {
            for t in &self.triggers {
                writeln!(
                    f,
                    "- **{}**: {} → {}+",
                    t.id,
                    t.description,
                    depth_display(&t.minimum_depth)
                )?;
            }
        }
        writeln!(f)?;

        writeln!(
            f,
            "## Confidence: {:.0}%",
            self.confidence * 100.0
        )?;

        if !self.pipeline.is_empty() {
            writeln!(f)?;
            writeln!(f, "## Next Step")?;
            let first = kind_display(&self.pipeline[0]);
            writeln!(f, "```")?;
            writeln!(
                f,
                "forgeplan new {} \"<title>\"",
                first.to_lowercase()
            )?;
            writeln!(f, "```")?;
        }

        if let Some(ref explanation) = self.explanation {
            writeln!(f)?;
            writeln!(f, "## Explanation")?;
            writeln!(f, "{}", explanation)?;
        }

        Ok(())
    }
}

/// Route a task description to depth + pipeline using rule engine.
pub fn route(description: &str) -> RoutingResult {
    let trimmed = description.trim();
    if trimmed.is_empty() {
        return RoutingResult {
            depth: crate::artifact::types::Mode::Tactical,
            pipeline: vec![],
            triggers: vec![],
            confidence: 0.0,
            level: 0,
            explanation: None,
        };
    }

    let signals = signals::extract(trimmed);
    let depth = rules::compute_depth(&signals);
    let pipeline = pipeline::for_depth(&depth);
    let confidence = rules::compute_confidence(&signals, &depth);

    RoutingResult {
        depth,
        pipeline,
        triggers: signals,
        confidence,
        level: 0,
        explanation: None,
    }
}

/// Route an existing artifact (post-factum calibration).
pub fn calibrate_artifact(body: &str, link_count: usize, has_epic: bool) -> RoutingResult {
    let mut signals = signals::extract(body);
    signals.extend(signals::extract_structural(body, link_count, has_epic));
    let depth = rules::compute_depth(&signals);
    let pipeline = pipeline::for_depth(&depth);
    let confidence = rules::compute_confidence(&signals, &depth);

    RoutingResult {
        depth,
        pipeline,
        triggers: signals,
        confidence,
        level: 0,
        explanation: None,
    }
}

/// Route a task description using LLM classification (Level 1) with fallback to Level 0.
///
/// If the LLM call succeeds and returns a parseable depth, returns a Level 1 result.
/// On any error (no API key, network, unparseable response), falls back to Level 0 keywords.
///
/// `fpf_context` — optional FPF knowledge base context to inject into the LLM prompt.
/// Build it via `llm::reason::build_fpf_context()` if a LanceStore is available.
pub async fn route_with_llm(description: &str, llm_config: &LlmConfig) -> RoutingResult {
    route_with_llm_and_context(description, llm_config, None).await
}

/// Route with optional FPF context injection into the LLM prompt.
pub async fn route_with_llm_and_context(
    description: &str,
    llm_config: &LlmConfig,
    fpf_context: Option<&str>,
) -> RoutingResult {
    // Short-circuit: empty or very short descriptions don't need LLM
    if description.trim().len() < 3 {
        return route(description);
    }

    // Check if API key is available before making the call
    if llm_config.resolve_api_key().is_none() && !llm_config.provider.eq("ollama") {
        return route(description);
    }

    // Route-specific timeout: 15 seconds (vs 120s global LLM timeout).
    // Route should be fast — if LLM takes too long, fall back to keywords.
    let llm_future = crate::llm::route::route_with_context(llm_config, description, fpf_context);
    let timeout_result = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        llm_future,
    ).await;

    match timeout_result {
        Err(_elapsed) => {
            // Timeout — fallback to Level 0
            route(description)
        }
        Ok(llm_result) => match llm_result {
            Ok(response) => match parse_llm_route_response(&response) {
                Some((depth, explanation)) => {
                    let pipeline = pipeline::for_depth(&depth);
                    RoutingResult {
                        depth,
                        pipeline,
                        triggers: vec![],
                        confidence: 0.9,
                        level: 1,
                        explanation: Some(explanation),
                    }
                }
                None => route(description),
            },
            Err(_) => route(description),
        },
    }
}

/// Parse LLM route response markdown to extract depth and reasoning.
///
/// Expected format from llm/route.rs:
/// ```text
/// ## Depth: Tactical|Standard|Deep|Critical
/// ...
/// ## Reasoning
/// Some explanation text
/// ```
///
/// Returns (Mode, explanation_text) or None if depth cannot be parsed.
pub fn parse_llm_route_response(response: &str) -> Option<(Mode, String)> {
    let mut depth: Option<Mode> = None;
    let mut reasoning_lines: Vec<&str> = Vec::new();
    let mut in_reasoning = false;

    for line in response.lines() {
        let trimmed = line.trim();

        // Parse "## Depth: <level>" line
        if let Some(rest) = trimmed.strip_prefix("## Depth:") {
            let level_str = rest.trim().to_lowercase();
            depth = match level_str.as_str() {
                "tactical" => Some(Mode::Tactical),
                "standard" => Some(Mode::Standard),
                "deep" | "deep/critical" => Some(Mode::Deep),
                "critical" => Some(Mode::Deep), // Critical maps to Deep (Mode enum)
                "note" => Some(Mode::Note),
                _ => None,
            };
            in_reasoning = false;
            continue;
        }

        // Detect reasoning section
        if trimmed.starts_with("## Reasoning") {
            in_reasoning = true;
            continue;
        }

        // Stop reasoning at next section header
        if in_reasoning && trimmed.starts_with("## ") {
            in_reasoning = false;
            continue;
        }

        if in_reasoning && !trimmed.is_empty() {
            reasoning_lines.push(trimmed);
        }
    }

    let explanation = if reasoning_lines.is_empty() {
        // Use full response as explanation if no Reasoning section found
        response.to_string()
    } else {
        reasoning_lines.join("\n")
    };

    depth.map(|d| (d, explanation))
}

fn depth_display(mode: &Mode) -> &'static str {
    match mode {
        Mode::Note => "Note",
        Mode::Tactical => "Tactical",
        Mode::Standard => "Standard",
        Mode::Deep => "Deep/Critical",
    }
}

fn kind_display(kind: &ArtifactKind) -> &'static str {
    match kind {
        ArtifactKind::Epic => "Epic",
        ArtifactKind::Prd => "PRD",
        ArtifactKind::Spec => "Spec",
        ArtifactKind::Rfc => "RFC",
        ArtifactKind::Adr => "ADR",
        ArtifactKind::Note => "Note",
        ArtifactKind::ProblemCard => "Problem",
        ArtifactKind::SolutionPortfolio => "Solution",
        ArtifactKind::EvidencePack => "Evidence",
        ArtifactKind::RefreshReport => "Refresh",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routing_result_has_level_field() {
        let result = route("Fix a typo");
        assert_eq!(result.level, 0);
        assert!(result.explanation.is_none());
    }

    #[test]
    fn test_route_level0_unchanged() {
        // Existing behavior: security keyword → Deep
        let result = route("Implement OAuth2 authentication");
        assert_eq!(result.level, 0);
        assert!(result.explanation.is_none());
        assert!(matches!(result.depth, Mode::Deep));
        assert!(!result.pipeline.is_empty());
    }

    #[test]
    fn test_route_level0_empty_input() {
        let result = route("");
        assert_eq!(result.level, 0);
        assert!(matches!(result.depth, Mode::Tactical));
        assert!(result.pipeline.is_empty());
    }

    #[test]
    fn test_parse_llm_route_response_tactical() {
        let response = "## Depth: Tactical\n\n## Artifacts\n- None\n\n## Pipeline\nNone\n\n## Reasoning\nSimple typo fix, no artifacts needed.";
        let (depth, explanation) = parse_llm_route_response(response).unwrap();
        assert!(matches!(depth, Mode::Tactical));
        assert!(explanation.contains("typo fix"));
    }

    #[test]
    fn test_parse_llm_route_response_standard() {
        let response = "## Depth: Standard\n\n## Artifacts\n- PRD: requirements\n- RFC: design\n\n## Pipeline\nPRD → RFC\n\n## Reasoning\nFeature requires planning across multiple files.";
        let (depth, explanation) = parse_llm_route_response(response).unwrap();
        assert!(matches!(depth, Mode::Standard));
        assert!(explanation.contains("planning"));
    }

    #[test]
    fn test_parse_llm_route_response_deep() {
        let response = "## Depth: Deep\n\n## Reasoning\nSecurity-critical change requiring thorough review.";
        let (depth, explanation) = parse_llm_route_response(response).unwrap();
        assert!(matches!(depth, Mode::Deep));
        assert!(explanation.contains("Security"));
    }

    #[test]
    fn test_parse_llm_route_response_critical() {
        let response = "## Depth: Critical\n\n## Reasoning\nCross-team strategic initiative.";
        let (depth, _) = parse_llm_route_response(response).unwrap();
        // Critical maps to Mode::Deep (highest in Mode enum)
        assert!(matches!(depth, Mode::Deep));
    }

    #[test]
    fn test_parse_llm_route_response_malformed() {
        // No "## Depth:" line
        let response = "This is some random text without proper formatting.";
        assert!(parse_llm_route_response(response).is_none());
    }

    #[test]
    fn test_parse_llm_route_response_unknown_depth() {
        let response = "## Depth: SuperDeep\n\n## Reasoning\nSome text.";
        assert!(parse_llm_route_response(response).is_none());
    }

    #[test]
    fn test_parse_llm_route_response_no_reasoning_section() {
        // When no ## Reasoning section, full response is used as explanation
        let response = "## Depth: Standard\n\nSome extra context here.";
        let (depth, explanation) = parse_llm_route_response(response).unwrap();
        assert!(matches!(depth, Mode::Standard));
        assert!(explanation.contains("Depth: Standard"));
    }

    #[test]
    fn test_parse_llm_route_response_deep_critical_variant() {
        let response = "## Depth: Deep/Critical\n\n## Reasoning\nComplex module.";
        let (depth, _) = parse_llm_route_response(response).unwrap();
        assert!(matches!(depth, Mode::Deep));
    }

    #[test]
    fn test_route_with_llm_fallback_on_missing_key() {
        // route_with_llm with a config that has no API key should fallback to Level 0
        let config = LlmConfig {
            provider: "openai".into(),
            api_key_env: Some("NONEXISTENT_ENV_VAR_FOR_TEST_12345".into()),
            ..Default::default()
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(route_with_llm("Fix a typo in readme", &config));
        assert_eq!(result.level, 0, "Should fallback to Level 0 when no API key");
    }

    #[test]
    fn test_calibrate_artifact_has_level_zero() {
        let result = calibrate_artifact("## FR\n- [ ] FR-001\n- [ ] FR-002\n- [ ] FR-003\n- [ ] FR-004\n", 3, true);
        assert_eq!(result.level, 0);
        assert!(result.explanation.is_none());
    }

    #[test]
    fn test_display_includes_level() {
        let result = route("Simple task");
        let display = format!("{result}");
        assert!(display.contains("## Level: Level 0 (keywords)"));
    }

    #[test]
    fn test_route_with_llm_short_input_skips_llm() {
        // Very short inputs (<3 chars) should skip LLM and use Level 0
        let config = LlmConfig {
            provider: "openai".into(),
            api_key_env: Some("GEMINI_API_KEY".into()),
            ..Default::default()
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(route_with_llm("ab", &config));
        assert_eq!(result.level, 0, "Short input should skip LLM");
    }

    #[test]
    fn test_route_with_llm_empty_input_skips_llm() {
        let config = LlmConfig {
            provider: "openai".into(),
            api_key_env: Some("GEMINI_API_KEY".into()),
            ..Default::default()
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(route_with_llm("", &config));
        assert_eq!(result.level, 0, "Empty input should skip LLM");
    }
}
