use serde::{Deserialize, Serialize};

use crate::config::LlmConfig;
use crate::db::auxiliary::AuxiliaryStore;
use crate::llm::LlmClient;

const ADI_SYSTEM_PROMPT: &str = r#"You are a structured reasoning engine using the FPF ADI cycle (Abduction → Deduction → Induction).

Given an artifact (PRD, RFC, Problem Card, etc.) WITH its project context (status, depth, relations, architecture), perform structured analysis.

CRITICAL RULES:
- Use the provided project context to ground your hypotheses in reality. Do NOT propose approaches that contradict the existing architecture.
- Every confidence rating MUST include a 1-2 sentence justification explaining WHY that level was chosen.
- If relations show this artifact depends on or is informed by other artifacts, reference them.

## Phase 1: Abduction (Generate Hypotheses)
Generate 3+ distinct hypotheses or approaches. For each:
- State the hypothesis clearly
- Identify key assumptions
- Rate confidence with justification (e.g., "High — aligns with existing LanceDB storage layer")

## Phase 2: Deduction (Evaluate Each)
For each hypothesis:
- Apply logical consequences considering the existing codebase
- Identify risks and failure modes
- Assess feasibility (Low/Medium/High) with justification
- Note missing evidence

## Phase 3: Induction (Synthesize)
- Rank hypotheses by strength of evidence
- Identify the recommended approach
- State confidence level with justification
- List remaining unknowns and next steps

Write in the same language as the artifact.

IMPORTANT: Return your analysis as JSON with this schema:
{
  "hypotheses": [{"id": "H1", "description": "...", "assumptions": ["..."], "confidence": "High — justification here"}],
  "deductions": [{"hypothesis_id": "H1", "consequence": "...", "risks": ["..."], "feasibility": "High — justification here"}],
  "evidence_needed": [{"for_hypothesis": "H1", "test": "...", "effort": "Low|Medium|High"}],
  "recommendation": "...",
  "confidence": "High — justification here"
}
If you cannot produce JSON, fall back to Markdown."#;

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
    let s = s
        .strip_prefix("```json")
        .map(|r| r.trim_start())
        .unwrap_or(s);
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

    build_fpf_context_from_search(artifact_title, |keyword| store.search_fpf(keyword, 3)).await
}

/// Build the ADI knowledge-base context without opening an artifact index.
///
/// Returns `None` for an absent KB or no matching title keywords. Search errors
/// propagate rather than presenting an unreadable KB as an empty one.
pub async fn build_fpf_context_auxiliary(
    store: &AuxiliaryStore,
    artifact_title: &str,
    _artifact_body: &str,
) -> anyhow::Result<Option<String>> {
    if !store.has_fpf() {
        return Ok(None);
    }
    build_fpf_context_from_search(artifact_title, |keyword| store.search_fpf(keyword, 3)).await
}

