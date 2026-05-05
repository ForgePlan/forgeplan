//! `forgeplan ingest` CLI surface (PRD-066 / SPEC-004).
//!
//! Wave 3 implementation by agent **w3b-cli-ingest-plugins**.
//!
//! Pipeline:
//!
//! 1. Validate `mapping` + `source` paths.
//! 2. Parse the mapping YAML into a `Mapping`.
//! 3. Walk the source directory (or read the single file) and parse every
//!    file matching one of the mapping's `sources[*].pattern` entries using
//!    the declared parser.
//! 4. Run `IngestEngine::apply` to assemble drafts.
//! 5. In dry-run mode, print the report and exit. Otherwise idempotently
//!    write each draft to LanceDB + markdown projection (PRD-066 AC-3,
//!    AC-5) and add the auto-links declared by the rule.
//! 6. Emit a single `Next:` / `Done.` line per PRD-071.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use console::style;
use serde_json::json;

// =====================================================================
// Resource limits (HIGH-S2 — Audit Round 1 finding)
// =====================================================================
//
// Mapping YAML and source files are read in full before parsing. Without
// bounds an attacker (or accidental commit) can crash the process with a
// multi-GB input or stack-bomb the YAML parser via deep nesting.

/// Maximum size of a mapping YAML file (1 MiB).
const MAX_MAPPING_SIZE: u64 = 1024 * 1024;

/// Maximum size of a single source file (10 MiB). Sources are markdown / YAML
/// snapshots produced by other tools — generous but bounded.
const MAX_SOURCE_SIZE: u64 = 10 * 1024 * 1024;

/// Maximum opener depth we tolerate in mapping YAML.
///
/// `serde_yaml` < 0.9 does not expose a recursion-limit knob, so we count
/// `{`/`[` characters as a cheap heuristic and reject before the parser
/// ever sees the input.
const MAX_MAPPING_NESTING: usize = 256;

/// Maximum opener depth we tolerate in source content (NEW-S-H1, Audit
/// Round 2). Mirrors [`MAX_MAPPING_NESTING`] for source YAML / JSON / front-
/// matter so a 10 MiB source with 5M `[` tokens cannot stack-overflow the
/// parser despite passing the size cap.
const MAX_SOURCE_NESTING: usize = 256;

