use forgeplan_core::stale;
use forgeplan_core::workspace;

pub fn run() -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let stale_artifacts = stale::find_stale(&ws)?;

    if stale_artifacts.is_empty() {
        println!("No stale artifacts found. All valid_until dates are current.");
        return Ok(());
    }

    println!(
        "Found {} stale artifact(s) with expired valid_until:\n",
        stale_artifacts.len()
    );

    println!(
        "  {:<12} {:<30} {:<14} {}",
        "ID", "Title", "Expired", "Days ago"
    );
    println!("  {}", "-".repeat(70));

    for sa in &stale_artifacts {
        let title = if sa.artifact.title.len() > 28 {
            format!("{}...", &sa.artifact.title[..25])
        } else {
            sa.artifact.title.clone()
        };
        println!(
            "  {:<12} {:<30} {:<14} {} days",
            sa.artifact.id, title, sa.valid_until, sa.days_expired
        );
    }

    println!();
    println!("Hint: Use `forgeplan score <ID>` to check R_eff impact of stale evidence.");

    Ok(())
}
