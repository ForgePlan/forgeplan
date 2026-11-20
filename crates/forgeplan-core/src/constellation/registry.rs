//! Stage 2 — the constellation registry: which named sets of `.forgeplan`
//! sources exist, and which one reads go through by default.
//!
//! The registry lives at `~/.forgeplan/constellations.yaml` — the same
//! per-user home that holds `models/` — so every ForgePlan instance on the
//! machine, from any working directory, sees one answer to "what is the
//! active constellation". Nothing about it is project-local.
//!
//! Writes are atomic: the file is serialized to a sibling tempfile, fsync'd,
//! and renamed over the previous one. A crash mid-write leaves either the
//! old registry or the new one, never a torn half — `doctor` exists partly
//! to prove that property stayed true.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::ConstellationError;

/// `formatVersion` of the registry file. Bump on incompatible layout.
pub const REGISTRY_FORMAT_VERSION: u32 = 1;

/// The registry filename under `~/.forgeplan/`.
pub const REGISTRY_FILENAME: &str = "constellations.yaml";

/// What a source's unavailability means to a query — the spec's
/// required/optional contract.
///
/// `Required` is the default: an operator who registers a source without
/// marking it optional is saying the answer is not complete without it,
/// so a strict query over a dead required source fails with
/// [`ConstellationError::SourceUnavailable`] instead of returning a
/// partial envelope that would read as complete. An `Optional` source
/// degrades honestly: its failure lands in `failures` and flips
/// `partial`, exactly as every source behaved before roles existed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceRole {
    /// A strict query fails outright when this source cannot answer.
    #[default]
    Required,
    /// A failure here produces `partial: true` plus a `failures` entry —
    /// the query still answers from the sources that did respond.
    Optional,
}

impl SourceRole {
    /// The wire/CLI label — matches the serde spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Required => "required",
            Self::Optional => "optional",
        }
    }
}

/// One federated source as recorded: a canonical `.forgeplan` directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistrySource {
    /// Canonical absolute path to the `.forgeplan` directory.
    pub path: String,
    /// When the source was added (RFC 3339). Informational — identity is
    /// the path, not the timestamp.
    pub added_at: String,
    /// Required or optional under the strict-query contract. Defaults to
    /// `required` so a registry written before roles existed keeps its
    /// meaning: every declared source was implicitly load-bearing.
    #[serde(default)]
    pub role: SourceRole,
}

/// One named constellation — a set of sources reads fan out over.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstellationDef {
    pub created_at: String,
    /// Sources in the order they were added — search hits interleave in
    /// this order, so it is part of the contract, not decoration.
    #[serde(default)]
    pub sources: Vec<RegistrySource>,
}

/// The whole registry file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstellationRegistry {
    #[serde(default = "default_version")]
    pub version: u32,
    /// The constellation reads default to. `None` is legal and means
    /// "registry exists, nothing selected" — not the same as a missing file.
    #[serde(default)]
    pub current: Option<String>,
    #[serde(default)]
    pub constellations: BTreeMap<String, ConstellationDef>,
}

fn default_version() -> u32 {
    REGISTRY_FORMAT_VERSION
}

/// Where the registry lives for a given user home. Parameterized so tests
/// and future `FORGEPLAN_HOME`-style overrides never touch real `~/`.
pub fn registry_path(home: &Path) -> PathBuf {
    home.join(".forgeplan").join(REGISTRY_FILENAME)
}

/// The registry path for the real user home.
pub fn default_registry_path() -> Result<PathBuf, ConstellationError> {
    let home = dirs::home_dir().ok_or_else(|| ConstellationError::Io {
        detail: "cannot locate the user home directory for the registry".to_string(),
    })?;
    Ok(registry_path(&home))
}

/// Load the registry, tolerating "not there yet" as an empty one.
///
/// A file that exists but does not parse is an error — silently treating a
/// corrupt registry as empty would let the next save discard someone's
/// constellation set without a word.
pub fn load(path: &Path) -> Result<ConstellationRegistry, ConstellationError> {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ConstellationRegistry {
                version: REGISTRY_FORMAT_VERSION,
                ..Default::default()
            });
        }
        Err(e) => {
            return Err(ConstellationError::Io {
                detail: format!("cannot read {}: {e}", path.display()),
            });
        }
    };
    serde_yaml::from_slice(&bytes).map_err(|e| ConstellationError::Io {
        detail: format!(
            "{} is not a valid constellation registry: {e}",
            path.display()
        ),
    })
}

