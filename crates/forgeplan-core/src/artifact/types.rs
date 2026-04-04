use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// All artifact kinds supported by Forgeplan.
/// 5 from Quint-code + 6 new for Forgeplan = 11 types.
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
    /// Lightweight project memory — shared bookmarks, no lifecycle overhead.
    /// Stored in .forgeplan/memory/, indexed in LanceDB as kind=memory.
    Memory,
}

impl std::str::FromStr for ArtifactKind {
    type Err = crate::error::ForgeplanError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "prd" => Ok(Self::Prd),
            "epic" => Ok(Self::Epic),
            "spec" => Ok(Self::Spec),
            "rfc" => Ok(Self::Rfc),
            "adr" => Ok(Self::Adr),
            "note" => Ok(Self::Note),
            "problem" | "problemcard" => Ok(Self::ProblemCard),
            "solution" | "solutionportfolio" => Ok(Self::SolutionPortfolio),
            "evidence" | "evidencepack" => Ok(Self::EvidencePack),
            "refresh" | "refreshreport" => Ok(Self::RefreshReport),
            "memory" => Ok(Self::Memory),
            other => Err(crate::error::ForgeplanError::InvalidKind(other.to_string())),
        }
    }
}

/// Kinds that represent decisions requiring evidence for health blind-spot checks.
/// Excludes: note (ephemeral), evidence (IS evidence), refresh (meta-evaluation).
pub const DECISION_KINDS_EVIDENCE: &[&str] =
    &["prd", "rfc", "adr", "epic", "spec", "problem", "solution"];

/// Kinds shown in the decision journal timeline.
/// Includes note (captured decisions) in addition to evidence-requiring kinds.
pub const DECISION_KINDS_JOURNAL: &[&str] = &[
    "adr", "note", "prd", "rfc", "epic", "spec", "problem", "solution",
];

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
            Self::Memory => "mem-",
        }
    }

    /// Returns the subdirectory name for this kind.
    pub fn dir_name(&self) -> &'static str {
        match self {
            Self::Prd => "prds",
            Self::Epic => "epics",
            Self::Spec => "specs",
            Self::Rfc => "rfcs",
            Self::Adr => "adrs",
            Self::ProblemCard => "problems",
            Self::SolutionPortfolio => "solutions",
            Self::EvidencePack => "evidence",
            Self::Note => "notes",
            Self::RefreshReport => "refresh",
            Self::Memory => "memory",
        }
    }

    /// Returns the template lookup key for this kind.
    pub fn template_key(&self) -> &'static str {
        match self {
            Self::Prd => "prd",
            Self::Epic => "epic",
            Self::Spec => "spec",
            Self::Rfc => "rfc",
            Self::Adr => "adr",
            Self::ProblemCard => "problem",
            Self::SolutionPortfolio => "solution",
            Self::EvidencePack => "evidence",
            Self::Note => "note",
            Self::RefreshReport => "refresh",
            Self::Memory => "memory",
        }
    }

    /// Whether this kind is a memory (excluded from health/gaps/score/validation).
    pub fn is_memory(&self) -> bool {
        matches!(self, Self::Memory)
    }
}

/// Convert title to filename slug.
pub fn slugify(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
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

impl std::str::FromStr for Mode {
    type Err = crate::error::ForgeplanError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "note" => Ok(Self::Note),
            "tactical" => Ok(Self::Tactical),
            "standard" => Ok(Self::Standard),
            "deep" | "critical" => Ok(Self::Deep),
            other => Err(crate::error::ForgeplanError::InvalidKind(format!(
                "invalid depth: {other}"
            ))),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str_all_kinds() {
        assert_eq!("prd".parse::<ArtifactKind>().unwrap(), ArtifactKind::Prd);
        assert_eq!("epic".parse::<ArtifactKind>().unwrap(), ArtifactKind::Epic);
        assert_eq!("spec".parse::<ArtifactKind>().unwrap(), ArtifactKind::Spec);
        assert_eq!("rfc".parse::<ArtifactKind>().unwrap(), ArtifactKind::Rfc);
        assert_eq!("adr".parse::<ArtifactKind>().unwrap(), ArtifactKind::Adr);
        assert_eq!("note".parse::<ArtifactKind>().unwrap(), ArtifactKind::Note);
        assert_eq!(
            "problem".parse::<ArtifactKind>().unwrap(),
            ArtifactKind::ProblemCard
        );
        assert_eq!(
            "solution".parse::<ArtifactKind>().unwrap(),
            ArtifactKind::SolutionPortfolio
        );
        assert_eq!(
            "evidence".parse::<ArtifactKind>().unwrap(),
            ArtifactKind::EvidencePack
        );
        assert_eq!(
            "refresh".parse::<ArtifactKind>().unwrap(),
            ArtifactKind::RefreshReport
        );
    }

    #[test]
    fn from_str_aliases() {
        assert_eq!(
            "problemcard".parse::<ArtifactKind>().unwrap(),
            ArtifactKind::ProblemCard
        );
        assert_eq!(
            "solutionportfolio".parse::<ArtifactKind>().unwrap(),
            ArtifactKind::SolutionPortfolio
        );
        assert_eq!(
            "evidencepack".parse::<ArtifactKind>().unwrap(),
            ArtifactKind::EvidencePack
        );
        assert_eq!(
            "refreshreport".parse::<ArtifactKind>().unwrap(),
            ArtifactKind::RefreshReport
        );
    }

    #[test]
    fn from_str_case_insensitive() {
        assert_eq!("PRD".parse::<ArtifactKind>().unwrap(), ArtifactKind::Prd);
        assert_eq!("Epic".parse::<ArtifactKind>().unwrap(), ArtifactKind::Epic);
        assert_eq!("RFC".parse::<ArtifactKind>().unwrap(), ArtifactKind::Rfc);
    }

    #[test]
    fn from_str_invalid() {
        assert!("unknown".parse::<ArtifactKind>().is_err());
        assert!("banana".parse::<ArtifactKind>().is_err());
    }

    #[test]
    fn dir_name_all_kinds() {
        assert_eq!(ArtifactKind::Prd.dir_name(), "prds");
        assert_eq!(ArtifactKind::Epic.dir_name(), "epics");
        assert_eq!(ArtifactKind::Note.dir_name(), "notes");
        assert_eq!(ArtifactKind::ProblemCard.dir_name(), "problems");
        assert_eq!(ArtifactKind::EvidencePack.dir_name(), "evidence");
    }

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Auth System"), "auth-system");
        assert_eq!(slugify("Hello World!"), "hello-world");
        assert_eq!(slugify("  multiple   spaces  "), "multiple-spaces");
    }
}
