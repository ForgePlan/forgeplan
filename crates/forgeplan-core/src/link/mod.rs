use std::path::Path;

use crate::artifact::frontmatter::{self, Frontmatter};
use crate::error::ForgeplanError;
use crate::projection::LinkUpsertOutcome;

/// Add a typed link to an artifact's frontmatter.
/// Writes the updated file back to disk.
pub async fn add_link(artifact_path: &Path, target_id: &str, relation: &str) -> anyhow::Result<()> {
    // Normalize target to uppercase for consistent storage and dedup
    let target_id = target_id.to_uppercase();
    add_link_inner(artifact_path, &target_id, relation, false).await
}

/// File-first counterpart of `store.add_relation` for constellation-migrated
/// workspaces: same contract, minus the index row.
///
/// Unlike [`add_link`] the target is stored **verbatim** — the caller already
/// canonicalized it (`PRD-001`) or deliberately passed a qualified
/// `KEY@qualifier` ref / a `mem-*` literal, none of which survive
/// `to_uppercase()` intact.
///
/// Guards mirrored from `store.add_relation`:
/// - [`crate::db::store::validate_link_target`] on the target (plain id or
///   `KEY@qualifier`; the qualifier charset is the injection boundary).
/// - self-link refusal when the file's own `id:` matches the target
///   case-insensitively.
/// - exact-duplicate refusal (`target` case-insensitive + `relation` exact)
///   via [`ForgeplanError::LinkExists`] — a parallel edge with a DIFFERENT
///   relation stays legal, matching `add_relation`'s dedup.
///
/// Writes the file only on success; duplicate / invalid input never touches
/// disk.
pub async fn add_link_verbatim(
    artifact_path: &Path,
    target_id: &str,
    relation: &str,
) -> anyhow::Result<()> {
    crate::db::store::validate_link_target(target_id)?;
    add_link_inner(artifact_path, target_id, relation, true).await
}

/// Write `(fm, body)` back, skipping the disk write when the rendered
/// content is byte-identical to what is already there (keeps mtime stable
/// on no-op mutations so a reconcile pass sees an unmoved source).
async fn write_links_doc(
    artifact_path: &Path,
    fm: &Frontmatter,
    body: &str,
    original: &str,
) -> anyhow::Result<()> {
    let output = frontmatter::render_frontmatter(fm, body)?;
    if output != original {
        tokio::fs::write(artifact_path, output).await?;
    }
    Ok(())
}

/// Remove `links:` entries matching `(target, relation)` — target compared
/// case-insensitively, relation exact (the file-side mirror of
/// `store.delete_relation` / `get_all_relations` matching in `unlink`).
///
/// Returns the number of entries removed. `0` means the edge did not exist
/// — callers decide whether that is an error (`unlink` surfaces it as
/// "relation not found"). A missing `links:` key is `Ok(0)`; a `links:` key
/// that is not a sequence is a frontmatter error, same as `add_link`.
///
/// Qualified `KEY@qualifier` targets remove by the same case-insensitive
/// match — verbatim equality modulo case, never parsed.
pub async fn remove_link(
    artifact_path: &Path,
    target_id: &str,
    relation: &str,
) -> anyhow::Result<usize> {
    let content = tokio::fs::read_to_string(artifact_path).await?;
    let (mut fm, body) = frontmatter::parse_frontmatter(&content)?;

    let Some(links) = fm.get_mut("links") else {
        return Ok(0);
    };
    let serde_yaml::Value::Sequence(seq) = links else {
        return Err(ForgeplanError::Frontmatter("'links' field is not a sequence".into()).into());
    };

    let before = seq.len();
    seq.retain(|entry| {
        if let serde_yaml::Value::Mapping(map) = entry {
            let t = map.get(serde_yaml::Value::String("target".into()));
            let r = map.get(serde_yaml::Value::String("relation".into()));
            let is_match = matches!(
                (t, r),
                (Some(serde_yaml::Value::String(t)), Some(serde_yaml::Value::String(r)))
                    if t.eq_ignore_ascii_case(target_id) && r == relation
            );
            !is_match
        } else {
            true
        }
    });
    let removed = before - seq.len();
    if removed == 0 {
        return Ok(0);
    }

    write_links_doc(artifact_path, &fm, &body, &content).await?;
    Ok(removed)
}

