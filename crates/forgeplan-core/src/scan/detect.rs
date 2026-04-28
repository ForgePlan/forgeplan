use std::path::Path;

use crate::artifact::frontmatter::parse_frontmatter;
use crate::artifact::types::ArtifactKind;

/// How the artifact kind was detected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectionTier {
    /// Tier 1: `kind` field in YAML frontmatter — most reliable.
    Frontmatter,
    /// Tier 2: Filename pattern (PRD-001, RFC-*, ADR-*) — high reliability.
    Filename,
    /// Tier 3: Content heuristics (## Problem, ## Decision) — moderate reliability.
    Content,
}

/// Result of artifact type detection.
#[derive(Debug, Clone)]
pub struct DetectionResult {
    pub kind: ArtifactKind,
    pub tier: DetectionTier,
    /// Suggested ID extracted from the document (e.g., "PRD-001" from frontmatter `id` field).
    pub suggested_id: Option<String>,
    /// Title extracted from frontmatter or first heading.
    pub suggested_title: Option<String>,
}

/// Detect artifact kind using 3-tier fallback chain:
/// frontmatter → filename → content heuristics.
///
/// Returns `None` if no tier can determine the type.
pub fn detect_kind(filename: &str, content: &str) -> Option<DetectionResult> {
    detect_kind_with_path(filename, None, content)
}

/// Path-aware variant of `detect_kind`. When `relative_path` matches a known
/// documentation location (PROB-047 mitigation 1), Tier 3 (content heuristic)
/// detection is suppressed — frontmatter and filename patterns remain
/// authoritative because they are explicit signals.
///
/// This prevents misclassification of guides, instruction files, and config
/// docs (e.g. `docs/methodology/FORGEPLAN-GUIDE.md` containing `## Goals`,
/// `CLAUDE.md` containing `## Problem`) as PRDs.
pub fn detect_kind_with_path(
    filename: &str,
    relative_path: Option<&Path>,
    content: &str,
) -> Option<DetectionResult> {
    // Tier 1: Frontmatter — explicit, always authoritative.
    if let Some(result) = detect_from_frontmatter(content) {
        return Some(result);
    }

    // Tier 2: Filename pattern (PRD-001, RFC-002…) — explicit naming convention.
    if let Some(result) = detect_from_filename(filename) {
        return Some(result);
    }

    // Tier 3: Content heuristics — suppressed for known documentation paths
    // because guides describing artifact structure naturally contain the
    // same headings as the artifacts themselves (PROB-047).
    if let Some(path) = relative_path
        && is_doc_path(path)
    {
        return None;
    }
    detect_from_content(content)
}

/// Returns `true` if `relative_path` points to a documentation file or
/// project meta-file that should NOT be auto-classified by content
/// heuristics (PROB-047 mitigation 1). Explicit frontmatter or filename
/// patterns still win — this guard only suppresses Tier 3.
///
/// Blacklist:
/// - any file under `docs/` or `marketplace/` (recursive)
/// - root-level project meta-files: `CLAUDE.md`, `AGENTS.md`, `README.md`,
///   `CONTRIBUTING.md`, `CHANGELOG.md`, `TODO.md`, `ROADMAP.md`,
///   `LICENSE.md`, `SECURITY.md`, plus their `.ru.md` localized variants
pub fn is_doc_path(relative_path: &Path) -> bool {
    let path_str = relative_path.to_string_lossy();
    let normalized = path_str.replace('\\', "/");
    let normalized = normalized.trim_start_matches("./");

    if normalized.starts_with("docs/") || normalized.starts_with("marketplace/") {
        return true;
    }

    // Root-level meta-files (no path separators after normalization).
    if !normalized.contains('/') {
        const ROOT_DOC_FILES: &[&str] = &[
            "CLAUDE.md",
            "CLAUDE.ru.md",
            "AGENTS.md",
            "AGENTS.ru.md",
            "README.md",
            "README.ru.md",
            "CONTRIBUTING.md",
            "CONTRIBUTING.ru.md",
            "CHANGELOG.md",
            "CHANGELOG.ru.md",
            "TODO.md",
            "TODO.ru.md",
            "ROADMAP.md",
            "ROADMAP.ru.md",
            "LICENSE.md",
            "SECURITY.md",
        ];
        if ROOT_DOC_FILES
            .iter()
            .any(|f| f.eq_ignore_ascii_case(normalized))
        {
            return true;
        }
    }

    false
}