/// Read a mapping YAML file with [`MAX_MAPPING_SIZE`] / [`MAX_MAPPING_NESTING`]
/// guards. Surfaces a structured `Err` rather than reading multi-GB into
/// the heap or letting `serde_yaml` blow the stack.
fn read_mapping_with_limits(path: &Path) -> Result<String> {
    let meta = std::fs::metadata(path)
        .with_context(|| format!("failed to stat mapping {}", path.display()))?;
    let len = meta.len();
    if len > MAX_MAPPING_SIZE {
        anyhow::bail!(
            "mapping {} too large: {} bytes (limit {} bytes)",
            path.display(),
            len,
            MAX_MAPPING_SIZE
        );
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read mapping {}", path.display()))?;
    let depth = content.bytes().filter(|b| *b == b'{' || *b == b'[').count();
    if depth > MAX_MAPPING_NESTING {
        anyhow::bail!(
            "mapping {} too deeply nested: {} bracket tokens (limit {})",
            path.display(),
            depth,
            MAX_MAPPING_NESTING
        );
    }
    Ok(content)
}

/// Read a source file with [`MAX_SOURCE_SIZE`] + [`MAX_SOURCE_NESTING`]
/// guards (NEW-S-H1, Audit Round 2). Returns an `Err` so the per-file
/// parser can downgrade to a soft warning if it chooses.
///
/// The nesting heuristic uses a string-literal-aware running balance of
/// `{`/`[` minus `}`/`]` opens — adversarial 10 MiB sources with 5M
/// unmatched opens are rejected before `serde_yaml::from_str` is invoked.
fn read_source_with_limits(path: &Path) -> Result<String> {
    let meta = std::fs::metadata(path)
        .with_context(|| format!("failed to stat source {}", path.display()))?;
    let len = meta.len();
    if len > MAX_SOURCE_SIZE {
        anyhow::bail!(
            "source {} too large: {} bytes (limit {} bytes)",
            path.display(),
            len,
            MAX_SOURCE_SIZE
        );
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read source {}", path.display()))?;
    if exceeds_nesting_depth(&content, MAX_SOURCE_NESTING) {
        anyhow::bail!(
            "source {} too deeply nested (peak depth exceeds {} open-bracket tokens; \
             stack-overflow defence)",
            path.display(),
            MAX_SOURCE_NESTING
        );
    }
    Ok(content)
}

use forgeplan_core::artifact::types::ArtifactKind;
use forgeplan_core::db::store::{ArtifactFilter, ArtifactRecord, LanceStore, NewArtifact};
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::ingest::{
    ArtifactTargetKind, DraftLink, IfExists, IngestArtifactDraft, IngestEngine, IngestOptions,
    IngestReport, Mapping, ParsedSource, SourceSpec, UpdateDecision, artifact_needs_update,
    exceeds_nesting_depth, extract_existing_source_hash, parser_for,
};
use forgeplan_core::link;
use forgeplan_core::projection;

use crate::commands::common;

/// `forgeplan ingest --mapping <f> --source <p> [--dry-run] [--update] [--json]`
pub async fn run(
    mapping_path: &Path,
    source_path: &Path,
    dry_run: bool,
    update: bool,
    json: bool,
) -> Result<()> {
    // ── 1. Validate paths ──────────────────────────────────────────────────
    if !mapping_path.exists() {
        emit_error(
            json,
            &format!("mapping file not found: {}", mapping_path.display()),
            Some("forgeplan plugins list"),
        );
        std::process::exit(2);
    }
    if !source_path.exists() {
        emit_error(
            json,
            &format!("source path not found: {}", source_path.display()),
            None,
        );
        std::process::exit(2);
    }

    // ── 2. Load mapping YAML (HIGH-S2: bound size + depth) ─────────────────
    let mapping_yaml = match read_mapping_with_limits(mapping_path) {
        Ok(s) => s,
        Err(e) => {
            emit_error(
                json,
                &format!("{e}"),
                Some(&format!(
                    "trim mapping {} below {} bytes",
                    mapping_path.display(),
                    MAX_MAPPING_SIZE
                )),
            );
            std::process::exit(2);
        }
    };
    let mapping: Mapping = match serde_yaml::from_str(&mapping_yaml) {
        Ok(m) => m,
        Err(e) => {
            emit_error(
                json,
                &format!("invalid mapping YAML: {e}"),
                Some(&format!(
                    "edit {} to match SPEC-004 schema",
                    mapping_path.display()
                )),
            );
            std::process::exit(2);
        }
    };

    // ── 3. Enumerate + parse sources ───────────────────────────────────────
    let parsed_sources = match collect_parsed_sources(&mapping, source_path) {
        Ok(srcs) => srcs,
        Err(e) => {
            emit_error(json, &format!("source ingest failed: {e}"), None);
            std::process::exit(2);
        }
    };

    if parsed_sources.is_empty() {
        if json {
            print_json(&json!({
                "drafts": [],
                "skipped": [],
                "errors": [],
                "written": [],
                "_next_action": null,
            }));
        } else {
            println!(
                "  {} no files matched mapping patterns under {}",
                style("⊘").dim(),
                source_path.display()
            );
            println!("\nDone.");
        }
        return Ok(());
    }

    // ── 4. Apply mapping ───────────────────────────────────────────────────
    let engine = IngestEngine::new().context("failed to construct ingest engine")?;
    let report = match engine.apply(&mapping, parsed_sources, IngestOptions { dry_run }) {
        Ok(r) => r,
        Err(e) => {
            emit_error(json, &format!("ingest engine error: {e}"), None);
            std::process::exit(2);
        }
    };

    // ── 5. Dry-run: report only ────────────────────────────────────────────
    if dry_run {
        return print_dry_run(&report, json);
    }

    // ── 6. Write drafts to workspace ───────────────────────────────────────
    let (ws, _lock, store) = common::open_store_locked().await?;

    let mut written: Vec<WrittenArtifact> = Vec::new();
    let mut skipped_existing: Vec<String> = Vec::new();
    let mut write_errors: Vec<String> = Vec::new();

    for draft in &report.drafts {
        match write_draft(&ws, &store, draft, update).await {
            Ok(WriteOutcome::Created(id)) | Ok(WriteOutcome::Updated(id)) => {
                written.push(WrittenArtifact {
                    id,
                    kind: kind_label(&draft.kind),
                    title: draft.title.clone(),
                    rule_id: draft.rule_id.clone(),
                    source_path: draft.source_path.clone(),
                });
            }
            Ok(WriteOutcome::Skipped(reason)) => skipped_existing.push(reason),
            Err(e) => write_errors.push(format!("{}: {}", draft.title, e)),
        }
    }

    // ── 7. Emit report ─────────────────────────────────────────────────────
    let next_action = primary_next_action(&written);

    // BUG-4 (Phase 6 real-world testing): the previous code happily emitted
    // a "Done." line and returned `Ok(())` even when some drafts could not
    // be written (LanceDB conflicts, projection failures) or the engine
    // produced rule-application errors. Capture those before the exit
    // decision so the JSON / text report is still complete.
    let had_write_errors = !write_errors.is_empty();
    let had_apply_errors = !report.errors.is_empty();

    if json {
        let payload = json!({
            "drafts": report.drafts.iter().map(draft_to_json).collect::<Vec<_>>(),
            "skipped": serde_json::to_value(&report.skipped).unwrap_or(json!([])),
            "errors": serde_json::to_value(&report.errors).unwrap_or(json!([])),
            "written": written.iter().map(WrittenArtifact::to_json).collect::<Vec<_>>(),
            "skipped_existing": skipped_existing,
            "write_errors": write_errors,
            "_next_action": next_action.as_deref(),
        });
        print_json(&payload);
    } else {
        print_text_report(&report, &written, &skipped_existing, &write_errors);
        match next_action {
            Some(cmd) => {
                let h: Vec<Hint> = vec![Hint::info("Validate the new artifact").with_action(cmd)];
                print!("{}", hints::render_next_action_line(&h));
            }
            None => println!("\nDone."),
        }
    }

    // Exit non-zero when the run produced visible errors. Idempotent skips
    // (`skipped_existing`) and engine-side `report.skipped` are *not*
    // failures — they're routine outcomes for a re-run.
    if had_write_errors || had_apply_errors {
        std::process::exit(1);
    }
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────────
// Source enumeration + parsing
// ────────────────────────────────────────────────────────────────────────────

/// Walk `source_path` (file or directory) and parse every file matching one
/// of the mapping's `sources[*].pattern` globs using the declared parser.
fn collect_parsed_sources(mapping: &Mapping, source_path: &Path) -> Result<Vec<ParsedSource>> {
    let mut out: Vec<ParsedSource> = Vec::new();

    if source_path.is_file() {
        // Single-file mode: pick the first matching SourceSpec by glob; if
        // none match, fall back to the first declared spec (caller picked
        // this file deliberately).
        let spec = pick_spec_for_path(&mapping.sources, source_path).unwrap_or(&mapping.sources[0]);
        if let Some(parsed) = parse_one(spec, source_path)? {
            out.push(parsed);
        }
        return Ok(out);
    }

    // Directory mode: enumerate every file and match against every spec.
    // A file may be paired with at most one spec (first match wins) so we
    // don't double-emit ParsedSources on overlapping patterns.
    let files = walk_files(source_path)?;
    for file in files {
        let rel = file.strip_prefix(source_path).unwrap_or(&file);
        let mut matched_spec: Option<&SourceSpec> = None;
        for spec in &mapping.sources {
            if simple_glob_match(&spec.pattern, rel) || simple_glob_match(&spec.pattern, &file) {
                matched_spec = Some(spec);
                break;
            }
        }
        if let Some(spec) = matched_spec
            && let Some(parsed) = parse_one(spec, &file)?
        {
            out.push(parsed);
        }
    }

    Ok(out)
}

/// Recursively walk `root`, returning every regular file. Symlinks are not
/// followed (avoid surprise blow-ups). Hidden directories starting with `.`
/// are descended into so that `.local/spike-*.md` etc. are reachable.
fn walk_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out: Vec<PathBuf> = Vec::new();
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    let mut seen: HashSet<PathBuf> = HashSet::new();
    while let Some(dir) = stack.pop() {
        let canon = std::fs::canonicalize(&dir).unwrap_or_else(|_| dir.clone());
        if !seen.insert(canon) {
            continue;
        }
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let ft = match entry.file_type() {
                Ok(t) => t,
                Err(_) => continue,
            };
            if ft.is_symlink() {
                continue;
            }
            if ft.is_dir() {
                stack.push(path);
            } else if ft.is_file() {
                out.push(path);
            }
        }
    }
    Ok(out)
}

/// First [`SourceSpec`] whose glob matches `path`.
fn pick_spec_for_path<'a>(specs: &'a [SourceSpec], path: &Path) -> Option<&'a SourceSpec> {
    specs.iter().find(|s| simple_glob_match(&s.pattern, path))
}

