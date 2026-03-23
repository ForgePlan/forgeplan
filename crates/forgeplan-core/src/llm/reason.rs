use serde::{Deserialize, Serialize};

use crate::config::LlmConfig;
use crate::llm::LlmClient;

const ADI_SYSTEM_PROMPT: &str = r#"You are a structured reasoning engine using the FPF ADI cycle (Abduction → Deduction → Induction).

Given an artifact (PRD, RFC, Problem Card, etc.), perform structured analysis:

## Phase 1: Abduction (Generate Hypotheses)
Generate 3+ distinct hypotheses or approaches. For each:
- State the hypothesis clearly
- Identify key assumptions
- Note what evidence would support or refute it

## Phase 2: Deduction (Evaluate Each)
For each hypothesis:
- Apply logical consequences
- Identify risks and failure modes
- Assess feasibility (Low/Medium/High)
- Note missing evidence

## Phase 3: Induction (Synthesize)
- Rank hypotheses by strength of evidence
- Identify the recommended approach
- State confidence level (Low/Medium/High)
- List remaining unknowns and next steps

Format your response as structured Markdown with clear ## headers for each phase.
Write in the same language as the artifact.

IMPORTANT: Return your analysis as JSON with this schema:
{
  "hypotheses": [{"id": "H1", "description": "...", "assumptions": ["..."], "confidence": "Low|Medium|High"}],
  "deductions": [{"hypothesis_id": "H1", "consequence": "...", "risks": ["..."], "feasibility": "Low|Medium|High"}],
  "evidence_needed": [{"for_hypothesis": "H1", "test": "...", "effort": "Low|Medium|High"}],
  "recommendation": "...",
  "confidence": "Low|Medium|High"
}
If you cannot produce JSON, fall back to the Markdown format above."#;

// --- ADI structured output types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdiHypothesis {
    pub id: String,
    pub description: String,
    #[serde(default)]
    pub assumptions: Vec<String>,
    #[serde(default = "default_confidence")]
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdiDeduction {
    pub hypothesis_id: String,
    pub consequence: String,
    #[serde(default)]
    pub risks: Vec<String>,
    #[serde(default = "default_feasibility")]
    pub feasibility: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdiEvidenceNeeded {
    pub for_hypothesis: String,
    pub test: String,
    #[serde(default = "default_effort")]
    pub effort: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdiOutput {
    #[serde(default)]
    pub hypotheses: Vec<AdiHypothesis>,
    #[serde(default)]
    pub deductions: Vec<AdiDeduction>,
    #[serde(default)]
    pub evidence_needed: Vec<AdiEvidenceNeeded>,
    #[serde(default)]
    pub recommendation: String,
    #[serde(default = "default_confidence")]
    pub confidence: String,
    /// Raw response if JSON parsing failed.
    #[serde(skip)]
    pub raw_markdown: Option<String>,
}

fn default_confidence() -> String {
    "Medium".to_string()
}
fn default_feasibility() -> String {
    "Medium".to_string()
}
fn default_effort() -> String {
    "Medium".to_string()
}

/// Try to parse LLM response as structured ADI JSON, with fallback to raw markdown.
pub fn parse_adi_output(response: &str) -> AdiOutput {
    // Strip code fences sequentially — each step feeds the next
    let s = response.trim();
    let s = s.strip_prefix("```json").map(|r| r.trim_start()).unwrap_or(s);
    let s = s.strip_prefix("```").map(|r| r.trim_start()).unwrap_or(s);
    let s = s.strip_suffix("```").map(|r| r.trim_end()).unwrap_or(s);
    // Find first '{' for cases where LLM adds text before JSON
    let cleaned = if s.starts_with('{') {
        s
    } else if let Some(pos) = s.find('{') {
        &s[pos..]
    } else {
        s
    };

    match serde_json::from_str::<AdiOutput>(cleaned) {
        Ok(mut output) => {
            output.raw_markdown = None;
            output
        }
        Err(_) => {
            // Fallback: return raw markdown
            AdiOutput {
                hypotheses: vec![],
                deductions: vec![],
                evidence_needed: vec![],
                recommendation: String::new(),
                confidence: "Unknown".to_string(),
                raw_markdown: Some(response.to_string()),
            }
        }
    }
}

/// Search FPF knowledge base for patterns relevant to the artifact, and build
/// a context string for injection into the ADI system prompt.
pub async fn build_fpf_context(
    store: &crate::db::store::LanceStore,
    artifact_title: &str,
    _artifact_body: &str,
) -> anyhow::Result<Option<String>> {
    if !store.has_fpf() {
        return Ok(None);
    }

    // Search by significant title keywords (skip short/common words, try each)
    let keywords: Vec<&str> = artifact_title
        .split_whitespace()
        .filter(|w| w.len() > 3 && w.chars().all(|c| c.is_alphanumeric()))
        .filter(|w| !matches!(w.to_lowercase().as_str(), "the" | "and" | "for" | "with" | "from" | "that" | "this"))
        .take(5)
        .collect();

    // Try each keyword, collect unique results
    let mut results = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();
    for keyword in &keywords {
        if let Ok(hits) = store.search_fpf(keyword, 3).await {
            for hit in hits {
                if seen_ids.insert(hit.section_id.clone()) {
                    results.push(hit);
                }
            }
        }
        if results.len() >= 3 {
            break;
        }
    }
    results.truncate(3);

    if results.is_empty() {
        return Ok(None);
    }

    let mut context = String::from("\n\n## Relevant FPF Patterns (from knowledge base)\n\n");
    for chunk in &results {
        // Include first 300 chars of each section
        let preview: String = chunk.body.chars().take(300).collect();
        context.push_str(&format!(
            "### {} — {}\n{}\n\n",
            chunk.section_id, chunk.title, preview
        ));
    }

    Ok(Some(context))
}

/// Run ADI reasoning cycle on an artifact.
/// Returns both the raw response string and the parsed AdiOutput.
pub async fn reason(
    config: &LlmConfig,
    artifact_id: &str,
    artifact_title: &str,
    artifact_kind: &str,
    artifact_body: &str,
    fpf_context: Option<&str>,
) -> anyhow::Result<(String, AdiOutput)> {
    let client = LlmClient::new(config.clone());

    let mut prompt = format!(
        "Analyze this {kind} artifact using the ADI cycle:\n\n\
         **ID**: {id}\n\
         **Title**: {title}\n\n\
         ---\n\n\
         {body}",
        kind = artifact_kind,
        id = artifact_id,
        title = artifact_title,
        body = artifact_body,
    );

    if let Some(ctx) = fpf_context {
        prompt.push_str(ctx);
    }

    let system = crate::llm::load_prompt("reason", ADI_SYSTEM_PROMPT);
    let response = client.generate(&prompt, Some(&system)).await?;
    let adi_output = parse_adi_output(&response);
    Ok((response, adi_output))
}