/// Write the registry atomically: tempfile → fsync → rename → dir fsync.
///
/// The tempfile sits in the same directory so the rename stays on one
/// filesystem; a rename across filesystems would degrade to a copy and
/// lose the atomicity this function exists for.
pub fn save(path: &Path, registry: &ConstellationRegistry) -> Result<(), ConstellationError> {
    let parent = path.parent().ok_or_else(|| ConstellationError::Io {
        detail: format!("{} has no parent directory", path.display()),
    })?;
    std::fs::create_dir_all(parent).map_err(|e| ConstellationError::Io {
        detail: format!("cannot create {}: {e}", parent.display()),
    })?;
    let body = serde_yaml::to_string(registry).map_err(|e| ConstellationError::Io {
        detail: format!("cannot serialize the registry: {e}"),
    })?;
    let tmp = parent.join(format!(".{REGISTRY_FILENAME}.tmp-{}", std::process::id()));
    let write_result = (|| -> std::io::Result<()> {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(body.as_bytes())?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, path)?;
        // The rename itself is only durable once the directory is flushed.
        std::fs::File::open(parent)?.sync_all()?;
        Ok(())
    })();
    write_result.map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        ConstellationError::Io {
            detail: format!("cannot write {} atomically: {e}", path.display()),
        }
    })
}

// -- Domain operations ---------------------------------------------------------

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Create a named constellation. Re-creating an existing name is an error —
/// a silent overwrite would drop its source list.
pub fn create(registry: &mut ConstellationRegistry, name: &str) -> Result<(), ConstellationError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(ConstellationError::Io {
            detail: "a constellation name cannot be empty".to_string(),
        });
    }
    if registry.constellations.contains_key(name) {
        return Err(ConstellationError::Io {
            detail: format!("constellation {name:?} already exists"),
        });
    }
    registry.constellations.insert(
        name.to_string(),
        ConstellationDef {
            created_at: now_rfc3339(),
            sources: Vec::new(),
        },
    );
    // The first constellation created becomes current — a user who just
    // made their only constellation should not have to `use` it too.
    if registry.current.is_none() {
        registry.current = Some(name.to_string());
    }
    Ok(())
}

/// Select the active constellation.
pub fn use_constellation(
    registry: &mut ConstellationRegistry,
    name: &str,
) -> Result<(), ConstellationError> {
    if !registry.constellations.contains_key(name) {
        return Err(ConstellationError::Io {
            detail: format!("no constellation named {name:?}"),
        });
    }
    registry.current = Some(name.to_string());
    Ok(())
}

/// Canonicalize a would-be source root under the rules every registration
/// path enforces.
///
/// The path must resolve to a real `.forgeplan` workspace (artifact dirs
/// present) and must not be a symlink — the same confinement rule the
/// reader applies, enforced at registration time so a bad entry fails loud
/// at `add`/`rebind`, not quietly at every later read. Returns the
/// canonical spelling every stored path carries.
fn canonical_source_root(spelled_path: &Path) -> Result<String, ConstellationError> {
    let meta = std::fs::symlink_metadata(spelled_path).map_err(|e| ConstellationError::Io {
        detail: format!("cannot stat {}: {e}", spelled_path.display()),
    })?;
    if meta.file_type().is_symlink() {
        return Err(ConstellationError::Io {
            detail: format!(
                "{} is a symlink; source roots must be canonical",
                spelled_path.display()
            ),
        });
    }
    let canonical = spelled_path
        .canonicalize()
        .map_err(|e| ConstellationError::Io {
            detail: format!("cannot canonicalize {}: {e}", spelled_path.display()),
        })?;
    if !crate::workspace::ARTIFACT_DIRS
        .iter()
        .any(|d| canonical.join(d).is_dir())
    {
        return Err(ConstellationError::Io {
            detail: format!(
                "{} does not look like a .forgeplan workspace (no artifact directories)",
                canonical.display()
            ),
        });
    }
    Ok(canonical.display().to_string())
}

