//! Ingest type definitions — Rust mirror of [SPEC-004 Mapping YAML schema][spec].
//!
//! These types are loaded from the `mapping.yaml` files shipped by external
//! plugin packs (c4-architecture, autoresearch, git-log, ddd-model, sparc-spec)
//! and used by the ingest engine ([PRD-066][prd]) to translate plugin output
//! into forge artifacts.
//!
//! # Security boundary
//!
//! [`Template`] enforces a strict whitelist of allowed Tera filters at
//! deserialization time. Any mapping containing an unknown filter is rejected
//! before it is ever evaluated. This is the primary defence against arbitrary
//! code execution from untrusted mapping packs.
//!
//! ## Tera built-in pre-emption (CRIT-T3)
//!
//! Several whitelisted filter names — notably `replace` and `default` — are
//! also Tera **built-ins**. Tera resolves built-ins before any user-registered
//! filter, so registering custom handlers with the same name is dead code
//! during render. The whitelist therefore allows these names *because the
//! Tera built-in semantics are acceptable*; downstream consumers must NOT
//! rely on a custom replacement being invoked. The validator
//! [`Template::from_str`] enforces only the **set of filter names** allowed —
//! the actual filter behavior is whatever Tera ships.
//!
//! # Hallucination-proof invariant
//!
//! Per [ADR-009][adr], every generated artifact MUST contain a `## Sources`
//! section pointing back to the originating file:line range. Therefore
//! [`SourcesSectionSpec::include`] is required to be `true`; deserialization
//! fails otherwise.
//!
//! [spec]: ../../../../.forgeplan/specs/SPEC-004-mapping-yaml-schema.md
//! [prd]: ../../../../.forgeplan/prds/PRD-066-ingest-engine-mapping-yaml-format-c4-to-forge-autoresearch-to-forge-git-to-forge-ddd-to-forge-spec-to-forge.md
//! [adr]: ../../../../.forgeplan/adrs/ADR-009-forgeplan-as-orchestrator-playbook-skill-agent-mapping-pack-marketplace-model.md

use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use schemars::JsonSchema;
use schemars::r#gen::SchemaGenerator;
use schemars::schema::{InstanceType, Metadata, Schema, SchemaObject};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use tera::ast::{Expr, ExprVal, Node};

// ---------------------------------------------------------------------------
// Whitelist
// ---------------------------------------------------------------------------

/// Filters allowed inside [`Template`] expressions.
///
/// The list mirrors SPEC-004 §`fields` plus the `table` filter added per
/// [EVID-088][evid] (Spike-1 finding for c4-to-forge mappings).
///
/// Any filter outside this list is rejected at load time — see
/// [`Template::from_str`].
///
/// [evid]: ../../../../.forgeplan/evidence/EVID-088-spike-1-c4-to-forge-mapping-concept-validated-on-scoring-module.md
pub const ALLOWED_FILTERS: &[&str] = &[
    "trim",
    "lower",
    "upper",
    "bullet_list",
    "comma_list",
    "slugify",
    "truncate",
    "default",
    "replace",
    "table",
];

// ---------------------------------------------------------------------------
// Top-level Mapping
// ---------------------------------------------------------------------------

/// Top-level mapping document — the parsed contents of one `mapping.yaml`.
///
/// Validated invariants (enforced at deserialize via [`RawMapping`] + `try_from`):
/// * [`Self::sources`] is non-empty.
/// * [`Self::rules`] is non-empty.
/// * Every [`Rule::fields`] map is non-empty.
/// * Every [`Rule::sources_section`] has `include: true`.
/// * No [`Template`] uses a filter outside [`ALLOWED_FILTERS`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, try_from = "RawMapping")]
pub struct Mapping {
    /// Schema format version, e.g. `"1.0"`.
    pub schema_version: String,
    /// Unique mapping identifier (kebab-case).
    pub name: String,
    /// Human-readable mapping name.
    pub title: String,
    /// Upstream plugin output compatibility range (semver).
    pub compat_spec_version: CompatSpecVersion,
    /// What kind of plugin output this mapping consumes.
    pub source_kind: SourceKind,
    /// Currently always [`TargetKind::Forge`].
    pub target_kind: TargetKind,
    /// Input file discovery rules (non-empty).
    pub sources: Vec<SourceSpec>,
    /// Transformation rules (non-empty).
    pub rules: Vec<Rule>,
    /// Optional invariants & safety limits.
    #[serde(default)]
    pub guards: Guards,
    /// Optional per-error policy overrides.
    #[serde(default)]
    pub errors: ErrorPolicy,
}

impl Mapping {
    /// Returns every distinct [`TargetSpec`] referenced by the mapping's rules.
    ///
    /// Useful for graph linkage and pre-flight artifact-kind permission checks
    /// (Wave 2 ingest engine).
    pub fn referenced_target_kinds(&self) -> HashSet<TargetSpec> {
        self.rules.iter().map(|r| r.target.clone()).collect()
    }

    /// Returns the set of all rule IDs declared in this mapping.
    pub fn all_rule_ids(&self) -> HashSet<&str> {
        self.rules.iter().map(|r| r.id.as_str()).collect()
    }

    /// Defence-in-depth: re-checks all template filters against the whitelist
    /// after construction. Should always return `None` for a successfully
    /// deserialized [`Mapping`] (the deserializer already rejects bad filters)
    /// — this method exists so callers that build mappings programmatically
    /// can still verify the invariant.
    ///
    /// Returns `Some((rule_id, bad_filter_name))` on the first violation.
    pub fn has_disallowed_filter(&self) -> Option<(&str, &str)> {
        for rule in &self.rules {
            for tpl in rule.fields.values() {
                if let Some(bad) = tpl.first_disallowed_filter() {
                    return Some((rule.id.as_str(), bad));
                }
            }
        }
        None
    }

