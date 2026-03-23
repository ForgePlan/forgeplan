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
    // Also try finding first '{' for cases where LLM adds text before JSON
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

/// Run ADI reasoning cycle on an artifact.
/// Returns both the raw response string and the parsed AdiOutput.
pub async fn reason(
    config: &LlmConfig,
    artifact_id: &str,
    artifact_title: &str,
    artifact_kind: &str,
    artifact_body: &str,
) -> anyhow::Result<(String, AdiOutput)> {
    let client = LlmClient::new(config.clone());

    let prompt = format!(
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

    let system = crate::llm::load_prompt("reason", ADI_SYSTEM_PROMPT);
    let response = client.generate(&prompt, Some(&system)).await?;
    let adi_output = parse_adi_output(&response);
    Ok((response, adi_output))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_adi_valid_json() {
        let json = r#"{"hypotheses":[{"id":"H1","description":"test"}],"recommendation":"use H1","confidence":"High"}"#;
        let out = parse_adi_output(json);
        assert!(out.raw_markdown.is_none());
        assert_eq!(out.hypotheses.len(), 1);
        assert_eq!(out.hypotheses[0].id, "H1");
        assert_eq!(out.recommendation, "use H1");
        assert_eq!(out.confidence, "High");
    }

    #[test]
    fn parse_adi_json_in_backticks() {
        let input = "```json\n{\"hypotheses\":[],\"recommendation\":\"none\",\"confidence\":\"Low\"}\n```";
        let out = parse_adi_output(input);
        assert!(out.raw_markdown.is_none());
        assert_eq!(out.confidence, "Low");
        assert_eq!(out.recommendation, "none");
    }

    #[test]
    fn parse_adi_malformed_json_falls_back() {
        let input = "{broken json here!!!";
        let out = parse_adi_output(input);
        assert!(out.raw_markdown.is_some());
        assert!(out.hypotheses.is_empty());
        assert_eq!(out.confidence, "Unknown");
    }

    #[test]
    fn parse_adi_empty_string_no_panic() {
        let out = parse_adi_output("");
        assert!(out.raw_markdown.is_some());
        assert!(out.hypotheses.is_empty());
    }

    #[test]
    fn parse_adi_partial_fields_uses_defaults() {
        let json = r#"{"hypotheses":[]}"#;
        let out = parse_adi_output(json);
        assert!(out.raw_markdown.is_none());
        assert_eq!(out.confidence, "Medium"); // default
        assert!(out.recommendation.is_empty()); // default empty
    }

    #[test]
    fn parse_adi_text_before_json() {
        let input = "Here is my analysis:\n\n{\"hypotheses\":[{\"id\":\"H1\",\"description\":\"d\"}],\"recommendation\":\"ok\"}";
        let out = parse_adi_output(input);
        assert!(out.raw_markdown.is_none());
        assert_eq!(out.hypotheses.len(), 1);
    }
}
