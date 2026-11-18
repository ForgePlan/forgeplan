//! External constellation sources behind a byte-level adapter.
//!
//! A constellation source is normally a `.forgeplan` directory on disk —
//! [`super::capture`] reads its markdown — or a git checkout pinned at an OID
//! — [`super::snapshot`] reads its objects. Both share one shape: a source
//! answers **bytes** for artifact-relative paths, and those bytes carry the
//! source's documents. This module makes that shape explicit so a system
//! **outside** `.forgeplan` — an external document store, a generated
//! corpus, a service — can stand in as a source without pretending to be a
//! filesystem or a git repository.
//!
//! The behavior this module exists to prove (F11, issue #538): an external
//! adapter can be a constellation source end to end. A caller opens an
//! [`AdapterScope`] over the adapter, reads the pinned bytes of an artifact
//! path, and receives the document's canonical identity — and the reverse
//! path holds: a document written **through** the adapter reads back as
//! canonical, through the same trait boundary, with the identical identity
//! the capture and git-snapshot channels derive (all three parse with the
//! shared [`parse_artifact_file`]).
//!
//! # What "pinned" means here
//!
//! An adapter holds no version history: `write_bytes` is **create-only** per
//! relative path — the same discipline `forgeplan new` applies to canonical
//! documents, whose key is published once and never overwritten — and every
//! `read_bytes` answers exactly the bytes that were stored. Pinning is the
//! storage itself: a path that exists is a path that was pinned, and nothing
//! in this module rewrites an existing pin.
//!
//! # What this is not
//!
//! Sources break, and an adapter that cannot answer must say so. Reads
//! report [`AdapterError::NotFound`] for an unpinned path and refuse a path
//! that could escape the source (`..`, absolute, empty, NUL) with
//! [`AdapterError::InvalidPath`]. There is no fallback that guesses bytes and
//! no channel that pretends a missing document exists.

use std::collections::BTreeMap;
use std::path::{Component, Path};
use std::sync::Arc;

use crate::artifact::canonical_key::ArtifactIdentity;

use super::capture::parse_artifact_file;
use super::jobs::DEFAULT_BODY_PREFIX_CHARS;

/// A constellation source that answers artifact bytes through this trait
/// rather than through the filesystem or git.
///
/// Implementors must be `Send + Sync`: a scope over an adapter may be opened
/// from a read handler and served on another thread. The four methods are
/// the whole contract — identity, enumeration, the pinned read, and the
/// create-only reverse path.
pub trait SourceAdapter: Send + Sync {
    /// Canonical identity of the source, stable for one external system.
    ///
    /// Two adapters that are the same source must answer the same string;
    /// two adapters that are different sources must answer different ones —
    /// the same rule [`super::federation::SourceRef`] applies to workspaces
    /// (its `source_id` field), so an identity collision can never merge two
    /// systems.
    fn source_id(&self) -> &str;

    /// The artifact-relative paths this source can answer, sorted
    /// lexicographically.
    ///
    /// This is the adapter's equivalent of a capture's enumeration pass.
    /// Every returned path must be servable by [`Self::read_bytes`]; a path
    /// that is listed but unreadable is a broken adapter, not a fact a
    /// caller should have to guess around.
    fn enumerate_artifacts(&self) -> Vec<String>;

    /// The pinned bytes for one artifact-relative path.
    ///
    /// Answers exactly the bytes that were stored by
    /// [`Self::write_bytes`] — never a re-derivation and never a different
    /// revision. [`AdapterError::NotFound`] when the path was never written;
    /// [`AdapterError::InvalidPath`] when the path could escape the source.
    fn read_bytes(&self, relative_path: &str) -> Result<Vec<u8>, AdapterError>;

    /// The reverse path: publish one document into the source.
    ///
    /// **Create-only** per relative path, matching canonical-document
    /// publication: a second write to an existing path is
    /// [`AdapterError::AlreadyExists`], whatever the bytes. A write never
    /// mutates a document that is already served, so a reader that pinned a
    /// path can trust the pin.
    fn write_bytes(&mut self, relative_path: &str, bytes: Vec<u8>) -> Result<(), AdapterError>;
}

/// Why an adapter could not answer.
///
/// The three read-side failures are distinct facts: the path was never
/// written, the path is not a legal artifact path, or the bytes at the path
/// are not a canonical document. Every variant names the path it is about —
/// a caller reporting the failure can say *which* document, and an absolute
/// path never appears.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterError {
    /// No bytes are pinned at this relative path.
    NotFound { relative_path: String },
    /// The path exists; a create-only write refused to replace it.
    AlreadyExists { relative_path: String },
    /// The path is not a safe artifact-relative path.
    InvalidPath {
        relative_path: String,
        reason: String,
    },
    /// The pinned bytes are not a canonical artifact document.
    NotCanonical {
        relative_path: String,
        reason: String,
    },
}