    /// Returns `true` if `plugin_version` satisfies this mapping's
    /// `compat_spec_version` requirement.
    pub fn version_compat(&self, plugin_version: &Version) -> bool {
        self.compat_spec_version.req.matches(plugin_version)
    }
}

// ---------------------------------------------------------------------------
// Shadow / Raw types — used to enforce invariants at deserialize.
// ---------------------------------------------------------------------------

/// Mirror of [`Mapping`] without invariant checks; used as deserialize target
/// for `try_from = "RawMapping"`. Kept private — consumers always see the
/// validated [`Mapping`].
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RawMapping {
    schema_version: String,
    name: String,
    title: String,
    compat_spec_version: CompatSpecVersion,
    source_kind: SourceKind,
    target_kind: TargetKind,
    sources: Vec<SourceSpec>,
    rules: Vec<Rule>,
    #[serde(default)]
    guards: Guards,
    #[serde(default)]
    errors: ErrorPolicy,
}

impl TryFrom<RawMapping> for Mapping {
    type Error = String;

    fn try_from(raw: RawMapping) -> Result<Self, Self::Error> {
        if raw.sources.is_empty() {
            return Err("`sources` must contain at least one entry".to_owned());
        }
        if raw.rules.is_empty() {
            return Err("`rules` must contain at least one entry".to_owned());
        }
        for rule in &raw.rules {
            if rule.fields.is_empty() {
                return Err(format!("rule `{}`: `fields` must be non-empty", rule.id));
            }
        }
        Ok(Mapping {
            schema_version: raw.schema_version,
            name: raw.name,
            title: raw.title,
            compat_spec_version: raw.compat_spec_version,
            source_kind: raw.source_kind,
            target_kind: raw.target_kind,
            sources: raw.sources,
            rules: raw.rules,
            guards: raw.guards,
            errors: raw.errors,
        })
    }
}

// ---------------------------------------------------------------------------
// Compat spec version
// ---------------------------------------------------------------------------

/// Upstream plugin output compatibility specifier.
///
/// Accepts two forms:
/// * `"^1.0"` — bare semver requirement
/// * `"plugin-name: ^1.0"` — namespaced (plugin name + requirement)
///
/// The plugin name is informational; matching is done against [`Self::req`].
#[derive(Debug, Clone, PartialEq)]
pub struct CompatSpecVersion {
    /// Optional plugin name prefix (the bit before `:`). Informational.
    pub plugin: Option<String>,
    /// The semver requirement against which the plugin's runtime version is
    /// matched.
    pub req: VersionReq,
}

impl FromStr for CompatSpecVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        let (plugin, req_str) = match trimmed.split_once(':') {
            Some((name, req)) => (Some(name.trim().to_owned()), req.trim()),
            None => (None, trimmed),
        };
        let req = VersionReq::parse(req_str)
            .map_err(|e| format!("invalid compat_spec_version `{trimmed}`: {e}"))?;
        Ok(CompatSpecVersion { plugin, req })
    }
}

impl std::fmt::Display for CompatSpecVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.plugin {
            Some(name) => write!(f, "{name}: {}", self.req),
            None => write!(f, "{}", self.req),
        }
    }
}

impl Serialize for CompatSpecVersion {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for CompatSpecVersion {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let s = String::deserialize(de)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl JsonSchema for CompatSpecVersion {
    fn schema_name() -> String {
        "CompatSpecVersion".to_owned()
    }

    fn json_schema(_g: &mut SchemaGenerator) -> Schema {
        Schema::Object(SchemaObject {
            instance_type: Some(InstanceType::String.into()),
            metadata: Some(Box::new(Metadata {
                description: Some(
                    "Semver requirement, optionally prefixed with a plugin name, e.g. \
                     `^1.0` or `c4-architecture: ^1.0`."
                        .to_owned(),
                ),
                ..Default::default()
            })),
            ..Default::default()
        })
    }
}

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// External plugin output domain consumed by a mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SourceKind {
    /// Output of a `c4-architecture:*` plugin (markdown C4 docs).
    C4Documentation,
    /// Output of an `autoresearch` plugin (research summaries).
    Autoresearch,
    /// Raw `.git/log` data.
    GitLog,
    /// Output of a DDD modelling plugin.
    DddModel,
    /// Output of a SPARC spec plugin.
    SparcSpec,
}

/// Where a mapping deposits the artifacts it generates.
///
/// Currently only [`Self::Forge`] is defined; the enum exists so future
/// targets (e.g. external knowledge bases) can be added without breaking the
/// schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum TargetKind {
    /// Forge artifact store (`.forgeplan/`).
    Forge,
}

/// Parser binding for a [`SourceSpec`]. Declarative — no embedded code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Parser {
    /// YAML frontmatter + `## sections`.
    FrontMatterPlusSections,
    /// Markdown without frontmatter.
    MarkdownOnly,
    /// `git log` + `git blame` — for ADR inference.
    LogWithBlame,
    /// Raw JSON document.
    Json,
    /// Raw YAML document.
    Yaml,
}

/// Conflict resolution strategy when an auto-generated link already exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IfExists {
    /// Silently skip — no warning, no error.
    #[default]
    Skip,
    /// Log a warning and continue.
    Warn,
    /// Abort the rule with an error.
    Error,
}

/// Forge artifact kind that a [`Rule`] produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ArtifactTargetKind {
    Prd,
    Adr,
    Epic,
    Note,
    Spec,
    Problem,
}

/// Hash precision for the `## Sources` block. See SPEC-004 §`sources_section`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SourcePrecision {
    #[default]
    Line,
    Block,
    File,
}

