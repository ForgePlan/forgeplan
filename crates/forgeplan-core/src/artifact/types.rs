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

/// Convert title to filename slug — ASCII-only.
///
/// Audit 2026-05-01 #10 (defense-in-depth): rejects non-ASCII characters
/// so the slug is always safe across cross-platform filesystems
/// (Windows rejects many non-ASCII filenames; old NFS strips them;
/// grep/CI tooling assumes ASCII). Falls back to id-only filename when
/// the title is fully non-ASCII (slug is empty, caller composes
/// `<id>-.md` which still validates).
///
/// Migration: workspaces with existing non-ASCII slugs are rebuilt by
/// `forgeplan reindex` — Phase 2 of the reindexer detects that the file
/// at the OLD slug doesn't match the NEW slug, and `delete_orphan_artifact`
/// removes the stale row. Users then see `forgeplan_health` flag the
/// renamed file and re-create the artifact via `scan-import`.
///
/// The path-traversal CVE class is closed independently by
/// `validate_artifact_id` (audit 2026-05-01 CRITICAL #1 fix).
pub fn slugify(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Valid kind prefixes for slug parsing (lowercase, without trailing `-`).
/// Mirrors `ArtifactKind::prefix()` minus the dash.
const VALID_KIND_PREFIXES: &[&str] = &[
    "prd", "rfc", "adr", "epic", "spec", "prob", "sol", "evid", "note", "ref", "mem",
];

/// Reserved suffix prefixes (after kind prefix) that cannot be used in user slugs.
/// Per SPEC-005: tmp- (test fixtures), draft-/pending- (reserved for future).
const RESERVED_SUFFIX_PREFIXES: &[&str] = &["tmp-", "draft-", "pending-"];

/// Validate slug per SPEC-005 rules (PROB-060).
///
/// Rules:
/// - Total length 3..=80 chars
/// - Lowercase ASCII alphanumeric + hyphens only
/// - Starts with a valid kind prefix followed by `-`
/// - Suffix (after kind prefix) is non-empty
/// - Suffix is not reserved (`tmp-`, `draft-`, `pending-`)
/// - Suffix is not numeric-only (e.g. `prd-074` reserved for display id form)
pub fn validate_slug(slug: &str) -> std::result::Result<(), crate::error::ForgeplanError> {
    use crate::error::ForgeplanError;

    if slug.len() < 3 || slug.len() > 80 {
        return Err(ForgeplanError::InvalidSlug(format!(
            "length must be 3..=80 chars, got {}",
            slug.len()
        )));
    }

    if !slug
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(ForgeplanError::InvalidSlug(format!(
            "must be lowercase ASCII alphanumeric + hyphens, got {slug:?}"
        )));
    }

    let suffix = VALID_KIND_PREFIXES
        .iter()
        .find_map(|prefix| slug.strip_prefix(&format!("{prefix}-")))
        .ok_or_else(|| {
            ForgeplanError::InvalidSlug(format!(
                "must start with one of {:?} followed by `-`, got {slug:?}",
                VALID_KIND_PREFIXES
            ))
        })?;

    if suffix.is_empty() {
        return Err(ForgeplanError::InvalidSlug(format!(
            "needs content after kind prefix, got {slug:?}"
        )));
    }

    for reserved in RESERVED_SUFFIX_PREFIXES {
        if suffix.starts_with(reserved) {
            return Err(ForgeplanError::InvalidSlug(format!(
                "uses reserved prefix `{reserved}` after kind, got {slug:?}"
            )));
        }
    }

    if suffix.chars().all(|c| c.is_ascii_digit()) {
        return Err(ForgeplanError::InvalidSlug(format!(
            "suffix must not be numeric-only (avoid collision with display id form), got {slug:?}"
        )));
    }

    Ok(())
}