impl std::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound { relative_path } => {
                write!(f, "adapter has no document pinned at {relative_path:?}")
            }
            Self::AlreadyExists { relative_path } => {
                write!(
                    f,
                    "adapter document at {relative_path:?} already exists; create-only"
                )
            }
            Self::InvalidPath {
                relative_path,
                reason,
            } => write!(f, "adapter path {relative_path:?} is invalid: {reason}"),
            Self::NotCanonical {
                relative_path,
                reason,
            } => write!(
                f,
                "adapter bytes at {relative_path:?} are not a canonical document: {reason}"
            ),
        }
    }
}

impl std::error::Error for AdapterError {}

/// A read scope opened over one external source.
///
/// The scope is the read side of [`SourceAdapter`], mirroring the crate's
/// read/write separation: it borrows the adapter and exposes exactly the
/// constellation-read verbs — name the source, enumerate its artifacts, read
/// the pinned bytes, and resolve a document's canonical identity from those
/// bytes. There is no write through a scope ([`SourceAdapter::write_bytes`]
/// is the only way a document enters the source) and no method here reads
/// anything but the adapter it was opened over.
///
/// Identity comes from the same [`parse_artifact_file`] the capture and
/// git-snapshot channels parse with, so an adapter document and a
/// `.forgeplan` document with the same bytes are canonically
/// indistinguishable: same canonical key, same display id, up to the shared
/// frontmatter rules.
#[derive(Clone, Copy)]
pub struct AdapterScope<'a> {
    adapter: &'a dyn SourceAdapter,
}

impl std::fmt::Debug for AdapterScope<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `dyn SourceAdapter` is not `Debug`, so the derive cannot stand in;
        // naming the source is the debugging-relevant part of the scope.
        f.debug_struct("AdapterScope")
            .field("source_id", &self.source_id())
            .finish()
    }
}

impl<'a> AdapterScope<'a> {
    /// Open a scope over one source.
    pub fn open(adapter: &'a dyn SourceAdapter) -> Self {
        Self { adapter }
    }

    /// The canonical identity of the source behind this scope.
    pub fn source_id(&self) -> &str {
        self.adapter.source_id()
    }

    /// Artifact-relative paths the source answers, in its own enumeration
    /// order.
    pub fn enumerate_artifacts(&self) -> Vec<String> {
        self.adapter.enumerate_artifacts()
    }

    /// The pinned bytes of one artifact, as stored by the adapter.
    ///
    /// The path is validated before the adapter is asked, so an escaping
    /// path is refused by the scope even against an adapter that does not
    /// validate its own inputs.
    pub fn pinned_bytes(&self, relative_path: &str) -> Result<Vec<u8>, AdapterError> {
        validate_relative_path(relative_path)?;
        self.adapter.read_bytes(relative_path)
    }

    /// The canonical identity of one pinned document.
    ///
    /// Reads the pinned bytes and parses them with the shared artifact
    /// parser. [`AdapterError::NotFound`] when the path was never written;
    /// [`AdapterError::NotCanonical`] when the bytes do not parse as a
    /// canonical artifact document — a document that is not canonical is
    /// reported, never resolved to a guessed identity.
    pub fn canonical_identity(
        &self,
        relative_path: &str,
    ) -> Result<ArtifactIdentity, AdapterError> {
        let bytes = self.pinned_bytes(relative_path)?;
        let parsed =
            parse_artifact_file(relative_path, Arc::from(bytes), DEFAULT_BODY_PREFIX_CHARS)
                .map_err(|diagnostic| AdapterError::NotCanonical {
                    relative_path: relative_path.to_string(),
                    reason: diagnostic.detail,
                })?;
        Ok(ArtifactIdentity {
            canonical_key: parsed.canonical_key,
            display_id: parsed.display_id,
        })
    }

    /// The canonical identity of every pinned document, sorted by path.
    ///
    /// A document whose bytes are not canonical is omitted — the per-path
    /// [`Self::canonical_identity`] is the strict form that reports it.
    pub fn identities(&self) -> Vec<(String, ArtifactIdentity)> {
        let mut out: Vec<(String, ArtifactIdentity)> = self
            .enumerate_artifacts()
            .into_iter()
            .filter_map(|path| {
                self.canonical_identity(&path)
                    .map(|identity| (path, identity))
                    .ok()
            })
            .collect();
        out.sort_by(|left, right| left.0.cmp(&right.0));
        out
    }
}

