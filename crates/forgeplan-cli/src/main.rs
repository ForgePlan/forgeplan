// PROB-060 Phase 0b: integration tests need to call select command modules
// in-process. To avoid duplicating module trees, share via the (small)
// library facade in `src/lib.rs` and import from there in the binary.
use forgeplan::commands;

use clap::{Parser, Subcommand};

/// Load `.env` from the nearest `.forgeplan/` workspace (walk-up from cwd).
///
/// dotenvy's default `dotenv()` only reads `.env` from the current directory,
/// which misses `.forgeplan/.env` — the canonical location for forgeplan
/// API keys (PROB-041). This walks up from cwd to find a workspace and
/// loads its `.env` first. Does not override already-set env vars, so
/// precedence is: shell env > workspace .env > cwd .env.
fn load_workspace_env() {
    if let Ok(cwd) = std::env::current_dir()
        && let Some(ws) = forgeplan_core::workspace::find_workspace(&cwd)
    {
        dotenvy::from_path(ws.join(".env")).ok();
    }
}

#[derive(Parser)]
#[command(
    name = "forgeplan",
    about = "Forge your plan -- structured artifacts with quality scoring"
)]
#[command(version, propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Query the activity log — append-only JSONL record of every MCP tool
    /// invocation at .forgeplan/logs/tools-YYYY-MM-DD.jsonl. Use this to
    /// reconstruct what the agent did over a time window, attribute LLM-token
    /// spend, or audit destructive operations.
    Activity {
        /// Time window in hours back from now (1..=720, default 24)
        #[arg(long, default_value_t = 24)]
        since_hours: u32,
        /// Filter by tool name. Comma-separated for multiple:
        /// "forgeplan_score,forgeplan_activate"
        #[arg(long)]
        tool: Option<String>,
        /// Filter by status: ok, tool_err, or rpc_err. Omit for all.
        #[arg(long)]
        status: Option<String>,
        /// Cap result set (most recent N). 1..=5000, default 500.
        #[arg(long, default_value_t = 500)]
        limit: u32,
        /// Output as JSON for machine consumption
        #[arg(long)]
        json: bool,
    },
    /// Aggregate statistics from the activity log grouped by tool name:
    /// count, error count, p50/p95 duration, total time. Use to attribute
    /// LLM-token spend and identify slow tools.
    ActivityStats {
        /// Time window in hours (1..=720, default 24)
        #[arg(long, default_value_t = 24)]
        since_hours: u32,
        /// Output as JSON for machine consumption
        #[arg(long)]
        json: bool,
    },
    /// Restore a soft-deleted artifact from the most recent non-consumed
    /// receipt in `.forgeplan/trash/`. Works for delete (recreates row +
    /// moves projection back), supersede (resets status + drops link), and
    /// deprecate (resets status). Refuses if a different artifact with the
    /// same ID currently exists. TTL default: 30 days from the destructive op.
    Restore {
        /// Artifact ID to recover from the most recent non-consumed receipt
        id: String,
        /// Output as JSON for machine consumption
        #[arg(long)]
        json: bool,
    },
    /// Reverse the most recent destructive operation (delete, supersede, or
    /// deprecate) by reading the soft-delete trash and applying restore to
    /// the most recently written non-consumed receipt. If no matching receipt
    /// is found, returns an error with guidance; the tool never guesses.
    UndoLast {
        /// Time window (hours) to search for the last destructive op (1..=720, default 24)
        #[arg(long, default_value_t = 24)]
        within_hours: u32,
        /// Output as JSON for machine consumption
        #[arg(long)]
        json: bool,
    },
    /// Initialize a new .forgeplan/ workspace
    Init {
        /// Force reinitialize even if .forgeplan/ exists
        #[arg(long)]
        force: bool,
        /// Non-interactive mode (skip prompts, use defaults)
        #[arg(long, short = 'y')]
        yes: bool,
        /// Scan for existing documents and import them
        #[arg(long)]
        scan: bool,
    },
    /// Create a new artifact from template
    New {
        /// Artifact kind: prd, epic, spec, rfc, adr, problem, solution, evidence, note, refresh
        kind: String,
        /// Artifact title
        title: String,
        /// Skip duplicate-detection prompt and create anyway
        #[arg(long, visible_alias = "force")]
        allow_duplicate: bool,
    },
    /// List artifacts
    List {
        /// Filter by kind (prd, epic, spec, rfc, adr, etc.)
        #[arg(long, short = 't')]
        r#type: Option<String>,
        /// Filter by status (draft, active, etc.)
        #[arg(long, short)]
        status: Option<String>,
        /// Filter by tag. Supports "key=value" or bare "key" (matches any value).
        /// Examples: --tag source=code, --tag legacy
        #[arg(long)]
        tag: Option<String>,
        /// Output as JSON for machine consumption
        #[arg(long)]
        json: bool,
    },
    /// Show project status dashboard
    Status,
    /// Add tags to an artifact
    Tag {
        /// Artifact ID (e.g. PRD-001)
        id: String,
        /// Tags to add (e.g. source=code layer=auth legacy)
        #[arg(required = true)]
        tags: Vec<String>,
    },
    /// Remove tags from an artifact
    Untag {
        /// Artifact ID
        id: String,
        /// Tags to remove
        #[arg(required = true)]
        tags: Vec<String>,
    },
    /// Start brownfield discovery — creates session, prints protocol for agent
    Discover {
        #[command(subcommand)]
        action: DiscoverAction,
    },
    /// Validate artifact completeness against schema rules
    Validate {
        /// Artifact ID (validates all if omitted)
        id: Option<String>,
        /// Output as JSON for machine consumption
        #[arg(long)]
        json: bool,
        /// Run adversarial (devil's advocate) review
        #[arg(long)]
        adversarial: bool,
        /// CI mode: exit code 1 if any MUST rules fail
        #[arg(long)]
        ci: bool,
    },
    /// Compute R_eff quality score for decisions with evidence
    Score {
        /// Artifact ID (omit with --all to score everything)
        id: Option<String>,
        /// Score all active decision artifacts and update cached R_eff
        #[arg(long)]
        all: bool,
        /// Output as JSON for machine consumption
        #[arg(long)]
        json: bool,
    },
    /// Estimate effort for an artifact based on FR and Phase items
    Estimate {
        /// Artifact ID to estimate
        id: String,
        /// Override grade for all items (junior|middle|senior|principal|ai)
        #[arg(long)]
        grade: Option<String>,
        /// Use grade profile from config (domain-aware)
        #[arg(long)]
        my_grade: bool,
        /// Use LLM-based complexity scoring instead of rule-based heuristics
        #[arg(long)]
        llm_score: bool,
        /// Manual complexity overrides: FR-001=5,FR-002=3 (Fibonacci: 1,2,3,5,8,13)
        #[arg(long)]
        complexity: Option<String>,
        /// Output as JSON for machine consumption
        #[arg(long)]
        json: bool,
    },
    /// Link two artifacts with a typed relationship
    Link {
        /// Source artifact ID
        source: String,
        /// Target artifact ID
        target: String,
        /// Relationship type: informs, based_on, supersedes, contradicts, refines
        #[arg(long, default_value = "informs")]
        relation: String,
    },
    /// Remove a relation between two artifacts
    Unlink {
        /// Source artifact ID
        source: String,
        /// Target artifact ID
        target: String,
        /// Relationship type to remove
        #[arg(long, default_value = "informs")]
        relation: String,
    },
    /// Generate mermaid dependency graph of linked artifacts
    Graph {
        /// Output as JSON for machine consumption
        #[arg(long)]
        json: bool,
    },
    /// Search artifacts (smart by default: keyword + semantic + boosters)
    Search {
        /// Search query
        query: String,
        /// Filter by kind (prd, rfc, adr, note, ...)
        #[arg(long, short = 't')]
        r#type: Option<String>,
        /// Filter by status (draft, active, superseded, deprecated, stale)
        #[arg(long, short = 's')]
        status: Option<String>,
        /// Filter by depth (tactical, standard, deep, critical)
        #[arg(long)]
        depth: Option<String>,
        /// Only artifacts with evidence linked (R_eff > 0)
        #[arg(long)]
        with_evidence: bool,
        /// Only artifacts WITHOUT evidence (blind spots)
        #[arg(long, conflicts_with = "with_evidence")]
        no_evidence: bool,
        /// Only artifacts created after this date (YYYY-MM-DD)
        #[arg(long)]
        since: Option<String>,
        /// Disable graph expansion (1-hop neighbors in results)
        #[arg(long)]
        no_expand: bool,
        /// Force keyword-only search (substring grep)
        #[arg(long, conflicts_with = "semantic")]
        keyword: bool,
        /// Force semantic-only search (vector similarity)
        #[arg(long, conflicts_with = "keyword")]
        semantic: bool,
        /// Max results to return (default: 20)
        #[arg(long, short = 'n', default_value = "20")]
        limit: usize,
        /// Output as JSON for machine consumption
        #[arg(long)]
        json: bool,
    },
    /// Detect stale artifacts with expired valid_until
    Stale {
        /// Output as JSON for machine consumption
        #[arg(long)]
        json: bool,
    },
    /// Show methodology session state (current phase, active artifact)
    Session {
        /// Reset session to Idle
        #[arg(long)]
        reset: bool,
    },
    /// Show checkbox progress for artifacts
    Progress {
        /// Artifact ID (shows all if omitted)
        id: Option<String>,
        /// Output as JSON for machine consumption
        #[arg(long)]
        json: bool,
    },
    /// Claim an artifact (PRD-057 multi-agent coordination — soft signal "I'm working on this")
    Claim {
        /// Artifact ID to claim (e.g. PRD-057)
        id: String,
        /// Agent identity ("name/version"). Defaults to `cli/<version>`.
        #[arg(long)]
        agent: Option<String>,
        /// Time-to-live in minutes (default 30, max 1440 = 24h, min 1)
        #[arg(long, default_value = "30")]
        ttl_minutes: u32,
        /// Optional free-form note surfaced by `forgeplan claims`
        #[arg(long)]
        note: Option<String>,
        /// Output as JSON for machine consumption
        #[arg(long)]
        json: bool,
    },
    /// List active claims (sorted by expiry, soonest first)
    Claims {
        /// Output as JSON for machine consumption
        #[arg(long)]
        json: bool,
    },
    /// Show evidence decay impact on R_eff scores
    Decay,
    /// Compare estimated vs actual hours — calibrate estimation accuracy
    CalibrateEstimate {
        /// Artifact ID to calibrate
        id: String,
        /// Actual hours spent
        #[arg(long)]
        actual_hours: f64,
        /// Grade to compare (junior, mid, senior). Defaults to total score.
        #[arg(long)]
        grade: Option<String>,
    },
    /// Suggest depth level (Tactical/Standard/Deep/Critical) based on artifact content
    Calibrate {
        /// Artifact ID (checks all if omitted)
        id: Option<String>,
    },
    /// Promote a memory to a full artifact (e.g., forgeplan promote mem-xxx --kind prd)
    Promote {
        /// Memory ID to promote (e.g., mem-auth-decisions)
        memory_id: String,
        /// Target artifact kind: prd, rfc, adr, note, problem, etc.
        #[arg(long)]
        kind: String,
    },
    /// Generate an artifact using AI from a natural language description
    Generate {
        /// Artifact kind: prd, epic, spec, rfc, adr, problem, solution, evidence
        kind: String,
        /// Description of what to generate
        description: String,
    },
    /// Analyze an artifact using FPF ADI reasoning cycle (Abduction→Deduction→Induction)
    Reason {
        /// Artifact ID to analyze
        id: String,
        /// Output structured JSON instead of markdown
        #[arg(long)]
        json: bool,
        /// Save ADI analysis as a Note artifact linked to the source
        #[arg(long)]
        save: bool,
        /// Inject relevant FPF patterns into the ADI prompt
        #[arg(long)]
        fpf: bool,
    },
    /// Decompose a PRD into RFC tasks using AI
    Decompose {
        /// PRD artifact ID to decompose
        id: String,
    },
    /// Single-call reasoning context — artifact + graph + validation + scoring
    Context {
        /// Artifact ID
        id: String,
        /// Output as JSON for machine consumption (primary mode for AI agents)
        #[arg(long)]
        json: bool,
    },
    /// Read a full artifact by ID
    Get {
        /// Artifact ID
        id: String,
        /// Output as JSON for machine consumption
        #[arg(long)]
        json: bool,
    },
    /// Update artifact metadata or body
    Update {
        /// Artifact ID
        id: String,
        /// New status (draft, active, superseded, deprecated)
        #[arg(long)]
        status: Option<String>,
        /// New title
        #[arg(long)]
        title: Option<String>,
        /// New depth (tactical, standard, deep)
        #[arg(long)]
        depth: Option<String>,
        /// New body content (use @filepath to read from file)
        #[arg(long)]
        body: Option<String>,
    },
    /// Delete an artifact
    Delete {
        /// Artifact ID
        id: String,
        /// Skip confirmation
        #[arg(long)]
        yes: bool,
    },
    /// Suggest depth level and artifact pipeline for a task description
    Route {
        /// Task description in natural language
        description: String,
        /// Optional: use LLM to explain the routing decision (deprecated, use --level 1)
        #[arg(long)]
        explain: bool,
        /// Routing level: 0 = keywords (default), 1 = LLM-classified
        #[arg(long)]
        level: Option<u8>,
    },
    /// Review an artifact — run validation and show lifecycle checklist
    Review {
        /// Artifact ID
        id: String,
    },
    /// Activate an artifact (draft → active) with validation gate
    Activate {
        /// Artifact ID
        id: String,
        /// Force activation even if validation has MUST errors
        #[arg(long)]
        force: bool,
    },
    /// Supersede an artifact (active → superseded) with replacement link
    Supersede {
        /// Artifact ID to supersede
        id: String,
        /// Replacement artifact ID
        #[arg(long)]
        by: String,
    },
    /// Deprecate an artifact (active/stale → deprecated) with reason
    Deprecate {
        /// Artifact ID
        id: String,
        /// Reason for deprecation
        #[arg(long)]
        reason: String,
    },
    /// Release a claim (PRD-057). Idempotent — missing claim = success.
    Release {
        /// Artifact ID to release
        id: String,
        /// Agent identity. Defaults to `cli/<version>` (or empty when --force).
        #[arg(long)]
        agent: Option<String>,
        /// Force-release regardless of holder (orchestrator escape hatch)
        #[arg(long)]
        force: bool,
        /// Output as JSON for machine consumption
        #[arg(long)]
        json: bool,
    },
    /// Generate Keep-a-Changelog–shaped release notes from artifacts that
    /// changed between two git refs. Walks `git log` over
    /// `.forgeplan/{prds,problems,evidence,rfcs,adrs,specs,epics,solutions}/`
    /// and categorises each touched artifact: PRD→Added, PROB→Fixed,
    /// EVID-on-security→Security, RFC/ADR→Changed. Quality gate: only
    /// artifacts with `status==active` or `r_eff_score > 0` are emitted
    /// (override with `--draft`).
    #[command(name = "release-notes")]
    ReleaseNotes {
        /// Git ref to start from (default: latest tag).
        #[arg(long)]
        since: Option<String>,
        /// Git ref to end at (default: HEAD).
        #[arg(long)]
        until: Option<String>,
        /// Output format: text, markdown (alias md), json. Default: markdown.
        #[arg(long, default_value = "markdown")]
        output: String,
        /// Disable the quality gate — include active artifacts without
        /// evidence and drafts with r_eff_score=0.
        #[arg(long)]
        draft: bool,
    },
    /// Renew a stale artifact (stale → active) with extended validity
    Renew {
        /// Artifact ID
        id: String,
        /// Reason for renewal
        #[arg(long)]
        reason: String,
        /// New valid_until date (YYYY-MM-DD)
        #[arg(long)]
        until: String,
    },
    /// Reopen an artifact — creates a NEW draft artifact, deprecates the old one
    Reopen {
        /// Artifact ID to reopen
        id: String,
        /// Reason for reopening
        #[arg(long)]
        reason: String,
    },
    /// Install /forge skill for Claude Code
    SetupSkill,
    /// FPF Knowledge Base — dashboard, ingest, search, sections
    #[command(subcommand)]
    Fpf(FpfCommands),
    /// Show pipeline compliance gaps by depth
    Gaps,
    /// Show F-G-R quality scores (Formality, Granularity, Reliability)
    Fgr {
        /// Artifact ID (scores all if omitted)
        id: Option<String>,
        /// Output as JSON for machine consumption
        #[arg(long)]
        json: bool,
    },
    /// Scan codebase for source modules
    Scan {
        /// Path to project root (default: current dir)
        #[arg(long)]
        path: Option<String>,
    },
    /// Show decision coverage per code module
    Coverage {
        /// Backfill "Affected Files" section into artifacts missing it
        #[arg(long)]
        backfill: bool,
    },
    /// Compute a parallel-safe work plan for N sub-agents (PRD-057 dispatcher)
    Dispatch {
        /// Number of sub-agents the orchestrator can hand work to (>=1, max 64)
        #[arg(long, short = 'n')]
        agents: u32,
        /// Optional filter: only artifacts with this parent Epic ID
        #[arg(long)]
        epic: Option<String>,
        /// Optional filter: only consider artifacts of this kind (prd/rfc/spec/...)
        #[arg(long, short = 't')]
        kind: Option<String>,
        /// Status filter (default `draft`; pass `any` for all states)
        #[arg(long, short = 's', default_value = "draft")]
        status: String,
        /// Jaccard threshold for file-overlap conflict detection (default 0.3)
        #[arg(long, default_value = "0.3")]
        overlap_threshold: f64,
        /// Output as JSON for machine consumption
        #[arg(long)]
        json: bool,
    },
    /// Check for drifted decisions (affected files changed after decision)
    Drift {
        /// Output as JSON for machine consumption
        #[arg(long)]
        json: bool,
    },
    /// Show blocked artifacts and their dependencies
    Blocked {
        /// Specific artifact ID to check (optional)
        id: Option<String>,
        /// Output as JSON for machine consumption
        #[arg(long)]
        json: bool,
    },
    /// Show blind spots — decisions without evidence, orphan artifacts
    Blindspots,
    /// Show decision journal — chronological timeline with R_eff scores
    Journal {
        /// Filter by kind (adr, note, problem, solution)
        #[arg(long, short = 't')]
        r#type: Option<String>,
        /// Show only at-risk decisions (no evidence, stale, low R_eff)
        #[arg(long)]
        risk: bool,
    },
    /// Show project health dashboard — gaps, risks, blind spots, next actions
    Health {
        /// Compact one-line output for hooks/scripts
        #[arg(long)]
        compact: bool,
        /// Output as JSON for machine consumption
        #[arg(long)]
        json: bool,
        /// CI mode: exit code 1 if issues found (for pipeline gates)
        #[arg(long)]
        ci: bool,
        /// Fail thresholds for --ci (e.g., "orphans=5,blind_spots=3,stale=2")
        #[arg(long)]
        fail_on: Option<String>,
        /// Strict mode: exit 1 if verdict is NeedsAttention/Unhealthy or
        /// any of {orphans, blind_spots, active_stubs, at_risk} > 0.
        /// Designed for CI gates that want a single boolean signal.
        /// Empty workspaces and advisory-only signals (e.g. phase mismatches)
        /// keep exit 0.
        #[arg(long)]
        strict: bool,
    },
    /// Capture a decision from conversation into a Note or ADR artifact
    Capture {
        /// The decision statement
        decision: String,
        /// Additional context (optional)
        #[arg(long)]
        context: Option<String>,
    },
    /// Export all artifacts to JSON file
    Export {
        /// Output file path (default: .forgeplan/export.json)
        #[arg(long, short)]
        output: Option<String>,
    },
    /// Import artifacts from JSON file
    Import {
        /// Path to JSON export file
        path: String,
        /// Overwrite existing artifacts
        #[arg(long)]
        force: bool,
    },
    /// Scan for existing docs and import as artifacts
    #[command(name = "scan-import")]
    ScanImport {
        /// Directory to scan (default: standard doc dirs)
        #[arg(long)]
        path: Option<String>,
        /// Preview only, don't actually import
        #[arg(long)]
        dry_run: bool,
    },
    /// Show artifact hierarchy as ASCII tree
    Tree {
        /// Root artifact ID (shows all roots if omitted)
        id: Option<String>,
        /// Maximum depth (default: unlimited)
        #[arg(long, default_value = "99")]
        depth: usize,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show artifacts in topological order (dependency order)
    Order {
        /// Output as JSON for machine consumption
        #[arg(long)]
        json: bool,
    },
    /// Read advisory phase state for an artifact. Returns current_phase,
    /// workflow_type, timestamps, and the full append-only transition history
    /// from `.forgeplan/state/<id>.yaml`. If no state file exists yet
    /// (pre-PRD-056 artifact or phase tracking was disabled), returns
    /// `current_phase: unknown` -- never an error. Phase tracking is advisory
    /// and never blocks other tools.
    Phase {
        /// Artifact ID whose phase state to read
        id: String,
        /// Output as JSON for machine consumption
        #[arg(long)]
        json: bool,
    },
    /// Manually advance (or set) the advisory phase marker for an artifact.
    /// Appends a transition to the history. Does NOT validate phase ordering --
    /// advisory layer allows out-of-order jumps (e.g. direct `done` override).
    /// Full phase enforcement lands in a later PRD under EPIC-005. Use when
    /// auto-advancement missed a transition or when reclassifying workflow state.
    #[command(name = "phase-advance")]
    PhaseAdvance {
        /// Artifact ID to advance
        id: String,
        /// Target phase: shape, validate, adi, code, test, audit, evidence, done
        #[arg(long, value_enum)]
        to: commands::phase_advance::PhaseArg,
        /// Optional reason / justification (recorded in history)
        #[arg(long)]
        reason: Option<String>,
        /// Output as JSON for machine consumption
        #[arg(long)]
        json: bool,
    },
    /// Run schema migrations on existing workspace
    Migrate,
    /// PROB-060 Phase 0b — atomic CI assigner of `assigned_number`. Walks
    /// `.forgeplan/**/*.md` in `--head`, finds candidates with
    /// `assigned_number: null`, computes `next = max(assigned_number)+1`
    /// per kind from `--base` git ref, rewrites frontmatter (no file
    /// rename — Phase 2.1). LanceDB-free per ADR-003. Wrapped in production
    /// by `.github/workflows/assign-id.yml` `concurrency: forgeplan-id-assign`.
    CiAssignId {
        /// PR number (informational, used in commit message). Required in CI.
        #[arg(long, default_value_t = 0)]
        pr: u64,
        /// Repo slug "owner/name" (informational). Default: detect from origin.
        #[arg(long)]
        repo: Option<String>,
        /// Git ref for "destination" state for max(assigned_number) lookup.
        #[arg(long, default_value = "origin/dev")]
        base: String,
        /// Git ref for "incoming" PR state.
        #[arg(long, default_value = "HEAD")]
        head: String,
        /// Workspace root. Default: cwd.
        #[arg(long)]
        workspace: Option<std::path::PathBuf>,
        /// Do not write frontmatter; print what would change.
        #[arg(long)]
        dry_run: bool,
        /// On slug collision (slug already exists on --base), suggest
        /// `<slug>-<assigned_number>` rename. Phase 0b: warning only.
        #[arg(long)]
        auto_suffix: bool,
        /// Emit machine-readable JSON to stdout.
        #[arg(long)]
        json: bool,
    },
    /// PROB-060 Phase 0b — EVID-C migration dry-run. Scans all artifacts
    /// in `.forgeplan/`, computes the slug each would receive under
    /// SPEC-005 rules, and detects per-kind collisions before Phase 4
    /// migration. Read-only — never mutates `.md` files.
    ///
    /// Hybrid resolution: default = fail-and-list (exit 1 on collisions);
    /// `--auto-suffix` adds `suggested_resolution` per collision in JSON.
    MigrateDryRun(commands::migrate_dry_run::MigrateDryRunArgs),
    /// PROB-060 Phase 2.4 (W2.C) — manual cleanup tool for post-merge
    /// identity drift. Scans `.forgeplan/<kind>/*.md`, detects four
    /// drift categories (filename mismatch, missing `predicted_number`,
    /// body-links drift, duplicate `assigned_number`), and either
    /// reports (`--check-only`) or auto-fixes the safe categories.
    /// LanceDB is never touched; run `forgeplan scan-import` afterwards
    /// if the index needs to be rebuilt.
    ReconcileIds(commands::reconcile_ids::ReconcileIdsArgs),
    /// Rebuild LanceDB index from .md files (files-first sync)
    Reindex,
    /// Generate embeddings for all artifacts (semantic search)
    Embed,
    /// Show change log — audit trail of artifact mutations
    Log {
        /// Filter by artifact ID
        id: Option<String>,
        /// Maximum number of entries (default: 20)
        #[arg(long, short = 'n', default_value = "20")]
        limit: usize,
        /// Filter by source (cli, file_edit, git_sync, reindex)
        #[arg(long)]
        source: Option<String>,
        /// Output as JSON for machine consumption
        #[arg(long)]
        json: bool,
    },
    /// Save a memory (fact, convention, procedure) for later recall
    Remember {
        /// Text to remember (omit for --list or --forget)
        text: Option<String>,
        /// Memory category: fact, convention, procedure, insight
        #[arg(long, short)]
        category: Option<String>,
        /// List all memories
        #[arg(long)]
        list: bool,
        /// Forget (delete) a memory by ID
        #[arg(long)]
        forget: Option<String>,
    },
    /// Recall memories — search, filter, list
    Recall {
        /// Search query (substring match in title/body)
        query: Option<String>,
        /// Filter by category
        #[arg(long, short)]
        category: Option<String>,
        /// Max results (default: 10)
        #[arg(long, short = 'n', default_value = "10")]
        limit: usize,
        /// Output as JSON for machine consumption
        #[arg(long)]
        json: bool,
    },
    /// Watch .forgeplan/ files and sync changes to LanceDB in real time
    Watch,
    /// Sync artifact changes from git operations (pull/merge) into LanceDB
    GitSync {
        /// Git ref to diff against (default: ORIG_HEAD from last pull/merge)
        #[arg(long)]
        since: Option<String>,
    },
    /// Start MCP server (stdio transport) for AI agent integration
    Serve,
    /// MCP integration helpers (install config for AI agent clients)
    Mcp {
        #[command(subcommand)]
        action: McpAction,
    },
    /// Playbook runtime (PRD-065 / SPEC-003) — declarative YAML
    /// orchestration with delegations to external plugins.
    Playbook {
        #[command(subcommand)]
        action: PlaybookAction,
    },
    /// Ingest engine (PRD-066 / SPEC-004) — apply mapping YAML to plugin
    /// outputs, generate forge artifacts with file:line source refs.
    Ingest {
        /// Path to mapping YAML file
        #[arg(long)]
        mapping: std::path::PathBuf,
        /// Source path (file or dir to ingest)
        #[arg(long)]
        source: std::path::PathBuf,
        /// Print drafts without writing
        #[arg(long)]
        dry_run: bool,
        /// Allow update of existing artifacts (vs skip on hash match)
        #[arg(long)]
        update: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Plugin detection + recommendations (PRD-067) — list installed
    /// plugins, doctor health-check, info on specific plugin.
    Plugins {
        #[command(subcommand)]
        action: PluginsAction,
    },
}

#[derive(Subcommand)]
enum McpAction {
    /// Start MCP server (alias for `forgeplan serve`)
    Serve,
    /// Install forgeplan MCP config into a client (Claude / Cursor / Windsurf).
    ///
    /// Smart-merge: replaces command/args/transport, preserves existing `env`.
    /// Idempotent — safe to re-run.
    Install {
        /// Target client: claude, cursor, or windsurf
        #[arg(long, short)]
        client: String,
        /// Config scope: user (global) or project (local). Default: user.
        #[arg(long, short, default_value = "user")]
        scope: String,
        /// Override binary path (default: detected from current_exe)
        #[arg(long, conflicts_with = "use_name")]
        binary_path: Option<std::path::PathBuf>,
        /// Use short name instead of absolute path: forgeplan or fpl.
        /// Requires PATH to include the binary's directory at MCP launch time.
        /// macOS GUI apps may not inherit shell PATH — use absolute path
        /// (the default) for maximum portability.
        #[arg(long, conflicts_with = "binary_path")]
        use_name: Option<String>,
        /// Print proposed change without writing
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum PlaybookAction {
    /// List available playbooks (built-in + installed packs) with applicable
    /// recommendations based on project signals.
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show playbook structure (steps, delegations, requires).
    Show {
        /// Playbook name (e.g. brownfield-code) or path to .yaml file
        target: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Validate playbook YAML against SPEC-003 schema.
    Validate {
        /// Path to playbook YAML file
        file: std::path::PathBuf,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Run a playbook — sequential step execution with delegations.
    Run {
        /// Playbook name or path
        target: String,
        /// Confirm execution (required — prevents accidental runs)
        #[arg(long)]
        yes: bool,
        /// Allow `Delegation::Command` (shell-exec) steps to run.
        /// PROB-053 / PRD-074 §FR-1: default-deny gate for the CWE-78
        /// surface. Implies `--yes`. Alternative: set `[playbook]
        /// allow_shell = true` in workspace `config.yaml`.
        #[arg(long)]
        allow_shell: bool,
        /// Print steps without executing
        #[arg(long)]
        dry_run: bool,
        /// Start from specific step (1-indexed)
        #[arg(long)]
        step: Option<usize>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum PluginsAction {
    /// List installed plugins (Claude / agent-skills / Cursor / Forgeplan).
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Health check across known plugins — reports missing, outdated, OK.
    Doctor {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show details for a specific plugin (path, version, description).
    Info {
        /// Plugin name (e.g. c4-architecture)
        name: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum DiscoverAction {
    /// Start a new discovery session — prints protocol for AI agent
    Start {
        /// Project name for the discovery session
        name: String,
    },
    /// List all discovery sessions in the workspace
    List,
    /// Show status of a discovery session
    Show {
        /// Session ID (e.g. disc-20260407-abc)
        session_id: String,
    },
    /// Mark a discovery session as completed
    Complete {
        /// Session ID to complete
        session_id: String,
    },
}

#[derive(Subcommand)]
enum FpfCommands {
    /// Show FPF dashboard — bounded contexts, quality scores, explore-exploit actions
    Dashboard,
    /// Ingest FPF spec into knowledge base
    Ingest {
        /// Path to FPF sections directory
        #[arg(long)]
        path: Option<String>,
    },
    /// Search FPF knowledge base
    Search {
        /// Search query
        query: String,
        /// Max results
        #[arg(long, default_value = "5")]
        limit: usize,
        /// Use semantic vector search (requires --features semantic-search; falls back to keyword otherwise)
        #[arg(long)]
        semantic: bool,
    },
    /// Show a specific FPF section
    Section {
        /// Section ID (e.g. "B.3", "C.2.2")
        id: String,
        /// Show summary only (first 500 chars)
        #[arg(long)]
        summary: bool,
    },
    /// List all FPF sections
    List,
    /// Show FPF knowledge base status — source, ingested count, staleness
    Status,
    /// List active FPF rules grouped by action (EXPLORE/INVESTIGATE/EXPLOIT)
    Rules {
        /// Flat priority-linear table instead of action-grouped tree
        #[arg(long)]
        flat: bool,
        /// Output full rule dump as JSON
        #[arg(long)]
        json: bool,
    },
    /// Check which FPF rules match a given artifact
    Check {
        /// Artifact ID (e.g. PRD-041)
        id: String,
        /// Show unmatched rule names too
        #[arg(long)]
        verbose: bool,
        /// Output full RuleCheckResult as JSON
        #[arg(long)]
        json: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env from workspace first (.forgeplan/.env via walk-up from cwd),
    // then fall back to cwd .env. Neither call overrides shell env vars —
    // precedence: shell env > workspace .env > cwd .env.
    load_workspace_env();
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    match cli.command {
        Commands::Activity {
            since_hours,
            tool,
            status,
            limit,
            json,
        } => {
            commands::activity::run(since_hours, tool.as_deref(), status.as_deref(), limit, json)
                .await
        }
        Commands::ActivityStats { since_hours, json } => {
            commands::activity_stats::run(since_hours, json).await
        }
        Commands::Restore { id, json } => commands::restore::run(&id, json).await,
        Commands::UndoLast { within_hours, json } => {
            commands::undo_last::run(within_hours, json).await
        }
        Commands::Init { force, yes, scan } => commands::init::run(force, yes, scan).await,
        Commands::New {
            kind,
            title,
            allow_duplicate,
        } => commands::new::run(&kind, &title, allow_duplicate).await,
        Commands::List {
            r#type,
            status,
            tag,
            json,
        } => commands::list::run(r#type.as_deref(), status.as_deref(), tag.as_deref(), json).await,
        Commands::Status => commands::status::run().await,
        Commands::Tag { id, tags } => commands::tag::run_add(&id, &tags).await,
        Commands::Untag { id, tags } => commands::tag::run_remove(&id, &tags).await,
        Commands::Discover { action } => match action {
            DiscoverAction::Start { name } => commands::discover::run_start(&name).await,
            DiscoverAction::List => commands::discover::run_list().await,
            DiscoverAction::Show { session_id } => commands::discover::run_show(&session_id).await,
            DiscoverAction::Complete { session_id } => {
                commands::discover::run_complete(&session_id).await
            }
        },
        Commands::Validate {
            id,
            json,
            adversarial,
            ci,
        } => commands::validate::run(id.as_deref(), json, adversarial, ci).await,
        Commands::Score { id, all, json } => {
            if all {
                commands::score::run_all(json).await
            } else {
                commands::score::run(id.as_deref(), json).await
            }
        }
        Commands::Estimate {
            id,
            grade,
            my_grade,
            llm_score,
            complexity,
            json,
        } => {
            commands::estimate::run(
                &id,
                grade.as_deref(),
                my_grade,
                llm_score,
                complexity.as_deref(),
                json,
            )
            .await
        }
        Commands::Link {
            source,
            target,
            relation,
        } => commands::link::run(&source, &target, &relation).await,
        Commands::Unlink {
            source,
            target,
            relation,
        } => commands::link::run_unlink(&source, &target, &relation).await,
        Commands::Graph { json } => commands::graph::run(json).await,
        Commands::Search {
            query,
            r#type,
            status,
            depth,
            with_evidence,
            no_evidence,
            since,
            no_expand,
            keyword,
            semantic,
            limit,
            json,
        } => {
            let mode = if keyword {
                commands::search::SearchMode::Keyword
            } else if semantic {
                commands::search::SearchMode::Semantic
            } else {
                commands::search::SearchMode::Smart
            };
            commands::search::run(
                &query,
                r#type.as_deref(),
                status.as_deref(),
                depth.as_deref(),
                with_evidence,
                no_evidence,
                since.as_deref(),
                no_expand,
                mode,
                limit,
                json,
            )
            .await
        }
        Commands::Stale { json } => commands::stale::run(json).await,
        Commands::Session { reset } => {
            if reset {
                commands::session::run_reset();
            } else {
                commands::session::run_status();
            }
            Ok(())
        }
        Commands::Progress { id, json } => commands::progress::run(id.as_deref(), json).await,
        Commands::Claim {
            id,
            agent,
            ttl_minutes,
            note,
            json,
        } => {
            commands::claim::run(
                &id,
                agent.as_deref(),
                Some(ttl_minutes),
                note.as_deref(),
                json,
            )
            .await
        }
        Commands::Claims { json } => commands::claims::run(json).await,
        Commands::Decay => commands::decay::run().await,
        Commands::Calibrate { id } => commands::calibrate::run(id.as_deref()).await,
        Commands::CalibrateEstimate {
            id,
            actual_hours,
            grade,
        } => commands::calibrate_estimate::run(&id, actual_hours, grade.as_deref()).await,
        Commands::Promote { memory_id, kind } => commands::promote::run(&memory_id, &kind).await,
        Commands::Generate { kind, description } => {
            commands::generate::run(&kind, &description).await
        }
        Commands::Reason {
            id,
            json,
            save,
            fpf,
        } => commands::reason::run(&id, json, save, fpf).await,
        Commands::Decompose { id } => commands::decompose::run(&id).await,
        Commands::Context { id, json } => commands::context::run(&id, json).await,
        Commands::Get { id, json } => commands::get::run(&id, json).await,
        Commands::Update {
            id,
            status,
            title,
            depth,
            body,
        } => {
            commands::update::run(
                &id,
                status.as_deref(),
                title.as_deref(),
                depth.as_deref(),
                body.as_deref(),
            )
            .await
        }
        Commands::Delete { id, yes } => commands::delete::run(&id, yes).await,
        Commands::Scan { path } => commands::coverage::run_scan(path.as_deref()).await,
        Commands::Coverage { backfill } => {
            if backfill {
                commands::coverage::run_backfill().await
            } else {
                commands::coverage::run_coverage().await
            }
        }
        Commands::Dispatch {
            agents,
            epic,
            kind,
            status,
            overlap_threshold,
            json,
        } => {
            commands::dispatch::run(
                agents,
                epic.as_deref(),
                kind.as_deref(),
                Some(status.as_str()),
                Some(overlap_threshold),
                json,
            )
            .await
        }
        Commands::Drift { json } => commands::drift::run(json).await,
        Commands::Blocked { id, json } => commands::blocked::run(id.as_deref(), json).await,
        Commands::Blindspots => commands::blindspots::run().await,
        Commands::Journal { r#type, risk } => commands::journal::run(r#type.as_deref(), risk).await,
        Commands::Health {
            compact,
            json,
            ci,
            fail_on,
            strict,
        } => commands::health::run(compact, json, ci, fail_on, strict).await,
        Commands::Route {
            description,
            explain,
            level,
        } => commands::route::run(&description, explain, level).await,
        Commands::Review { id } => commands::review::run(&id).await,
        Commands::Activate { id, force } => commands::activate::run(&id, force).await,
        Commands::Supersede { id, by } => commands::supersede::run(&id, &by).await,
        Commands::Deprecate { id, reason } => commands::deprecate::run(&id, &reason).await,
        Commands::Release {
            id,
            agent,
            force,
            json,
        } => commands::release::run(&id, agent.as_deref(), force, json).await,
        Commands::ReleaseNotes {
            since,
            until,
            output,
            draft,
        } => commands::release_notes::run(since.as_deref(), until.as_deref(), &output, draft).await,
        Commands::Renew { id, reason, until } => commands::renew::run(&id, &reason, &until).await,
        Commands::Reopen { id, reason } => commands::reopen::run(&id, &reason).await,
        Commands::SetupSkill => commands::setup_skill::run().await,
        Commands::Fpf(sub) => match sub {
            FpfCommands::Dashboard => commands::fpf::run_dashboard().await,
            FpfCommands::Ingest { path } => commands::fpf::run_ingest(path.as_deref()).await,
            FpfCommands::Search {
                query,
                limit,
                semantic,
            } => commands::fpf::run_search(&query, limit, semantic).await,
            FpfCommands::Section { id, summary } => commands::fpf::run_section(&id, summary).await,
            FpfCommands::List => commands::fpf::run_list().await,
            FpfCommands::Status => commands::fpf::run_status().await,
            FpfCommands::Rules { flat, json } => commands::fpf::run_rules(flat, json).await,
            FpfCommands::Check { id, verbose, json } => {
                commands::fpf::run_check(&id, verbose, json).await
            }
        },
        Commands::Gaps => commands::gaps::run().await,
        Commands::Fgr { id, json } => commands::fgr::run(id.as_deref(), json).await,
        Commands::Capture { decision, context } => {
            commands::capture::run(&decision, context.as_deref()).await
        }
        Commands::Export { output } => commands::export::run(output.as_deref()).await,
        Commands::Import { path, force } => commands::import_cmd::run(&path, force).await,
        Commands::ScanImport { path, dry_run } => {
            commands::scan_import::run(path.as_deref(), dry_run).await
        }
        Commands::Tree { id, depth, json } => commands::tree::run(id.as_deref(), depth, json).await,
        Commands::Order { json } => commands::order::run(json).await,
        Commands::Phase { id, json } => commands::phase::run(&id, json).await,
        Commands::PhaseAdvance {
            id,
            to,
            reason,
            json,
        } => commands::phase_advance::run(&id, to, reason.as_deref(), json).await,
        Commands::Migrate => commands::migrate::run().await,
        Commands::CiAssignId {
            pr,
            repo,
            base,
            head,
            workspace,
            dry_run,
            auto_suffix,
            json,
        } => {
            // PROB-060 Phase 0b — propagate exit codes 0/1/2/3/4 per CD-1.
            let args = commands::ci_assign_id::CiAssignIdArgs {
                pr,
                repo,
                base,
                head,
                workspace,
                dry_run,
                auto_suffix,
                json,
            };
            let code = commands::ci_assign_id::run(args).await?;
            if code != 0 {
                std::process::exit(code);
            }
            Ok(())
        }
        Commands::MigrateDryRun(args) => {
            // PROB-060 Phase 0b — propagate non-zero exit codes via std::process::exit
            // so shells distinguish "no collisions (0)" / "collisions (1)" /
            // "scan error (2)". Returning anyhow::Result<()> alone collapses
            // 1/2 to a generic 1.
            let code = commands::migrate_dry_run::run(args).await?;
            if code != 0 {
                std::process::exit(code);
            }
            Ok(())
        }
        Commands::ReconcileIds(args) => {
            // PROB-060 Phase 2.4 — same exit-code propagation as
            // migrate-dry-run. 0 = clean, 1 = drift detected, 2 = scan error.
            let code = commands::reconcile_ids::run(args)?;
            if code != 0 {
                std::process::exit(code);
            }
            Ok(())
        }
        Commands::Reindex => commands::reindex::run().await,
        Commands::Embed => commands::embed::run().await,
        Commands::Log {
            id,
            limit,
            source,
            json,
        } => commands::log_cmd::run(id.as_deref(), source.as_deref(), limit, json).await,
        Commands::Remember {
            text,
            category,
            list,
            forget,
        } => {
            commands::remember::run(
                text.as_deref(),
                category.as_deref(),
                list,
                forget.as_deref(),
            )
            .await
        }
        Commands::Recall {
            query,
            category,
            limit,
            json,
        } => commands::recall::run(query.as_deref(), category.as_deref(), limit, json).await,
        Commands::Watch => commands::watch::run().await,
        Commands::GitSync { since } => commands::git_sync::run(since.as_deref()).await,
        Commands::Serve => {
            let cwd = std::env::current_dir()?;
            forgeplan_mcp::run_stdio(cwd).await
        }
        Commands::Mcp { action } => match action {
            McpAction::Serve => {
                let cwd = std::env::current_dir()?;
                forgeplan_mcp::run_stdio(cwd).await
            }
            McpAction::Install {
                client,
                scope,
                binary_path,
                use_name,
                dry_run,
            } => {
                let opts = commands::mcp::InstallOptions {
                    client: commands::mcp::McpClient::parse(&client)?,
                    scope: commands::mcp::Scope::parse(&scope)?,
                    binary_path,
                    use_name,
                    dry_run,
                };
                commands::mcp::run_install(opts).await
            }
        },
        Commands::Playbook { action } => match action {
            PlaybookAction::List { json } => commands::playbook::run_list(json).await,
            PlaybookAction::Show { target, json } => {
                commands::playbook::run_show(&target, json).await
            }
            PlaybookAction::Validate { file, json } => {
                commands::playbook::run_validate(&file, json).await
            }
            PlaybookAction::Run {
                target,
                yes,
                allow_shell,
                dry_run,
                step,
                json,
            } => {
                commands::playbook::run_execute(&target, yes, allow_shell, dry_run, step, json)
                    .await
            }
        },
        Commands::Ingest {
            mapping,
            source,
            dry_run,
            update,
            json,
        } => commands::ingest::run(&mapping, &source, dry_run, update, json).await,
        Commands::Plugins { action } => match action {
            PluginsAction::List { json } => commands::plugins::run_list(json).await,
            PluginsAction::Doctor { json } => commands::plugins::run_doctor(json).await,
            PluginsAction::Info { name, json } => commands::plugins::run_info(&name, json).await,
        },
    }
}
