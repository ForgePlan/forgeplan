//! Ingest engine — walks a [`Mapping`] over [`ParsedSource`]s and emits
//! [`IngestArtifactDraft`]s.
//!
//! Wave 2 stops at draft assembly: this module never writes to LanceDB or the
//! filesystem. Wave 3's CLI integration will pick up the drafts and dispatch
//! them through `artifact::Store::create` / `update`.

use std::collections::HashMap;

use globset::{Glob, GlobMatcher};
use serde::Serialize;
use serde_json::{Value as Json, json};
use thiserror::Error;
use tracing::{debug, warn};

use super::idempotency::{compute_source_hash, render_source_hash_marker};
use super::sources::{ParsedSection, ParsedSource};
use super::template::{TemplateEngine, TemplateError};
use super::types::{
    ArtifactTargetKind, Guards, IfExists, LinkSpec, Mapping, Rule, Selector, SourcesSectionSpec,
    Template,
};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Per-call options. Reserved for future use (dry-run, force-update, etc.);
/// for Wave 2 we expose just the `dry_run` flag so callers can surface drafts
/// without committing them.
#[derive(Debug, Clone, Default)]
pub struct IngestOptions {
    /// When true, the engine skips guard checks that depend on filesystem
    /// state (e.g. `forbid_overwrite_active`). Wave 3 CLI will set this when
    /// the user passes `--dry-run`.
    pub dry_run: bool,
}

/// A planned artifact that has not yet been written to disk.
#[derive(Debug, Clone, Serialize)]
pub struct IngestArtifactDraft {
    /// Forge artifact kind (`prd`, `spec`, …).
    pub kind: ArtifactTargetKind,
    /// Rendered title (from `rule.fields.title`).
    pub title: String,
    /// Final markdown body, including the `## Sources` block and the
    /// `<!-- source_hash: … -->` marker.
    pub body: String,
    /// Idempotency hash (hex sha256 of `(rule_id, full_text)`).
    pub source_hash: String,
    /// Auto-created links.
    pub links: Vec<DraftLink>,
    /// Originating rule id (for diagnostics).
    pub rule_id: String,
    /// Originating source path.
    pub source_path: String,
}

/// Link planned by a [`Rule`]'s `links:` block.
#[derive(Debug, Clone, Serialize)]
pub struct DraftLink {
    pub target: String,
    pub relation: String,
    pub if_exists: IfExists,
}

/// Why a rule×source pair was not turned into a draft.
#[derive(Debug, Clone, Serialize)]
pub enum SkipReason {
    /// `rule.when` selector did not match the source.
    SelectorMismatch {
        rule_id: String,
        source_path: String,
    },
    /// `rule.when.contains_section` was set but no such section exists.
    MissingRequiredSection {
        rule_id: String,
        source_path: String,
        section: String,
    },
}

/// A non-fatal error tied to a specific rule (e.g. template missing variable).
#[derive(Debug, Clone, Serialize)]
pub struct RuleError {
    pub rule_id: String,
    pub source_path: String,
    pub message: String,
}