/// Refuse a relative path that could escape the source.
///
/// An artifact path is a workspace-relative path like `prds/PRD-001.md`.
/// Empty, absolute, NUL-carrying and `..`-containing paths are all ways an
/// adapter-backed source could be made to address something outside itself;
/// `validate_relative_path` is the single place that rule lives, shared by
/// the scope reads and the trait implementors.
fn validate_relative_path(relative_path: &str) -> Result<(), AdapterError> {
    let invalid = |reason: &str| AdapterError::InvalidPath {
        relative_path: relative_path.to_string(),
        reason: reason.to_string(),
    };
    if relative_path.is_empty() {
        return Err(invalid("a path cannot be empty"));
    }
    if relative_path.contains('\0') {
        return Err(invalid("a path cannot contain a NUL byte"));
    }
    let path = Path::new(relative_path);
    if path.is_absolute() {
        return Err(invalid("a path cannot be absolute"));
    }
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(invalid("a path cannot traverse upward (`..`)"));
    }
    Ok(())
}

/// An in-memory [`SourceAdapter`]: the reference implementation, one pinned
/// byte set per relative path.
///
/// This is the adapter an external system's adapter is measured against, and
/// the one the round-trip tests run through. It is deliberately plain: a
/// `BTreeMap` keyed by validated relative path, create-only writes, exact
/// byte reads. It is not a test double — it is a real store behind the real
/// trait boundary, and it can serve any caller that needs an adapter it does
/// not have to stand up a service for.
#[derive(Debug, Clone, Default)]
pub struct InMemoryAdapter {
    source_id: String,
    documents: BTreeMap<String, Vec<u8>>,
}

impl InMemoryAdapter {
    /// A new, empty adapter naming one external source.
    pub fn empty(source_id: &str) -> Self {
        Self {
            source_id: source_id.to_string(),
            documents: BTreeMap::new(),
        }
    }
}

impl SourceAdapter for InMemoryAdapter {
    fn source_id(&self) -> &str {
        &self.source_id
    }

    fn enumerate_artifacts(&self) -> Vec<String> {
        self.documents.keys().cloned().collect()
    }

    fn read_bytes(&self, relative_path: &str) -> Result<Vec<u8>, AdapterError> {
        validate_relative_path(relative_path)?;
        self.documents
            .get(relative_path)
            .cloned()
            .ok_or_else(|| AdapterError::NotFound {
                relative_path: relative_path.to_string(),
            })
    }

