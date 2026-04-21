use std::fs;

use anyhow::Result;

pub async fn run() -> Result<()> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;

    let skill_dir = home.join(".claude").join("skills").join("forge");
    fs::create_dir_all(&skill_dir)?;

    // Bundled copy. Authoritative upstream lives in the peer
    // `forgeplan-marketplace` repo at
    // `plugins/forgeplan-workflow/skills/forgeplan-methodology/SKILL.md`.
    // Keep this file in sync with that upstream on release.
    let skill_content = include_str!("forge-skill.md");

    let skill_path = skill_dir.join("SKILL.md");
    fs::write(&skill_path, skill_content)?;

    println!("  Installed /forge skill to {}", skill_path.display());
    println!("  Use /forge in Claude Code to activate Forgeplan workflow");

    Ok(())
}