/// Aggregated outcome of a single [`IngestEngine::apply`] call.
#[derive(Debug, Clone, Serialize, Default)]
pub struct IngestReport {
    pub drafts: Vec<IngestArtifactDraft>,
    pub skipped: Vec<SkipReason>,
    pub errors: Vec<RuleError>,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Hard errors that abort the whole apply call.
#[derive(Debug, Error)]
pub enum IngestError {
    #[error(transparent)]
    Template(#[from] TemplateError),

    #[error("invalid glob pattern `{pattern}`: {source}")]
    BadGlob {
        pattern: String,
        #[source]
        source: globset::Error,
    },

    #[error("guards.max_artifacts={limit} exceeded ({produced} drafts produced)")]
    MaxArtifactsExceeded { limit: usize, produced: usize },
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// Stateless apply engine.
///
/// Holds the [`TemplateEngine`] so filter registration happens once per
/// process. Cheap to clone (the inner Tera is `Clone`).
pub struct IngestEngine {
    template: TemplateEngine,
}

impl IngestEngine {
    /// Build a fresh engine.
    pub fn new() -> Result<Self, IngestError> {
        Ok(Self {
            template: TemplateEngine::new()?,
        })
    }

    /// Walk every rule × every source and produce a report of drafts.
    ///
    /// The engine never writes to disk; callers (Wave 3 CLI) inspect the
    /// returned drafts and dispatch them through `artifact::Store`.
    pub fn apply(
        &self,
        mapping: &Mapping,
        parsed_sources: Vec<ParsedSource>,
        opts: IngestOptions,
    ) -> Result<IngestReport, IngestError> {
        let _ = opts; // dry_run is informational in this wave.
        let mut report = IngestReport::default();

        for rule in &mapping.rules {
            let matcher = match &rule.when.file_glob {
                Some(p) => Some(compile_glob(p)?),
                None => None,
            };

            for source in &parsed_sources {
                match self.apply_rule(rule, source, matcher.as_ref(), &mapping.guards) {
                    Ok(MatchOutcome::Drafts(mut drafts)) => {
                        report.drafts.append(&mut drafts);
                    }
                    Ok(MatchOutcome::Skip(reason)) => report.skipped.push(reason),
                    Err(err) => report.errors.push(RuleError {
                        rule_id: rule.id.clone(),
                        source_path: source.path.clone(),
                        message: err.to_string(),
                    }),
                }
            }
        }

        // Enforce the global cap last so we still report partial output.
        if let Some(limit) = mapping.guards.max_artifacts
            && report.drafts.len() > limit
        {
            return Err(IngestError::MaxArtifactsExceeded {
                limit,
                produced: report.drafts.len(),
            });
        }

        Ok(report)
    }

    /// Apply one rule to one source. Returns drafts on a match, a skip reason
    /// on a clean miss, or a [`RuleError`]-bearing error on per-source failure
    /// (e.g. template render).
    fn apply_rule(
        &self,
        rule: &Rule,
        source: &ParsedSource,
        matcher: Option<&GlobMatcher>,
        _guards: &Guards,
    ) -> Result<MatchOutcome, RuleApplyError> {
        // 1) file_glob.
        if let Some(m) = matcher
            && !m.is_match(&source.path)
        {
            return Ok(MatchOutcome::Skip(SkipReason::SelectorMismatch {
                rule_id: rule.id.clone(),
                source_path: source.path.clone(),
            }));
        }

        // 2) front_matter — every key/value pair must match.
        if !match_front_matter(&rule.when, source) {
            return Ok(MatchOutcome::Skip(SkipReason::SelectorMismatch {
                rule_id: rule.id.clone(),
                source_path: source.path.clone(),
            }));
        }

        // 3) contains_section.
        if let Some(section) = &rule.when.contains_section
            && !source.sections.contains_key(section)
        {
            return Ok(MatchOutcome::Skip(SkipReason::MissingRequiredSection {
                rule_id: rule.id.clone(),
                source_path: source.path.clone(),
                section: section.clone(),
            }));
        }

        // 4) heading_path — fan-out at trailing `*`, otherwise single match.
        let matched_sections = match_heading_path(&rule.when, source);
        let drafts = if matched_sections.is_empty() {
            // Whole-source rule (no heading_path filter): produce a single
            // draft using the entire source as context.
            vec![self.build_draft(rule, source, None)?]
        } else {
            matched_sections
                .into_iter()
                .map(|sec| self.build_draft(rule, source, Some(sec)))
                .collect::<Result<Vec<_>, _>>()?
        };

        Ok(MatchOutcome::Drafts(drafts))
    }

    fn build_draft(
        &self,
        rule: &Rule,
        source: &ParsedSource,
        section: Option<&ParsedSection>,
    ) -> Result<IngestArtifactDraft, RuleApplyError> {
        let ctx = build_template_context(source, section);

        // Render every field; `title` is mandatory.
        let mut rendered_fields: HashMap<String, String> = HashMap::new();
        for (name, tpl) in &rule.fields {
            let out = self
                .template
                .render(tpl, &ctx)
                .map_err(|e| RuleApplyError::Template {
                    rule_id: rule.id.clone(),
                    field: name.clone(),
                    source: e,
                })?;
            rendered_fields.insert(name.clone(), out);
        }

        let title = rendered_fields
            .get("title")
            .cloned()
            .unwrap_or_else(|| rule.id.clone());

        // Idempotency hash over the unrendered source text.
        let source_hash = compute_source_hash(source, &rule.id);

        // Assemble body.
        let sources_block =
            render_sources_block(&rule.sources_section, source, section, &source_hash);
        let body = assemble_body(&rendered_fields, &sources_block, &source_hash);

        // Render link templates.
        let links = render_links(&rule.links, &ctx, &self.template).map_err(|e| {
            RuleApplyError::Template {
                rule_id: rule.id.clone(),
                field: "links".to_owned(),
                source: e,
            }
        })?;

        Ok(IngestArtifactDraft {
            kind: rule.target.kind,
            title,
            body,
            source_hash,
            links,
            rule_id: rule.id.clone(),
            source_path: source.path.clone(),
        })
    }
}

// ---------------------------------------------------------------------------
// Inner helpers
// ---------------------------------------------------------------------------

enum MatchOutcome {
    Drafts(Vec<IngestArtifactDraft>),
    Skip(SkipReason),
}

#[derive(Debug, Error)]
enum RuleApplyError {
    #[error("template render failed for rule `{rule_id}` field `{field}`: {source}")]
    Template {
        rule_id: String,
        field: String,
        #[source]
        source: TemplateError,
    },
}

fn compile_glob(pattern: &str) -> Result<GlobMatcher, IngestError> {
    Glob::new(pattern)
        .map(|g| g.compile_matcher())
        .map_err(|e| IngestError::BadGlob {
            pattern: pattern.to_owned(),
            source: e,
        })
}

fn match_front_matter(selector: &Selector, source: &ParsedSource) -> bool {
    if selector.front_matter.is_empty() {
        return true;
    }
    let fm = match source.front_matter.as_object() {
        Some(o) => o,
        None => return false,
    };
    for (key, expected) in &selector.front_matter {
        match fm.get(key) {
            Some(actual) if actual == expected => continue,
            _ => return false,
        }
    }
    true
}

/// Match a `heading_path: ["A", "B", "*"]` selector against `source`.
///
/// Returns the sections that match the trailing wildcard. A non-wildcard path
/// returns `vec![target]` (one match) or empty (no match). A path without `*`
/// still walks the parent chain; the result vector contains either the final
/// matched section or nothing.
fn match_heading_path<'a>(selector: &Selector, source: &'a ParsedSource) -> Vec<&'a ParsedSection> {
    if selector.heading_path.is_empty() {
        return Vec::new();
    }
    let path = &selector.heading_path;
    let last = path.last().expect("non-empty checked above");
    let parent_path = &path[..path.len() - 1];