/// Validate and add a source to a constellation.
///
/// The path must resolve to a real `.forgeplan` workspace (artifact dirs
/// present) and must not be a symlink — the same confinement rule the
/// reader applies, enforced at registration time so a bad entry fails loud
/// at `add`, not quietly at every later read. Adding the same canonical
/// path twice is an error; aliases of one workspace would double-count
/// every hit.
///
/// `role` is the source's standing under the strict-query contract —
/// [`SourceRole::Required`] makes its unavailability a query error,
/// [`SourceRole::Optional`] makes it a `failures` entry on a partial
/// answer.
pub fn add_source(
    registry: &mut ConstellationRegistry,
    constellation: &str,
    spelled_path: &Path,
    role: SourceRole,
) -> Result<String, ConstellationError> {
    let def = registry
        .constellations
        .get_mut(constellation)
        .ok_or_else(|| ConstellationError::Io {
            detail: format!("no constellation named {constellation:?}"),
        })?;
    let canonical_string = canonical_source_root(spelled_path)?;
    if def.sources.iter().any(|s| s.path == canonical_string) {
        return Err(ConstellationError::Io {
            detail: format!("{canonical_string} is already a source of {constellation:?}"),
        });
    }
    def.sources.push(RegistrySource {
        path: canonical_string.clone(),
        added_at: now_rfc3339(),
        role,
    });
    Ok(canonical_string)
}

/// Remove a source by its spelled path (canonicalized before matching).
pub fn remove_source(
    registry: &mut ConstellationRegistry,
    constellation: &str,
    spelled_path: &Path,
) -> Result<String, ConstellationError> {
    let def = registry
        .constellations
        .get_mut(constellation)
        .ok_or_else(|| ConstellationError::Io {
            detail: format!("no constellation named {constellation:?}"),
        })?;
    let canonical = spelled_path
        .canonicalize()
        .unwrap_or_else(|_| spelled_path.to_path_buf())
        .display()
        .to_string();
    let before = def.sources.len();
    def.sources.retain(|s| s.path != canonical);
    if def.sources.len() == before {
        return Err(ConstellationError::Io {
            detail: format!("{canonical} is not a source of {constellation:?}"),
        });
    }
    Ok(canonical)
}

/// What [`rebind_source`] changed: the identity the registry dropped and
/// the one it now points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebindOutcome {
    /// The canonical path the source had before the rebind — the moved
    /// checkout's old home. It typically no longer exists; that is the
    /// normal case, not an error.
    pub old_path: String,
    /// The canonical path the source has now.
    pub new_path: String,
    /// `src:` id derived from `old_path`. Every `KEY@src:` reference held
    /// against the old location stops resolving — a moved checkout is a new
    /// source identity, by the same path-derived rule that minted this id.
    pub old_source_id: String,
    /// `src:` id derived from `new_path`.
    pub new_source_id: String,
}

/// The `src:` identity a *stored* path reports.
///
/// Stored paths are canonical by construction, so the id is a pure function
/// of the string — no I/O. That is exactly why a moved source, whose whole
/// reason for a rebind is that its path is dead, still answers to its old
/// id: [`super::federation::spec_source_id`] resolves the path live and so
/// cannot serve this case.
fn stored_source_id(stored_path: &str) -> String {
    super::federation::source_id_for(Path::new(stored_path))
}

