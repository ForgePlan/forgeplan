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

/// One federated source as recorded: a canonical `.forgeplan` directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistrySource {
    /// Canonical absolute path to the `.forgeplan` directory.
    pub path: String,
    /// When the source was added (RFC 3339). Informational — identity is
    /// the path, not the timestamp.
    pub added_at: String,
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

/// Validate and add a source to a constellation.
///
/// The path must resolve to a real `.forgeplan` workspace (artifact dirs
/// present) and must not be a symlink — the same confinement rule the
/// reader applies, enforced at registration time so a bad entry fails loud
/// at `add`, not quietly at every later read. Adding the same canonical
/// path twice is an error; aliases of one workspace would double-count
/// every hit.
pub fn add_source(
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
    let canonical_string = canonical.display().to_string();
    if def.sources.iter().any(|s| s.path == canonical_string) {
        return Err(ConstellationError::Io {
            detail: format!("{canonical_string} is already a source of {constellation:?}"),
        });
    }
    def.sources.push(RegistrySource {
        path: canonical_string.clone(),
        added_at: now_rfc3339(),
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

/// The current constellation's sources as reader input. `None` when no
/// constellation is selected — callers report that as its own diagnostic.
pub fn current_sources(
    registry: &ConstellationRegistry,
) -> Option<Vec<super::federation::SourceSpec>> {
    let name = registry.current.as_deref()?;
    registry.constellations.get(name).map(|def| {
        def.sources
            .iter()
            .map(|s| super::federation::SourceSpec::new(PathBuf::from(&s.path)))
            .collect()
    })
}

// -- Doctor ---------------------------------------------------------------------

/// Per-source health row for `doctor`.
#[derive(Debug)]
pub struct SourceHealth {
    pub path: String,
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
                    readable: false,
                    indexed: false,
                    detail: Some(format!("cannot stat: {e}")),
                },
                Ok(meta) if meta.file_type().is_symlink() => SourceHealth {
                    path: source.path.clone(),
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
        let canonical = add_source(&mut registry, "alpha", tmp.path()).unwrap();
        assert_eq!(registry.constellations["alpha"].sources.len(), 1);
        assert!(Path::new(&canonical).is_absolute());

        // The first constellation becomes current without an explicit use.
        assert_eq!(registry.current.as_deref(), Some("alpha"));
        let specs = current_sources(&registry).unwrap();
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].path, PathBuf::from(&canonical));

        // Re-adding the same canonical path is refused.
        assert!(add_source(&mut registry, "alpha", tmp.path()).is_err());
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
        assert!(add_source(&mut registry, "c", tmp.path()).is_err());
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
        let err = add_source(&mut registry, "c", &link).unwrap_err();
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
        add_source(&mut registry, "c", indexed.path()).unwrap();
        add_source(&mut registry, "c", plain.path()).unwrap();
        registry
            .constellations
            .get_mut("c")
            .unwrap()
            .sources
            .push(RegistrySource {
                path: "/definitely/missing".into(),
                added_at: now_rfc3339(),
            });

        let rows = doctor(&registry);
        assert_eq!(rows.len(), 3);
        assert!(rows[0].1.readable && rows[0].1.indexed);
        assert!(rows[1].1.readable && !rows[1].1.indexed);
        assert!(!rows[2].1.readable);
    }
}
