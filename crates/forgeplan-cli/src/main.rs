use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "forgeplan", about = "Forge your plan — structured artifacts with quality scoring")]
#[command(version, propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new .forgeplan/ workspace
    Init,
    /// Create a new artifact from template
    New {
        /// Artifact kind: prd, epic, spec, rfc, adr
        kind: String,
        /// Artifact title
        title: String,
    },
    /// List artifacts
    List {
        /// Filter by kind
        kind: Option<String>,
    },
    /// Show project status with progress bars
    Status,
    /// Validate artifact completeness
    Validate {
        /// Artifact ID (validates all if omitted)
        id: Option<String>,
    },
    /// Show R_eff quality score
    Score {
        /// Artifact ID
        id: Option<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => {
            println!("forgeplan init — TODO: create .forgeplan/ workspace");
        }
        Commands::New { kind, title } => {
            println!("forgeplan new {kind} \"{title}\" — TODO: create artifact");
        }
        Commands::List { kind } => {
            println!("forgeplan list {:?} — TODO: list artifacts", kind);
        }
        Commands::Status => {
            println!("forgeplan status — TODO: show dashboard");
        }
        Commands::Validate { id } => {
            println!("forgeplan validate {:?} — TODO: validate", id);
        }
        Commands::Score { id } => {
            println!("forgeplan score {:?} — TODO: show R_eff", id);
        }
    }

    Ok(())
}
