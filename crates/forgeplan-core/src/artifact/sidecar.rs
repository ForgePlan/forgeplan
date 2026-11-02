//! Sidecar addressing — the file name a per-document sidecar is stored under.
//!
//! A *sidecar* is a small per-document file that lives beside the workspace
//! and holds state about exactly one document: a claim, a phase marker, a
//! receipt. It is addressed by the document's **canonical key**, never by its
//! display alias.
//!
//! The reason is the alias itself. `PRD-001` is a name two documents may
//! both answer to — one in this project, one in another. A sidecar named
//! after the alias gives those two documents one row, so one agent's claim
//! silently blocks the other's work and one document's phase marker stands
//! for the other. The canonical key is unique, so a sidecar named after the
//! key belongs to exactly one document.
//!
//! # The file-name mapping
//!
//! A canonical key is not a file name. It may carry a source qualifier
//! (`forgeplan/plugins/x--PRD-074`), a percent sign, or any other byte a
//! path reads as structure. Dropping such a key into a path verbatim lets a
//! `/` create a directory and a `..` leave one.
//!
//! The mapping is:
//!
//! * ASCII letters, digits, `-`, `_` and `.` are kept verbatim;
//! * every other byte becomes `%` followed by two uppercase hex digits.
//!
//! So `/` → `%2F`, `@` → `%40`, `%` → `%25`, a space → `%20`.
//!
//! The mapping is injective — two different keys never produce the same file
//! name — and total: every key has exactly one file name. Two documents that
//! share a display alias therefore never share a sidecar, because their keys
//! differ.
//!
//! # Reading an old workspace
//!
//! A sidecar written before the key existed is named after the display alias,
//! and one written after the key existed but before the key was escaped is
//! named after the key itself. Both are still found when ownership is clear:
//! [`resolve_sidecar_path`] reads an alias-named file only after a complete
//! workspace scan proves the alias has a unique owner. Reads never migrate it.

use std::path::{Path, PathBuf};

/// The uppercase hexadecimal digits, indexed by nibble.
const HEX_UPPER: [char; 16] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F',
];

/// The file-name stem of a document's sidecar, derived from its canonical key.
///
/// The result is pure ASCII and holds no path separator, so it is always
/// exactly one path component. See the module documentation for the mapping.
pub fn sidecar_file_stem(canonical_key: &str) -> String {
    let mut out = String::with_capacity(canonical_key.len());
    for byte in canonical_key.as_bytes() {
        let c = *byte as char;
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
            out.push(c);
        } else {
            out.push('%');
            out.push(HEX_UPPER[usize::from(byte >> 4)]);
            out.push(HEX_UPPER[usize::from(byte & 0x0f)]);
        }
    }
    out
}

/// The path of a document's sidecar inside `dir`.
///
/// `extension` is the file extension without the dot (`yaml`).
pub fn sidecar_path(dir: &Path, canonical_key: &str, extension: &str) -> PathBuf {
    dir.join(format!("{}.{extension}", sidecar_file_stem(canonical_key)))
}

/// The file-name stems a pre-key sidecar may carry.
///
/// A sidecar written before the canonical key existed is named after the
/// document's display alias — the alias as it was written, and the alias
/// upper-cased (the claim store upper-cased what it was given). A sidecar
/// written after the key existed but before the key was escaped is named
/// after the key itself.
///
/// Only stems that are one safe path component are returned: a stem that
/// carried a separator would address a file outside the sidecar directory.
pub fn legacy_sidecar_file_stems(canonical_key: &str, display_alias: &str) -> Vec<String> {
    let mut stems: Vec<String> = Vec::new();
    for candidate in [
        display_alias.to_string(),
        display_alias.to_uppercase(),
        canonical_key.to_string(),
    ] {
        if is_safe_path_component(&candidate) && !stems.contains(&candidate) {
            stems.push(candidate);
        }
    }
    stems
}

/// The paths a pre-key sidecar may live at, in the order they are tried.
pub fn legacy_sidecar_paths(
    dir: &Path,
    canonical_key: &str,
    display_alias: &str,
    extension: &str,
) -> Vec<PathBuf> {
    legacy_sidecar_file_stems(canonical_key, display_alias)
        .into_iter()
        .map(|stem| dir.join(format!("{stem}.{extension}")))
        .collect()
}