async fn build_fpf_context_from_search<'a, Search, SearchFuture>(
    artifact_title: &'a str,
    mut search: Search,
) -> anyhow::Result<Option<String>>
where
    Search: FnMut(&'a str) -> SearchFuture,
    SearchFuture: Future<Output = anyhow::Result<Vec<crate::db::store::FpfChunk>>>,
{
    // Keep the title-only selection and three-section prompt budget identical
    // for both store handles.
    let keywords = artifact_title
        .split_whitespace()
        .filter(|w| w.len() > 3 && w.chars().all(|c| c.is_alphanumeric()))
        .filter(|w| {
            !matches!(
                w.to_lowercase().as_str(),
                "the" | "and" | "for" | "with" | "from" | "that" | "this"
            )
        })
        .take(5);

    let mut results = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();
    for keyword in keywords {
        for hit in search(keyword).await? {
            if seen_ids.insert(hit.section_id.clone()) {
                results.push(hit);
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

/// Artifact metadata for enriching the ADI prompt context.
#[derive(Debug, Clone, Default)]
pub struct ArtifactContext {
    pub status: String,
    pub depth: String,
    pub r_eff_score: f64,
    /// Related artifacts: (target_id, relation_type, title)
    pub relations: Vec<(String, String, String)>,
    /// Brief project architecture summary
    pub architecture_hint: Option<String>,
    /// Bounded context info: (cluster_name, member_count, cohesion)
    pub bounded_context: Option<(String, usize, f64)>,
}

/// Build the metadata section for the ADI user prompt.
pub fn build_metadata_section(ctx: &ArtifactContext) -> String {
    let mut section = String::from("\n\n## Project Context\n\n");
    section.push_str(&format!("- **Status**: {}\n", ctx.status));
    section.push_str(&format!("- **Depth**: {}\n", ctx.depth));
    section.push_str(&format!("- **R_eff score**: {:.2}\n", ctx.r_eff_score));

    if !ctx.relations.is_empty() {
        section.push_str("\n### Relations\n\n");
        for (target, rel_type, title) in &ctx.relations {
            if title.is_empty() {
                section.push_str(&format!("- {} ({})\n", target, rel_type));
            } else {
                section.push_str(&format!("- {} — \"{}\" ({})\n", target, title, rel_type));
            }
        }
    }

    if let Some((name, members, cohesion)) = &ctx.bounded_context {
        section.push_str(&format!(
            "\n### Bounded Context\n\n- **Cluster**: {}\n- **Members**: {} artifacts\n- **Cohesion**: {:.0}%\n",
            name, members, cohesion * 100.0
        ));
    }

    if let Some(hint) = &ctx.architecture_hint {
        section.push_str(&format!("\n### Architecture\n\n{}\n", hint));
    }

    section
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
    artifact_context: Option<&ArtifactContext>,
) -> anyhow::Result<(String, AdiOutput)> {
    // Lower temperature for structured reasoning — less creative noise
    // Uses reason_temperature from config if set, otherwise caps at 0.3
    let mut reason_config = config.clone();
    reason_config.temperature = config
        .reason_temperature
        .unwrap_or_else(|| config.temperature.min(0.3));
    // PROB-096: ADI is not a short call. Measured 237s against the 120s
    // general budget, so `reason` used to fail by timeout on a correct
    // configuration — and the error pointed at the config, which was fine.
    reason_config.timeout_seconds = config
        .reason_timeout_seconds
        .unwrap_or_else(crate::config::types::default_reason_timeout_seconds);
    let client = LlmClient::new(reason_config);

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

    if let Some(ctx) = artifact_context {
        prompt.push_str(&build_metadata_section(ctx));
    }

    if let Some(ctx) = fpf_context {
        prompt.push_str(ctx);
    }

    let system = crate::llm::load_prompt("reason", ADI_SYSTEM_PROMPT);
    let response = client.generate(&prompt, Some(&system)).await?;
    let adi_output = parse_adi_output(&response);
    Ok((response, adi_output))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn auxiliary_fpf_context_matches_legacy_keyword_selection_and_prompt_budget() {
        use crate::db::store::{FpfChunk, LanceStore};

        let tmp = tempfile::TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let auxiliary = AuxiliaryStore::open_or_create(&ws).await.unwrap();
        let long_body = "é".repeat(320);
        let chunks: Vec<FpfChunk> = [
            ("trust", "A.1", "Trust anchor", long_body.as_str()),
            ("repeat", "A.1", "Context repeat", "Duplicate section."),
            ("context", "B.1", "Context boundary", "Second body."),
            ("ledger", "C.1", "Ledger budget", "Third body."),
            ("signals", "D.1", "Signals ignored", "Fourth body."),
            ("purpose", "E.1", "Purpose ignored", "Fifth body."),
            ("excluded", "F.1", "Excluded ignored", "Sixth body."),
        ]
        .into_iter()
        .map(|(id, section_id, title, body)| FpfChunk {
            id: id.to_string(),
            section_id: section_id.to_string(),
            parent_section: None,
            title: title.to_string(),
            body: body.to_string(),
            line_count: 1,
            file_path: format!("{id}.md"),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        })
        .collect();
        auxiliary.insert_fpf_chunks(&chunks, None).await.unwrap();
        let title = "The Trust with Context Ledger Signals Purpose Excluded";
        let expected = format!(
            "\n\n## Relevant FPF Patterns (from knowledge base)\n\n\
             ### A.1 — Trust anchor\n{}\n\n\
             ### B.1 — Context boundary\nSecond body.\n\n\
             ### C.1 — Ledger budget\nThird body.\n\n",
            "é".repeat(300),
        );
        assert_eq!(
            build_fpf_context_auxiliary(&auxiliary, title, "Unused body.")
                .await
                .unwrap(),
            Some(expected.clone()),
        );
        for table in ["artifacts.lance", "evidence.lance", "relations.lance"] {
            assert!(!ws.join("lance").join(table).exists());
        }

        let legacy_ws = tmp.path().join("legacy/.forgeplan");
        let guard = crate::workspace::lock::acquire_workspace_lock(&legacy_ws).await.unwrap();
        let legacy = LanceStore::init_new(&legacy_ws, &guard).await.unwrap();
        legacy.insert_fpf_chunks(&chunks, None).await.unwrap();
        assert_eq!(
            build_fpf_context(&legacy, title, "Unused body.")
                .await
                .unwrap(),
            Some(expected),
        );
        for title in [
            "the with from that this",
            "Trust,",
            "Alpha Beta Gamma Delta Epsilon Trust",
        ] {
            assert_eq!(
                build_fpf_context_auxiliary(&auxiliary, title, "Trust Context Ledger")
                    .await
                    .unwrap(),
                None,
                "only the first five significant, alphanumeric title words are searched",
            );
            assert_eq!(
                build_fpf_context(&legacy, title, "Trust Context Ledger")
                    .await
                    .unwrap(),
                None,
            );
        }
    }

    #[tokio::test]
    async fn auxiliary_fpf_context_distinguishes_absent_and_empty_knowledge_bases() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let absent = AuxiliaryStore::open_existing(&ws).await.unwrap();
        assert!(!absent.has_fpf());
        assert_eq!(
            build_fpf_context_auxiliary(&absent, "Trust context", "Trust")
                .await
                .unwrap(),
            None,
        );
        assert!(!ws.join("lance").exists());

        let empty = AuxiliaryStore::open_or_create(&ws).await.unwrap();
        assert!(empty.has_fpf());
        assert_eq!(
            build_fpf_context_auxiliary(&empty, "Trust context", "Trust")
                .await
                .unwrap(),
            None,
        );
        for table in ["artifacts.lance", "evidence.lance", "relations.lance"] {
            assert!(!ws.join("lance").join(table).exists());
        }
    }

    #[test]
    fn build_metadata_section_includes_all_fields() {
        let ctx = ArtifactContext {
            status: "active".to_string(),
            depth: "standard".to_string(),
            r_eff_score: 0.85,
            relations: vec![
                (
                    "RFC-001".to_string(),
                    "implements".to_string(),
                    "CLI Architecture".to_string(),
                ),
                (
                    "EPIC-001".to_string(),
                    "parent".to_string(),
                    "Build Forgeplan".to_string(),
                ),
            ],
            architecture_hint: Some("Rust CLI with LanceDB".to_string()),
            bounded_context: None,
        };
        let section = build_metadata_section(&ctx);
        assert!(section.contains("**Status**: active"));
        assert!(section.contains("**Depth**: standard"));
        assert!(section.contains("**R_eff score**: 0.85"));
        assert!(section.contains("RFC-001"));
        assert!(section.contains("implements"));
        assert!(section.contains("CLI Architecture"));
        assert!(section.contains("EPIC-001"));
        assert!(section.contains("Build Forgeplan"));
        assert!(section.contains("Rust CLI with LanceDB"));
    }

    #[test]
    fn build_metadata_section_relation_without_title() {
        let ctx = ArtifactContext {
            status: "draft".to_string(),
            depth: "tactical".to_string(),
            r_eff_score: 0.0,
            relations: vec![("PRD-001".to_string(), "informs".to_string(), String::new())],
            architecture_hint: None,
            bounded_context: None,
        };
        let section = build_metadata_section(&ctx);
        assert!(section.contains("PRD-001 (informs)"));
        assert!(!section.contains("\"\""));
    }

    #[test]
    fn build_metadata_section_no_relations() {
        let ctx = ArtifactContext {
            status: "draft".to_string(),
            depth: "tactical".to_string(),
            r_eff_score: 0.0,
            relations: vec![],
            architecture_hint: None,
            bounded_context: None,
        };
        let section = build_metadata_section(&ctx);
        assert!(section.contains("**Status**: draft"));
        assert!(!section.contains("### Relations"));
        assert!(!section.contains("### Architecture"));
    }

    #[test]
    fn build_metadata_section_zero_r_eff() {
        let ctx = ArtifactContext::default();
        let section = build_metadata_section(&ctx);
        assert!(section.contains("**R_eff score**: 0.00"));
    }

    #[test]
    fn build_metadata_section_with_bounded_context() {
        let ctx = ArtifactContext {
            status: "active".to_string(),
            depth: "standard".to_string(),
            r_eff_score: 0.5,
            relations: vec![],
            architecture_hint: None,
            bounded_context: Some(("Context-1 (PRD)".to_string(), 5, 0.8)),
        };
        let section = build_metadata_section(&ctx);
        assert!(section.contains("### Bounded Context"));
        assert!(section.contains("**Cluster**: Context-1 (PRD)"));
        assert!(section.contains("**Members**: 5 artifacts"));
        assert!(section.contains("**Cohesion**: 80%"));
    }

    #[test]
    fn parse_adi_output_valid_json() {
        let json = r#"{"hypotheses":[{"id":"H1","description":"test","assumptions":[],"confidence":"High — good reason"}],"deductions":[],"evidence_needed":[],"recommendation":"do H1","confidence":"High — justified"}"#;
        let output = parse_adi_output(json);
        assert!(output.raw_markdown.is_none());
        assert_eq!(output.hypotheses.len(), 1);
        assert_eq!(output.hypotheses[0].id, "H1");
        assert!(output.confidence.contains("High"));
    }

    #[test]
    fn parse_adi_output_with_code_fence() {
        let json = "```json\n{\"hypotheses\":[],\"deductions\":[],\"evidence_needed\":[],\"recommendation\":\"none\",\"confidence\":\"Low\"}\n```";
        let output = parse_adi_output(json);
        assert!(output.raw_markdown.is_none());
        assert_eq!(output.confidence, "Low");
    }

    #[test]
    fn parse_adi_output_invalid_falls_back() {
        let bad = "This is not JSON at all, just free text analysis.";
        let output = parse_adi_output(bad);
        assert!(output.raw_markdown.is_some());
        assert!(output.hypotheses.is_empty());
    }
}