/// Idempotent link upsert on the artifact's `links:` frontmatter — the
/// file-first counterpart of
/// [`crate::projection::replace_link_with_projection`], sharing its
/// outcome contract:
///
/// - `(target, relation)` already present → [`LinkUpsertOutcome::Unchanged`],
///   no write.
/// - `(target, other_relation)` present → the FIRST such entry's relation
///   is rewritten in place → [`LinkUpsertOutcome::Replaced`]. A pre-existing
///   many-edge state is NOT collapsed — same as the projection layer.
/// - no edge to `target` → append `{target, relation}` →
///   [`LinkUpsertOutcome::Created`].
///
/// `target` is stored verbatim (canonical id, qualified `KEY@qualifier`, or
/// `mem-*` literal) and matched case-insensitively. Validation and the
/// self-link refusal mirror [`add_link_verbatim`].
pub async fn replace_link(
    artifact_path: &Path,
    target_id: &str,
    relation: &str,
) -> anyhow::Result<LinkUpsertOutcome> {
    crate::db::store::validate_link_target(target_id)?;

    let content = tokio::fs::read_to_string(artifact_path).await?;
    let (mut fm, body) = frontmatter::parse_frontmatter(&content)?;

    // Self-link guard (PROB-019 parity): the source is this file's own id.
    if let Some(source) = fm.get("id").and_then(|v| v.as_str())
        && source.eq_ignore_ascii_case(target_id)
    {
        anyhow::bail!("Self-link not allowed: {source} cannot link to itself");
    }

    let links = fm
        .entry("links".to_string())
        .or_insert_with(|| serde_yaml::Value::Sequence(Vec::new()));
    let serde_yaml::Value::Sequence(seq) = links else {
        return Err(ForgeplanError::Frontmatter("'links' field is not a sequence".into()).into());
    };

    let is_target = |entry: &serde_yaml::Value| -> bool {
        if let serde_yaml::Value::Mapping(map) = entry {
            matches!(
                map.get(serde_yaml::Value::String("target".into())),
                Some(serde_yaml::Value::String(t)) if t.eq_ignore_ascii_case(target_id)
            )
        } else {
            false
        }
    };
    let entry_relation = |entry: &serde_yaml::Value| -> Option<String> {
        if let serde_yaml::Value::Mapping(map) = entry {
            map.get(serde_yaml::Value::String("relation".into()))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        } else {
            None
        }
    };

    let outcome = if seq
        .iter()
        .any(|e| is_target(e) && entry_relation(e).as_deref() == Some(relation))
    {
        LinkUpsertOutcome::Unchanged
    } else if let Some(pos) = seq.iter().position(is_target) {
        let old_relation = entry_relation(&seq[pos]).unwrap_or_default();
        if let serde_yaml::Value::Mapping(map) = &mut seq[pos] {
            map.insert(
                serde_yaml::Value::String("relation".into()),
                serde_yaml::Value::String(relation.into()),
            );
        }
        LinkUpsertOutcome::Replaced { old_relation }
    } else {
        let mut entry = serde_yaml::Mapping::new();
        entry.insert(
            serde_yaml::Value::String("target".into()),
            serde_yaml::Value::String(target_id.to_string()),
        );
        entry.insert(
            serde_yaml::Value::String("relation".into()),
            serde_yaml::Value::String(relation.into()),
        );
        seq.push(serde_yaml::Value::Mapping(entry));
        LinkUpsertOutcome::Created
    };

    if outcome == LinkUpsertOutcome::Unchanged {
        return Ok(outcome);
    }
    write_links_doc(artifact_path, &fm, &body, &content).await?;
    Ok(outcome)
}