/// Per-error policy decision used by [`ErrorPolicy`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorAction {
    /// Treat as error (default for hard failures).
    Error,
    /// Log a warning and continue.
    Warn,
    /// Silently skip.
    Skip,
}

// ---------------------------------------------------------------------------
// Source spec / Selector / Target / Rule
// ---------------------------------------------------------------------------

/// One entry in the top-level `sources:` array — describes how to discover
/// input files.
///
/// Unknown fields are ignored on nested rule-side specs to allow forward-
/// compatible mapping packs to ship without breaking older runtimes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SourceSpec {
    /// Glob pattern relative to the project root, e.g. `"docs/**/*.md"`.
    pub pattern: String,
    /// Logical input type tag (free-form — mainly informational; the actual
    /// parsing strategy is decided by `parser`).
    #[serde(rename = "type")]
    pub kind: String,
    /// Parser strategy applied to each matched file.
    pub parser: Parser,
}

/// Match conditions for a [`Rule`] — all listed selectors are AND-combined.
///
/// A non-match silently skips the rule (not an error).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct Selector {
    /// Restrict the rule to files matching this glob.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_glob: Option<String>,
    /// Map of frontmatter keys/values that must all be present.
    ///
    /// Values are stored as [`serde_json::Value`] for portability and
    /// `JsonSchema` support — YAML scalars/sequences/maps round-trip cleanly
    /// through serde_yaml → JSON.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub front_matter: HashMap<String, serde_json::Value>,
    /// Document must contain this section heading.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contains_section: Option<String>,
    /// Heading path, e.g. `["Code Elements", "Core Types", "*"]`. The trailing
    /// `"*"` denotes "any heading at this level — fan out".
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub heading_path: Vec<String>,
}

/// `target:` block inside a [`Rule`]. Distinct from the top-level
/// [`TargetKind`] which decides the back-end.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct TargetSpec {
    /// Forge artifact kind to produce.
    pub kind: ArtifactTargetKind,
}

/// One transformation rule.
///
/// Field-level invariants:
/// * [`Self::fields`] is non-empty (enforced by [`Mapping`]'s `try_from`).
/// * [`Self::sources_section`] has `include: true` (enforced by
///   [`SourcesSectionSpec::deserialize`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Rule {
    /// Stable kebab-case rule id (unique within the mapping).
    pub id: String,
    /// Match conditions.
    pub when: Selector,
    /// What to produce.
    pub target: TargetSpec,
    /// Forge artifact field templates. Map key = forge field name.
    pub fields: HashMap<String, Template>,
    /// Hallucination-proof `## Sources` configuration. `include` MUST be true.
    pub sources_section: SourcesSectionSpec,
    /// Optional auto-created links.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<LinkSpec>,
}

/// `links:` entry — either a templated lookup or a static artifact id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LinkSpec {
    /// Template reference resolved at apply-time
    /// (e.g. `"{{front_matter.parent_container}}"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<Template>,
    /// Static artifact id (e.g. `"EPIC-006"`). Mutually exclusive with
    /// [`Self::target`] but the schema does not enforce that — Wave 2 engine
    /// will warn if both are set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_artifact_id: Option<String>,
    /// Forgeplan link relation.
    pub relation: String,
    /// Behaviour when an equivalent link already exists.
    #[serde(default)]
    pub if_exists: IfExists,
}

/// `sources_section:` block — hallucination-proof invariant container.
///
/// `include: false` deserializes to a hard error per [ADR-009][adr]: every
/// generated artifact MUST cite its origin in a `## Sources` section.
///
/// [adr]: ../../../../.forgeplan/adrs/ADR-009-forgeplan-as-orchestrator-playbook-skill-agent-mapping-pack-marketplace-model.md
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct SourcesSectionSpec {
    /// MUST be `true`. Deserializing `false` returns an error.
    pub include: bool,
    /// Format string controlling how each cited source line is rendered, e.g.
    /// `"{path}:{line_start}-{line_end}"`.
    #[serde(default = "default_sources_format")]
    pub format: String,
    /// Hash precision used for idempotent re-runs.
    #[serde(default)]
    pub precision: SourcePrecision,
    /// Whether to include a content-hash for idempotency (PRD-066 AC-3).
    #[serde(default = "default_true")]
    pub source_hash: bool,
}

fn default_sources_format() -> String {
    "{path}:{line_start}-{line_end}".to_owned()
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSourcesSectionSpec {
    include: bool,
    #[serde(default = "default_sources_format")]
    format: String,
    #[serde(default)]
    precision: SourcePrecision,
    #[serde(default = "default_true")]
    source_hash: bool,
}

impl<'de> Deserialize<'de> for SourcesSectionSpec {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let raw = RawSourcesSectionSpec::deserialize(de)?;
        if !raw.include {
            return Err(serde::de::Error::custom(
                "sources_section.include must be true (ADR-009 hallucination-proof invariant)",
            ));
        }
        Ok(SourcesSectionSpec {
            include: raw.include,
            format: raw.format,
            precision: raw.precision,
            source_hash: raw.source_hash,
        })
    }
}

// ---------------------------------------------------------------------------
// Guards / ErrorPolicy
// ---------------------------------------------------------------------------

/// Safety limits + invariants that apply to a whole mapping.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Guards {
    /// Hard cap on the number of artifacts a mapping can produce in one run.
    /// `None` means unlimited; mappings are expected to set this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_artifacts: Option<usize>,
    /// Sections that every produced artifact must contain.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub require_section: Vec<String>,
    /// If true, an apply will refuse to overwrite an `active` artifact and
    /// only update `draft` artifacts. Default `true`.
    #[serde(default = "default_true")]
    pub forbid_overwrite_active: bool,
}

impl Default for Guards {
    fn default() -> Self {
        Self {
            max_artifacts: None,
            require_section: Vec::new(),
            forbid_overwrite_active: true,
        }
    }
}