/// Locate a sidecar without mutating the workspace.
///
/// Reads may discover a legacy file, but they must not migrate it: migration
/// needs the workspace mutation lock and a complete identity revalidation.
/// Alias-named files are deliberately not attributed here because an exact
/// key match does not prove that the alias has only one owner.
pub async fn resolve_sidecar_path(
    dir: &Path,
    canonical_key: &str,
    display_alias: &str,
    extension: &str,
) -> std::io::Result<PathBuf> {
    // A failed scan cannot prove ownership of an old alias or the uniqueness
    // of a key, including when a keyed sidecar already exists.
    let workspace = dir.parent().unwrap_or(dir);
    let candidates = crate::artifact::documents::workspace_candidates(workspace)
        .await
        .map_err(std::io::Error::other)?;
    use crate::artifact::canonical_key::{resolve_artifact, ResolveError};

    let key_resolution = resolve_artifact(canonical_key, &candidates);
    if let Err(error @ ResolveError::ConflictingCanonicalKey { .. }) = &key_resolution {
        return Err(std::io::Error::other(error.clone()));
    }
    let display_alias = if display_alias == canonical_key {
        candidates.iter().find(|candidate| candidate.canonical_key == canonical_key)
            .map_or(display_alias, |candidate| candidate.display_id.as_str())
    } else {
        display_alias
    };
    let keyed = sidecar_path(dir, canonical_key, extension);
    let keyed_exists = tokio::fs::try_exists(&keyed).await?;
    if keyed_exists {
        if let Err(error @ ResolveError::Ambiguous { .. }) = key_resolution {
            return Err(std::io::Error::other(error));
        }
        if !display_alias.eq_ignore_ascii_case(canonical_key) {
            return Ok(keyed);
        }
    }
    let owner = match resolve_artifact(display_alias, &candidates) {
        Ok(owner) => owner,
        Err(error @ ResolveError::Ambiguous { .. }) => {
            let mut entries = match tokio::fs::read_dir(dir).await {
                Ok(entries) => Some(entries),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => return Err(error),
            };
            while let Some(entry) = match &mut entries {
                Some(entries) => entries.next_entry().await?,
                None => None,
            } {
                let name = entry.file_name();
                let path = Path::new(&name);
                if path.extension().and_then(|value| value.to_str()) != Some(extension) {
                    continue;
                }
                let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
                    continue;
                };
                if let Err(ResolveError::Ambiguous { candidates: owners, .. }) =
                    resolve_artifact(stem, &candidates)
                {
                    if owners.iter().any(|owner| owner.canonical_key == canonical_key) {
                        return Err(std::io::Error::other(error));
                    }
                }
            }
            None
        }
        Err(ResolveError::InvalidSelector { .. })
            if !candidates.iter().any(|candidate| candidate.canonical_key == canonical_key) => None,
        Err(error) => return Err(std::io::Error::other(error)),
    };
    if keyed_exists {
        return Ok(keyed);
    }
    if owner.is_some_and(|owner| owner.identity.canonical_key == canonical_key) {
        for legacy in legacy_sidecar_paths(dir, canonical_key, display_alias, extension) {
            if legacy != keyed && tokio::fs::try_exists(&legacy).await? {
                return Ok(legacy);
            }
        }
    }
    Ok(keyed)
}

