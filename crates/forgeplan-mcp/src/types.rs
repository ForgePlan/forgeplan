use rmcp::schemars::{self, JsonSchema};
use serde::{Deserialize, Serialize};

// ── Shared DTOs ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ArtifactSummaryDto {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ArtifactRecordDto {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub title: String,
    pub body: String,
    pub depth: String,
    pub author: Option<String>,
    pub parent_epic: Option<String>,
    pub r_eff_score: f64,
    pub valid_until: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidationFindingDto {
    pub rule_id: String,
    pub severity: String,
    pub message: String,
    pub section: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidationResultDto {
    pub artifact_id: String,
    pub kind: String,
    pub depth: String,
    pub passed: bool,
    pub error_count: usize,
    pub warning_count: usize,
    pub findings: Vec<ValidationFindingDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProgressDto {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub total: usize,
    pub completed: usize,
    pub percent: f64,
}

// ── Response types ───────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct InitResponse {
    pub workspace: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DuplicateWarning {
    pub id: String,
    pub title: String,
    pub similarity: f64,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct NewArtifactResponse {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub filepath: String,
    /// Methodology hint: what to do next after creating this artifact.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _next_action: Option<String>,
    /// Duplicate warnings (FR-004 of PRD-043). Empty if no similar artifacts found.
    /// Artifact is still created — AI agent decides how to react.
    #[serde(default)]
    pub warnings: Vec<DuplicateWarning>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListResponse {
    pub artifacts: Vec<ArtifactSummaryDto>,
    pub total: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct StatusResponse {
    pub project: String,
    pub workspace: String,
    pub total: usize,
    pub by_kind: Vec<KindCount>,
    pub by_status: Vec<StatusCount>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct KindCount {
    pub kind: String,
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct StatusCount {
    pub status: String,
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ValidateResponse {
    pub results: Vec<ValidationResultDto>,
    pub total_artifacts: usize,
    pub total_passed: usize,
    pub total_errors: usize,
    pub total_warnings: usize,
    /// Methodology hint: what to do after validation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _next_action: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ScoreResponse {
    pub id: String,
    pub title: String,
    pub r_eff: f64,
    pub evidence: Vec<EvidenceDto>,
    // F-G-R enrichment (Wave 2)
    pub self_score: f64,
    pub formality: f64,
    pub granularity: f64,
    pub reliability: f64,
    pub overall_grade: String,
    pub weakest_link: Option<String>,
    pub factors: Vec<String>,
    pub decay_penalty: f64,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct EvidenceDto {
    pub id: String,
    pub verdict: String,
    pub congruence_level: u8,
    pub score: f64,
    pub expired: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LinkResponse {
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GraphResponse {
    pub mermaid: String,
}

// ── PRD-057 Inc 3: claim protocol DTOs ───────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ClaimParams {
    /// Artifact ID to claim (e.g. `PRD-057`). Normalized to uppercase on disk.
    pub id: String,
    /// Agent identity ("name/version" or free-form). Optional — defaults to
    /// the MCP caller's clientInfo when omitted. Providing this explicitly
    /// lets an orchestrator claim on behalf of a sub-agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// Time-to-live in minutes. Default 30, max 1440 (24h), min 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl_minutes: Option<u32>,
    /// Optional free-form note surfaced by `forgeplan_claims --active`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReleaseParams {
    pub id: String,
    /// Required unless `force=true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// Force-release regardless of holder (orchestrator escape hatch).
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ClaimsListParams {
    /// Reserved for future filters; currently always returns only live claims.
    #[serde(default)]
    pub active: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ClaimDto {
    pub id: String,
    pub agent_id: String,
    pub claimed_at: String,
    pub expires_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl From<forgeplan_core::claim::Claim> for ClaimDto {
    fn from(c: forgeplan_core::claim::Claim) -> Self {
        Self {
            id: c.id,
            agent_id: c.agent_id,
            claimed_at: c.claimed_at.to_rfc3339(),
            expires_at: c.expires_at.to_rfc3339(),
            note: c.note,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ClaimsListResponse {
    pub count: usize,
    /// R2 audit MED: number of claim files skipped because they failed to
    /// parse or exceeded the size cap. Orchestrators should investigate
    /// any non-zero value — silent dropping of these was an Inc 3 bug.
    #[serde(default)]
    pub skipped: usize,
    pub claims: Vec<ClaimDto>,
}

// ── PRD-057 Inc 4: dispatcher tool DTOs ──────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DispatchParams {
    /// Number of sub-agents the orchestrator can hand work to. Required;
    /// clamped to `>= 1` downstream.
    pub agents: usize,
    /// Optional filter: only consider artifacts of this kind
    /// (`prd`/`rfc`/`spec`/etc.). When omitted, all kinds are considered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Optional filter: only artifacts with this parent Epic ID. Matches
    /// the `parent_epic` frontmatter field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub epic: Option<String>,
    /// Optional filter: consider only artifacts in this status (default
    /// `draft`). Set to `"any"` to include every lifecycle state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Per-agent skill lists in index order. Empty entries default to
    /// "any skill"; omit to disable skill matching entirely.
    #[serde(default)]
    pub agent_skills: Vec<Vec<String>>,
    /// Jaccard threshold above which two artifacts are considered
    /// file-conflicting. Default 0.3. Clamp 0.0..=1.0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlap_threshold: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DispatchResponse {
    pub buckets: Vec<Vec<String>>,
    pub serial_queue: Vec<String>,
    pub reasoning: Vec<String>,
    pub generated_at: String,
    pub agent_count: usize,
    pub overlap_threshold: f64,
    pub candidate_count: usize,
    pub claimed_count: usize,
    /// R3 audit M-4: number of candidate artifact markdown files that
    /// couldn't be read/parsed and were dropped from the plan (not the
    /// same as "no affected_files declared"). Non-zero → check logs.
    #[serde(default)]
    pub skipped_parse_errors: usize,
    /// R3 audit task-completion MED (FR-003): number of candidates that
    /// were dropped because a structural dependency hasn't resolved yet.
    /// Orchestrator unblocks them by activating/superseding the deps.
    #[serde(default)]
    pub blocked_count: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct BlockedParams {
    /// Optional artifact ID to check. If omitted, shows all blocked artifacts.
    pub id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct BlockedResponse {
    pub blocked: Vec<BlockedEntry>,
    pub ready_count: usize,
    pub blocked_count: usize,
    pub cycles: Vec<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct BlockedEntry {
    pub id: String,
    pub blocked_by: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct OrderResponse {
    pub order: Vec<String>,
    pub ready: Vec<String>,
    pub blocked: Vec<BlockedEntry>,
    pub cycles: Vec<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchResponse {
    pub results: Vec<SearchResultDto>,
    pub total: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchResultDto {
    pub id: String,
    pub kind: String,
    pub title: String,
    #[serde(default)]
    pub matched_lines: Vec<String>,
    /// Status of the artifact (draft, active, ...). Defaults empty for legacy clients.
    #[serde(default)]
    pub status: String,
    /// Combined smart-search score (BM25 + semantic + boosters).
    #[serde(default)]
    pub score: f64,
    /// BM25 normalized score in [0.0, 1.0].
    #[serde(default)]
    pub bm25_score: f64,
    /// Semantic (cosine) similarity, 0 if embeddings unavailable.
    #[serde(default)]
    pub semantic_score: f64,
    /// R_eff quality score of the artifact.
    #[serde(default)]
    pub r_eff: f64,
    /// If present, this result was added via graph expansion from the given parent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expanded_from: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct StaleResponse {
    pub stale: Vec<StaleArtifactDto>,
    pub total: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct StaleArtifactDto {
    pub id: String,
    pub title: String,
    pub valid_until: String,
    pub days_expired: i64,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ProgressResponse {
    pub artifacts: Vec<ProgressDto>,
    pub total_checkboxes: usize,
    pub total_completed: usize,
}

// ── Reason types ─────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReasonResponse {
    pub artifact_id: String,
    pub artifact_title: String,
    pub analysis: String,
    pub provider: String,
    pub model: String,
}

// ── Decompose types ──────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DecomposeResponse {
    pub prd_id: String,
    pub prd_title: String,
    pub tasks: String,
    pub provider: String,
    pub model: String,
}

// ── Generate types ───────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GenerateResponse {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub filepath: String,
    pub provider: String,
    pub model: String,
}

// ── Decay types ──────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DecayResponse {
    pub entries: Vec<DecayEntryDto>,
    pub total_affected: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DecayEntryDto {
    pub artifact_id: String,
    pub artifact_title: String,
    pub current_r_eff: f64,
    pub fresh_r_eff: f64,
    pub r_eff_drop: f64,
    pub expired_evidence: Vec<ExpiredEvidenceDto>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ExpiredEvidenceDto {
    pub id: String,
    pub valid_until: String,
    pub days_expired: i64,
    pub score: f64,
}

// ── Calibrate types ──────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CalibrateResponse {
    pub results: Vec<CalibrationDto>,
    pub total_escalations: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CalibrationDto {
    pub artifact_id: String,
    pub artifact_title: String,
    pub current_depth: String,
    pub suggested_depth: String,
    pub escalation_needed: bool,
    pub signals: Vec<SignalDto>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SignalDto {
    pub name: String,
    pub value: String,
    pub minimum_depth: String,
}

// ── FPF Knowledge Base types ────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FpfSearchResponse {
    pub query: String,
    /// Whether the query was executed via semantic (vector) search. When the
    /// `semantic-search` feature is not compiled in, or when the semantic path
    /// failed at runtime, the handler transparently falls back to keyword
    /// search and surfaces the reason via `warning`.
    pub semantic: bool,
    pub count: usize,
    pub results: Vec<FpfSearchHit>,
    /// Non-null when the semantic path was requested but could not complete
    /// (feature not compiled in, embedder init failure, encode failure, or
    /// vector search failure) and the handler fell back to keyword search.
    /// Serializes as `"warning": null` when absent, matching the pre-existing
    /// JSON contract emitted by the handler.
    pub warning: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FpfSearchHit {
    pub id: String,
    pub section_id: String,
    pub title: String,
    pub snippet: String,
    pub line_count: i32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FpfSectionResponse {
    pub section_id: String,
    pub title: String,
    pub body: String,
    pub line_count: i32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FpfListResponse {
    pub sections: Vec<FpfListItem>,
    pub total: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FpfListItem {
    pub section_id: String,
    pub title: String,
    pub line_count: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fpf_search_response_none_warning_serializes_as_null() {
        let resp = FpfSearchResponse {
            query: "trust".to_string(),
            semantic: false,
            count: 0,
            results: vec![],
            warning: None,
        };
        let v = serde_json::to_value(&resp).unwrap();
        assert_eq!(v["query"], "trust");
        assert_eq!(v["semantic"], false);
        assert_eq!(v["count"], 0);
        assert!(v["results"].is_array());
        // Pre-existing json! macro contract: warning is emitted as null, not omitted.
        assert!(v.get("warning").is_some(), "warning key must be present");
        assert!(
            v["warning"].is_null(),
            "warning: None must serialize as null"
        );
    }

    #[test]
    fn fpf_search_response_some_warning_serializes_as_string() {
        let resp = FpfSearchResponse {
            query: "q".to_string(),
            semantic: true,
            count: 1,
            results: vec![FpfSearchHit {
                id: "fpf-b3".to_string(),
                section_id: "B.3".to_string(),
                title: "Trust Calculus".to_string(),
                snippet: "...".to_string(),
                line_count: 42,
            }],
            warning: Some("fell back".to_string()),
        };
        let v = serde_json::to_value(&resp).unwrap();
        assert_eq!(v["warning"], "fell back");
        assert_eq!(v["results"][0]["id"], "fpf-b3");
        assert_eq!(v["results"][0]["section_id"], "B.3");
        assert_eq!(v["results"][0]["line_count"], 42);
    }
}