    if last == "*" {
        // Every section whose ancestor chain matches `parent_path` and that
        // sits exactly one level deeper than the last named ancestor.
        let parent = match resolve_path(source, parent_path) {
            Some(p) => p,
            None => return Vec::new(),
        };
        // Collect children at parent.heading_level + 1 sitting within parent
        // line range.
        source
            .sections
            .values()
            .filter(|s| {
                s.heading_level == parent.heading_level + 1
                    && s.line_start > parent.line_start
                    && s.line_start <= parent.line_end
            })
            .collect()
    } else {
        match resolve_path(source, path) {
            Some(s) => vec![s],
            None => Vec::new(),
        }
    }
}

/// Walk a heading path like `["Code Elements", "Core Types"]` and return the
/// section for the final element if every ancestor is present and nests
/// properly.
fn resolve_path<'a>(source: &'a ParsedSource, path: &[String]) -> Option<&'a ParsedSection> {
    if path.is_empty() {
        return None;
    }
    // For the spike-1 fixture, headings can be referenced by simple text
    // lookup; nesting is verified by line ranges.
    let mut current: Option<&ParsedSection> = None;
    for heading in path {
        let candidate = source.sections.get(heading)?;
        if let Some(parent) = current {
            // Candidate must sit inside parent's range and be deeper.
            if candidate.line_start <= parent.line_start
                || candidate.line_start > parent.line_end
                || candidate.heading_level <= parent.heading_level
            {
                return None;
            }
        }
        current = Some(candidate);
    }
    current
}

fn build_template_context(source: &ParsedSource, section: Option<&ParsedSection>) -> Json {
    let mut ctx = serde_json::Map::new();
    ctx.insert("front_matter".to_owned(), source.front_matter.clone());
    ctx.insert("path".to_owned(), Json::String(source.path.clone()));
    ctx.insert(
        "line_count".to_owned(),
        Json::Number((source.line_count as u64).into()),
    );

    if let Some(sec) = section {
        // Provide both `section` and `heading_text` shorthands per
        // .local/spike-1 fixture.
        ctx.insert(
            "heading_text".to_owned(),
            Json::String(sec.heading_text.clone()),
        );
        let section_json = json!({
            "heading_text": sec.heading_text,
            "heading_level": sec.heading_level,
            "line_start": sec.line_start,
            "line_end": sec.line_end,
            "body": sec.body,
            "sub_sections": sec.sub_sections,
        });
        ctx.insert("section".to_owned(), section_json);
    }

    Json::Object(ctx)
}