/// Find the one source `selector` names inside `def`.
///
/// A selector is a fragment of the stored canonical path, or the `src:` id
/// (whole or a prefix of it) — the same two readings the `@qualifier` match
/// in `get` accepts, with one deliberate difference: the id is computed
/// from the *stored* path rather than a live resolution. A source that
/// needs rebinding is usually dead on disk, and dead is when the id match
/// matters most.
///
/// Zero matches is a miss that names what is registered — never a silent
/// pick. More than one is an ambiguity the caller resolves with a longer
/// fragment; picking the first could rebind the wrong checkout.
fn find_source(
    def: &ConstellationDef,
    constellation: &str,
    selector: &str,
) -> Result<usize, ConstellationError> {
    let selector = selector.trim();
    if selector.is_empty() {
        return Err(ConstellationError::Io {
            detail: "an empty selector names every source and none".to_string(),
        });
    }
    let matches: Vec<usize> = def
        .sources
        .iter()
        .enumerate()
        .filter(|(_, s)| {
            s.path.contains(selector) || stored_source_id(&s.path).starts_with(selector)
        })
        .map(|(index, _)| index)
        .collect();
    let describe = |index: usize| {
        let s = &def.sources[index];
        format!("{} ({})", s.path, stored_source_id(&s.path))
    };
    match matches.as_slice() {
        [] => Err(ConstellationError::Io {
            detail: format!(
                "selector {selector:?} matches no source of {constellation:?}. Registered:\n  {}",
                (0..def.sources.len())
                    .map(describe)
                    .collect::<Vec<_>>()
                    .join("\n  ")
            ),
        }),
        [one] => Ok(*one),
        many => Err(ConstellationError::Io {
            detail: format!(
                "selector {selector:?} is ambiguous in {constellation:?} — it matches {} sources; \
                 give a longer fragment:\n  {}",
                many.len(),
                many.iter()
                    .map(|&i| describe(i))
                    .collect::<Vec<_>>()
                    .join("\n  ")
            ),
        }),
    }
}

/// Rebind a source to a new path — the moved-checkout verb.
///
/// Source identity is path-derived, so a checkout that moved is a new
/// source: this rewrites the one registry field that carries that identity
/// and leaves `added_at` alone — it records when the workspace first
/// joined the constellation, which stays true across the move.
///
/// `selector` is resolved by [`find_source`]; `new_path` is validated under
/// exactly the rules [`add_source`] applies (no symlink, canonicalized,
/// forgeplan-shaped). A `new_path` already claimed by a *different* source
/// is refused; rebinding to the path the source already holds is allowed —
/// it is how a caller adopts a foreign index's marker without moving
/// anything.
///
/// The registry is mutated in memory; persisting it is the caller's
/// `save`, exactly as with [`add_source`] — a caller that refuses the
/// target afterwards leaves the file untouched.
pub fn rebind_source(
    registry: &mut ConstellationRegistry,
    constellation: &str,
    selector: &str,
    new_path: &Path,
) -> Result<RebindOutcome, ConstellationError> {
    let def = registry
        .constellations
        .get_mut(constellation)
        .ok_or_else(|| ConstellationError::Io {
            detail: format!("no constellation named {constellation:?}"),
        })?;
    let index = find_source(def, constellation, selector)?;
    let canonical = canonical_source_root(new_path)?;
    if def
        .sources
        .iter()
        .enumerate()
        .any(|(j, s)| j != index && s.path == canonical)
    {
        return Err(ConstellationError::Io {
            detail: format!("{canonical} is already a source of {constellation:?}"),
        });
    }
    let old_path = std::mem::replace(&mut def.sources[index].path, canonical.clone());
    Ok(RebindOutcome {
        old_source_id: stored_source_id(&old_path),
        new_source_id: stored_source_id(&canonical),
        old_path,
        new_path: canonical,
    })
}

/// The current constellation's sources as reader input. `None` when no
/// constellation is selected — callers report that as its own diagnostic.
pub fn current_sources(
    registry: &ConstellationRegistry,
) -> Option<Vec<super::federation::SourceSpec>> {
    let name = registry.current.as_deref()?;
    named_sources(registry, name)
}

/// One named constellation's sources as reader input — the same
/// [`SourceSpec`]s [`current_sources`] builds, for a constellation that
/// is not necessarily the active one (the MCP read profile opens scopes
/// by name). `None` when the name is not registered.
pub fn named_sources(
    registry: &ConstellationRegistry,
    name: &str,
) -> Option<Vec<super::federation::SourceSpec>> {
    registry.constellations.get(name).map(|def| {
        def.sources
            .iter()
            .map(|s| super::federation::SourceSpec::new(PathBuf::from(&s.path)).with_role(s.role))
            .collect()
    })
}

// -- Doctor ---------------------------------------------------------------------

