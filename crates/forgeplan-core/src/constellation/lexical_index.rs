//! The trigram side-index: Microsoft's `tgrep-core` bound to a published
//! generation.
//!
//! # What it is for
//!
//! `search_pinned_lexical` on its own scans every row of the pinned
//! artifacts table. On a small workspace that is fine; on a large one it is
//! a full-table body read per query. A trigram index answers the same
//! question by consulting postings — and, because the corpus is the
//! markdown sources themselves, it extends the channel to regular
//! expressions and to content the artifact table does not carry.
//!
//! # The binding that keeps it honest
//!
//! A trigram index built from disk is a view of *files*, while the
//! Constellation contract answers from a *pinned generation*. The two agree
//! only if the index was built from exactly the bytes the generation
//! pinned. So this module binds an index to a generation the same way the
//! publish path validates itself: build, then **re-hash every source file
//! and compare against the captured `source_digest`s**. A single drifted
//! file means the index is written as stale — never tagged with a
//! generation it does not faithfully represent.
//!
//! The sidecar `.forgeplan/tgrep/generation.json` records
//! `(generation_id, source_set_digest)`; readers accept the index only when
//! both equal the manifest's values. A tgrep index without a matching
//! sidecar is just files on disk — the caller falls back to the pinned
//! scan, which is slow and never wrong.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::ConstellationError;
use super::capture::SourceEntry;

/// Directory under `.forgeplan/` that holds the trigram index.
pub const LEXICAL_INDEX_DIR: &str = "tgrep";

/// Sidecar binding the index to a generation.
const SIDECAR: &str = "generation.json";

/// What the sidecar records: which generation this index was verified
/// against, and the source-set digest that generation pinned.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LexicalTag {
    generation_id: String,
    source_set_digest: String,
    built_at: String,
}

/// The state of the trigram index relative to a manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexicalBinding {
    /// The index was built from the bytes this generation pinned.
    Bound,
    /// An index exists but was bound to a different generation or digest —
    /// it answers about files that are not what the manifest claims.
    Stale,
    /// No index or no readable sidecar.
    Missing,
}

/// What one rebuild produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexicalRebind {
    /// Built, verified byte-for-byte against the captured digests, tagged.
    Bound { files_indexed: usize },
    /// Built, but at least one source file changed between capture and the
    /// re-hash — the index is left untagged so reads fall back.
    Drifted { key: String },
    /// No source files to index.
    Empty,
}

pub fn index_dir(workspace: &Path) -> PathBuf {
    workspace.join(LEXICAL_INDEX_DIR)
}

fn tag_path(workspace: &Path) -> PathBuf {
    index_dir(workspace).join(SIDECAR)
}

/// Rebuild the trigram index from the current source files, verify it
/// against `entries` (the exact digests the generation pinned), and tag it.
///
/// Non-fatal by design: a failure returns `Err` for the caller to report,
/// but the pinned scan is always available, so a broken lexical index is a
/// performance loss, never a wrong answer.
pub fn rebuild_for_generation(
    workspace: &Path,
    generation_id: &str,
    source_set_digest: &str,
    entries: &[SourceEntry],
) -> Result<LexicalRebind, ConstellationError> {
    if entries.is_empty() {
        return Ok(LexicalRebind::Empty);
    }
    let index = index_dir(workspace);
    std::fs::create_dir_all(&index).map_err(|e| ConstellationError::Io {
        detail: format!("cannot create {}: {e}", index.display()),
    })?;

    // Invalidate the old binding before modifying the index. A reader that
    // checked the old tag must check again after the query before using rows.
    let invalid_tag = LexicalTag {
        generation_id: String::new(),
        source_set_digest: String::new(),
        built_at: super::rfc3339_now(),
    };
    let tag_file = tag_path(workspace);
    let tmp = tag_file.with_extension("json.tmp");
    std::fs::write(
        &tmp,
        serde_json::to_vec(&invalid_tag).map_err(|e| ConstellationError::Io {
            detail: format!("cannot encode invalidated lexical tag: {e}"),
        })?,
    )
    .map_err(|e| ConstellationError::Io {
        detail: format!("cannot invalidate {}: {e}", tag_file.display()),
    })?;
    std::fs::rename(&tmp, &tag_file).map_err(|e| ConstellationError::Io {
        detail: format!("cannot install invalidated tag {}: {e}", tag_file.display()),
    })?;

    // Index exactly the files the published capture claims. Walking the whole
    // workspace also includes config and scratch files, whose matches could
    // exhaust a bounded shortlist before a real artifact is reached.
    let files: Vec<PathBuf> = entries
        .iter()
        .map(|entry| workspace.join(&entry.relative_path))
        .collect();
    let outcome = tgrep_core::builder::build_index_for_files(
        workspace,
        &index,
        &files,
        tgrep_core::builder::DEFAULT_INDEX_BUFFER_BYTES,
    )
    .map_err(|e| ConstellationError::Io {
        detail: format!("trigram index build failed: {e}"),
    })?;
    if outcome.indexed != entries.len() {
        return Ok(LexicalRebind::Drifted {
            key: outcome
                .unreadable
                .first()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "incomplete lexical coverage".to_string()),
        });
    }

    // The binding check: every source file re-hashed must equal the digest
    // the capture recorded. One drifted file means the index answers about
    // bytes this generation did not pin — write no tag and report it.
    for entry in entries {
        let path = workspace.join(&entry.relative_path);
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(_) => {
                return Ok(LexicalRebind::Drifted {
                    key: entry.canonical_key.clone(),
                });
            }
        };
        if super::sha256_hex(&bytes) != entry.source_digest {
            return Ok(LexicalRebind::Drifted {
                key: entry.canonical_key.clone(),
            });
        }
    }

    let tag = LexicalTag {
        generation_id: generation_id.to_string(),
        source_set_digest: source_set_digest.to_string(),
        built_at: super::rfc3339_now(),
    };
    let json = serde_json::to_string_pretty(&tag).map_err(|e| ConstellationError::Io {
        detail: format!("cannot serialise the lexical tag: {e}"),
    })?;
    let tmp = tag_file.with_extension("json.tmp");
    std::fs::write(&tmp, &json).map_err(|e| ConstellationError::Io {
        detail: format!("cannot write {}: {e}", tmp.display()),
    })?;
    // Temp + rename: a reader either sees the old tag or the new one, never
    // a torn JSON half-written mid-build.
    std::fs::rename(&tmp, &tag_file).map_err(|e| ConstellationError::Io {
        detail: format!("cannot publish {}: {e}", tag_file.display()),
    })?;

    Ok(LexicalRebind::Bound {
        files_indexed: outcome.indexed,
    })
}