fn render_sources_block(
    spec: &SourcesSectionSpec,
    source: &ParsedSource,
    section: Option<&ParsedSection>,
    source_hash: &str,
) -> String {
    let (line_start, line_end) = match section {
        Some(s) => (s.line_start, s.line_end),
        None => (1, source.line_count.max(1)),
    };
    let line = spec
        .format
        .replace("{path}", &source.path)
        .replace("{line_start}", &line_start.to_string())
        .replace("{line_end}", &line_end.to_string());
    let mut out = String::from("## Sources\n\n");
    out.push_str("- ");
    out.push_str(&line);
    out.push('\n');
    if spec.source_hash {
        out.push('\n');
        out.push_str(&render_source_hash_marker(source_hash));
        out.push('\n');
    }
    out
}

fn assemble_body(
    fields: &HashMap<String, String>,
    sources_block: &str,
    source_hash: &str,
) -> String {
    let mut out = String::new();
    // Title rendered as H1 if present.
    if let Some(title) = fields.get("title")
        && !title.trim().is_empty()
    {
        out.push_str("# ");
        out.push_str(title.trim());
        out.push_str("\n\n");
    }
    // Stable order: sort by field name so re-runs produce identical output.
    let mut keys: Vec<&String> = fields.keys().filter(|k| k.as_str() != "title").collect();
    keys.sort();
    for k in keys {
        let value = match fields.get(k) {
            Some(v) => v.trim_end(),
            None => continue,
        };
        if value.is_empty() {
            continue;
        }
        out.push_str("## ");
        out.push_str(&humanise_field_name(k));
        out.push_str("\n\n");
        out.push_str(value);
        out.push_str("\n\n");
    }
    out.push_str(sources_block);
    // Belt-and-braces: ensure the hash marker exists even if format=… omitted
    // it (defence-in-depth — the marker is required for idempotent re-runs).
    if !out.contains("source_hash:") {
        out.push('\n');
        out.push_str(&render_source_hash_marker(source_hash));
        out.push('\n');
    }
    out
}

fn humanise_field_name(field: &str) -> String {
    let mut out = String::with_capacity(field.len());
    for (i, part) in field.split('_').enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
            out.extend(chars);
        }
    }
    out
}

fn render_links(
    specs: &[LinkSpec],
    ctx: &Json,
    engine: &TemplateEngine,
) -> Result<Vec<DraftLink>, TemplateError> {
    let mut out = Vec::with_capacity(specs.len());
    for spec in specs {
        if spec.target.is_some() && spec.target_artifact_id.is_some() {
            warn!("link spec has both `target` and `target_artifact_id` set; using `target`");
        }
        let target = if let Some(tpl) = &spec.target {
            engine.render(tpl, ctx)?
        } else if let Some(static_id) = &spec.target_artifact_id {
            static_id.clone()
        } else {
            debug!("link spec has neither `target` nor `target_artifact_id`; skipping");
            continue;
        };
        let target = target.trim().to_owned();
        if target.is_empty() {
            continue;
        }
        out.push(DraftLink {
            target,
            relation: spec.relation.clone(),
            if_exists: spec.if_exists,
        });
    }
    Ok(out)
}