/// Tier 1: Parse YAML frontmatter for `kind` field.
fn detect_from_frontmatter(content: &str) -> Option<DetectionResult> {
    let (fm, _body) = parse_frontmatter(content).ok()?;

    let kind_str = fm.get("kind").and_then(|v| v.as_str())?;
    let kind: ArtifactKind = kind_str.parse().ok()?;

    let suggested_id = fm
        .get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_uppercase());

    let suggested_title = fm
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.trim_matches('"').to_string());

    Some(DetectionResult {
        kind,
        tier: DetectionTier::Frontmatter,
        suggested_id,
        suggested_title,
    })
}

/// Tier 2: Detect from filename patterns like `PRD-001-title.md`, `RFC-002.md`.
fn detect_from_filename(filename: &str) -> Option<DetectionResult> {
    let name = filename
        .strip_suffix(".md")
        .or_else(|| filename.strip_suffix(".markdown"))
        .unwrap_or(filename);
    let upper = name.to_uppercase();

    let patterns: &[(&str, ArtifactKind)] = &[
        ("PRD-", ArtifactKind::Prd),
        ("RFC-", ArtifactKind::Rfc),
        ("ADR-", ArtifactKind::Adr),
        ("EPIC-", ArtifactKind::Epic),
        ("SPEC-", ArtifactKind::Spec),
        ("NOTE-", ArtifactKind::Note),
        ("PROB-", ArtifactKind::ProblemCard),
        ("SOL-", ArtifactKind::SolutionPortfolio),
        ("EVID-", ArtifactKind::EvidencePack),
        ("REFRESH-", ArtifactKind::RefreshReport),
        ("REF-", ArtifactKind::RefreshReport),
    ];

    for (prefix, kind) in patterns {
        if let Some(after) = upper.strip_prefix(prefix) {
            // For REF- prefix, require digits after prefix to avoid false positives
            // (e.g., REF-DOCS-ANALYSIS.md is NOT a RefreshReport)
            if *prefix == "REF-"
                && (after.is_empty() || !after.starts_with(|c: char| c.is_ascii_digit()))
            {
                continue;
            }
            // Extract ID: everything before first non-ID character
            let id = extract_id_from_name(&upper, prefix);
            let title = extract_title_from_name(name, prefix);
            return Some(DetectionResult {
                kind: kind.clone(),
                tier: DetectionTier::Filename,
                suggested_id: Some(id),
                suggested_title: title,
            });
        }
    }

    None
}

/// Extract the ID portion (e.g., "PRD-001") from a filename like "PRD-001-my-title".
fn extract_id_from_name(upper_name: &str, prefix: &str) -> String {
    let after_prefix = &upper_name[prefix.len()..];
    let num_end = after_prefix
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(after_prefix.len());

    let kind_prefix = prefix.trim_end_matches('-');
    if num_end > 0 {
        let num_str = &after_prefix[..num_end];
        format!("{}-{}", kind_prefix, num_str)
    } else {
        format!("{}-001", kind_prefix)
    }
}

/// Extract a human-readable title from filename (e.g., "my-title" from "PRD-001-my-title").
fn extract_title_from_name(name: &str, prefix: &str) -> Option<String> {
    let after_prefix = &name[prefix.len()..];
    // Skip digits
    let after_num = after_prefix.trim_start_matches(|c: char| c.is_ascii_digit());
    let title_part = after_num.trim_start_matches('-').trim_start_matches('_');
    if title_part.is_empty() {
        None
    } else {
        Some(title_part.replace(['-', '_'], " "))
    }
}