/// Build a canonical slug from kind + title per SPEC-005.
///
/// Format: `<kind_prefix>-<slugified-title>` truncated at the last hyphen
/// before 80 chars. Returns InvalidSlug if title produces empty slug
/// (all non-ASCII or empty) or otherwise fails validation.
pub fn slug_from_kind_title(
    kind: &ArtifactKind,
    title: &str,
) -> std::result::Result<String, crate::error::ForgeplanError> {
    use crate::error::ForgeplanError;

    let prefix = kind.prefix().trim_end_matches('-');
    let title_slug = slugify(title);

    if title_slug.is_empty() {
        return Err(ForgeplanError::InvalidSlug(format!(
            "title produces empty slug (all non-ASCII or empty): {title:?}"
        )));
    }

    let max_title_len = 80usize.saturating_sub(prefix.len() + 1);
    let truncated = if title_slug.len() > max_title_len {
        let cut = title_slug[..max_title_len]
            .rfind('-')
            .unwrap_or(max_title_len);
        &title_slug[..cut]
    } else {
        title_slug.as_str()
    };

    if truncated.is_empty() {
        return Err(ForgeplanError::InvalidSlug(format!(
            "truncation produced empty title-suffix for prefix {prefix:?}"
        )));
    }

    let slug = format!("{prefix}-{truncated}");
    validate_slug(&slug)?;
    Ok(slug)
}

/// Render derived display id per SPEC-005 (PROB-060).
///
/// - `assigned = Some(n)` → uppercase prefix + zero-padded 3-digit number (`PRD-074`)
/// - `assigned = None`    → uppercase prefix + predicted number + `?` marker (`PRD-74?`)
///
/// The `?` marker visually signals that the number is local prediction
/// and may change at merge.
pub fn render_display_id(kind: &ArtifactKind, predicted: u32, assigned: Option<u32>) -> String {
    let prefix = kind.prefix().trim_end_matches('-').to_uppercase();
    match assigned {
        Some(n) => format!("{prefix}-{n:03}"),
        None => format!("{prefix}-{predicted}?"),
    }
}

