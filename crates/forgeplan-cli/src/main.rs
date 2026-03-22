mod commands;

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
    },
    /// Show project status dashboard
    Status,
    /// Validate artifact completeness against schema rules
    Validate {
        /// Artifact ID (validates all if omitted)
        id: Option<String>,
    },
    /// Compute R_eff quality score for decisions with evidence
    Score {
        /// Artifact ID
        id: Option<String>,
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
    /// Generate mermaid dependency graph of linked artifacts
    Graph,
    /// Search artifacts by keyword (or --semantic for vector similarity search)
    Search {
        /// Search query
        query: String,
        /// Filter by kind
        #[arg(long, short = 't')]
        r#type: Option<String>,
        /// Use semantic (vector) search instead of substring match
        #[arg(long)]
        semantic: bool,
    },
    /// Detect stale artifacts with expired valid_until
    Stale,
    /// Show checkbox progress for artifacts
    Progress {
        /// Artifact ID (shows all if omitted)
        id: Option<String>,
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
    },
    /// Decompose a PRD into RFC tasks using AI
    Decompose {
        /// PRD artifact ID to decompose
        id: String,
    },
    /// Read a full artifact by ID
    Get {
        /// Artifact ID
        id: String,
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
    },
    /// Show project health dashboard — gaps, risks, blind spots, next actions
    Health {
        /// Compact one-line output for hooks/scripts
        #[arg(long)]
        compact: bool,
    },
    /// Capture a decision from conversation into a Note or ADR artifact
    Capture {
        /// The decision statement
        decision: String,
        /// Additional context (optional)
        #[arg(long)]
        context: Option<String>,
    },
    /// Start MCP server (stdio transport) for AI agent integration
    Serve,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { force } => commands::init::run(force).await,
        Commands::New { kind, title } => commands::new::run(&kind, &title).await,
        Commands::List { r#type, status } => {
            commands::list::run(r#type.as_deref(), status.as_deref()).await
        }
        Commands::Status => commands::status::run().await,
        Commands::Validate { id } => commands::validate::run(id.as_deref()).await,
        Commands::Score { id } => commands::score::run(id.as_deref()).await,
        Commands::Link {
            source,
            target,
            relation,
        } => commands::link::run(&source, &target, &relation).await,
        Commands::Graph => commands::graph::run().await,
        Commands::Search {
            query,
            r#type,
            semantic,
        } => {
            commands::search::run(&query, r#type.as_deref(), semantic).await
        }
        Commands::Stale => commands::stale::run().await,
        Commands::Progress { id } => commands::progress::run(id.as_deref()).await,
        Commands::Decay => commands::decay::run().await,
        Commands::Calibrate { id } => commands::calibrate::run(id.as_deref()).await,
        Commands::Generate { kind, description } => {
            commands::generate::run(&kind, &description).await
        }
        Commands::Reason { id } => commands::reason::run(&id).await,
        Commands::Decompose { id } => commands::decompose::run(&id).await,
        Commands::Get { id } => commands::get::run(&id).await,
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
        Commands::Health { compact } => commands::health::run(compact).await,
        Commands::Route { description } => commands::route::run(&description).await,
        Commands::Capture { decision, context } => {
            commands::capture::run(&decision, context.as_deref()).await
        }
        Commands::Serve => {
            let cwd = std::env::current_dir()?;
            forgeplan_mcp::run_stdio(cwd).await
        }
    }
}