/// Tier 3: Content-based heuristics — look for section headings that indicate artifact type.
fn detect_from_content(content: &str) -> Option<DetectionResult> {
    let lower = content.to_lowercase();

    // Extract title from first H1
    let title = content
        .lines()
        .find(|l| l.starts_with("# "))
        .map(|l| l.trim_start_matches('#').trim().to_string());

    // ADR indicators
    if (lower.contains("## decision") || lower.contains("## context"))
        && (lower.contains("## status") || lower.contains("## consequences"))
    {
        return Some(DetectionResult {
            kind: ArtifactKind::Adr,
            tier: DetectionTier::Content,
            suggested_id: None,
            suggested_title: title,
        });
    }

    // PRD indicators
    if (lower.contains("## problem") || lower.contains("## motivation"))
        && (lower.contains("## goals")
            || lower.contains("## success criteria")
            || lower.contains("## requirements")
            || lower.contains("## functional requirements"))
    {
        return Some(DetectionResult {
            kind: ArtifactKind::Prd,
            tier: DetectionTier::Content,
            suggested_id: None,
            suggested_title: title,
        });
    }

    // RFC indicators
    if (lower.contains("## proposal")
        || lower.contains("## design")
        || lower.contains("## approach"))
        && (lower.contains("## alternatives") || lower.contains("## implementation"))
    {
        return Some(DetectionResult {
            kind: ArtifactKind::Rfc,
            tier: DetectionTier::Content,
            suggested_id: None,
            suggested_title: title,
        });
    }

    // Spec indicators
    if (lower.contains("## api")
        || lower.contains("## endpoints")
        || lower.contains("## data model"))
        && (lower.contains("## request")
            || lower.contains("## response")
            || lower.contains("## schema"))
    {
        return Some(DetectionResult {
            kind: ArtifactKind::Spec,
            tier: DetectionTier::Content,
            suggested_id: None,
            suggested_title: title,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier1_frontmatter_detection() {
        let content = "---\nkind: prd\nid: PRD-001\ntitle: \"My PRD\"\n---\n# PRD-001";
        let result = detect_kind("anything.md", content).unwrap();
        assert_eq!(result.kind, ArtifactKind::Prd);
        assert_eq!(result.tier, DetectionTier::Frontmatter);
        assert_eq!(result.suggested_id, Some("PRD-001".to_string()));
        assert_eq!(result.suggested_title, Some("My PRD".to_string()));
    }

    #[test]
    fn tier1_frontmatter_rfc() {
        let content = "---\nkind: rfc\nid: RFC-002\ntitle: Architecture\n---\n# RFC";
        let result = detect_kind("some-file.md", content).unwrap();
        assert_eq!(result.kind, ArtifactKind::Rfc);
        assert_eq!(result.tier, DetectionTier::Frontmatter);
    }

    #[test]
    fn tier2_filename_prd() {
        let content = "# Just a document\nNo frontmatter here.";
        let result = detect_kind("PRD-003-auth-system.md", content).unwrap();
        assert_eq!(result.kind, ArtifactKind::Prd);
        assert_eq!(result.tier, DetectionTier::Filename);
        assert!(result.suggested_title.is_some());
    }

    #[test]
    fn tier2_filename_adr() {
        let content = "No frontmatter";
        let result = detect_kind("ADR-001-use-rust.md", content).unwrap();
        assert_eq!(result.kind, ArtifactKind::Adr);
        assert_eq!(result.tier, DetectionTier::Filename);
    }

    #[test]
    fn tier3_content_adr() {
        let content = "# Use PostgreSQL\n\n## Context\nWe need a database.\n\n## Decision\nUse PostgreSQL.\n\n## Status\nAccepted\n\n## Consequences\nGood.";
        let result = detect_kind("random-name.md", content).unwrap();
        assert_eq!(result.kind, ArtifactKind::Adr);
        assert_eq!(result.tier, DetectionTier::Content);
    }

    #[test]
    fn tier3_content_prd() {
        let content = "# Auth System\n\n## Problem\nUsers can't log in.\n\n## Goals\nSecure auth.\n\n## Requirements\nFR-001";
        let result = detect_kind("auth.md", content).unwrap();
        assert_eq!(result.kind, ArtifactKind::Prd);
        assert_eq!(result.tier, DetectionTier::Content);
    }

    #[test]
    fn tier3_content_rfc() {
        let content = "# API Redesign\n\n## Proposal\nNew REST API.\n\n## Alternatives\nGraphQL.\n\n## Implementation\nPhased.";
        let result = detect_kind("api.md", content).unwrap();
        assert_eq!(result.kind, ArtifactKind::Rfc);
        assert_eq!(result.tier, DetectionTier::Content);
    }

    #[test]
    fn unknown_returns_none() {
        let content = "# Shopping List\n\n- Milk\n- Bread";
        assert!(detect_kind("shopping.md", content).is_none());
    }

    #[test]
    fn frontmatter_takes_priority_over_filename() {
        let content = "---\nkind: rfc\nid: RFC-001\n---\n# RFC";
        let result = detect_kind("PRD-001-looks-like-prd.md", content).unwrap();
        assert_eq!(result.kind, ArtifactKind::Rfc); // frontmatter wins
        assert_eq!(result.tier, DetectionTier::Frontmatter);
    }

    #[test]
    fn ref_prefix_requires_digits() {
        // REF-001 should detect as RefreshReport
        let content = "No frontmatter";
        let result = detect_kind("REF-001-review.md", content).unwrap();
        assert_eq!(result.kind, ArtifactKind::RefreshReport);
        assert_eq!(result.tier, DetectionTier::Filename);
    }

    #[test]
    fn ref_docs_not_detected_as_refresh() {
        // REF-DOCS-ANALYSIS.md should NOT be RefreshReport
        let content = "# Reference Analysis\nSome analysis...";
        assert!(detect_kind("REF-DOCS-ANALYSIS.md", content).is_none());
    }

    #[test]
    fn refresh_prefix_detected() {
        let content = "No frontmatter";
        let result = detect_kind("REFRESH-001-quarterly.md", content).unwrap();
        assert_eq!(result.kind, ArtifactKind::RefreshReport);
    }

    #[test]
    fn tier3_content_spec() {
        let content = "# Payment API\n\n## API\nREST endpoints.\n\n## Schema\nJSON schema.\n\n## Request\nPOST /pay";
        let result = detect_kind("payment.md", content).unwrap();
        assert_eq!(result.kind, ArtifactKind::Spec);
        assert_eq!(result.tier, DetectionTier::Content);
    }

    #[test]
    fn empty_content_returns_none() {
        assert!(detect_kind("empty.md", "").is_none());
    }

    #[test]
    fn binary_looking_content_returns_none() {
        let content = "\0\0\0binary garbage\x01\x02";
        assert!(detect_kind("binary.md", content).is_none());
    }

    // ============================================================
    // PROB-047 mitigation 1: filename/path blacklist for docs
    // ============================================================

    #[test]
    fn is_doc_path_under_docs_directory() {
        assert!(is_doc_path(Path::new(
            "docs/methodology/FORGEPLAN-GUIDE.md"
        )));
        assert!(is_doc_path(Path::new("docs/foo.md")));
        assert!(is_doc_path(Path::new("./docs/foo.md")));
    }

    #[test]
    fn is_doc_path_under_marketplace_directory() {
        assert!(is_doc_path(Path::new("marketplace/playbooks/foo.md")));
        assert!(is_doc_path(Path::new("./marketplace/foo.md")));
    }

    #[test]
    fn is_doc_path_root_level_meta_files() {
        assert!(is_doc_path(Path::new("CLAUDE.md")));
        assert!(is_doc_path(Path::new("AGENTS.md")));
        assert!(is_doc_path(Path::new("README.md")));
        assert!(is_doc_path(Path::new("README.ru.md")));
        assert!(is_doc_path(Path::new("CHANGELOG.md")));
        assert!(is_doc_path(Path::new("CONTRIBUTING.md")));
        assert!(is_doc_path(Path::new("TODO.md")));
        assert!(is_doc_path(Path::new("ROADMAP.md")));
    }

    #[test]
    fn is_doc_path_meta_files_case_insensitive() {
        assert!(is_doc_path(Path::new("claude.md")));
        assert!(is_doc_path(Path::new("Readme.md")));
    }

    #[test]
    fn is_doc_path_not_blacklisted() {
        // Real artifact locations
        assert!(!is_doc_path(Path::new(".forgeplan/prds/PRD-001-real.md")));
        assert!(!is_doc_path(Path::new(".forgeplan/adrs/ADR-001.md")));
        // Sub-crate CHANGELOG isn't blacklisted — keeps classifier conservative
        assert!(!is_doc_path(Path::new("crates/foo/CHANGELOG.md")));
        // Random root .md files are NOT blacklisted (only specific stems)
        assert!(!is_doc_path(Path::new("PRD-001-design.md")));
        assert!(!is_doc_path(Path::new("notes.md")));
    }

    #[test]
    fn path_aware_suppresses_content_tier_under_docs() {
        // FORGEPLAN-GUIDE has `## Problem` and `## Goals` — Tier 3 would
        // classify it as PRD without the path guard.
        let content = "# Forgeplan Guide\n\n## Problem\nDescribed.\n\n## Goals\nListed.";
        let path = Path::new("docs/methodology/FORGEPLAN-GUIDE.md");

        // Without path: Tier 3 classifies as PRD (the bug).
        assert_eq!(
            detect_kind("FORGEPLAN-GUIDE.md", content).map(|r| r.tier),
            Some(DetectionTier::Content)
        );

        // With path: Tier 3 suppressed — guide stays unclassified.
        assert!(detect_kind_with_path("FORGEPLAN-GUIDE.md", Some(path), content).is_none());
    }

    #[test]
    fn path_aware_suppresses_content_tier_for_root_claude_md() {
        // CLAUDE.md contains `## Problem` (in red lines section) and
        // would otherwise hit PRD content heuristic.
        let content = "# CLAUDE.md\n\n## Problem\nFoo.\n\n## Goals\nBar.";
        let path = Path::new("CLAUDE.md");

        assert!(detect_kind_with_path("CLAUDE.md", Some(path), content).is_none());
    }

    #[test]
    fn path_aware_honors_explicit_frontmatter_under_docs() {
        // Explicit `kind: prd` frontmatter is authoritative even under docs/
        // — user opted in deliberately.
        let content = "---\nkind: prd\nid: PRD-099\n---\n# Real PRD in docs/\n\n## Problem\nX.";
        let path = Path::new("docs/PRD-099-architecture.md");

        let result = detect_kind_with_path("PRD-099-architecture.md", Some(path), content).unwrap();
        assert_eq!(result.kind, ArtifactKind::Prd);
        assert_eq!(result.tier, DetectionTier::Frontmatter);
    }

    #[test]
    fn path_aware_honors_filename_pattern_under_docs() {
        // Filename pattern PRD-001 is also explicit — keep classifying it.
        let content = "# Real PRD in docs\n\nNo frontmatter, but filename is explicit.";
        let path = Path::new("docs/PRD-099-arch.md");

        let result = detect_kind_with_path("PRD-099-arch.md", Some(path), content).unwrap();
        assert_eq!(result.kind, ArtifactKind::Prd);
        assert_eq!(result.tier, DetectionTier::Filename);
    }

    #[test]
    fn path_aware_passthrough_for_non_blacklisted_paths() {
        // Files under .forgeplan/ or random locations are unaffected.
        let content = "# Foo\n\n## Problem\nX.\n\n## Goals\nY.";
        let path = Path::new(".forgeplan/prds/PRD-099.md");

        let result = detect_kind_with_path("PRD-099.md", Some(path), content).unwrap();
        // Filename tier wins here, but if we strip filename pattern...
        assert_eq!(result.kind, ArtifactKind::Prd);
    }

    #[test]
    fn path_aware_no_path_falls_back_to_old_behavior() {
        // detect_kind_with_path(.., None, ..) === detect_kind
        let content = "# Foo\n\n## Problem\nX.\n\n## Goals\nY.";
        let r1 = detect_kind("foo.md", content);
        let r2 = detect_kind_with_path("foo.md", None, content);
        assert_eq!(r1.map(|r| r.kind), r2.map(|r| r.kind));
    }
}