/// Whether a string is exactly one safe path component.
///
/// A component is safe when it is not empty, is not `.` or `..`, and carries
/// no path separator — a stem that carried one would address a file outside
/// the sidecar directory.
fn is_safe_path_component(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && !value.contains('/')
        && !value.contains('\\')
        && !value.contains('\0')
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn a_plain_key_is_its_own_stem() {
        assert_eq!(sidecar_file_stem("PRD-001"), "PRD-001");
        assert_eq!(
            sidecar_file_stem("prd-weekly-review-k0123456789abcdef0123456789abcdef"),
            "prd-weekly-review-k0123456789abcdef0123456789abcdef"
        );
    }

    #[test]
    fn a_separator_is_escaped_not_kept() {
        assert_eq!(
            sidecar_file_stem("forgeplan/plugins/x--PRD-074"),
            "forgeplan%2Fplugins%2Fx--PRD-074"
        );
        assert_eq!(sidecar_file_stem("a@b"), "a%40b");
        assert_eq!(sidecar_file_stem("100%"), "100%25");
        assert_eq!(sidecar_file_stem("a b"), "a%20b");
    }

    #[test]
    fn the_mapping_is_injective() {
        // A key that carries a separator and the key that spells the escape
        // land on two different names.
        assert_ne!(sidecar_file_stem("a/b"), sidecar_file_stem("a%2Fb"));
        assert_ne!(sidecar_file_stem("a/b"), sidecar_file_stem("a-b"));
    }

    #[test]
    fn a_stem_is_always_one_path_component() {
        for key in ["a/b", "..", "../escape", "a\\b", "%2F"] {
            let stem = sidecar_file_stem(key);
            assert!(!stem.contains('/'), "{stem} must not carry a separator");
            assert!(!stem.contains('\\'), "{stem} must not carry a separator");
        }
    }

    #[test]
    fn legacy_stems_name_the_pre_key_names() {
        let stems = legacy_sidecar_file_stems("prd-auth-system", "PRD-001");
        assert_eq!(stems, vec!["PRD-001", "prd-auth-system"]);
    }

    #[test]
    fn legacy_stems_skip_an_unsafe_component() {
        // A key that carries a separator cannot name a legacy file: the old
        // code would have written into a subdirectory.
        assert!(
            legacy_sidecar_file_stems("forgeplan/plugins/x", "PRD-001")
                .contains(&"PRD-001".to_string())
        );
        assert!(
            !legacy_sidecar_file_stems("forgeplan/plugins/x", "PRD-001")
                .contains(&"forgeplan/plugins/x".to_string())
        );
    }

    #[tokio::test]
    async fn a_read_uses_unique_alias_state_without_moving_it() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("claims");
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::create_dir_all(tmp.path().join("prds"))
            .await
            .unwrap();
        tokio::fs::write(
            tmp.path().join("prds/alpha.md"),
            "---\nid: PRD-001\nslug: prd-auth-system\ntitle: Alpha\nkind: prd\n---\n",
        )
        .await
        .unwrap();
        tokio::fs::write(dir.join("PRD-001.yaml"), "legacy")
            .await
            .unwrap();

        let path = resolve_sidecar_path(&dir, "prd-auth-system", "PRD-001", "yaml")
            .await
            .unwrap();
        assert_eq!(path, dir.join("PRD-001.yaml"));
        assert_eq!(tokio::fs::read_to_string(&path).await.unwrap(), "legacy");
        assert!(!sidecar_path(&dir, "prd-auth-system", "yaml").exists());
    }

    #[tokio::test]
    async fn a_shared_alias_sidecar_is_not_assigned_to_either_key() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("claims");
        let prds = tmp.path().join("prds");
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::create_dir_all(&prds).await.unwrap();
        for (filename, slug) in [
            ("alpha.md", "prd-alpha-k11111111111111111111111111111111"),
            ("beta.md", "prd-beta-k22222222222222222222222222222222"),
        ] {
            tokio::fs::write(
                prds.join(filename),
                format!("---\nid: PRD-001\nslug: {slug}\ntitle: {slug}\nkind: prd\n---\n"),
            )
            .await
            .unwrap();
        }
        let legacy = dir.join("PRD-001.yaml");
        tokio::fs::write(&legacy, "shared legacy state").await.unwrap();

        for key in [
            "prd-alpha-k11111111111111111111111111111111",
            "prd-beta-k22222222222222222222222222222222",
        ] {
            let error = resolve_sidecar_path(&dir, key, "PRD-001", "yaml")
                .await
                .unwrap_err();
            assert!(matches!(
                error.get_ref().and_then(|source| source.downcast_ref::<crate::artifact::canonical_key::ResolveError>()),
                Some(crate::artifact::canonical_key::ResolveError::Ambiguous { .. })
            ));
            assert!(!sidecar_path(&dir, key, "yaml").exists());
        }
        assert_eq!(tokio::fs::read_to_string(legacy).await.unwrap(), "shared legacy state");
    }

    #[tokio::test]
    async fn a_keyed_file_wins_over_a_pre_key_file() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("claims");
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("PRD-001.yaml"), "legacy")
            .await
            .unwrap();
        tokio::fs::write(dir.join("prd-auth-system.yaml"), "keyed")
            .await
            .unwrap();

        let path = resolve_sidecar_path(&dir, "prd-auth-system", "PRD-001", "yaml")
            .await
            .unwrap();
        assert_eq!(
            tokio::fs::read_to_string(&path).await.unwrap(),
            "keyed".to_string()
        );
        // The pre-key file is left alone: it is data, and the keyed file is
        // the one that counts.
        assert!(dir.join("PRD-001.yaml").exists());
    }

    #[tokio::test]
    async fn normalized_aliases_refuse_shared_phase_reads_and_writes() {
        use crate::artifact::canonical_key::ResolveError;
        use crate::phase::{store, Phase};

        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path().join(".forgeplan");
        let prds = workspace.join("prds");
        let state = workspace.join("state");
        tokio::fs::create_dir_all(&prds).await.unwrap();
        tokio::fs::create_dir_all(&state).await.unwrap();
        let docs = [
            ("alpha.md", "PRD-001", "prd-alpha-k11111111111111111111111111111111"),
            ("beta.md", "PRD-1", "prd-beta-k22222222222222222222222222222222"),
        ];
        for (file, id, key) in docs {
            tokio::fs::write(prds.join(file), format!(
                "---\nid: {id}\nslug: {key}\ntitle: {key}\nkind: prd\n---\n"
            )).await.unwrap();
        }
        let legacy = state.join("PRD-001.yaml");
        let bytes = b"artifact_id: PRD-001\ncurrent_phase: shape\n";
        tokio::fs::write(&legacy, bytes).await.unwrap();
        for (_, id, key) in docs {
            for error in [
                store::read_phase_for(&workspace, key, id).await.unwrap_err(),
                store::read_phase(&workspace, key).await.unwrap_err(),
                store::advance_phase_unchecked(&workspace, key, Phase::Code, None).await.unwrap_err(),
                store::advance_phase_for(&workspace, key, id, Phase::Code, None).await.unwrap_err(),
            ] {
                assert!(matches!(error.downcast_ref::<ResolveError>(),
                    Some(ResolveError::Ambiguous { candidates, .. }) if candidates.len() == 2
                ), "{error:#}");
            }
            assert!(!sidecar_path(&state, key, "yaml").exists());
        }
        assert_eq!(tokio::fs::read(&legacy).await.unwrap(), bytes);
        tokio::fs::remove_file(&legacy).await.unwrap();
        let alternate = state.join("PRD-0001.yaml");
        tokio::fs::write(&alternate, bytes).await.unwrap();
        for (_, _, key) in docs {
            for alias in ["PRD-0001", "PRD-001", "PRD-1"] {
                let error = store::read_phase_for(&workspace, key, alias).await.unwrap_err();
                assert!(matches!(error.downcast_ref::<ResolveError>(),
                    Some(ResolveError::Ambiguous { candidates, .. }) if candidates.len() == 2
                ), "{error:#}");
                let error = store::advance_phase_for(&workspace, key, alias, Phase::Code, None).await.unwrap_err();
                assert!(matches!(error.downcast_ref::<ResolveError>(),
                    Some(ResolveError::Ambiguous { candidates, .. }) if candidates.len() == 2
                ), "{error:#}");
            }
            assert!(!sidecar_path(&state, key, "yaml").exists());
        }
        assert_eq!(tokio::fs::read(&alternate).await.unwrap(), bytes);
        tokio::fs::write(&legacy, bytes).await.unwrap();
        let keyed = sidecar_path(&state, docs[0].2, "yaml");
        tokio::fs::write(&keyed, b"keyed").await.unwrap();
        assert_eq!(resolve_sidecar_path(&state, docs[0].2, docs[0].1, "yaml").await.unwrap(), keyed);
        assert_eq!(tokio::fs::read(&legacy).await.unwrap(), bytes);
    }

    #[tokio::test]
    async fn phase_preserves_conflicting_canonical_key_error() {
        use crate::artifact::canonical_key::ResolveError;
        use crate::phase::{store, Phase};

        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path().join(".forgeplan");
        let prds = workspace.join("prds");
        let state = workspace.join("state");
        tokio::fs::create_dir_all(&prds).await.unwrap();
        tokio::fs::create_dir_all(&state).await.unwrap();
        let key = "prd-alpha-k11111111111111111111111111111111";
        for (file, id) in [("alpha.md", "PRD-001"), ("beta.md", "PRD-002")] {
            tokio::fs::write(prds.join(file), format!(
                "---\nid: {id}\nslug: {key}\ntitle: {id}\nkind: prd\n---\n"
            )).await.unwrap();
        }
        let keyed = sidecar_path(&state, key, "yaml");
        let bytes = b"unchanged";
        tokio::fs::write(&keyed, bytes).await.unwrap();
        for error in [
            store::read_phase_for(&workspace, key, "PRD-001").await.unwrap_err(),
            store::advance_phase_for(&workspace, key, "PRD-001", Phase::Code, None).await.unwrap_err(),
        ] {
            assert!(matches!(error.downcast_ref::<ResolveError>(),
                Some(ResolveError::ConflictingCanonicalKey { candidates, .. }) if candidates.len() == 2
            ), "{error:#}");
        }
        assert_eq!(tokio::fs::read(&keyed).await.unwrap(), bytes);
    }

    #[tokio::test]
    async fn a_missing_sidecar_resolves_to_its_keyed_name() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("claims");
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let path = resolve_sidecar_path(&dir, "prd-auth-system", "PRD-001", "yaml")
            .await
            .unwrap();
        assert_eq!(path, sidecar_path(&dir, "prd-auth-system", "yaml"));
    }
}