    fn write_bytes(&mut self, relative_path: &str, bytes: Vec<u8>) -> Result<(), AdapterError> {
        validate_relative_path(relative_path)?;
        if self.documents.contains_key(relative_path) {
            return Err(AdapterError::AlreadyExists {
                relative_path: relative_path.to_string(),
            });
        }
        self.documents.insert(relative_path.to_string(), bytes);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::canonical_key::canonical_key_from_document;

    /// A canonical document: frontmatter with `id`, a minted `slug` (the
    /// canonical key), `kind`, `status`, `title` and a body.
    fn canonical_doc(display_id: &str, slug: &str, title: &str, body: &str) -> String {
        format!(
            "---\nid: {display_id}\nslug: {slug}\nkind: prd\nstatus: active\ntitle: {title}\n---\n\n{body}\n"
        )
    }

    const SLUG: &str = "prd-external-k0123456789abcdef0123456789abcdef";

    #[test]
    fn a_scope_reads_the_pinned_bytes_and_names_the_source() {
        let mut adapter = InMemoryAdapter::empty("external-system-1");
        let doc = canonical_doc("PRD-042", SLUG, "External plan", "External body.");
        adapter
            .write_bytes("prds/PRD-042.md", doc.clone().into_bytes())
            .unwrap();

        let scope = AdapterScope::open(&adapter);
        assert_eq!(scope.source_id(), "external-system-1");
        assert_eq!(
            scope.enumerate_artifacts(),
            vec!["prds/PRD-042.md".to_string()]
        );
        // The read answers exactly the stored bytes — a pin is the storage.
        assert_eq!(
            scope.pinned_bytes("prds/PRD-042.md").unwrap(),
            doc.as_bytes()
        );
        // A path that was never written is a named NotFound, not a guess.
        assert_eq!(
            scope.pinned_bytes("prds/never-written.md").unwrap_err(),
            AdapterError::NotFound {
                relative_path: "prds/never-written.md".to_string()
            }
        );
    }

    #[test]
    fn the_scope_returns_the_canonical_identity_of_pinned_bytes() {
        let mut adapter = InMemoryAdapter::empty("ext-2");
        let doc = canonical_doc("PRD-042", SLUG, "External plan", "External body.");
        adapter
            .write_bytes("prds/PRD-042.md", doc.clone().into_bytes())
            .unwrap();

        let scope = AdapterScope::open(&adapter);
        let identity = scope
            .canonical_identity("prds/PRD-042.md")
            .expect("the pinned bytes are a canonical document");
        assert_eq!(identity.canonical_key, SLUG);
        assert_eq!(identity.display_id, "PRD-042");
        // Identical to the shared resolver the workspace-side code uses: the
        // adapter did not invent an identity, it reports the document's.
        assert_eq!(
            canonical_key_from_document("PRD-042", &doc),
            identity.canonical_key
        );
        // The listing form agrees with the per-path form.
        assert_eq!(scope.identities().len(), 1);
        assert_eq!(scope.identities()[0].0, "prds/PRD-042.md");
        assert_eq!(scope.identities()[0].1, identity);
    }

    #[test]
    fn a_document_created_through_the_adapter_reads_as_canonical() {
        let mut adapter = InMemoryAdapter::empty("ext-3");
        let doc = canonical_doc("PRD-042", SLUG, "External plan", "External body.");
        // The reverse path: the document enters the constellation *through*
        // the adapter — the only way a document gets in.
        adapter
            .write_bytes("ext/prd-042.md", doc.clone().into_bytes())
            .unwrap();

        let scope = AdapterScope::open(&adapter);
        let identity = scope
            .canonical_identity("ext/prd-042.md")
            .expect("a document created through the adapter must read as canonical");

        // The twin test: the same bytes on disk, parsed by the same shared
        // parser the capture channel uses, must name the same document. An
        // adapter document and a .forgeplan document are then canonically
        // indistinguishable — the path never participates in the identity.
        let parsed = parse_artifact_file(
            "prds/PRD-042.md",
            Arc::from(doc.clone().into_bytes()),
            DEFAULT_BODY_PREFIX_CHARS,
        )
        .expect("the twin on disk is a canonical document");
        assert_eq!(parsed.canonical_key, identity.canonical_key);
        assert_eq!(parsed.display_id, identity.display_id);
    }

    #[test]
    fn writes_are_create_only_and_absent_paths_are_not_found() {
        let mut adapter = InMemoryAdapter::empty("ext-4");
        let doc = canonical_doc("PRD-042", SLUG, "External plan", "External body.");
        adapter
            .write_bytes("prds/PRD-042.md", doc.clone().into_bytes())
            .unwrap();

        // A second write to an existing path is refused whatever the bytes —
        // identical or not; a pin never moves.
        let err = adapter
            .write_bytes("prds/PRD-042.md", doc.into_bytes())
            .unwrap_err();
        assert_eq!(
            err,
            AdapterError::AlreadyExists {
                relative_path: "prds/PRD-042.md".to_string()
            }
        );
        assert_eq!(adapter.documents.len(), 1, "the pin survived the refusal");
    }

    #[test]
    fn unsafe_relative_paths_are_refused_on_read_and_write() {
        let mut adapter = InMemoryAdapter::empty("ext-5");
        let doc = canonical_doc("PRD-042", SLUG, "External plan", "External body.");
        for bad in ["", "../escape.md", "/abs/prd.md", "prds/a\0b.md"] {
            let write_err = adapter
                .write_bytes(bad, doc.clone().into_bytes())
                .expect_err("an escaping path must not enter the adapter");
            assert!(
                matches!(write_err, AdapterError::InvalidPath { ref relative_path, .. }
                    if relative_path == bad),
                "write of {bad:?} produced {write_err:?}"
            );

            let scope = AdapterScope::open(&adapter);
            let read_err = scope
                .pinned_bytes(bad)
                .expect_err("an escaping path must not reach the adapter");
            assert!(
                matches!(read_err, AdapterError::InvalidPath { ref relative_path, .. }
                    if relative_path == bad),
                "read of {bad:?} produced {read_err:?}"
            );
        }
    }

    #[test]
    fn non_canonical_bytes_are_reported_not_resolved() {
        let mut adapter = InMemoryAdapter::empty("ext-6");
        adapter
            .write_bytes("prds/broken.md", b"not frontmatter at all".to_vec())
            .unwrap();
        let scope = AdapterScope::open(&adapter);
        let err = scope
            .canonical_identity("prds/broken.md")
            .expect_err("bytes that are not a canonical document must not resolve to an identity");
        assert!(
            matches!(err, AdapterError::NotCanonical { ref relative_path, .. }
            if relative_path == "prds/broken.md")
        );
        assert!(scope.identities().is_empty());
    }
}
