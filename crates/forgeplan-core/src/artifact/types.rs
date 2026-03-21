use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// All artifact kinds supported by Forgeplan.
/// 5 from Quint-code + 5 new for Forgeplan = 10 types.
/// DecisionRecord merged into ADR (ADR at deep+ depth includes DDR fields).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    // From Quint-code
    Note,
    ProblemCard,
    SolutionPortfolio,
    EvidencePack,
    RefreshReport,
    // New for Forgeplan
    Prd,
    Epic,
    Spec,
    Rfc,
    Adr,
}

impl ArtifactKind {
    /// Returns the ID prefix for this kind (e.g., "prd-", "epic-").
    pub fn prefix(&self) -> &'static str {
        match self {
            Self::Note => "note-",
            Self::ProblemCard => "prob-",
            Self::SolutionPortfolio => "sol-",
            Self::EvidencePack => "evid-",
            Self::RefreshReport => "ref-",
            Self::Prd => "prd-",
            Self::Epic => "epic-",
            Self::Spec => "spec-",
            Self::Rfc => "rfc-",
            Self::Adr => "adr-",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Draft,
    Active,
    Superseded,
    Deprecated,
    RefreshDue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkType {
    Informs,
    BasedOn,
    Supersedes,
    Contradicts,
    Refines,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub target: String,
    pub relation: LinkType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Note,
    Tactical,
    Standard,
    Deep,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub id: String,
    pub kind: ArtifactKind,
    pub version: u32,
    pub status: Status,
    pub title: String,
    pub context: Option<String>,
    pub mode: Option<Mode>,
    pub valid_until: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub links: Vec<Link>,
    pub parent_epic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub meta: Meta,
    pub body: String,
    pub embedding: Option<Vec<f32>>,
}