/// Per-error policy overrides.
///
/// Unknown keys are accepted and stored in [`Self::extra`] — Wave 2 engine
/// will surface unknown entries as warnings.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct ErrorPolicy {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub missing_required_field: Option<ErrorAction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_unreachable: Option<ErrorAction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duplicate_source_hash: Option<ErrorAction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_filter_violation: Option<ErrorAction>,
    /// Forward-compat: any extra fields land here.
    #[serde(flatten)]
    pub extra: HashMap<String, ErrorAction>,
}

// ---------------------------------------------------------------------------
// Template — security-critical newtype.
// ---------------------------------------------------------------------------

/// Tera-templated string with a hardened filter whitelist.
///
/// Constructed via [`FromStr`] / `Deserialize`. The constructor parses the
/// template through the real Tera parser to guarantee:
/// 1. Syntactic validity (un-balanced braces etc. fail loudly).
/// 2. Every filter referenced is in [`ALLOWED_FILTERS`].
///
/// Storage is the original source text — the runtime re-parses through Tera
/// when rendering. Rationale: the AST cannot be serialized round-trip cheaply;
/// re-parse cost is negligible compared with file IO; and the source string is
/// what users wrote, which keeps diagnostics readable.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Template(String);

impl Template {
    /// Returns the original template source.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the variable paths referenced by `{{ ... }}` blocks, in source
    /// order, without dedup. E.g. `"{{front_matter.name | trim}}"` →
    /// `["front_matter.name"]`.
    pub fn extract_paths(&self) -> Vec<&str> {
        // Re-parse so we can walk a fresh AST tied to `self.0`'s lifetime.
        // Templates are tiny — this stays cheap.
        let mut out: Vec<&str> = Vec::new();
        let parsed = match tera::Template::new("inline", None, &self.0) {
            Ok(t) => t,
            Err(_) => return out,
        };
        // `parsed.ast` is owned by `parsed`; collect owned Strings then
        // resolve them back to slices of `self.0` for the lifetime contract.
        let owned = collect_idents(&parsed.ast);
        for path in owned {
            if let Some(start) = self.0.find(&path) {
                let end = start + path.len();
                // SAFETY: indices come from `find` on `self.0`.
                out.push(&self.0[start..end]);
            }
        }
        out
    }

    /// Returns the first filter name not present in [`ALLOWED_FILTERS`], or
    /// `None` if the template is clean. Used by [`Mapping::has_disallowed_filter`].
    pub fn first_disallowed_filter(&self) -> Option<&'static str> {
        let parsed = tera::Template::new("inline", None, &self.0).ok()?;
        for filter in collect_filters(&parsed.ast) {
            if let Some(bad) = ALLOWED_FILTERS
                .iter()
                .find(|allowed| **allowed == filter.as_str())
                .map_or(Some(filter), |_| None)
            {
                // We need to return a `&'static str` matching the bad name.
                // None of the allowed filters matched, but the bad name itself
                // is owned. We re-leak only when truly unknown — the caller
                // typically forwards to a serde error, so leaking is fine.
                let leaked: &'static str = Box::leak(bad.into_boxed_str());
                return Some(leaked);
            }
        }
        None
    }
}

impl FromStr for Template {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parsed = tera::Template::new("inline", None, s)
            .map_err(|e| format!("template parse error: {e}"))?;
        for filter in collect_filters(&parsed.ast) {
            if !ALLOWED_FILTERS.contains(&filter.as_str()) {
                return Err(format!(
                    "filter `{filter}` is not in the allowed whitelist {ALLOWED_FILTERS:?}"
                ));
            }
        }
        Ok(Template(s.to_owned()))
    }
}

impl Serialize for Template {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Template {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let s = String::deserialize(de)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl JsonSchema for Template {
    fn schema_name() -> String {
        "Template".to_owned()
    }

