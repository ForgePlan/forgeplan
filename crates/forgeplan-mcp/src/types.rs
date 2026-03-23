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

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct NewArtifactResponse {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub filepath: String,
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
    pub matched_lines: Vec<String>,
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
    pub results: Vec<FpfSearchHit>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FpfSearchHit {
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