/// Shared append over the `links:` frontmatter sequence. `target_id` is
/// stored as received — normalization is each public caller's choice.
/// `self_check` enables the `store.add_relation` self-link refusal against
/// the file's own `id:` field.
async fn add_link_inner(
    artifact_path: &Path,
    target_id: &str,
    relation: &str,
    self_check: bool,
) -> anyhow::Result<()> {
    let content = tokio::fs::read_to_string(artifact_path).await?;
    let (mut fm, body) = frontmatter::parse_frontmatter(&content)?;

    // Self-link guard (PROB-019 parity): the source is this file's own id.
    if self_check
        && let Some(source) = fm.get("id").and_then(|v| v.as_str())
        && source.eq_ignore_ascii_case(target_id)
    {
        anyhow::bail!("Self-link not allowed: {source} cannot link to itself");
    }

    // Get or create links array
    let links = fm
        .entry("links".to_string())
        .or_insert_with(|| serde_yaml::Value::Sequence(Vec::new()));

    if let serde_yaml::Value::Sequence(seq) = links {
        // Check for duplicates
        let already_exists = seq.iter().any(|entry| {
            if let serde_yaml::Value::Mapping(map) = entry {
                let t = map.get(serde_yaml::Value::String("target".into()));
                let r = map.get(serde_yaml::Value::String("relation".into()));
                matches!((t, r), (Some(serde_yaml::Value::String(t)), Some(serde_yaml::Value::String(r)))
                    if t.eq_ignore_ascii_case(target_id) && r == relation)
            } else {
                false
            }
        });

        if already_exists {
            return Err(ForgeplanError::LinkExists {
                from: fm
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
                    .to_string(),
                relation: relation.to_string(),
                to: target_id.to_string(),
            }
            .into());
        }

        let mut entry = serde_yaml::Mapping::new();
        entry.insert(
            serde_yaml::Value::String("target".into()),
            serde_yaml::Value::String(target_id.to_string()),
        );
        entry.insert(
            serde_yaml::Value::String("relation".into()),
            serde_yaml::Value::String(relation.into()),
        );
        seq.push(serde_yaml::Value::Mapping(entry));
    } else {
        return Err(ForgeplanError::Frontmatter("'links' field is not a sequence".into()).into());
    }

    let output = frontmatter::render_frontmatter(&fm, &body)?;
    tokio::fs::write(artifact_path, output).await?;
    Ok(())
}

/// List all links from an artifact's frontmatter.
/// Returns Vec<(target_id, relation)>.
pub fn list_links(fm: &Frontmatter) -> Vec<(String, String)> {
    let mut results = Vec::new();
    if let Some(serde_yaml::Value::Sequence(seq)) = fm.get("links") {
        for entry in seq {
            if let serde_yaml::Value::Mapping(map) = entry {
                let target = map
                    .get(serde_yaml::Value::String("target".into()))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let relation = map
                    .get(serde_yaml::Value::String("relation".into()))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !target.is_empty() {
                    results.push((target.to_string(), relation.to_string()));
                }
            }
        }
    }
    results
}

/// Valid link relation types.
///
/// Audit-r7 M3 (HIGH): added brownfield relations (Epic #287) to the
/// allowlist. Previously `forgeplan_link` enforced this list but
/// `scan-import` / direct file edit / graph render assumed file-sourced
/// `relation:` strings were trustworthy — opening a mermaid-injection
/// vector when a hand-edited link's relation contained `|`, `{`, `}`,
/// `\n`, etc. With brownfield relations now in the allowlist, the
/// uniform read-path validation closes that gap.
pub const VALID_RELATIONS: &[&str] = &[
    "informs",
    "based_on",
    "supersedes",
    "contradicts",
    "refines",
    "supports",
    // Audit-r7 M3: Epic #287 brownfield relations.
    "demonstrates",
    "covers",
    "triangulates",
    "references",
    "belongs_to",
];

