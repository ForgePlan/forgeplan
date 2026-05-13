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

    /// Map a slug prefix (lowercase, no trailing dash) back to [`ArtifactKind`].
    ///
    /// PROB-060 / SPEC-005 Phase 1.5: `resolve_id` accepts slug-form input
    /// like `prd-auth-system` and needs to determine which kind's records
    /// to scan for the slug field.
    ///
    /// Note: slug prefixes diverge from `from_str` aliases — slug uses
    /// `prob` (not `problem`), `sol` (not `solution`), `evid` (not
    /// `evidence`), `ref` (not `refresh`), `mem` (not `memory`).
    pub fn from_slug_prefix(prefix: &str) -> Option<Self> {
        match prefix {
            "prd" => Some(Self::Prd),
            "rfc" => Some(Self::Rfc),
            "adr" => Some(Self::Adr),
            "epic" => Some(Self::Epic),
            "spec" => Some(Self::Spec),
            "prob" => Some(Self::ProblemCard),
            "sol" => Some(Self::SolutionPortfolio),
            "evid" => Some(Self::EvidencePack),
            "note" => Some(Self::Note),
            "ref" => Some(Self::RefreshReport),
            "mem" => Some(Self::Memory),
            _ => None,
        }
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

/// Minimum allowed slug length (kind prefix + dash + at least 1 suffix char).
///
/// Cross-phase audit (code-analyzer #3): named const replacing the magic
/// `3` previously inlined in `validate_slug`. Single source of truth.
pub const MIN_SLUG_LEN: usize = 3;

/// Maximum allowed slug length (chosen for filesystem-path headroom across
/// platforms; macOS HFS+/APFS, Windows NTFS, Linux ext4 all support ≥255
/// chars but we leave room for the trailing `.md` extension and post-merge
/// `<KIND>-<NNN>-` prefix expansion).
///
/// Cross-phase audit (code-analyzer #3): named const replacing the magic
/// `80` previously inlined in `validate_slug` AND `slug_from_kind_title`
/// truncation logic — silent coupling that would have drifted if changed
/// in only one place.
pub const MAX_SLUG_LEN: usize = 80;

/// Valid kind prefixes for slug parsing (lowercase, without trailing `-`).
/// Mirrors `ArtifactKind::prefix()` minus the dash. **Sync invariant**: see
/// `kind_prefixes_in_sync_with_artifact_kind` test which enforces equality
/// against `ArtifactKind::prefix()` for every variant — adding a new kind
/// requires updating both lists or this test fails.
const VALID_KIND_PREFIXES: &[&str] = &[
    "prd", "rfc", "adr", "epic", "spec", "prob", "sol", "evid", "note", "ref", "mem",
];

/// Same as [`VALID_KIND_PREFIXES`] but with the trailing dash, precomputed at
/// const time so [`validate_slug`] avoids per-call `format!` allocations on
/// the hot path (audit 2026-05-06 H3 — validation runs on every artifact load
/// during reindex of 1000+ artifacts).
const VALID_KIND_PREFIXES_WITH_DASH: &[&str] = &[
    "prd-", "rfc-", "adr-", "epic-", "spec-", "prob-", "sol-", "evid-", "note-", "ref-", "mem-",
];

/// Reserved suffix prefixes (after kind prefix) that cannot appear in user slugs.
/// Per SPEC-005: tmp- (test fixtures), draft-/pending- (reserved for future).
///
/// Note: title-derived slugs that would otherwise hit this check (e.g. user
/// title "Draft alternative…") are escaped in [`slug_from_kind_title`] by
/// prepending an `x-` marker — see audit 2026-05-06 H1.
const RESERVED_SUFFIX_PREFIXES: &[&str] = &["tmp-", "draft-", "pending-"];

/// Validate slug per SPEC-005 rules (PROB-060).
///
/// # Rules
/// - Lowercase ASCII alphanumeric + hyphens only — **checked first** so
///   length is then guaranteed to count chars == bytes (audit H2 fix).
/// - Total length 3..=80 chars (== bytes, since ASCII-only)
/// - Starts with a valid kind prefix followed by `-`
/// - Suffix (after kind prefix) is non-empty
/// - Suffix is not reserved (`tmp-`, `draft-`, `pending-`)
/// - Suffix is not numeric-only (e.g. `prd-074` reserved for display id form)
///
/// # Errors
/// Returns [`ForgeplanError::InvalidSlug`] with a precise reason for any
/// rule violation. The error message includes the offending slug for
/// debug ergonomics.
///
/// # Performance
/// Hot path — called on every artifact load. Const arrays
/// [`VALID_KIND_PREFIXES_WITH_DASH`] and [`RESERVED_SUFFIX_PREFIXES`]
/// avoid per-call allocations (audit H3 fix).
pub fn validate_slug(slug: &str) -> std::result::Result<(), crate::error::ForgeplanError> {
    use crate::error::ForgeplanError;

    // ASCII check FIRST — guarantees subsequent length check counts chars == bytes.
    if !slug
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(ForgeplanError::InvalidSlug(format!(
            "must be lowercase ASCII alphanumeric + hyphens, got {slug:?}"
        )));
    }

    if slug.len() < MIN_SLUG_LEN || slug.len() > MAX_SLUG_LEN {
        return Err(ForgeplanError::InvalidSlug(format!(
            "length must be {MIN_SLUG_LEN}..={MAX_SLUG_LEN} chars, got {}",
            slug.len()
        )));
    }

    // Strip prefix using const array with precomputed dashes (no allocations).
    let suffix = VALID_KIND_PREFIXES_WITH_DASH
        .iter()
        .find_map(|prefix| slug.strip_prefix(*prefix))
        .ok_or_else(|| {
            ForgeplanError::InvalidSlug(format!(
                "must start with one of {VALID_KIND_PREFIXES:?} followed by `-`, got {slug:?}"
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
/// before 80 chars total.
///
/// # Reserved-prefix escaping (audit H1)
/// If the title slug would start with a reserved prefix (`tmp-`, `draft-`,
/// `pending-`) — e.g. user title `"Draft alternative…"` produces
/// `draft-alternative-…` — an `x-` marker is prepended so the result is
/// `prd-x-draft-alternative-…`. This avoids opaque `InvalidSlug` errors
/// while preserving the human-readable mapping.
///
/// # Errors
/// Returns [`ForgeplanError::InvalidSlug`] when:
/// - title slugifies to empty (all non-ASCII or empty input)
/// - aggressive truncation of single-word title leaves empty suffix
/// - generated slug fails [`validate_slug`] for any other reason (defense-
///   in-depth — should not normally happen)
pub fn slug_from_kind_title(
    kind: &ArtifactKind,
    title: &str,
) -> std::result::Result<String, crate::error::ForgeplanError> {
    use crate::error::ForgeplanError;

    let prefix = kind.prefix().trim_end_matches('-');
    let mut title_slug = slugify(title);

    if title_slug.is_empty() {
        return Err(ForgeplanError::InvalidSlug(format!(
            "title produces empty slug (all non-ASCII or empty): {title:?}"
        )));
    }

    // Audit H1: if the title slug starts with a reserved prefix (e.g. user
    // title "Draft auth proposal" → "draft-auth-proposal"), `validate_slug`
    // would reject the assembled slug. Escape by prepending `x-` marker —
    // result `prd-x-draft-auth-proposal` is unambiguous and validates.
    let reserved_hit = RESERVED_SUFFIX_PREFIXES
        .iter()
        .any(|reserved| title_slug.starts_with(reserved));
    if reserved_hit {
        title_slug = format!("x-{title_slug}");
    }

    // Truncation. Slugify is documented ASCII-only — every byte is one char,
    // so byte-indexed slicing is safe. Defensive note: if slugify ever returns
    // non-ASCII (contract change), this would panic on non-char-boundary cuts.
    // The doc on `slugify` (line 116-143) and the ASCII guard in `validate_slug`
    // jointly enforce the invariant; a future relax requires re-auditing here.
    let max_title_len = MAX_SLUG_LEN.saturating_sub(prefix.len() + 1);
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

    // Audit 2026-05-06 fixes — regression tests.

    #[test]
    fn audit_h1_title_starting_with_reserved_word_is_escaped() {
        // Title "Draft alternative" should NOT fail with opaque InvalidSlug.
        // Slug builder escapes via `x-` prefix.
        let slug = slug_from_kind_title(&ArtifactKind::Prd, "Draft alternative").unwrap();
        assert_eq!(slug, "prd-x-draft-alternative");
        assert!(validate_slug(&slug).is_ok());
    }

    #[test]
    fn audit_h1_tmp_pending_titles_also_escaped() {
        let slug = slug_from_kind_title(&ArtifactKind::Prd, "Tmp fixture for X").unwrap();
        assert_eq!(slug, "prd-x-tmp-fixture-for-x");

        let slug = slug_from_kind_title(&ArtifactKind::Rfc, "Pending discussion").unwrap();
        assert_eq!(slug, "rfc-x-pending-discussion");
    }

    #[test]
    fn audit_h1_non_reserved_words_unchanged() {
        // Sanity: only reserved-prefix titles get escaped.
        let slug = slug_from_kind_title(&ArtifactKind::Prd, "Authentication system").unwrap();
        assert_eq!(slug, "prd-authentication-system");
    }

    #[test]
    fn audit_h2_validate_slug_ascii_check_runs_before_length() {
        // Non-ASCII string with byte-length > 80 must fail with the ASCII
        // error (specific), not the length error (misleading on bytes vs chars).
        // Pre-fix: length was checked first using slug.len() (bytes).
        let cyrillic = "тест-".repeat(20); // 100 bytes, but Cyrillic.
        let prefixed = format!("prd-{cyrillic}");
        let err = validate_slug(&prefixed).unwrap_err().to_string();
        assert!(
            err.contains("must be lowercase ASCII"),
            "expected ASCII error first, got: {err}"
        );
    }

    // PROB-060 / SPEC-005 Phase 1.5 — slug prefix mapping back to kind.

    #[test]
    fn from_slug_prefix_round_trips_all_kinds() {
        let pairs = [
            ("prd", ArtifactKind::Prd),
            ("rfc", ArtifactKind::Rfc),
            ("adr", ArtifactKind::Adr),
            ("epic", ArtifactKind::Epic),
            ("spec", ArtifactKind::Spec),
            ("prob", ArtifactKind::ProblemCard),
            ("sol", ArtifactKind::SolutionPortfolio),
            ("evid", ArtifactKind::EvidencePack),
            ("note", ArtifactKind::Note),
            ("ref", ArtifactKind::RefreshReport),
            ("mem", ArtifactKind::Memory),
        ];
        for (prefix, expected_kind) in pairs {
            assert_eq!(ArtifactKind::from_slug_prefix(prefix), Some(expected_kind));
        }
    }

    #[test]
    fn from_slug_prefix_rejects_unknown() {
        assert_eq!(ArtifactKind::from_slug_prefix("foo"), None);
        assert_eq!(ArtifactKind::from_slug_prefix(""), None);
        assert_eq!(ArtifactKind::from_slug_prefix("PRD"), None); // case-sensitive
        assert_eq!(ArtifactKind::from_slug_prefix("problem"), None); // alias-form, not slug-form
    }

    #[test]
    fn from_slug_prefix_covers_every_valid_kind_prefix() {
        // Drift defense: every entry in VALID_KIND_PREFIXES must round-trip
        // through from_slug_prefix back to a real ArtifactKind.
        for prefix in VALID_KIND_PREFIXES {
            assert!(
                ArtifactKind::from_slug_prefix(prefix).is_some(),
                "VALID_KIND_PREFIXES entry {prefix:?} has no from_slug_prefix mapping"
            );
        }
    }

    // Phase 1.6 — slug roundtrip property test (audit code-analyzer L3).
    //
    // Without external proptest dependency: generate 200 ASCII-letter
    // titles of varying length and verify the invariant
    //   slug_from_kind_title(kind, title).is_ok() ⟹ validate_slug(slug).is_ok()
    // for every variant. Catches truncation edge cases, reserved-prefix
    // escape correctness, and validation/builder consistency.

    #[test]
    fn slug_roundtrip_property_holds_for_all_kinds() {
        let all_kinds = [
            ArtifactKind::Prd,
            ArtifactKind::Rfc,
            ArtifactKind::Adr,
            ArtifactKind::Epic,
            ArtifactKind::Spec,
            ArtifactKind::ProblemCard,
            ArtifactKind::SolutionPortfolio,
            ArtifactKind::EvidencePack,
            ArtifactKind::Note,
            ArtifactKind::RefreshReport,
            ArtifactKind::Memory,
        ];

        // Deterministic-pseudo-random titles using a simple linear congruential
        // generator. Avoids external rand dep while giving enough variety.
        let mut state: u64 = 0xDEAD_BEEF_CAFE_F00D;
        let mut next_byte = || -> u8 {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((state >> 33) & 0xFF) as u8
        };

        for kind in &all_kinds {
            for trial in 0..200 {
                // Generate title of variable length 1..120 chars from
                // ASCII letter alphabet + space (ASCII-only is the slugify
                // contract — non-ASCII would fail upstream).
                let len = 1 + (trial % 120);
                let alphabet: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ ";
                let title: String = (0..len)
                    .map(|_| alphabet[next_byte() as usize % alphabet.len()] as char)
                    .collect();

                match slug_from_kind_title(kind, &title) {
                    Ok(slug) => {
                        // Invariant: every successful build must validate.
                        assert!(
                            validate_slug(&slug).is_ok(),
                            "kind {:?} title {:?} → slug {:?} fails validate_slug",
                            kind,
                            title,
                            slug
                        );
                        // Invariant: slug starts with expected kind prefix.
                        let expected_prefix = format!("{}-", kind.prefix().trim_end_matches('-'));
                        assert!(
                            slug.starts_with(&expected_prefix),
                            "kind {:?} title {:?} → slug {:?} does not start with {:?}",
                            kind,
                            title,
                            slug,
                            expected_prefix
                        );
                        // Invariant: total length within bounds.
                        assert!(
                            slug.len() >= MIN_SLUG_LEN && slug.len() <= MAX_SLUG_LEN,
                            "kind {:?} title {:?} → slug {:?} length {} out of bounds",
                            kind,
                            title,
                            slug,
                            slug.len()
                        );
                    }
                    Err(_) => {
                        // Build can legitimately fail only when title
                        // slugifies to empty (all spaces or all non-ASCII;
                        // alphabet here is ASCII-only so non-ASCII path
                        // is unreachable). Reserved prefixes are escaped
                        // via x- in slug_from_kind_title and don't fail.
                        let title_slug = slugify(&title);
                        assert!(
                            title_slug.is_empty(),
                            "unexpected build failure for kind {:?} title {:?}: title_slug = {:?}",
                            kind,
                            title,
                            title_slug
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn audit_l1_kind_prefixes_in_sync_with_artifact_kind() {
        // Defense against drift: every ArtifactKind variant's prefix (minus
        // dash) must appear in VALID_KIND_PREFIXES exactly once.
        let all_kinds = [
            ArtifactKind::Prd,
            ArtifactKind::Rfc,
            ArtifactKind::Adr,
            ArtifactKind::Epic,
            ArtifactKind::Spec,
            ArtifactKind::ProblemCard,
            ArtifactKind::SolutionPortfolio,
            ArtifactKind::EvidencePack,
            ArtifactKind::Note,
            ArtifactKind::RefreshReport,
            ArtifactKind::Memory,
        ];

        let kind_prefixes: Vec<&str> = all_kinds
            .iter()
            .map(|k| k.prefix().trim_end_matches('-'))
            .collect();

        // Every ArtifactKind prefix is in VALID_KIND_PREFIXES.
        for prefix in &kind_prefixes {
            assert!(
                VALID_KIND_PREFIXES.contains(prefix),
                "ArtifactKind prefix {prefix:?} missing from VALID_KIND_PREFIXES"
            );
        }
        // Conversely: every VALID_KIND_PREFIXES entry corresponds to an ArtifactKind.
        for valid in VALID_KIND_PREFIXES {
            assert!(
                kind_prefixes.contains(valid),
                "VALID_KIND_PREFIXES entry {valid:?} has no matching ArtifactKind"
            );
        }
        // Counts match (catches duplicates).
        assert_eq!(
            kind_prefixes.len(),
            VALID_KIND_PREFIXES.len(),
            "ArtifactKind variant count {} != VALID_KIND_PREFIXES length {}",
            kind_prefixes.len(),
            VALID_KIND_PREFIXES.len()
        );

        // VALID_KIND_PREFIXES_WITH_DASH stays in sync.
        assert_eq!(
            VALID_KIND_PREFIXES.len(),
            VALID_KIND_PREFIXES_WITH_DASH.len(),
            "with-dash array length must equal bare-prefix length"
        );
        for (bare, with_dash) in VALID_KIND_PREFIXES
            .iter()
            .zip(VALID_KIND_PREFIXES_WITH_DASH.iter())
        {
            assert_eq!(
                format!("{bare}-"),
                **with_dash,
                "with-dash array entry {with_dash:?} doesn't match bare {bare:?}"
            );
        }
    }
}
