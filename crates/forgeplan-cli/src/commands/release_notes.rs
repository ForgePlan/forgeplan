//! `forgeplan release-notes` — auto-generate Keep-a-Changelog–shaped
//! release notes from artifacts that changed between two git refs.
//!
//! Closes the v0.31.0 sprint pain point (Wave 4 MAJOR-3) where the
//! changelog had to be reconstructed by hand by reading every artifact
//! that landed since the last tag.
//!
//! The heavy lifting (git walk + classification + formatting) lives in
//! `forgeplan_core::release_notes`. This file is the thin CLI rind:
//! parse the `--output` flag, call `core::release_notes::generate`,
//! pick a formatter, emit hint.

use anyhow::Result;

use forgeplan_core::hints::{self, Hint};
use forgeplan_core::release_notes;

use crate::commands::common;

#[derive(Debug, Clone, Copy)]
enum OutputFormat {
    Text,
    Markdown,
    Json,
}

fn parse_output(s: &str) -> Result<OutputFormat> {
    match s {
        "text" => Ok(OutputFormat::Text),
        "markdown" | "md" => Ok(OutputFormat::Markdown),
        "json" => Ok(OutputFormat::Json),
        other => {
            anyhow::bail!("unsupported --output {other:?}; expected one of: text, markdown, json")
        }
    }
}

pub async fn run(
    since: Option<&str>,
    until: Option<&str>,
    output: &str,
    draft: bool,
) -> Result<()> {
    let format = parse_output(output)?;

    let (ws, source, _read_guard) = common::open_read_source().await?;

    // `find_workspace` returns the `.forgeplan/` directory; git commands run
    // against the parent (the actual repo root).
    let repo_root = ws
        .parent()
        .ok_or_else(|| anyhow::anyhow!("workspace has no parent — bad .forgeplan/ path"))?;

    // The git walk is identical on both layouts; only the artifact lookup the
    // classifier performs differs. On a migrated workspace the old code opened
    // a `LanceStore` directly — the one read path that still resurrected the
    // retired table instead of going through the shared open.
    let notes = match &source {
        common::ReadSource::Legacy(store) => {
            release_notes::generate(store, &_read_guard, repo_root, since, until, draft).await?
        }
        common::ReadSource::Captured(read) => {
            release_notes::generate_captured(read, repo_root, since, until, draft).await?
        }
    };

    match format {
        OutputFormat::Markdown => print!("{}", release_notes::format_markdown(&notes)),
        OutputFormat::Text => print!("{}", release_notes::format_text(&notes)),
        OutputFormat::Json => {
            let val = release_notes::format_json(&notes);
            println!("{}", serde_json::to_string_pretty(&val)?);
        }
    }

    let hint_msg = if notes.is_empty() {
        format!(
            "No artifacts changed between {} and {}. Widen the range or pass --draft.",
            notes.since, notes.until
        )
    } else {
        format!(
            "{} entries. Paste under [Unreleased] in CHANGELOG.md.",
            notes.total()
        )
    };
    let hints_vec = vec![Hint::info(hint_msg)];
    if matches!(format, OutputFormat::Json) {
        if let Some(next) = hints::primary_action(&hints_vec) {
            eprintln!("Next: {next}");
        }
    } else {
        print!("{}", hints::render_next_action_line(&hints_vec));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_output_accepts_synonyms_and_rejects_garbage() {
        assert!(matches!(
            parse_output("md").unwrap(),
            OutputFormat::Markdown
        ));
        assert!(matches!(
            parse_output("markdown").unwrap(),
            OutputFormat::Markdown
        ));
        assert!(matches!(parse_output("text").unwrap(), OutputFormat::Text));
        assert!(matches!(parse_output("json").unwrap(), OutputFormat::Json));
        assert!(parse_output("yaml").is_err());
        assert!(parse_output("").is_err());
    }
}