/// Per-source health row for `doctor`.
#[derive(Debug)]
pub struct SourceHealth {
    pub path: String,
    /// The registered role — a broken `required` source is a query
    /// failure waiting to happen, a broken `optional` one is a partial
    /// answer. `doctor` prints it either way.
    pub role: SourceRole,
    /// `true` when the path resolves to a readable `.forgeplan` workspace.
    pub readable: bool,
    /// `true` when a published Constellation index exists under `lance/`.
    pub indexed: bool,
    pub detail: Option<String>,
}

/// Check every source of every constellation — the answer "which of my
/// registered sources would actually answer right now".
pub fn doctor(registry: &ConstellationRegistry) -> Vec<(String, SourceHealth)> {
    let mut rows = Vec::new();
    for (name, def) in &registry.constellations {
        for source in &def.sources {
            let path = Path::new(&source.path);
            let health = match std::fs::symlink_metadata(path) {
                Err(e) => SourceHealth {
                    path: source.path.clone(),
                    role: source.role,
                    readable: false,
                    indexed: false,
                    detail: Some(format!("cannot stat: {e}")),
                },
                Ok(meta) if meta.file_type().is_symlink() => SourceHealth {
                    path: source.path.clone(),
                    role: source.role,
                    readable: false,
                    indexed: false,
                    detail: Some("the stored path is now a symlink".into()),
                },
                Ok(_) => {
                    let has_dirs = crate::workspace::ARTIFACT_DIRS
                        .iter()
                        .any(|d| path.join(d).is_dir());
                    // `lance/` alone is not "indexed" — `init` creates it
                    // empty. A published CURRENT pointer is what makes an
                    // index real.
                    let indexed = path
                        .join("lance")
                        .join(super::storage::CURRENT_FILE_NAME)
                        .is_file();
                    SourceHealth {
                        path: source.path.clone(),
                        role: source.role,
                        readable: has_dirs,
                        indexed,
                        detail: if has_dirs {
                            None
                        } else {
                            Some("no artifact directories found".into())
                        },
                    }
                }
            };
            rows.push((name.clone(), health));
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::ARTIFACT_DIRS;
    use tempfile::TempDir;

    fn workspace() -> TempDir {
        let tmp = TempDir::new().unwrap();
        for dir in ARTIFACT_DIRS {
            std::fs::create_dir_all(tmp.path().join(dir)).unwrap();
        }
        tmp
    }

    #[test]
    fn missing_registry_file_loads_as_empty() {
        let tmp = TempDir::new().unwrap();
        let registry = load(&registry_path(tmp.path())).unwrap();
        assert!(registry.constellations.is_empty());
        assert_eq!(registry.current, None);
    }

    #[test]
    fn a_saved_registry_roundtrips_atomically() {
        let tmp = TempDir::new().unwrap();
        let path = registry_path(tmp.path());
        let mut registry = ConstellationRegistry {
            version: REGISTRY_FORMAT_VERSION,
            ..Default::default()
        };
        create(&mut registry, "core").unwrap();
        save(&path, &registry).unwrap();

        let loaded = load(&path).unwrap();
        assert_eq!(loaded.current.as_deref(), Some("core"));
        assert!(loaded.constellations.contains_key("core"));
        // No tempfile litter is left behind an atomic save.
        let leftovers: Vec<_> = std::fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp-"))
            .collect();
        assert!(leftovers.is_empty());
    }

    #[test]
    fn a_corrupt_registry_is_an_error_not_an_empty_one() {
        let tmp = TempDir::new().unwrap();
        let path = registry_path(tmp.path());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "version: [not, yaml").unwrap();
        assert!(load(&path).is_err());
    }

    #[test]
    fn create_then_add_then_use() {
        let tmp = workspace();
        let mut registry = ConstellationRegistry {
            version: REGISTRY_FORMAT_VERSION,
            ..Default::default()
        };
        create(&mut registry, "alpha").unwrap();
        let canonical =
            add_source(&mut registry, "alpha", tmp.path(), SourceRole::Required).unwrap();
        assert_eq!(registry.constellations["alpha"].sources.len(), 1);
        assert!(Path::new(&canonical).is_absolute());

        // The first constellation becomes current without an explicit use.
        assert_eq!(registry.current.as_deref(), Some("alpha"));
        let specs = current_sources(&registry).unwrap();
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].path, PathBuf::from(&canonical));

        // Re-adding the same canonical path is refused.
        assert!(add_source(&mut registry, "alpha", tmp.path(), SourceRole::Required).is_err());
        // A second constellation does not steal `current`.
        create(&mut registry, "beta").unwrap();
        assert_eq!(registry.current.as_deref(), Some("alpha"));
        use_constellation(&mut registry, "beta").unwrap();
        assert_eq!(registry.current.as_deref(), Some("beta"));
        assert!(use_constellation(&mut registry, "nope").is_err());
        // Removing round-trips through canonicalization too.
        assert_eq!(
            remove_source(&mut registry, "alpha", tmp.path()).unwrap(),
            canonical
        );
        assert!(registry.constellations["alpha"].sources.is_empty());
    }

    #[test]
    fn add_refuses_a_directory_that_is_not_a_workspace() {
        let tmp = TempDir::new().unwrap();
        let mut registry = ConstellationRegistry::default();
        create(&mut registry, "c").unwrap();
        assert!(add_source(&mut registry, "c", tmp.path(), SourceRole::Required).is_err());
    }

    #[test]
    fn rebind_by_stored_path_rewrites_the_entry_and_reports_both_ids() {
        let old = workspace();
        let new = workspace();
        let mut registry = ConstellationRegistry::default();
        create(&mut registry, "c").unwrap();
        let old_canonical =
            add_source(&mut registry, "c", old.path(), SourceRole::Required).unwrap();

        let outcome = rebind_source(&mut registry, "c", &old_canonical, new.path()).unwrap();
        assert_eq!(outcome.old_path, old_canonical);
        assert!(Path::new(&outcome.new_path).is_absolute());
        assert_eq!(registry.constellations["c"].sources.len(), 1);
        assert_eq!(
            registry.constellations["c"].sources[0].path,
            outcome.new_path
        );
        assert!(outcome.old_source_id.starts_with("src:"));
        assert!(outcome.new_source_id.starts_with("src:"));
        assert_ne!(outcome.old_source_id, outcome.new_source_id);
        // A moved checkout is a new source identity: the new id is exactly
        // what an `add` of the new path would have minted.
        assert_eq!(outcome.new_source_id, stored_source_id(&outcome.new_path));
    }

    #[test]
    fn rebind_by_src_fragment_selects_a_moved_source() {
        // The classic rebind case: the checkout was renamed, so the stored
        // path is dead — and the `src:` selector must still resolve it.
        let holder = TempDir::new().unwrap();
        let before = holder.path().join("before");
        for dir in ARTIFACT_DIRS {
            std::fs::create_dir_all(before.join(dir)).unwrap();
        }
        let mut registry = ConstellationRegistry::default();
        create(&mut registry, "c").unwrap();
        let old_canonical = add_source(&mut registry, "c", &before, SourceRole::Required).unwrap();

        // The checkout moves; the stored path is dead from here on.
        let after = holder.path().join("after");
        std::fs::rename(&before, &after).unwrap();
        assert!(!Path::new(&old_canonical).exists());

        let old_id = stored_source_id(&old_canonical);
        let fragment = &old_id[..12]; // "src:" + first id chars
        let outcome = rebind_source(&mut registry, "c", fragment, &after).unwrap();
        assert_eq!(outcome.old_path, old_canonical);
        assert_eq!(outcome.old_source_id, old_id);
        assert_eq!(
            registry.constellations["c"].sources[0].path,
            outcome.new_path
        );
    }

    #[test]
    fn rebind_selector_miss_names_what_is_registered() {
        let ws = workspace();
        let mut registry = ConstellationRegistry::default();
        create(&mut registry, "c").unwrap();
        let canonical = add_source(&mut registry, "c", ws.path(), SourceRole::Required).unwrap();

        let err = rebind_source(&mut registry, "c", "no-such-fragment", ws.path()).unwrap_err();
        let text = format!("{err}");
        assert!(text.contains("matches no source"), "{text}");
        assert!(text.contains(&canonical), "{text}");
        // The registry is untouched by a miss.
        assert_eq!(registry.constellations["c"].sources[0].path, canonical);
    }

    #[test]
    fn rebind_refuses_a_path_another_source_already_claims() {
        let a = workspace();
        let b = workspace();
        let mut registry = ConstellationRegistry::default();
        create(&mut registry, "c").unwrap();
        let a_canonical = add_source(&mut registry, "c", a.path(), SourceRole::Required).unwrap();
        add_source(&mut registry, "c", b.path(), SourceRole::Required).unwrap();

        let err = rebind_source(&mut registry, "c", &a_canonical, b.path()).unwrap_err();
        assert!(format!("{err}").contains("already a source"));
    }

    #[test]
    fn rebind_refuses_a_target_that_is_not_a_workspace() {
        let ws = workspace();
        let plain = TempDir::new().unwrap();
        let mut registry = ConstellationRegistry::default();
        create(&mut registry, "c").unwrap();
        let canonical = add_source(&mut registry, "c", ws.path(), SourceRole::Required).unwrap();

        assert!(rebind_source(&mut registry, "c", &canonical, plain.path()).is_err());
        assert_eq!(registry.constellations["c"].sources[0].path, canonical);
    }

    #[test]
    fn rebind_to_the_same_path_is_allowed_and_reports_identity_unchanged() {
        // Adopting a foreign marker without moving the checkout needs a
        // rebind that changes nothing in the registry — the marker rewrite
        // is the real work in that case.
        let ws = workspace();
        let mut registry = ConstellationRegistry::default();
        create(&mut registry, "c").unwrap();
        let canonical = add_source(&mut registry, "c", ws.path(), SourceRole::Required).unwrap();

        let outcome = rebind_source(&mut registry, "c", &canonical, ws.path()).unwrap();
        assert_eq!(outcome.old_path, outcome.new_path);
        assert_eq!(outcome.old_source_id, outcome.new_source_id);
    }

    #[test]
    fn an_ambiguous_selector_is_an_error_not_a_first_match() {
        let holder = TempDir::new().unwrap();
        let mut registry = ConstellationRegistry::default();
        create(&mut registry, "c").unwrap();
        for name in ["one", "two"] {
            let dir = holder.path().join(name);
            for artifact in ARTIFACT_DIRS {
                std::fs::create_dir_all(dir.join(artifact)).unwrap();
            }
            add_source(&mut registry, "c", &dir, SourceRole::Required).unwrap();
        }
        // Both stored paths contain the holder's canonical path — the
        // fragment resolves nothing on its own. (Stored paths are
        // canonical, so the selector must be too: a bare TempDir path is
        // not a substring of its canonicalized self on every platform.)
        let err = rebind_source(
            &mut registry,
            "c",
            &holder.path().canonicalize().unwrap().display().to_string(),
            holder.path(),
        )
        .unwrap_err();
        assert!(format!("{err}").contains("ambiguous"));
    }

    #[cfg(unix)]
    #[test]
    fn rebind_refuses_a_symlinked_new_path() {
        let ws = workspace();
        let target = workspace();
        let holder = TempDir::new().unwrap();
        let link = holder.path().join("alias");
        std::os::unix::fs::symlink(target.path(), &link).unwrap();
        let mut registry = ConstellationRegistry::default();
        create(&mut registry, "c").unwrap();
        let canonical = add_source(&mut registry, "c", ws.path(), SourceRole::Required).unwrap();

        let err = rebind_source(&mut registry, "c", &canonical, &link).unwrap_err();
        assert!(format!("{err}").contains("symlink"));
    }

    #[cfg(unix)]
    #[test]
    fn add_refuses_a_symlinked_source_root() {
        let real = workspace();
        let holder = TempDir::new().unwrap();
        let link = holder.path().join("alias");
        std::os::unix::fs::symlink(real.path(), &link).unwrap();
        let mut registry = ConstellationRegistry::default();
        create(&mut registry, "c").unwrap();
        let err = add_source(&mut registry, "c", &link, SourceRole::Required).unwrap_err();
        assert!(format!("{err}").contains("symlink"));
    }

    #[test]
    fn doctor_reports_readable_indexed_and_broken_sources() {
        let indexed = workspace();
        std::fs::create_dir_all(indexed.path().join("lance")).unwrap();
        std::fs::write(indexed.path().join("lance/CURRENT.json"), "{}").unwrap();
        let plain = workspace();
        let mut registry = ConstellationRegistry::default();
        create(&mut registry, "c").unwrap();
        add_source(&mut registry, "c", indexed.path(), SourceRole::Required).unwrap();
        add_source(&mut registry, "c", plain.path(), SourceRole::Required).unwrap();
        registry
            .constellations
            .get_mut("c")
            .unwrap()
            .sources
            .push(RegistrySource {
                path: "/definitely/missing".into(),
                added_at: now_rfc3339(),
                role: SourceRole::Required,
            });

        let rows = doctor(&registry);
        assert_eq!(rows.len(), 3);
        assert!(rows[0].1.readable && rows[0].1.indexed);
        assert!(rows[1].1.readable && !rows[1].1.indexed);
        assert!(!rows[2].1.readable);
    }

    // -- Source roles ---------------------------------------------------------

    #[test]
    fn a_source_written_before_roles_loads_as_required() {
        // Back-compat: a registry from before the `role` field has no key
        // at all — serde's default fills it with `required`, which is the
        // meaning those registries carried implicitly.
        let tmp = TempDir::new().unwrap();
        let path = registry_path(tmp.path());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "version: 1\ncurrent: c\nconstellations:\n  c:\n    created_at: \"2026-01-01T00:00:00Z\"\n    sources:\n      - path: /some/.forgeplan\n        added_at: \"2026-01-01T00:00:00Z\"\n",
        )
        .unwrap();
        let registry = load(&path).unwrap();
        assert_eq!(
            registry.constellations["c"].sources[0].role,
            SourceRole::Required
        );
    }

    #[test]
    fn a_source_role_roundtrips_through_the_file() {
        let tmp = TempDir::new().unwrap();
        let ws = workspace();
        let path = registry_path(tmp.path());
        let mut registry = ConstellationRegistry::default();
        create(&mut registry, "c").unwrap();
        add_source(&mut registry, "c", ws.path(), SourceRole::Optional).unwrap();
        save(&path, &registry).unwrap();

        let loaded = load(&path).unwrap();
        assert_eq!(
            loaded.constellations["c"].sources[0].role,
            SourceRole::Optional
        );
        // The wire spelling is the snake_case contract string.
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains("role: optional"), "{body}");

        // And the reader-facing specs carry it too.
        let specs = current_sources(&loaded).unwrap();
        assert_eq!(specs[0].role, SourceRole::Optional);
    }

    #[test]
    fn add_marks_the_role_and_named_sources_carry_it() {
        let required_ws = workspace();
        let optional_ws = workspace();
        let mut registry = ConstellationRegistry::default();
        create(&mut registry, "c").unwrap();
        // Second constellation — proves `named_sources` reaches a
        // non-current def (what the MCP read profile opens by name).
        create(&mut registry, "other").unwrap();
        add_source(&mut registry, "c", required_ws.path(), SourceRole::Required).unwrap();
        add_source(
            &mut registry,
            "other",
            optional_ws.path(),
            SourceRole::Optional,
        )
        .unwrap();

        let current = current_sources(&registry).unwrap();
        assert_eq!(current[0].role, SourceRole::Required);
        let other = named_sources(&registry, "other").unwrap();
        assert_eq!(other[0].role, SourceRole::Optional);
        assert!(named_sources(&registry, "missing").is_none());

        // Doctor surfaces the role so a broken required source reads as
        // the outage it is, not a bearable degradation.
        let rows = doctor(&registry);
        assert_eq!(rows[0].1.role, SourceRole::Required);
        assert_eq!(rows[1].1.role, SourceRole::Optional);
    }

    #[test]
    fn source_role_labels_match_the_serde_spelling() {
        assert_eq!(SourceRole::Required.as_str(), "required");
        assert_eq!(SourceRole::Optional.as_str(), "optional");
        assert_eq!(SourceRole::default(), SourceRole::Required);
    }
}