/// Parse relation string, accepting both snake_case and kebab-case.
pub fn normalize_relation(input: &str) -> anyhow::Result<String> {
    let normalized = input.replace('-', "_").to_lowercase();
    if VALID_RELATIONS.contains(&normalized.as_str()) {
        Ok(normalized)
    } else {
        Err(ForgeplanError::InvalidRelation(input.to_string()).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // --- normalize_relation ---

    #[test]
    fn normalize_relation_valid_snake_case() {
        for &rel in VALID_RELATIONS {
            let result = normalize_relation(rel).unwrap();
            assert_eq!(result, rel);
        }
    }

    #[test]
    fn normalize_relation_kebab_to_snake() {
        assert_eq!(normalize_relation("based-on").unwrap(), "based_on");
        assert_eq!(normalize_relation("based-ON").unwrap(), "based_on");
    }

    #[test]
    fn normalize_relation_invalid_returns_error() {
        let err = normalize_relation("unknown-relation").unwrap_err();
        assert!(err.to_string().contains("unknown-relation"));
    }

    // --- list_links ---

    #[test]
    fn list_links_empty_frontmatter() {
        let fm = std::collections::BTreeMap::new();
        assert!(list_links(&fm).is_empty());
    }

    #[test]
    fn list_links_with_sequence() {
        let yaml = r#"
links:
  - target: PRD-001
    relation: informs
  - target: RFC-002
    relation: based_on
"#;
        let fm: crate::artifact::frontmatter::Frontmatter = serde_yaml::from_str(yaml).unwrap();
        let links = list_links(&fm);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0], ("PRD-001".to_string(), "informs".to_string()));
        assert_eq!(links[1], ("RFC-002".to_string(), "based_on".to_string()));
    }

    #[test]
    fn list_links_non_sequence_links_field() {
        let yaml = r#"links: "not-a-sequence""#;
        let fm: crate::artifact::frontmatter::Frontmatter = serde_yaml::from_str(yaml).unwrap();
        // Non-sequence links field should return empty vec
        assert!(list_links(&fm).is_empty());
    }

    // --- add_link ---

    fn make_artifact(dir: &std::path::Path, id: &str, extra_yaml: &str) -> std::path::PathBuf {
        let path = dir.join(format!("{}.md", id));
        let content = format!(
            "---\nid: {}\ntitle: Test\nkind: prd\nstatus: Draft\n{}---\n\nBody text.\n",
            id, extra_yaml
        );
        fs::write(&path, content).unwrap();
        path
    }

    #[tokio::test]
    async fn add_link_creates_link_entry() {
        let tmp = TempDir::new().unwrap();
        let path = make_artifact(tmp.path(), "PRD-001", "");
        add_link(&path, "RFC-001", "informs").await.unwrap();

        let content = fs::read_to_string(&path).unwrap();
        let (fm, _) = crate::artifact::frontmatter::parse_frontmatter(&content).unwrap();
        let links = list_links(&fm);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].0, "RFC-001");
        assert_eq!(links[0].1, "informs");
    }

    #[tokio::test]
    async fn add_link_writes_target_uppercase() {
        let tmp = TempDir::new().unwrap();
        let path = make_artifact(tmp.path(), "PRD-001", "");
        add_link(&path, "rfc-001", "informs").await.unwrap();

        let content = fs::read_to_string(&path).unwrap();
        let (fm, _) = crate::artifact::frontmatter::parse_frontmatter(&content).unwrap();
        let links = list_links(&fm);
        assert_eq!(links[0].0, "RFC-001");
    }

    #[tokio::test]
    async fn add_link_detects_case_insensitive_duplicate() {
        let tmp = TempDir::new().unwrap();
        let path = make_artifact(tmp.path(), "PRD-001", "");
        add_link(&path, "RFC-001", "informs").await.unwrap();
        // Adding duplicate with different case should fail
        let err = add_link(&path, "rfc-001", "informs").await.unwrap_err();
        assert!(err.to_string().contains("RFC-001") || err.to_string().contains("already"));
    }

    // --- add_link_verbatim ---

    #[tokio::test]
    async fn add_link_verbatim_stores_target_verbatim() {
        let tmp = TempDir::new().unwrap();
        let path = make_artifact(tmp.path(), "ADR-001", "");
        // Qualified ref: verbatim storage is the contract (no uppercasing).
        add_link_verbatim(&path, "ADR-001@src:ws-b", "based_on")
            .await
            .unwrap();
        let (fm, _) =
            crate::artifact::frontmatter::parse_frontmatter(&fs::read_to_string(&path).unwrap())
                .unwrap();
        let links = list_links(&fm);
        assert_eq!(links[0].0, "ADR-001@src:ws-b");
    }

    #[tokio::test]
    async fn add_link_verbatim_refuses_self_link() {
        let tmp = TempDir::new().unwrap();
        let path = make_artifact(tmp.path(), "PRD-001", "");
        let err = add_link_verbatim(&path, "prd-001", "informs")
            .await
            .unwrap_err();
        assert!(err.to_string().contains("Self-link"), "got: {err}");
    }

    #[tokio::test]
    async fn add_link_verbatim_allows_parallel_different_relation() {
        let tmp = TempDir::new().unwrap();
        let path = make_artifact(tmp.path(), "PRD-001", "");
        add_link_verbatim(&path, "RFC-001", "informs")
            .await
            .unwrap();
        // Same target, DIFFERENT relation stays legal (add_relation parity).
        add_link_verbatim(&path, "RFC-001", "based_on")
            .await
            .unwrap();
        // Same (target, relation) is a duplicate.
        assert!(
            add_link_verbatim(&path, "RFC-001", "informs")
                .await
                .is_err()
        );
    }

    // --- remove_link ---

    #[tokio::test]
    async fn remove_link_drops_matching_entry_case_insensitive_target() {
        let tmp = TempDir::new().unwrap();
        let path = make_artifact(tmp.path(), "ADR-001", "");
        add_link(&path, "PRD-001", "refines").await.unwrap();
        add_link(&path, "RFC-001", "informs").await.unwrap();

        let removed = remove_link(&path, "prd-001", "refines").await.unwrap();
        assert_eq!(removed, 1);

        let (fm, _) =
            crate::artifact::frontmatter::parse_frontmatter(&fs::read_to_string(&path).unwrap())
                .unwrap();
        let links = list_links(&fm);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].0, "RFC-001");
    }

    #[tokio::test]
    async fn remove_link_reports_zero_when_edge_missing() {
        let tmp = TempDir::new().unwrap();
        let path = make_artifact(tmp.path(), "ADR-001", "");
        assert_eq!(remove_link(&path, "PRD-001", "refines").await.unwrap(), 0);
    }

    #[tokio::test]
    async fn remove_link_matches_relation_exactly() {
        let tmp = TempDir::new().unwrap();
        let path = make_artifact(tmp.path(), "ADR-001", "");
        add_link(&path, "PRD-001", "refines").await.unwrap();
        // Wrong relation → nothing removed.
        assert_eq!(remove_link(&path, "PRD-001", "informs").await.unwrap(), 0);
    }

    // --- replace_link ---

    #[tokio::test]
    async fn replace_link_creates_when_no_edge_exists() {
        let tmp = TempDir::new().unwrap();
        let path = make_artifact(tmp.path(), "ADR-001", "");
        let outcome = replace_link(&path, "PRD-001", "refines").await.unwrap();
        assert_eq!(outcome, LinkUpsertOutcome::Created);
        let (fm, _) =
            crate::artifact::frontmatter::parse_frontmatter(&fs::read_to_string(&path).unwrap())
                .unwrap();
        assert_eq!(list_links(&fm)[0].1, "refines");
    }

    #[tokio::test]
    async fn replace_link_unchanged_when_same_edge_exists() {
        let tmp = TempDir::new().unwrap();
        let path = make_artifact(tmp.path(), "ADR-001", "");
        add_link(&path, "PRD-001", "refines").await.unwrap();
        let outcome = replace_link(&path, "prd-001", "refines").await.unwrap();
        assert_eq!(outcome, LinkUpsertOutcome::Unchanged);
    }

    #[tokio::test]
    async fn replace_link_rewrites_relation_on_existing_edge() {
        let tmp = TempDir::new().unwrap();
        let path = make_artifact(tmp.path(), "ADR-001", "");
        add_link(&path, "PRD-001", "informs").await.unwrap();
        let outcome = replace_link(&path, "PRD-001", "refines").await.unwrap();
        assert_eq!(
            outcome,
            LinkUpsertOutcome::Replaced {
                old_relation: "informs".to_string()
            }
        );
        let (fm, _) =
            crate::artifact::frontmatter::parse_frontmatter(&fs::read_to_string(&path).unwrap())
                .unwrap();
        let links = list_links(&fm);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].1, "refines");
    }

    #[tokio::test]
    async fn replace_link_refuses_self_link() {
        let tmp = TempDir::new().unwrap();
        let path = make_artifact(tmp.path(), "ADR-001", "");
        let err = replace_link(&path, "ADR-001", "refines").await.unwrap_err();
        assert!(err.to_string().contains("Self-link"), "got: {err}");
    }
}