/// Read + parse a single source file. Returns `Ok(None)` on parse error or
/// over-size so we can surface a soft warning instead of aborting the whole
/// ingest run on a single misbehaving file (HIGH-S2).
fn parse_one(spec: &SourceSpec, path: &Path) -> Result<Option<ParsedSource>> {
    let content = match read_source_with_limits(path) {
        Ok(s) => s,
        Err(e) => {
            // Soft warning + continue — one oversized file should not blow
            // up the whole ingest. The user gets visibility without DoS risk.
            eprintln!(
                "  {} skipping {} (size guard): {}",
                style("⚠").yellow(),
                path.display(),
                e
            );
            return Ok(None);
        }
    };
    let parser = parser_for(&spec.parser);
    match parser.parse(path, &content) {
        Ok(p) => Ok(Some(p)),
        Err(e) => {
            eprintln!(
                "  {} parse failed for {}: {}",
                style("⚠").yellow(),
                path.display(),
                e
            );
            Ok(None)
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Minimal-feature glob matcher
// ────────────────────────────────────────────────────────────────────────────

/// Matches a path against a shell-style glob: supports `*`, `**`, and `?`.
///
/// Comparison is done on the path's textual form with forward-slash
/// separators (Windows normalised). Sufficient for SPEC-004 patterns like
/// `docs/**/*.md` or `.local/spike-1-c4-*.md`. We avoid pulling `globset` as
/// a CLI-crate dep.
fn simple_glob_match(pattern: &str, path: &Path) -> bool {
    let path_str = path.to_string_lossy().replace('\\', "/");
    glob_match_str(pattern, &path_str)
}

fn glob_match_str(pattern: &str, text: &str) -> bool {
    // Convert pattern to regex-equivalent vec-of-tokens then walk via
    // recursive descent. Small patterns; cost is negligible.
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    matcher(&p, 0, &t, 0)
}

fn matcher(p: &[char], pi: usize, t: &[char], ti: usize) -> bool {
    if pi == p.len() {
        return ti == t.len();
    }
    match p[pi] {
        '*' => {
            // Detect `**` (matches across path separators) vs `*` (no `/`).
            let is_double = pi + 1 < p.len() && p[pi + 1] == '*';
            if is_double {
                // Skip the `**` and any following `/`.
                let mut rest_start = pi + 2;
                if rest_start < p.len() && p[rest_start] == '/' {
                    rest_start += 1;
                }
                // Try matching at every position in t (including end).
                if matcher(p, rest_start, t, ti) {
                    return true;
                }
                let mut k = ti;
                while k < t.len() {
                    k += 1;
                    if matcher(p, rest_start, t, k) {
                        return true;
                    }
                }
                false
            } else {
                // `*` consumes any chars except `/`.
                if matcher(p, pi + 1, t, ti) {
                    return true;
                }
                let mut k = ti;
                while k < t.len() && t[k] != '/' {
                    k += 1;
                    if matcher(p, pi + 1, t, k) {
                        return true;
                    }
                }
                false
            }
        }
        '?' => {
            if ti < t.len() && t[ti] != '/' {
                matcher(p, pi + 1, t, ti + 1)
            } else {
                false
            }
        }
        c => {
            if ti < t.len() && t[ti] == c {
                matcher(p, pi + 1, t, ti + 1)
            } else {
                false
            }
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Draft writes (artifact + links)
// ────────────────────────────────────────────────────────────────────────────

enum WriteOutcome {
    Created(String),
    Updated(String),
    Skipped(String),
}

/// Write a single draft to LanceDB + markdown projection. Honours
/// idempotency (`source_hash`) and `--update` flag (PRD-066 AC-3).
async fn write_draft(
    ws: &Path,
    store: &LanceStore,
    draft: &IngestArtifactDraft,
    allow_update: bool,
) -> Result<WriteOutcome> {
    let kind = artifact_kind_from_draft(&draft.kind);
    let template_key = kind.template_key();

    // Compare against any existing artifact of the same kind+title (idempotency).
    if let Some(existing_id) = find_existing_by_title(store, template_key, &draft.title).await?
        && let Some(record) = store.get_record(&existing_id).await?
    {
        let existing_hash = extract_existing_source_hash(&record.body);
        let decision = artifact_needs_update(existing_hash.as_deref(), &draft.source_hash);
        match decision {
            UpdateDecision::Skip => {
                return Ok(WriteOutcome::Skipped(format!(
                    "{} unchanged (hash match)",
                    existing_id
                )));
            }
            UpdateDecision::Update if !allow_update => {
                return Ok(WriteOutcome::Skipped(format!(
                    "{} differs but --update not passed",
                    existing_id
                )));
            }
            UpdateDecision::Update => {
                update_existing(ws, store, &record, draft).await?;
                add_links(ws, store, &existing_id, &draft.links).await;
                return Ok(WriteOutcome::Updated(existing_id));
            }
            UpdateDecision::Create => {
                // Stale title-collision with no hash marker — fall through to create.
            }
            // `UpdateDecision` is `#[non_exhaustive]` (Audit Round 2 T-H4):
            // unknown variants fall through to the create path so a stale
            // CLI build still produces an artifact rather than silently
            // dropping the draft.
            _ => {}
        }
    }

    let prefix = kind.prefix().trim_end_matches('-').to_uppercase();
    let id = store.next_id(&prefix).await?;

    let new = NewArtifact {
        id: id.clone(),
        kind: template_key.to_string(),
        status: "draft".to_string(),
        title: draft.title.clone(),
        body: draft.body.clone(),
        depth: default_depth_for(&kind).to_string(),
        author: None,
        parent_epic: None,
        valid_until: None,
        tags: vec![format!("source=ingest:{}", draft.rule_id)],
    };
    // PRD-073 Phase 3b: file-first helper writes the markdown projection
    // FIRST then syncs to LanceDB. Previous DB-first order would have
    // stranded the artifact in DB-only state on a crash between the two.
    projection::create_artifact_with_projection(&projection::MutationContext::new(ws, store), &new)
        .await
        .with_context(|| format!("create_artifact_with_projection failed for {id}"))?;

    common::log_change(store, &id, "ingest_create", "cli").await;

    add_links(ws, store, &id, &draft.links).await;

    Ok(WriteOutcome::Created(id))
}

async fn update_existing(
    ws: &Path,
    store: &LanceStore,
    record: &ArtifactRecord,
    draft: &IngestArtifactDraft,
) -> Result<()> {
    // PRD-073 Phase 3b: file-first body update — helper writes file then DB.
    projection::update_body_with_projection(
        &projection::MutationContext::new(ws, store),
        &record.id,
        &draft.body,
    )
    .await
    .with_context(|| format!("update_body_with_projection failed for {}", record.id))?;
    common::log_change(store, &record.id, "ingest_update", "cli").await;
    Ok(())
}

/// Add each draft link to the source artifact's projection. Honours the
/// per-link [`IfExists`] policy and silently skips missing target IDs (the
/// link target may itself be a draft from the same ingest run).
async fn add_links(ws: &Path, store: &LanceStore, source_id: &str, links: &[DraftLink]) {
    if links.is_empty() {
        return;
    }
    let record = match store.get_record(source_id).await {
        Ok(Some(r)) => r,
        _ => return,
    };
    let kind = record
        .kind
        .parse::<ArtifactKind>()
        .unwrap_or(ArtifactKind::Note);
    let dir = ws.join(kind.dir_name());
    let slug = forgeplan_core::artifact::types::slugify(&record.title);
    let path = dir.join(format!("{}-{}.md", record.id, slug));

    for lnk in links {
        let relation = match link::normalize_relation(&lnk.relation) {
            Ok(r) => r,
            Err(_) => continue,
        };
        // PRD-073 Phase 3b: file-first link helper handles bidirectional render.
        if let Err(e) = projection::add_link_with_projection(
            &projection::MutationContext::new(ws, store),
            source_id,
            &lnk.target,
            &relation,
        )
        .await
        {
            match lnk.if_exists {
                IfExists::Skip => {}
                IfExists::Warn => eprintln!(
                    "  {} link {} -> {} ({}) skipped: {}",
                    style("⚠").yellow(),
                    source_id,
                    lnk.target,
                    relation,
                    e
                ),
                IfExists::Error => eprintln!(
                    "  {} link {} -> {} ({}) error: {}",
                    style("✗").red(),
                    source_id,
                    lnk.target,
                    relation,
                    e
                ),
                // `IfExists` is `#[non_exhaustive]` (Audit Round 2 T-H4):
                // future policies (`Merge`, `Defer`) fall back to the
                // safest behaviour — silently skip — until the CLI grows
                // explicit handling.
                _ => {}
            }
        }
        // Best-effort frontmatter mirror; ignore errors so we don't half-write.
        if path.exists() {
            let _ = link::add_link(&path, &lnk.target, &relation).await;
        }
    }
}

/// Return the latest existing artifact ID with the same kind + title.
async fn find_existing_by_title(
    store: &LanceStore,
    kind: &str,
    title: &str,
) -> Result<Option<String>> {
    let filter = ArtifactFilter {
        kind: Some(kind.to_string()),
        status: None,
    };
    let summaries = store.list_artifacts(Some(&filter)).await?;
    Ok(summaries
        .into_iter()
        .find(|s| s.title.eq_ignore_ascii_case(title))
        .map(|s| s.id))
}

// ────────────────────────────────────────────────────────────────────────────
// Output helpers
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct WrittenArtifact {
    id: String,
    kind: String,
    title: String,
    rule_id: String,
    source_path: String,
}

impl WrittenArtifact {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "id": self.id,
            "kind": self.kind,
            "title": self.title,
            "rule_id": self.rule_id,
            "source_path": self.source_path,
        })
    }
}

fn print_text_report(
    report: &IngestReport,
    written: &[WrittenArtifact],
    skipped_existing: &[String],
    write_errors: &[String],
) {
    println!(
        "  {} {} draft(s) produced ({} skipped by engine, {} errors)",
        style("✓").green(),
        report.drafts.len(),
        report.skipped.len(),
        report.errors.len()
    );

    if !written.is_empty() {
        println!("\n  Written:");
        for w in written {
            println!(
                "    {} {} {:8} \"{}\"  ({})",
                style("+").green(),
                w.id,
                w.kind,
                w.title,
                style(&w.rule_id).dim()
            );
        }
    }

    if !skipped_existing.is_empty() {
        println!("\n  Idempotent skips:");
        for s in skipped_existing {
            println!("    {} {}", style("~").yellow(), s);
        }
    }

    if !write_errors.is_empty() {
        println!("\n  Write errors:");
        for e in write_errors {
            println!("    {} {}", style("✗").red(), e);
        }
    }
}

fn print_dry_run(report: &IngestReport, json: bool) -> Result<()> {
    if json {
        let payload = json!({
            "drafts": report.drafts.iter().map(draft_to_json).collect::<Vec<_>>(),
            "skipped": serde_json::to_value(&report.skipped).unwrap_or(json!([])),
            "errors": serde_json::to_value(&report.errors).unwrap_or(json!([])),
            "_next_action": null,
        });
        print_json(&payload);
        return Ok(());
    }

    println!(
        "  {} dry-run: {} draft(s) would be written",
        style("⊘").dim(),
        report.drafts.len()
    );
    for d in &report.drafts {
        println!(
            "    {} {:8} \"{}\"  ({})",
            style("+").green(),
            kind_label(&d.kind),
            d.title,
            style(&d.rule_id).dim()
        );
    }
    if !report.errors.is_empty() {
        println!("\n  Errors during apply:");
        for e in &report.errors {
            println!(
                "    {} [{}] {}: {}",
                style("✗").red(),
                e.rule_id,
                e.source_path,
                e.message
            );
        }
    }
    println!("\nDone.");
    Ok(())
}

fn primary_next_action(written: &[WrittenArtifact]) -> Option<String> {
    written
        .first()
        .map(|w| format!("forgeplan validate {}", w.id))
}

fn draft_to_json(d: &IngestArtifactDraft) -> serde_json::Value {
    json!({
        "kind": kind_label(&d.kind),
        "title": d.title,
        "rule_id": d.rule_id,
        "source_path": d.source_path,
        "source_hash": d.source_hash,
        "links": d.links.iter().map(|l| json!({
            "target": l.target,
            "relation": l.relation,
            "if_exists": format!("{:?}", l.if_exists).to_lowercase(),
        })).collect::<Vec<_>>(),
    })
}

fn print_json(value: &serde_json::Value) {
    match serde_json::to_string_pretty(value) {
        Ok(s) => println!("{s}"),
        Err(_) => println!("{value}"),
    }
}

fn emit_error(json: bool, message: &str, fix_hint: Option<&str>) {
    if json {
        let payload = json!({
            "error": message,
            "fix": fix_hint,
            "_next_action": null,
        });
        print_json(&payload);
    } else {
        eprintln!("Error: {message}");
        if let Some(fix) = fix_hint {
            eprintln!("Fix: {fix}");
        }
    }
}

fn kind_label(k: &ArtifactTargetKind) -> String {
    match k {
        ArtifactTargetKind::Prd => "prd",
        ArtifactTargetKind::Adr => "adr",
        ArtifactTargetKind::Epic => "epic",
        ArtifactTargetKind::Note => "note",
        ArtifactTargetKind::Spec => "spec",
        ArtifactTargetKind::Problem => "problem",
        // `ArtifactTargetKind` is `#[non_exhaustive]` (Audit Round 2):
        // unknown variants render as `unknown` so the CLI doesn't crash on
        // a stale binary against a newer mapping.
        _ => "unknown",
    }
    .to_string()
}

fn artifact_kind_from_draft(k: &ArtifactTargetKind) -> ArtifactKind {
    match k {
        ArtifactTargetKind::Prd => ArtifactKind::Prd,
        ArtifactTargetKind::Adr => ArtifactKind::Adr,
        ArtifactTargetKind::Epic => ArtifactKind::Epic,
        ArtifactTargetKind::Note => ArtifactKind::Note,
        ArtifactTargetKind::Spec => ArtifactKind::Spec,
        ArtifactTargetKind::Problem => ArtifactKind::ProblemCard,
        // `ArtifactTargetKind` is `#[non_exhaustive]` (Audit Round 2):
        // fall back to `Note` (lowest-trust artifact) on unknown variants.
        _ => ArtifactKind::Note,
    }
}

fn default_depth_for(kind: &ArtifactKind) -> &'static str {
    match kind {
        ArtifactKind::Note
        | ArtifactKind::EvidencePack
        | ArtifactKind::ProblemCard
        | ArtifactKind::SolutionPortfolio
        | ArtifactKind::RefreshReport => "tactical",
        _ => "standard",
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn glob_simple_star_in_segment() {
        assert!(glob_match_str("docs/*.md", "docs/foo.md"));
        assert!(!glob_match_str("docs/*.md", "docs/sub/foo.md"));
    }

    #[test]
    fn glob_double_star_crosses_segments() {
        assert!(glob_match_str("docs/**/*.md", "docs/sub/foo.md"));
        assert!(glob_match_str("docs/**/*.md", "docs/foo.md"));
        assert!(glob_match_str("**/foo.md", "a/b/foo.md"));
    }

    #[test]
    fn glob_question_mark_one_char() {
        assert!(glob_match_str("a?.md", "ab.md"));
        assert!(!glob_match_str("a?.md", "abc.md"));
        assert!(!glob_match_str("a?.md", "a/.md"));
    }

    #[test]
    fn glob_literal_segment_match() {
        assert!(glob_match_str(
            ".local/spike-1-c4-*.md",
            ".local/spike-1-c4-scoring.md"
        ));
        assert!(!glob_match_str(
            ".local/spike-1-c4-*.md",
            ".local/spike-2-c4-scoring.md"
        ));
    }

    #[test]
    fn simple_glob_match_path_uses_forward_slashes() {
        let p = PathBuf::from("docs").join("foo.md");
        assert!(simple_glob_match("docs/foo.md", &p));
    }

    #[test]
    fn kind_label_covers_all_target_kinds() {
        assert_eq!(kind_label(&ArtifactTargetKind::Prd), "prd");
        assert_eq!(kind_label(&ArtifactTargetKind::Adr), "adr");
        assert_eq!(kind_label(&ArtifactTargetKind::Epic), "epic");
        assert_eq!(kind_label(&ArtifactTargetKind::Note), "note");
        assert_eq!(kind_label(&ArtifactTargetKind::Spec), "spec");
        assert_eq!(kind_label(&ArtifactTargetKind::Problem), "problem");
    }

    #[test]
    fn artifact_kind_from_draft_maps_each_variant() {
        assert!(matches!(
            artifact_kind_from_draft(&ArtifactTargetKind::Prd),
            ArtifactKind::Prd
        ));
        assert!(matches!(
            artifact_kind_from_draft(&ArtifactTargetKind::Problem),
            ArtifactKind::ProblemCard
        ));
        assert!(matches!(
            artifact_kind_from_draft(&ArtifactTargetKind::Note),
            ArtifactKind::Note
        ));
    }

    #[test]
    fn default_depth_selection() {
        assert_eq!(default_depth_for(&ArtifactKind::Prd), "standard");
        assert_eq!(default_depth_for(&ArtifactKind::Note), "tactical");
        assert_eq!(default_depth_for(&ArtifactKind::EvidencePack), "tactical");
        assert_eq!(default_depth_for(&ArtifactKind::Epic), "standard");
    }

    // ── HIGH-S2: size + nesting limits ──────────────────────────────

    #[test]
    fn read_mapping_with_limits_rejects_oversized_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("big.yaml");
        let big = "a: ".to_string() + &"x".repeat((MAX_MAPPING_SIZE as usize) + 4096);
        std::fs::write(&p, big).unwrap();
        let err = read_mapping_with_limits(&p).expect_err("should reject");
        assert!(format!("{err}").contains("too large"));
    }

    #[test]
    fn read_mapping_with_limits_rejects_too_deep_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("deep.yaml");
        let mut s = String::new();
        for _ in 0..(MAX_MAPPING_NESTING + 50) {
            s.push('[');
        }
        std::fs::write(&p, s).unwrap();
        let err = read_mapping_with_limits(&p).expect_err("should reject");
        assert!(format!("{err}").contains("too deeply nested"));
    }

    #[test]
    fn read_source_with_limits_rejects_oversized_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("big.md");
        // Slightly over MAX_SOURCE_SIZE with literal bytes.
        let len = (MAX_SOURCE_SIZE as usize) + 4096;
        let big = "x".repeat(len);
        std::fs::write(&p, big).unwrap();
        let err = read_source_with_limits(&p).expect_err("should reject");
        assert!(format!("{err}").contains("too large"));
    }

    #[test]
    fn read_source_with_limits_accepts_normal_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("ok.md");
        std::fs::write(&p, "# heading\nbody\n").unwrap();
        let content = read_source_with_limits(&p).expect("ok");
        assert!(content.contains("heading"));
    }

    // ── NEW-S-H1 (Audit Round 2): source nesting guard ──────────────────

    #[test]
    fn read_source_with_limits_rejects_deep_nesting() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("bomb.yaml");
        // 1000 unmatched `[` opens — well over MAX_SOURCE_NESTING (256).
        let mut content = String::new();
        for _ in 0..1000 {
            content.push('[');
        }
        std::fs::write(&p, content).unwrap();
        let err = read_source_with_limits(&p).expect_err("should reject");
        let msg = format!("{err}");
        assert!(
            msg.contains("too deeply nested") || msg.contains("stack-overflow"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn read_source_with_limits_string_literals_do_not_count() {
        // A realistic source containing 1000 literal `[` bytes inside a
        // double-quoted YAML scalar must NOT trip the nesting guard.
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("literal.yaml");
        let mut content = String::from("key: \"");
        for _ in 0..1000 {
            content.push('[');
        }
        content.push('"');
        content.push('\n');
        std::fs::write(&p, content).unwrap();
        let _ = read_source_with_limits(&p).expect("string-literal `[` must not count");
    }
}
