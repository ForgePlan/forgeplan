use forgeplan_core::artifact::store;
use forgeplan_core::link;
use forgeplan_core::workspace;

pub fn run(source_id: &str, target_id: &str, relation: &str) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    // Normalize relation
    let relation = link::normalize_relation(relation)?;

    // Find source artifact
    let artifacts = store::list_artifacts(&ws)?;
    let source = artifacts
        .iter()
        .find(|a| a.id.eq_ignore_ascii_case(source_id))
        .ok_or_else(|| anyhow::anyhow!("Source artifact '{}' not found", source_id))?;

    // Verify target exists
    let target_exists = artifacts
        .iter()
        .any(|a| a.id.eq_ignore_ascii_case(target_id));
    if !target_exists {
        eprintln!(
            "Warning: Target artifact '{}' not found in workspace (creating link anyway)",
            target_id
        );
    }

    // Add link
    link::add_link(&source.path, target_id, &relation)?;

    println!(
        "Linked: {} --{}--> {}",
        source.id, relation, target_id
    );
    Ok(())
}
