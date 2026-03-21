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
    /// Validate artifact completeness (placeholder)
    Validate {
        /// Artifact ID (validates all if omitted)
        id: Option<String>,
    },
    /// Show R_eff quality score (placeholder)
    Score {
        /// Artifact ID
        id: Option<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { force } => commands::init::run(force),
        Commands::New { kind, title } => commands::new::run(&kind, &title),
        Commands::List { r#type, status } => {
            commands::list::run(r#type.as_deref(), status.as_deref())
        }
        Commands::Status => commands::status::run(),
        Commands::Validate { id } => {
            println!("forgeplan validate {:?} -- coming in Phase 3B", id);
            Ok(())
        }
        Commands::Score { id } => {
            println!("forgeplan score {:?} -- coming in Phase 3B", id);
            Ok(())
        }
    }
}