/// Artifact lifecycle status.
///
/// State machine (see CLAUDE.md + docs/methodology/):
/// `draft → active → {superseded | deprecated | stale}`
/// `stale → {active via renew | deprecated + new draft via reopen}`
///
/// PROB-040 C1 fix (2026-04-21): renamed `RefreshDue` → `Stale` to match
/// documented state machine + existing string-based lifecycle checks
/// (see `lifecycle::mod.rs` — compares against `"stale"` literal).
/// Serialization is now `stale` (was: `refresh_due`). No artifacts
/// in workspace serialized with the old value (verified via grep).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Draft,
    Active,
    Superseded,
    Deprecated,
    Stale,
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

    // PROB-060 / SPEC-005 — slug validation, building, and display id rendering.

    #[test]
    fn validate_slug_happy_path() {
        assert!(validate_slug("prd-auth-system").is_ok());
        assert!(validate_slug("rfc-mtls-rollout").is_ok());
        assert!(validate_slug("prob-api-panic").is_ok());
        assert!(validate_slug("evid-stack-trace").is_ok());
    }

    #[test]
    fn validate_slug_rejects_too_short() {
        assert!(validate_slug("ab").is_err());
        assert!(validate_slug("p").is_err());
    }

    #[test]
    fn validate_slug_rejects_too_long() {
        let long = format!("prd-{}", "a".repeat(80));
        assert!(validate_slug(&long).is_err());
    }

    #[test]
    fn validate_slug_rejects_uppercase() {
        assert!(validate_slug("PRD-auth").is_err());
        assert!(validate_slug("prd-Auth").is_err());
    }

    #[test]
    fn validate_slug_rejects_unknown_prefix() {
        assert!(validate_slug("foo-auth-system").is_err());
        assert!(validate_slug("xxx-bar").is_err());
    }

    #[test]
    fn validate_slug_rejects_empty_suffix() {
        assert!(validate_slug("prd-").is_err());
    }

    #[test]
    fn validate_slug_rejects_reserved_prefix() {
        assert!(validate_slug("prd-tmp-fixture").is_err());
        assert!(validate_slug("prd-draft-foo").is_err());
        assert!(validate_slug("prd-pending-bar").is_err());
    }

    #[test]
    fn validate_slug_rejects_numeric_only_suffix() {
        // Avoid collision with display form `PRD-074`.
        assert!(validate_slug("prd-074").is_err());
        assert!(validate_slug("prd-1").is_err());
        assert!(validate_slug("rfc-9").is_err());
    }

    #[test]
    fn validate_slug_accepts_alphanumeric_mix() {
        assert!(validate_slug("prd-h2-database").is_ok());
        assert!(validate_slug("prd-v2-rollout").is_ok());
        assert!(validate_slug("rfc-3way-merge").is_ok());
    }

    #[test]
    fn validate_slug_rejects_special_chars() {
        assert!(validate_slug("prd-auth_system").is_err());
        assert!(validate_slug("prd-auth.system").is_err());
        assert!(validate_slug("prd-auth!system").is_err());
        assert!(validate_slug("prd-auth/system").is_err());
    }

    #[test]
    fn slug_from_kind_title_basic() {
        assert_eq!(
            slug_from_kind_title(&ArtifactKind::Prd, "Auth System").unwrap(),
            "prd-auth-system"
        );
        assert_eq!(
            slug_from_kind_title(&ArtifactKind::Rfc, "mTLS Rollout").unwrap(),
            "rfc-mtls-rollout"
        );
        assert_eq!(
            slug_from_kind_title(&ArtifactKind::ProblemCard, "API panic").unwrap(),
            "prob-api-panic"
        );
    }

    #[test]
    fn slug_from_kind_title_truncates_long_titles() {
        let long_title = "long ".repeat(50);
        let result = slug_from_kind_title(&ArtifactKind::Prd, &long_title).unwrap();
        assert!(result.len() <= 80, "got {}: {}", result.len(), result);
        assert!(result.starts_with("prd-"));
    }

    #[test]
    fn slug_from_kind_title_rejects_empty_title() {
        assert!(slug_from_kind_title(&ArtifactKind::Prd, "").is_err());
    }

    #[test]
    fn slug_from_kind_title_rejects_pure_non_ascii() {
        // Per existing slugify (ASCII-only): пустой результат → ошибка.
        assert!(slug_from_kind_title(&ArtifactKind::Prd, "тест").is_err());
    }

    #[test]
    fn slug_from_kind_title_validates_output() {
        // Roundtrip: built slug always passes validation.
        let slug = slug_from_kind_title(&ArtifactKind::Prd, "Auth System").unwrap();
        assert!(validate_slug(&slug).is_ok());
    }

    #[test]
    fn slug_from_kind_title_handles_special_chars_in_title() {
        let result =
            slug_from_kind_title(&ArtifactKind::Prd, "Auth/System: rate-limiter (v2)").unwrap();
        assert_eq!(result, "prd-auth-system-rate-limiter-v2");
    }

    #[test]
    fn render_display_id_assigned_zero_pads_to_3() {
        assert_eq!(
            render_display_id(&ArtifactKind::Prd, 74, Some(74)),
            "PRD-074"
        );
        assert_eq!(
            render_display_id(&ArtifactKind::Adr, 1, Some(12)),
            "ADR-012"
        );
        assert_eq!(
            render_display_id(&ArtifactKind::Prd, 74, Some(123)),
            "PRD-123"
        );
    }

    #[test]
    fn render_display_id_predicted_uses_question_mark() {
        assert_eq!(render_display_id(&ArtifactKind::Prd, 74, None), "PRD-74?");
        assert_eq!(render_display_id(&ArtifactKind::Rfc, 9, None), "RFC-9?");
    }

    #[test]
    fn render_display_id_legacy_kinds() {
        assert_eq!(
            render_display_id(&ArtifactKind::ProblemCard, 60, Some(60)),
            "PROB-060"
        );
        assert_eq!(
            render_display_id(&ArtifactKind::EvidencePack, 113, None),
            "EVID-113?"
        );
        assert_eq!(
            render_display_id(&ArtifactKind::SolutionPortfolio, 5, Some(5)),
            "SOL-005"
        );
    }

    #[test]
    fn render_display_id_assigned_supersedes_predicted() {
        // When assigned is set, predicted is ignored.
        assert_eq!(
            render_display_id(&ArtifactKind::Prd, 999, Some(74)),
            "PRD-074"
        );
    }
}
