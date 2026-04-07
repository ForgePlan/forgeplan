use anyhow::Result;
use console::style;

use forgeplan_core::db::store::ArtifactFilter;

use crate::commands::common;
use crate::ui;

pub async fn run(
    kind_filter: Option<&str>,
    status_filter: Option<&str>,
    tag_filter: Option<&str>,
    json: bool,
) -> Result<()> {
    let store = common::store().await?;

    let artifacts = if let Some(tag) = tag_filter {
        let mut records = store.list_by_tag(tag).await?;
        if let Some(k) = kind_filter {
            let kl = k.to_lowercase();
            records.retain(|r| r.kind.eq_ignore_ascii_case(&kl));
        }
        if let Some(s) = status_filter {
            let sl = s.to_lowercase();
            records.retain(|r| r.status.eq_ignore_ascii_case(&sl));
        }
        records.iter().map(|r| r.to_summary()).collect()
    } else {
        let filter = if kind_filter.is_some() || status_filter.is_some() {
            Some(ArtifactFilter {
                kind: kind_filter.map(|s| s.to_lowercase()),
                status: status_filter.map(|s| s.to_lowercase()),
            })
        } else {
            None
        };
        store.list_artifacts(filter.as_ref()).await?
    };

    if artifacts.is_empty() {
        if json {
            println!("[]");
        } else {
            println!("  No artifacts found.");
        }
        return Ok(());
    }

    if json {
        let json_data: Vec<_> = artifacts
            .iter()
            .map(|a| {
                serde_json::json!({
                    "id": a.id,
                    "kind": a.kind,
                    "status": a.status,
                    "title": a.title,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_data)?);
        return Ok(());
    }

    // Calculate column widths for alignment
    let id_width = artifacts
        .iter()
        .map(|a| a.id.len())
        .max()
        .unwrap_or(6)
        .max(2);
    let kind_width = artifacts
        .iter()
        .map(|a| a.kind.len())
        .max()
        .unwrap_or(6)
        .max(4);
    let status_width = artifacts
        .iter()
        .map(|a| a.status.len())
        .max()
        .unwrap_or(6)
        .max(6);

    // Print header — bold underlined
    println!(
        "{:<id_w$}  {:<kind_w$}  {:<status_w$}  {}",
        style("ID").bold().underlined(),
        style("Kind").bold().underlined(),
        style("Status").bold().underlined(),
        style("Title").bold().underlined(),
        id_w = id_width,
        kind_w = kind_width,
        status_w = status_width,
    );

    // Print rows
    for a in &artifacts {
        // Pad status manually so ANSI codes don't break alignment
        let status_plain_len = a.status.len();
        let status_styled = ui::styled_status(&a.status);
        let status_padding = if status_width > status_plain_len {
            " ".repeat(status_width - status_plain_len)
        } else {
            String::new()
        };

        println!(
            "{:<id_w$}  {:<kind_w$}  {}{}  {}",
            style(&a.id).bold(),
            a.kind,
            status_styled,
            status_padding,
            a.title,
            id_w = id_width,
            kind_w = kind_width,
        );
    }

    println!("\n  {} artifact(s) total", artifacts.len());
    Ok(())
}