/// Whether the trigram index faithfully represents `manifest`'s generation.
pub fn binding_for(
    workspace: &Path,
    manifest: &super::manifest::GenerationManifest,
) -> LexicalBinding {
    let Ok(json) = std::fs::read_to_string(tag_path(workspace)) else {
        return LexicalBinding::Missing;
    };
    let Ok(tag) = serde_json::from_str::<LexicalTag>(&json) else {
        return LexicalBinding::Missing;
    };
    if tag.generation_id == manifest.generation_id
        && tag.source_set_digest == manifest.source_set_digest
    {
        LexicalBinding::Bound
    } else {
        LexicalBinding::Stale
    }
}

/// The set of `relative_path`s matching `query` through the bound trigram
/// index. Returns `Ok(None)` when the index is missing, stale, or the query
/// cannot be planned — the caller then falls back to the pinned scan.
///
/// Only called once `binding_for` has returned `Bound`; rechecking inside
/// would read the manifest twice and let the two disagree.
pub fn query_paths(
    workspace: &Path,
    query: &str,
    limit: usize,
) -> Result<Option<Vec<String>>, ConstellationError> {
    let index = index_dir(workspace);
    // The public search is a literal substring search. Regex planning would
    // interpret `.` and `|` as operators and narrow on the wrong expression.
    let plan = tgrep_core::query::build_literal_plan(query, true);
    if plan.is_match_all() {
        return Ok(None);
    }
    let mut reader = match tgrep_core::reader::IndexReader::open(&index) {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };

    let file_ids =
        tgrep_core::query::execute_plan(&plan, &|trigram| reader.lookup_trigram(trigram));

    let mut paths = Vec::with_capacity(file_ids.len().min(limit));
    for file_id in file_ids {
        if let Some(path) = reader.file_path(file_id) {
            // tgrep stores paths relative to the build root (the `.forgeplan`
            // dir) — the same shape `SourceEntry::relative_path` carries.
            paths.push(path.to_string());
            if paths.len() >= limit {
                break;
            }
        }
    }
    reader.close();
    Ok(Some(paths))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a real trigram index over a temp corpus and query it — the
    /// seam that failed in production must first be proven here.
    #[test]
    fn query_paths_finds_indexed_file() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = tmp.path();
        std::fs::create_dir_all(ws.join("specs")).unwrap();
        std::fs::write(
            ws.join("specs/SPEC-001-cache.md"),
            "---\nid: SPEC-001\n---\n# cache invalidation\n",
        )
        .unwrap();
        std::fs::create_dir_all(ws.join("adrs")).unwrap();
        std::fs::write(
            ws.join("adrs/ADR-001-other.md"),
            "---\nid: ADR-001\n---\n# x\n",
        )
        .unwrap();

        let index = index_dir(ws);
        std::fs::create_dir_all(&index).unwrap();
        tgrep_core::builder::build_index_with_options(
            ws,
            Some(&index),
            &tgrep_core::builder::BuildOptions {
                include_hidden: true,
                no_require_git: true,
                exclude_dirs: vec!["tgrep".to_string(), ".git".to_string()],
                ..Default::default()
            },
        )
        .unwrap();

        let paths = query_paths(ws, "cache", 10).unwrap();
        assert_eq!(paths, Some(vec!["specs/SPEC-001-cache.md".to_string()]));

        // A nonexistent term finds nothing, and a 1-char pattern is
        // unplannable → None (caller falls back to the pinned scan).
        let none = query_paths(ws, "zzznotfound", 10).unwrap();
        assert_eq!(none, Some(vec![]));
        // A search term is literal. Regex-planning `cache.` would otherwise
        // shortlist `cache invalidation` even though it has no dot.
        let punctuation = query_paths(ws, "cache.", 10).unwrap();
        assert!(
            !punctuation
                .unwrap_or_default()
                .contains(&"specs/SPEC-001-cache.md".to_string())
        );
    }
}
