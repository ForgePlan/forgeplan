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
    /// Search artifacts by keyword
    Search {
        /// Search query
        query: String,
        /// Filter by kind
        #[arg(long, short = 't')]
        r#type: Option<String>,
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
        Commands::Search { query, r#type } => {
            commands::search::run(&query, r#type.as_deref()).await
        }
        Commands::Stale => commands::stale::run().await,
        Commands::Progress { id } => commands::progress::run(id.as_deref()).await,
        Commands::Decay => commands::decay::run().await,
        Commands::Calibrate { id } => commands::calibrate::run(id.as_deref()).await,
        Commands::Serve => {
            let cwd = std::env::current_dir()?;
            forgeplan_mcp::run_stdio(cwd).await
        }
    }
}