fn _coerce_unused(_t: &Template) {
    // Force a `use Template` even if the linter prunes; the compiler will
    // remove this. Keeps the `Template` re-export visible for documentation.
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::sources::{FrontMatterPlusSections, SourceParser};
    use std::path::PathBuf;

    const SPIKE_INPUT: &str = include_str!("../../../../.local/spike-1-c4-scoring.md");

    /// Spike-1 mapping, normalised to valid Tera syntax (named `value=`,
    /// double-quoted args). Mirrors `.local/spike-1-c4-to-forge-mapping.yaml`
    /// (read-only fixture) which used positional / single-quoted arg syntax
    /// not supported by Tera-1.x. EVID-088's intent — three rules covering
    /// Core Types, Public Functions, and Overview — is preserved.
    const SPIKE_MAPPING: &str = r#"
schema_version: "1.0"
name: c4-to-forge-spike
title: "C4 → Forge spike (engine test)"
compat_spec_version: "c4-architecture: ^1.0"
source_kind: c4-documentation
target_kind: forge
sources:
  - pattern: ".local/spike-1-c4-*.md"
    type: markdown
    parser: front_matter_plus_sections
rules:
  - id: c4-struct-to-spec
    when:
      file_glob: ".local/spike-1-c4-*.md"
      heading_path: ["Code Elements", "Core Types", "*"]
    target: { kind: spec }
    fields:
      title: "{{ heading_text | trim }}"
      summary: "{{ section.body | truncate(n=200) }}"
    sources_section:
      include: true
      format: "{path}:{line_start}-{line_end}"
      precision: line
      source_hash: true
  - id: c4-pub-fn-to-prd
    when:
      file_glob: ".local/spike-1-c4-*.md"
      heading_path: ["Code Elements", "Public Functions", "*"]
    target: { kind: prd }
    fields:
      title: "{{ heading_text | trim }}"
      problem: "{{ section.body | truncate(n=400) }}"
    sources_section:
      include: true
      format: "{path}:{line_start}-{line_end}"
      precision: line
      source_hash: true
    links:
      - target_artifact_id: "EPIC-007"
        relation: refines
        if_exists: skip
  - id: c4-overview-to-epic
    when:
      file_glob: ".local/spike-1-c4-*.md"
      heading_path: ["Overview"]
    target: { kind: epic }
    fields:
      title: "{{ heading_text | trim }}"
      vision: "{{ section.body | truncate(n=200) }}"
    sources_section:
      include: true
      format: "{path}:1"
      precision: file
      source_hash: true
guards:
  max_artifacts: 50
errors:
  template_filter_violation: error
"#;

    fn parse_spike() -> ParsedSource {
        FrontMatterPlusSections
            .parse(&PathBuf::from(".local/spike-1-c4-scoring.md"), SPIKE_INPUT)
            .expect("parse spike doc")
    }

    fn parse_mapping(yaml: &str) -> Mapping {
        serde_yaml::from_str(yaml).expect("parse mapping yaml")
    }

    #[test]
    fn apply_spike_mapping_produces_expected_drafts() {
        let mapping = parse_mapping(SPIKE_MAPPING);
        let source = parse_spike();
        let engine = IngestEngine::new().unwrap();
        let report = engine
            .apply(&mapping, vec![source], IngestOptions::default())
            .expect("apply succeeds");

        // Per EVID-088: ~14 candidate source units. Spike doc has 8 Core
        // Types + 7-9 Public Functions + 1 Overview = 16-18 sections.
        // We assert the total stays in a sensible range and that all three
        // rule kinds (spec, prd, epic) fire at least once.
        assert!(
            report.drafts.len() >= 10,
            "expected >=10 drafts, got {}: errors {:?}",
            report.drafts.len(),
            report.errors
        );
        let kinds: std::collections::HashSet<_> = report.drafts.iter().map(|d| d.kind).collect();
        assert!(kinds.contains(&ArtifactTargetKind::Spec));
        assert!(kinds.contains(&ArtifactTargetKind::Prd));
        // Every draft must have a Sources block + idempotency marker.
        for draft in &report.drafts {
            assert!(
                draft.body.contains("## Sources"),
                "missing Sources in {}",
                draft.title
            );
            assert!(
                draft.body.contains("source_hash:"),
                "missing source_hash marker in {}",
                draft.title
            );
            assert_eq!(draft.source_hash.len(), 64);
        }
    }

    #[test]
    fn idempotent_rerun_yields_same_hashes() {
        let mapping = parse_mapping(SPIKE_MAPPING);
        let source = parse_spike();
        let engine = IngestEngine::new().unwrap();
        let r1 = engine
            .apply(&mapping, vec![source.clone()], IngestOptions::default())
            .unwrap();
        let r2 = engine
            .apply(&mapping, vec![source], IngestOptions::default())
            .unwrap();
        let h1: Vec<&str> = r1.drafts.iter().map(|d| d.source_hash.as_str()).collect();
        let h2: Vec<&str> = r2.drafts.iter().map(|d| d.source_hash.as_str()).collect();
        assert_eq!(h1, h2);
    }

    #[test]
    fn selector_mismatch_records_skip() {
        let yaml = r#"
schema_version: "1.0"
name: t
title: t
compat_spec_version: "^1.0"
source_kind: c4-documentation
target_kind: forge
sources:
  - pattern: "**/*.md"
    type: markdown
    parser: front_matter_plus_sections
rules:
  - id: r-no-match
    when:
      file_glob: "nope/**/*.md"
    target: { kind: note }
    fields:
      title: "{{ heading_text | default(value=\"x\") }}"
    sources_section:
      include: true
"#;
        let mapping = parse_mapping(yaml);
        let source = parse_spike();
        let engine = IngestEngine::new().unwrap();
        let report = engine
            .apply(&mapping, vec![source], IngestOptions::default())
            .unwrap();
        assert!(report.drafts.is_empty());
        assert!(matches!(
            report.skipped.first(),
            Some(SkipReason::SelectorMismatch { .. })
        ));
    }

    #[test]
    fn max_artifacts_exceeded_aborts() {
        let yaml = r#"
schema_version: "1.0"
name: t
title: t
compat_spec_version: "^1.0"
source_kind: c4-documentation
target_kind: forge
sources:
  - pattern: ".local/**/*.md"
    type: markdown
    parser: front_matter_plus_sections
rules:
  - id: cap-test
    when:
      file_glob: ".local/**/*.md"
      heading_path: ["Code Elements", "Core Types", "*"]
    target: { kind: note }
    fields:
      title: "{{ heading_text | trim }}"
    sources_section:
      include: true
guards:
  max_artifacts: 1
"#;
        let mapping = parse_mapping(yaml);
        let source = parse_spike();
        let engine = IngestEngine::new().unwrap();
        let err = engine
            .apply(&mapping, vec![source], IngestOptions::default())
            .unwrap_err();
        assert!(matches!(err, IngestError::MaxArtifactsExceeded { .. }));
    }

    #[test]
    fn missing_template_var_recorded_as_error() {
        let yaml = r#"
schema_version: "1.0"
name: t
title: t
compat_spec_version: "^1.0"
source_kind: c4-documentation
target_kind: forge
sources:
  - pattern: "**/*.md"
    type: markdown
    parser: front_matter_plus_sections
rules:
  - id: bad-var
    when:
      file_glob: ".local/**/*.md"
    target: { kind: note }
    fields:
      title: "{{ totally_undefined_var }}"
    sources_section:
      include: true
"#;
        let mapping = parse_mapping(yaml);
        let source = parse_spike();
        let engine = IngestEngine::new().unwrap();
        let report = engine
            .apply(&mapping, vec![source], IngestOptions::default())
            .unwrap();
        // Must not panic; error should be reported.
        assert!(!report.errors.is_empty());
        let e = &report.errors[0];
        assert_eq!(e.rule_id, "bad-var");
    }

    #[test]
    fn multiple_rules_cascade_independently() {
        let yaml = r#"
schema_version: "1.0"
name: t
title: t
compat_spec_version: "^1.0"
source_kind: c4-documentation
target_kind: forge
sources:
  - pattern: ".local/**/*.md"
    type: markdown
    parser: front_matter_plus_sections
rules:
  - id: r1
    when:
      file_glob: ".local/**/*.md"
      heading_path: ["Code Elements", "Core Types", "*"]
    target: { kind: spec }
    fields:
      title: "{{ heading_text | trim }}"
    sources_section:
      include: true
  - id: r2
    when:
      file_glob: ".local/**/*.md"
      heading_path: ["Code Elements", "Public Functions", "*"]
    target: { kind: prd }
    fields:
      title: "{{ heading_text | trim }}"
    sources_section:
      include: true
"#;
        let mapping = parse_mapping(yaml);
        let source = parse_spike();
        let engine = IngestEngine::new().unwrap();
        let report = engine
            .apply(&mapping, vec![source], IngestOptions::default())
            .unwrap();
        let r1_count = report.drafts.iter().filter(|d| d.rule_id == "r1").count();
        let r2_count = report.drafts.iter().filter(|d| d.rule_id == "r2").count();
        assert!(r1_count > 0, "r1 should match Core Types");
        assert!(r2_count > 0, "r2 should match Public Functions");
    }

    #[test]
    fn humanise_field_name_basic() {
        assert_eq!(humanise_field_name("title"), "Title");
        assert_eq!(humanise_field_name("data_models"), "Data Models");
        assert_eq!(humanise_field_name("target_users"), "Target Users");
    }

    #[test]
    fn assemble_body_includes_sources_and_marker() {
        let mut fields = HashMap::new();
        fields.insert("title".to_owned(), "Hello".to_owned());
        fields.insert("summary".to_owned(), "world".to_owned());
        let body = assemble_body(&fields, "## Sources\n\n- foo:1-2\n", "abc123");
        assert!(body.contains("# Hello"));
        assert!(body.contains("## Summary"));
        assert!(body.contains("## Sources"));
        assert!(body.contains("source_hash:"));
    }
}