    fn json_schema(_g: &mut SchemaGenerator) -> Schema {
        Schema::Object(SchemaObject {
            instance_type: Some(InstanceType::String.into()),
            metadata: Some(Box::new(Metadata {
                description: Some(format!(
                    "Tera template restricted to whitelisted filters: {ALLOWED_FILTERS:?}. \
                     Arbitrary Tera constructs (set, macros, custom filters) are rejected."
                )),
                ..Default::default()
            })),
            ..Default::default()
        })
    }
}

// ---------------------------------------------------------------------------
// Tera AST walking helpers
// ---------------------------------------------------------------------------

/// Collects the names of every filter referenced anywhere in `nodes`.
fn collect_filters(nodes: &[Node]) -> Vec<String> {
    let mut out = Vec::new();
    for node in nodes {
        walk_node_filters(node, &mut out);
    }
    out
}

fn walk_node_filters(node: &Node, out: &mut Vec<String>) {
    // Exhaustive match — every Tera `Node` variant must be covered. New
    // variants in future Tera releases will surface as compile errors here,
    // ensuring the security walker stays in sync. See [CRIT-S1] in the
    // Phase 5 audit Round 1 for the original bypass that motivated this.
    match node {
        Node::VariableBlock(_, expr) => walk_expr_filters(expr, out),
        Node::FilterSection(_, fs, _) => {
            out.push(fs.filter.name.clone());
            for arg in fs.filter.args.values() {
                walk_expr_filters(arg, out);
            }
            for n in &fs.body {
                walk_node_filters(n, out);
            }
        }
        Node::Set(_, set) => walk_expr_filters(&set.value, out),
        Node::If(if_, _) => {
            for (_, cond, body) in &if_.conditions {
                walk_expr_filters(cond, out);
                for n in body {
                    walk_node_filters(n, out);
                }
            }
            if let Some((_, body)) = &if_.otherwise {
                for n in body {
                    walk_node_filters(n, out);
                }
            }
        }
        Node::Forloop(_, fl, _) => {
            walk_expr_filters(&fl.container, out);
            for n in &fl.body {
                walk_node_filters(n, out);
            }
            if let Some(body) = &fl.empty_body {
                for n in body {
                    walk_node_filters(n, out);
                }
            }
        }
        Node::Block(_, blk, _) => {
            for n in &blk.body {
                walk_node_filters(n, out);
            }
        }
        Node::MacroDefinition(_, mdef, _) => {
            // Default arg values can themselves carry filters; previously
            // ignored which let `{% macro m(x=y|striptags) %}` smuggle through.
            for default in mdef.args.values().flatten() {
                walk_expr_filters(default, out);
            }
            for n in &mdef.body {
                walk_node_filters(n, out);
            }
        }
        // Variants that carry no expressions: literally cannot host filters.
        Node::Super
        | Node::Text(_)
        | Node::Extends(_, _)
        | Node::Include(_, _, _)
        | Node::ImportMacro(_, _, _)
        | Node::Raw(_, _, _)
        | Node::Break(_)
        | Node::Continue(_)
        | Node::Comment(_, _) => {}
    }
}

fn walk_expr_filters(expr: &Expr, out: &mut Vec<String>) {
    for f in &expr.filters {
        out.push(f.name.clone());
        for arg in f.args.values() {
            walk_expr_filters(arg, out);
        }
    }
    walk_exprval_filters(&expr.val, out);
}

fn walk_exprval_filters(val: &ExprVal, out: &mut Vec<String>) {
    // Exhaustive match — every Tera `ExprVal` variant must be covered. The
    // previous wildcard arm let `Test` and `StringConcat` smuggle filters
    // past the whitelist (CRIT-S1). The compiler will now reject any new
    // variant added in future Tera versions until the walker is updated.
    match val {
        ExprVal::Math(m) => {
            walk_expr_filters(&m.lhs, out);
            walk_expr_filters(&m.rhs, out);
        }
        ExprVal::Logic(l) => {
            walk_expr_filters(&l.lhs, out);
            walk_expr_filters(&l.rhs, out);
        }
        ExprVal::FunctionCall(fc) => {
            for arg in fc.args.values() {
                walk_expr_filters(arg, out);
            }
        }
        ExprVal::MacroCall(mc) => {
            for arg in mc.args.values() {
                walk_expr_filters(arg, out);
            }
        }
        ExprVal::Array(items) => {
            for it in items {
                walk_expr_filters(it, out);
            }
        }
        ExprVal::In(in_) => {
            walk_expr_filters(&in_.lhs, out);
            walk_expr_filters(&in_.rhs, out);
        }
        ExprVal::Test(t) => {
            // Tera evaluates filters inside test args during render; the
            // walker MUST recurse here. Without this, the bypass
            // `{% if x is defined(y | striptags) %}` slips past the whitelist.
            for arg in &t.args {
                walk_expr_filters(arg, out);
            }
        }
        ExprVal::StringConcat(sc) => {
            // `values: Vec<ExprVal>` — no filters at this level (filters
            // live on `Expr`, not `ExprVal`), but the nested ExprVals can
            // contain `FunctionCall` / `Array` / `In` etc. with filtered
            // sub-expressions. Recurse so e.g. `"a" ~ f(x | striptags)` is
            // caught.
            for v in &sc.values {
                walk_exprval_filters(v, out);
            }
        }
        // True leaves — cannot host filters or sub-expressions.
        ExprVal::String(_)
        | ExprVal::Int(_)
        | ExprVal::Float(_)
        | ExprVal::Bool(_)
        | ExprVal::Ident(_) => {}
    }
}

/// Collects every `Ident(...)` used inside `{{ ... }}` blocks, in source order.
fn collect_idents(nodes: &[Node]) -> Vec<String> {
    let mut out = Vec::new();
    for node in nodes {
        walk_node_idents(node, &mut out);
    }
    out
}

fn walk_node_idents(node: &Node, out: &mut Vec<String>) {
    if let Node::VariableBlock(_, expr) = node {
        walk_expr_idents(expr, out);
    }
}

fn walk_expr_idents(expr: &Expr, out: &mut Vec<String>) {
    match &expr.val {
        ExprVal::Ident(name) => out.push(name.clone()),
        ExprVal::Math(m) => {
            walk_expr_idents(&m.lhs, out);
            walk_expr_idents(&m.rhs, out);
        }
        ExprVal::Logic(l) => {
            walk_expr_idents(&l.lhs, out);
            walk_expr_idents(&l.rhs, out);
        }
        ExprVal::Array(items) => {
            for it in items {
                walk_expr_idents(it, out);
            }
        }
        ExprVal::In(in_) => {
            walk_expr_idents(&in_.lhs, out);
            walk_expr_idents(&in_.rhs, out);
        }
        ExprVal::FunctionCall(fc) => {
            for arg in fc.args.values() {
                walk_expr_idents(arg, out);
            }
        }
        _ => {}
    }
    for f in &expr.filters {
        for arg in f.args.values() {
            walk_expr_idents(arg, out);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_yaml() -> &'static str {
        r#"
schema_version: "1.0"
name: c4-to-forge-spike
title: "C4 → Forge spike"
compat_spec_version: "c4-architecture: ^1.0"
source_kind: c4-documentation
target_kind: forge
sources:
  - pattern: "docs/**/*.md"
    type: markdown
    parser: front_matter_plus_sections
rules:
  - id: c4-struct-to-spec
    when:
      file_glob: "docs/**/*.md"
    target:
      kind: spec
    fields:
      title: "{{front_matter.name | trim}}"
    sources_section:
      include: true
      format: "{path}:{line_start}-{line_end}"
      precision: line
      source_hash: true
"#
    }

    // -- 1. parse_minimal_mapping ---------------------------------------------

    #[test]
    fn parse_minimal_mapping() {
        let m: Mapping = serde_yaml::from_str(minimal_yaml()).expect("parse");
        assert_eq!(m.name, "c4-to-forge-spike");
        assert_eq!(m.source_kind, SourceKind::C4Documentation);
        assert_eq!(m.target_kind, TargetKind::Forge);
        assert_eq!(m.sources.len(), 1);
        assert_eq!(m.rules.len(), 1);
        assert_eq!(m.rules[0].target.kind, ArtifactTargetKind::Spec);
        assert!(m.rules[0].sources_section.include);
        // default Guards
        assert!(m.guards.forbid_overwrite_active);
    }

    // -- 2. reject_sources_section_include_false ------------------------------

    #[test]
    fn reject_sources_section_include_false() {
        let bad = minimal_yaml().replace("include: true", "include: false");
        let err = serde_yaml::from_str::<Mapping>(&bad).expect_err("must reject");
        let msg = err.to_string();
        assert!(
            msg.contains("include must be true") || msg.contains("hallucination-proof"),
            "unexpected error: {msg}"
        );
    }

    // -- 3. reject_empty_sources_array ---------------------------------------

    #[test]
    fn reject_empty_sources_array() {
        let yaml = minimal_yaml().replace(
            "sources:\n  - pattern: \"docs/**/*.md\"\n    type: markdown\n    parser: front_matter_plus_sections",
            "sources: []",
        );
        let err = serde_yaml::from_str::<Mapping>(&yaml).expect_err("must reject");
        assert!(
            err.to_string().contains("sources"),
            "unexpected error: {err}"
        );
    }

    // -- 4. reject_empty_rules_array -----------------------------------------

    #[test]
    fn reject_empty_rules_array() {
        // Replace the `rules:` block with an empty array. We slice up to
        // `rules:` and append the empty literal.
        let yaml = minimal_yaml();
        let (head, _) = yaml.split_once("rules:").unwrap();
        let trimmed = format!("{head}rules: []\n");
        let err = serde_yaml::from_str::<Mapping>(&trimmed).expect_err("must reject");
        assert!(err.to_string().contains("rules"), "unexpected error: {err}");
    }

    // -- 5. template_accepts_whitelisted_filter ------------------------------

    #[test]
    fn template_accepts_whitelisted_filter() {
        for filter in ALLOWED_FILTERS {
            let src = format!("{{{{ x | {filter} }}}}");
            let parsed: Template = src
                .parse()
                .unwrap_or_else(|e| panic!("filter `{filter}` should parse: {e}"));
            assert_eq!(parsed.as_str(), src);
        }
    }

    // -- 6. template_rejects_arbitrary_filter --------------------------------

    #[test]
    fn template_rejects_arbitrary_filter() {
        let err = "{{ x | unsafe_filter }}".parse::<Template>().unwrap_err();
        assert!(err.contains("unsafe_filter"), "unexpected error: {err}");
        assert!(err.contains("whitelist"), "unexpected error: {err}");
    }

    // -- 7. template_extract_paths -------------------------------------------

    #[test]
    fn template_extract_paths() {
        let t: Template = "{{ front_matter.name | trim }}".parse().unwrap();
        let paths = t.extract_paths();
        assert_eq!(paths, vec!["front_matter.name"]);

        let t: Template = "{{ a.b }} and {{ c.d.e | upper }}".parse().unwrap();
        let paths = t.extract_paths();
        assert!(paths.contains(&"a.b"));
        assert!(paths.contains(&"c.d.e"));
    }

    // -- 8. parse_all_5_source_kinds -----------------------------------------

    #[test]
    fn parse_all_5_source_kinds() {
        let cases = [
            ("c4-documentation", SourceKind::C4Documentation),
            ("autoresearch", SourceKind::Autoresearch),
            ("git-log", SourceKind::GitLog),
            ("ddd-model", SourceKind::DddModel),
            ("sparc-spec", SourceKind::SparcSpec),
        ];
        for (raw, expected) in cases {
            let yaml = minimal_yaml().replace(
                "source_kind: c4-documentation",
                &format!("source_kind: {raw}"),
            );
            let m: Mapping = serde_yaml::from_str(&yaml)
                .unwrap_or_else(|e| panic!("kind `{raw}` should parse: {e}"));
            assert_eq!(m.source_kind, expected);
        }
    }

    // -- 9. version_compat_satisfied / mismatch ------------------------------

    #[test]
    fn version_compat_satisfied() {
        let m: Mapping = serde_yaml::from_str(minimal_yaml()).unwrap();
        let v = Version::parse("1.3.5").unwrap();
        assert!(m.version_compat(&v));
    }

    #[test]
    fn version_compat_mismatch() {
        let m: Mapping = serde_yaml::from_str(minimal_yaml()).unwrap();
        let v = Version::parse("2.0.0").unwrap();
        assert!(!m.version_compat(&v));
    }

    // -- 10. referenced_target_kinds_collects_unique -------------------------

    #[test]
    fn referenced_target_kinds_collects_unique() {
        let yaml = r#"
schema_version: "1.0"
name: multi-rule
title: "Two rules, one target"
compat_spec_version: "^1.0"
source_kind: c4-documentation
target_kind: forge
sources:
  - pattern: "**/*.md"
    type: markdown
    parser: front_matter_plus_sections
rules:
  - id: r1
    when: {}
    target: { kind: spec }
    fields:
      title: "{{ x }}"
    sources_section:
      include: true
  - id: r2
    when: {}
    target: { kind: spec }
    fields:
      title: "{{ y }}"
    sources_section:
      include: true
"#;
        let m: Mapping = serde_yaml::from_str(yaml).expect("parse");
        let kinds = m.referenced_target_kinds();
        assert_eq!(kinds.len(), 1);
        assert!(kinds.contains(&TargetSpec {
            kind: ArtifactTargetKind::Spec
        }));
    }

    // -- 11. all_rule_ids ----------------------------------------------------

    #[test]
    fn all_rule_ids_collects() {
        let m: Mapping = serde_yaml::from_str(minimal_yaml()).unwrap();
        let ids = m.all_rule_ids();
        assert!(ids.contains("c4-struct-to-spec"));
    }

    // -- 12. has_disallowed_filter (defence-in-depth) ------------------------

    #[test]
    fn has_disallowed_filter_clean_for_valid_mapping() {
        let m: Mapping = serde_yaml::from_str(minimal_yaml()).unwrap();
        assert!(m.has_disallowed_filter().is_none());
    }

    // -- 13. compat_spec_version with and without plugin prefix --------------

    #[test]
    fn compat_spec_version_with_plugin_prefix() {
        let v: CompatSpecVersion = "c4-architecture: ^1.0".parse().unwrap();
        assert_eq!(v.plugin.as_deref(), Some("c4-architecture"));
        assert!(v.req.matches(&Version::parse("1.2.3").unwrap()));
    }

    #[test]
    fn compat_spec_version_bare() {
        let v: CompatSpecVersion = "^2.1".parse().unwrap();
        assert!(v.plugin.is_none());
        assert!(v.req.matches(&Version::parse("2.1.5").unwrap()));
        assert!(!v.req.matches(&Version::parse("3.0.0").unwrap()));
    }

    // -- 14. spike-1 fixture round-trips -------------------------------------

    #[test]
    fn parse_spike_one_fixture() {
        // Minimal slice of the .local/spike-1 fixture, modified to fit the
        // current types (heading_path is in Selector). Confirms the canonical
        // c4-to-forge mapping shape works end-to-end.
        let yaml = r#"
schema_version: "1.0"
name: c4-to-forge-spike
title: "C4 Code-level docs → Forge"
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
      summary: "{{ section.purpose | default(value=\"Code-level type from C4 docs\") }}"
      contract: "{{ section.fields | table }}"
    sources_section:
      include: true
      format: "{path}:{line_start}-{line_end}"
      precision: line
      source_hash: true
guards:
  max_artifacts: 50
errors:
  template_filter_violation: error
"#;
        let m: Mapping = serde_yaml::from_str(yaml).expect("spike fixture parses");
        assert_eq!(m.rules.len(), 1);
        assert_eq!(m.guards.max_artifacts, Some(50));
        assert_eq!(m.errors.template_filter_violation, Some(ErrorAction::Error));
    }

    // -- 15. CRIT-S1 regression: walker must not skip Test/StringConcat ------
    //
    // Bypass class: filters smuggled through ExprVal variants the walker
    // previously hit with a `_ => ()` wildcard. Tera evaluates these at
    // render, so missing them means the whitelist is advisory at best.
    // Each test below targets a different ExprVal variant.

    /// Filter inside a `Test` argument — `is defined(y | striptags)`.
    #[test]
    fn template_rejects_filter_in_test_arg() {
        // Tera grammar: `test_arg = { logic_expr | array_filter }` — args may
        // carry filters. Pre-fix, the walker ignored them entirely.
        let src = "{% if x is defined(y | striptags) %}hi{% endif %}";
        let err = src.parse::<Template>().unwrap_err();
        assert!(
            err.contains("striptags"),
            "must mention disallowed filter, got: {err}"
        );
        assert!(
            err.contains("whitelist"),
            "must reference whitelist, got: {err}"
        );
    }

    /// StringConcat with a function call whose arg uses a forbidden filter.
    /// Bypass shape: `"a" ~ f(x | striptags)` — the StringConcat values are
    /// `Vec<ExprVal>`, the function call's kwargs are `Vec<Expr>` with
    /// filters. The walker now recurses through ExprVal::StringConcat.
    #[test]
    fn template_rejects_filter_in_string_concat() {
        let src = r#"{{ "a" ~ f(arg=x | striptags) }}"#;
        let err = src.parse::<Template>().unwrap_err();
        assert!(
            err.contains("striptags"),
            "must catch filter inside string concat fn arg, got: {err}"
        );
    }

    /// Filter inside an `In` lhs — Tera grammar `in_cond` permits a
    /// `basic_expr_filter` on the lhs, so `(x | striptags) in items` is a
    /// real parse path. The walker covered `In` already; this test pins
    /// the behaviour so a future regression is loud.
    #[test]
    fn template_rejects_filter_in_in_expr() {
        let src = "{% if (x | striptags) in items %}hi{% endif %}";
        let err = src.parse::<Template>().unwrap_err();
        assert!(
            err.contains("striptags") || err.contains("parse"),
            "must catch filter inside `in` expr, got: {err}"
        );
    }

    /// Filter inside a function-call argument — `f(arg=x | striptags)`.
    /// Walker had this covered; lock it in as part of the bypass surface.
    #[test]
    fn template_rejects_filter_in_function_call_arg() {
        let src = "{{ f(arg=x | striptags) }}";
        let err = src.parse::<Template>().unwrap_err();
        assert!(
            err.contains("striptags"),
            "must catch filter inside fn arg, got: {err}"
        );
    }

    /// Filter inside an array literal — `[x | striptags, y]`.
    #[test]
    fn template_rejects_filter_in_array_literal() {
        // Arrays appear as `array_filter` in Tera grammar; their elements
        // are full Exprs and may carry filters.
        let src = "{% set v = [x | striptags, y] %}";
        let err = src.parse::<Template>().unwrap_err();
        assert!(
            err.contains("striptags"),
            "must catch filter inside array literal, got: {err}"
        );
    }

    /// Filter inside a Math expression — `(x | striptags) + 1`.
    #[test]
    fn template_rejects_filter_in_math_expr() {
        let src = "{{ (x | striptags) + 1 }}";
        let err = src.parse::<Template>().unwrap_err();
        assert!(
            err.contains("striptags"),
            "must catch filter inside math expr, got: {err}"
        );
    }

    /// Filter inside a Logic comparison — `x | striptags > 0`.
    #[test]
    fn template_rejects_filter_in_logic_expr() {
        let src = "{% if x | striptags == \"a\" %}hi{% endif %}";
        let err = src.parse::<Template>().unwrap_err();
        assert!(
            err.contains("striptags"),
            "must catch filter inside logic expr, got: {err}"
        );
    }

    /// Filter inside a `{% filter %}...{% endfilter %}` block fn-call args.
    #[test]
    fn template_rejects_filter_in_filter_section_args() {
        // The block-level filter name itself was already collected; what was
        // missing was filters embedded in its kwargs.
        let src = "{% filter upper(arg=x | striptags) %}body{% endfilter %}";
        let err = src.parse::<Template>().unwrap_err();
        assert!(
            err.contains("striptags"),
            "must catch filter inside filter-section kwargs, got: {err}"
        );
    }

    /// Filter inside a macro definition's default arg value.
    #[test]
    fn template_rejects_filter_in_macro_default_arg() {
        let src = r#"{% macro greet(name=x | striptags) %}hi{% endmacro %}"#;
        let err = src.parse::<Template>().unwrap_err();
        assert!(
            err.contains("striptags"),
            "must catch filter inside macro default arg, got: {err}"
        );
    }

    // -- 16. Walker exhaustiveness: every ExprVal variant has an arm --------
    //
    // The match in `walk_exprval_filters` and `walk_node_filters` is
    // exhaustive (no `_` wildcard). This compile-time guarantee is what
    // closes the door on future Tera versions adding variants we don't
    // know about. We additionally exercise each known variant at runtime
    // so a refactor that re-introduces a wildcard is caught.

    /// Hand-built representative templates for each `ExprVal` variant. If
    /// any walker arm regresses to skip a variant, at least one of the
    /// embedded forbidden filters will leak through.
    #[test]
    fn template_walker_covers_all_exprval_variants() {
        // Each entry: (description, template source). All MUST be rejected
        // because each contains a `striptags` filter somewhere a walker arm
        // is required to recurse into.
        let bypass_attempts: &[(&str, &str)] = &[
            ("Math", "{{ (x | striptags) + 1 }}"),
            ("Logic", "{% if x | striptags == \"a\" %}hi{% endif %}"),
            ("FunctionCall", "{{ f(arg=x | striptags) }}"),
            ("Array", "{% set v = [x | striptags] %}"),
            ("In", "{% if (x | striptags) in items %}hi{% endif %}"),
            ("Test", "{% if x is defined(y | striptags) %}hi{% endif %}"),
            ("StringConcat", r#"{{ "a" ~ f(arg=x | striptags) }}"#),
            // Leaf variants (String/Int/Float/Bool/Ident) cannot host a
            // filter directly; their walker arms intentionally do nothing.
            // They are exercised implicitly by every other test parsing OK.
        ];
        for (variant, src) in bypass_attempts {
            let err = src.parse::<Template>().unwrap_err();
            assert!(
                err.contains("striptags") || err.contains("parse"),
                "{variant}: bypass not caught — `{src}` returned `{err}`"
            );
        }
    }

    // -- 17. CRIT-T3 regression: whitelist-name contract for `replace` /  ---
    //                            `default` (Tera built-ins pre-empt) -------

    /// `replace` is a Tera built-in; the whitelist allows the **name**, so
    /// using it in a template must parse successfully even though no custom
    /// implementation is registered.
    #[test]
    fn template_accepts_tera_builtin_replace() {
        let src = r#"{{ x | replace(from="a", to="b") }}"#;
        let t: Template = src
            .parse()
            .unwrap_or_else(|e| panic!("Tera built-in `replace` must parse: {e}"));
        assert_eq!(t.as_str(), src);
    }

    /// `default` is also a Tera built-in; same contract as `replace`.
    #[test]
    fn template_accepts_tera_builtin_default() {
        let src = r#"{{ x | default(value="fallback") }}"#;
        let t: Template = src
            .parse()
            .unwrap_or_else(|e| panic!("Tera built-in `default` must parse: {e}"));
        assert_eq!(t.as_str(), src);
    }

    /// Tera built-ins NOT in our whitelist (e.g. `striptags`, `safe`,
    /// `json_encode`, `escape`) MUST be rejected even though Tera ships an
    /// implementation. The whitelist is an allow-list, not a block-list.
    #[test]
    fn template_rejects_arbitrary_tera_builtin() {
        for forbidden in ["striptags", "safe", "json_encode", "escape"] {
            let src = format!("{{{{ x | {forbidden} }}}}");
            let err = src
                .parse::<Template>()
                .err()
                .unwrap_or_else(|| panic!("Tera built-in `{forbidden}` slipped past whitelist"));
            assert!(
                err.contains(forbidden) || err.contains("whitelist"),
                "{forbidden}: error did not mention filter or whitelist: {err}"
            );
        }
    }

    /// Filters that don't exist anywhere — neither built-in nor whitelisted.
    /// Belt-and-suspenders: covers the case where someone deletes the
    /// whitelist contains-check by mistake.
    #[test]
    fn template_rejects_unknown_filter() {
        let err = "{{ x | nonexistent_filter }}"
            .parse::<Template>()
            .unwrap_err();
        assert!(
            err.contains("nonexistent_filter") || err.contains("whitelist"),
            "unexpected error: {err}"
        );
    }
}
