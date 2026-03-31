mod commands;
mod ui;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "forgeplan", about = "Forge your plan -- structured artifacts with quality scoring")]
#[command(version, propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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
    },
    /// List artifacts
    List {
        /// Filter by kind (prd, epic, spec, rfc, adr, etc.)
        #[arg(long, short = 't')]
        r#type: Option<String>,
        /// Filter by status (draft, active, etc.)
        #[arg(long, short)]
        status: Option<String>,
        /// Output as JSON for machine consumption
        #[arg(long)]
        json: bool,
    },
    /// Show project status dashboard
    Status,
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
        /// Filter by kind
        #[arg(long, short = 't')]
        r#type: Option<String>,
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
    /// Show checkbox progress for artifacts
    Progress {
        /// Artifact ID (shows all if omitted)
        id: Option<String>,
        /// Output as JSON for machine consumption
        #[arg(long)]
        json: bool,
    },
    /// Show evidence decay impact on R_eff scores
    Decay,
    /// Suggest depth level (Tactical/Standard/Deep/Critical) based on artifact content
    Calibrate {
        /// Artifact ID (checks all if omitted)
        id: Option<String>,
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
    /// Deprecate an artifact (active → deprecated) with reason
    Deprecate {
        /// Artifact ID
        id: String,
        /// Reason for deprecation
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
    /// Run schema migrations on existing workspace
    Migrate,
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
    /// Start MCP server (stdio transport) for AI agent integration
    Serve,
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { force, yes, scan } => commands::init::run(force, yes, scan).await,
        Commands::New { kind, title } => commands::new::run(&kind, &title).await,
        Commands::List { r#type, status, json } => {
            commands::list::run(r#type.as_deref(), status.as_deref(), json).await
        }
        Commands::Status => commands::status::run().await,
        Commands::Validate { id, json, adversarial } => {
            commands::validate::run(id.as_deref(), json, adversarial).await
        }
        Commands::Score { id, all, json } => {
            if all {
                commands::score::run_all(json).await
            } else {
                commands::score::run(id.as_deref(), json).await
            }
        }
        Commands::Estimate { id, grade, my_grade, llm_score, json } => {
            commands::estimate::run(&id, grade.as_deref(), my_grade, llm_score, json).await
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
            commands::search::run(&query, r#type.as_deref(), mode, limit, json).await
        }
        Commands::Stale { json } => commands::stale::run(json).await,
        Commands::Progress { id, json } => commands::progress::run(id.as_deref(), json).await,
        Commands::Decay => commands::decay::run().await,
        Commands::Calibrate { id } => commands::calibrate::run(id.as_deref()).await,
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
        Commands::Drift { json } => commands::drift::run(json).await,
        Commands::Blocked { id, json } => commands::blocked::run(id.as_deref(), json).await,
        Commands::Blindspots => commands::blindspots::run().await,
        Commands::Journal { r#type, risk } => {
            commands::journal::run(r#type.as_deref(), risk).await
        }
        Commands::Health { compact, json } => commands::health::run(compact, json).await,
        Commands::Route {
            description,
            explain,
            level,
        } => commands::route::run(&description, explain, level).await,
        Commands::Review { id } => commands::review::run(&id).await,
        Commands::Activate { id, force } => commands::activate::run(&id, force).await,
        Commands::Supersede { id, by } => commands::supersede::run(&id, &by).await,
        Commands::Deprecate { id, reason } => commands::deprecate::run(&id, &reason).await,
        Commands::SetupSkill => commands::setup_skill::run().await,
        Commands::Fpf(sub) => match sub {
            FpfCommands::Dashboard => commands::fpf::run_dashboard().await,
            FpfCommands::Ingest { path } => commands::fpf::run_ingest(path.as_deref()).await,
            FpfCommands::Search { query, limit } => commands::fpf::run_search(&query, limit).await,
            FpfCommands::Section { id, summary } => commands::fpf::run_section(&id, summary).await,
            FpfCommands::List => commands::fpf::run_list().await,
            FpfCommands::Status => commands::fpf::run_status().await,
        },
        Commands::Gaps => commands::gaps::run().await,
        Commands::Fgr { id, json } => commands::fgr::run(id.as_deref(), json).await,
        Commands::Capture { decision, context } => {
            commands::capture::run(&decision, context.as_deref()).await
        }
        Commands::Export { output } => commands::export::run(output.as_deref()).await,
        Commands::Import { path, force } => commands::import_cmd::run(&path, force).await,
        Commands::ScanImport { path, dry_run } => commands::scan_import::run(path.as_deref(), dry_run).await,
        Commands::Tree { id, depth, json } => {
            commands::tree::run(id.as_deref(), depth, json).await
        }
        Commands::Order { json } => commands::order::run(json).await,
        Commands::Migrate => commands::migrate::run().await,
        Commands::Reindex => commands::reindex::run().await,
        Commands::Embed => commands::embed::run().await,
        Commands::Log { id, limit, source, json } => {
            commands::log_cmd::run(id.as_deref(), source.as_deref(), limit, json).await
        }
        Commands::Remember { text, category, list, forget } => {
            commands::remember::run(
                text.as_deref(),
                category.as_deref(),
                list,
                forget.as_deref(),
            ).await
        }
        Commands::Recall { query, category, limit, json } => {
            commands::recall::run(query.as_deref(), category.as_deref(), limit, json).await
        }
        Commands::Watch => commands::watch::run().await,
        Commands::Serve => {
            let cwd = std::env::current_dir()?;
            forgeplan_mcp::run_stdio(cwd).await
        }
    }
}
