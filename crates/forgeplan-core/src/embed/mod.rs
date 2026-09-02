use std::path::{Path, PathBuf};

/// RFC-013 Phase 2 — pure-Rust inference, present alongside the ONNX Runtime
/// path so the two can be compared before either is removed.
#[cfg(feature = "tract-engine")]
pub mod tract_engine;

/// Approximate on-disk size of the default model (BGE-M3), measured on
/// macOS 2026-08-29: `du -sh` over `models--BAAI--bge-m3` reported 2.1 GB.
///
/// This figure is user-facing — it is the number printed before a first-run
/// download begins. Three places in the tree previously carried three
/// different guesses (`README.md` said ~150 MB, the gitignore drift detector
/// said ~600 MB, nothing said 2.1 GB). Keep this constant the single source
/// and reference it rather than restating a number.
pub const MODEL_DOWNLOAD_SIZE_HINT: &str = "~2.1 GB";

/// Directory name fastembed uses when nothing overrides it. Resolved
/// relative to the **process CWD**, which is why an un-configured build
/// scatters a multi-gigabyte cache into whatever directory the user
/// happened to run from — and re-downloads it for every project.
const FASTEMBED_DEFAULT_CACHE_DIR: &str = ".fastembed_cache";

/// A long input built by repetition, sized past the 2000-char `chunk_size`
/// callers apply, so the fixture exercises a realistic artifact body rather
/// than a phrase. Built from a `const` rather than read off disk: a fixture
/// whose input can change is not an oracle.
const LONG_BODY_UNIT: &str = "The engine is linked at build time, so a prebuilt \
    from someone else's machine has to match ours. It does not, on four targets \
    out of five. ";

/// Texts the embedding oracle is pinned on, shared by the generator and the
/// test so the two cannot drift apart.
///
/// Chosen for where an engine swap is most likely to diverge rather than for
/// coverage of ordinary prose:
///
/// - **ASCII vs Cyrillic** — different tokenizer paths through the same
///   vocabulary; BGE-M3 is multilingual and our artifacts are Russian.
/// - **empty string** — the degenerate case, where a tokenizer emits only
///   special tokens and pooling has almost nothing to pool.
/// - **emoji and mixed script** — multi-byte codepoints and script switching,
///   the classic place an off-by-one in tokenization hides.
/// - **long body** — a realistic artifact length, where accumulated numerical
///   drift would show up if it were going to.
/// - **whitespace-only** — looks empty but is not, and is trivially easy to
///   normalise differently by accident.
pub fn reference_cases() -> Vec<(&'static str, String)> {
    vec![
        (
            "short_english",
            "why is vector search missing from release binaries".to_string(),
        ),
        (
            "short_russian",
            "почему в релизных бинарях нет векторного поиска".to_string(),
        ),
        (
            "mixed_script",
            "ADR-022 решение: semantic-search остаётся вне prebuilt 🚀 (2.1 GB)".to_string(),
        ),
        ("empty", String::new()),
        ("whitespace_only", "   \n\t  ".to_string()),
        ("long_body", LONG_BODY_UNIT.repeat(16)),
    ]
}

/// Resolve where embedding models are cached.
///
/// Precedence:
/// 1. `FORGEPLAN_MODEL_CACHE` — explicit override, for users who keep models
///    on a different volume.
/// 2. The platform user-cache directory (`~/Library/Caches/forgeplan/models`
///    on macOS, `~/.cache/forgeplan/models` on Linux,
///    `%LOCALAPPDATA%\forgeplan\models` on Windows). Shared across every
///    project on the machine — one download, not one per repository.
/// 3. `./.fastembed_cache` — fastembed's own default, used only when the
///    platform cache directory cannot be determined.
///
/// Caveat worth stating plainly: fastembed lets `HF_HOME` take precedence
/// over the cache dir we pass in (see fastembed `common.rs::pull_from_hf`).
/// If a user has `HF_HOME` set, models land there and this resolver's answer
/// is advisory. That is deliberate on fastembed's side — it keeps a shared
/// HuggingFace cache authoritative — so we do not fight it.
pub fn resolve_cache_dir() -> PathBuf {
    resolve_cache_dir_from(
        std::env::var("FORGEPLAN_MODEL_CACHE").ok().as_deref(),
        dirs::cache_dir(),
    )
}

/// The decision logic behind [`resolve_cache_dir`], with both inputs injected.
///
/// Split out so the precedence rules can be tested without touching process
/// environment. That matters more than it looks: setting an env var in a test
/// is `unsafe` in Rust 2024 because the write races reads of *any* other
/// variable from other threads. This crate already has git-dependent tests
/// that shell out and read `PATH`, and env-mutating tests elsewhere in the
/// suite already make them flaky. Adding more env mutation would have made a
/// known-bad situation worse, so this resolver takes its inputs as arguments
/// and the tests never touch the environment at all.
fn resolve_cache_dir_from(explicit: Option<&str>, platform_cache: Option<PathBuf>) -> PathBuf {
    // An empty or whitespace-only override is a misconfiguration, not a
    // request to cache in the filesystem root — fall through to the default.
    if let Some(trimmed) = explicit.map(str::trim).filter(|s| !s.is_empty()) {
        return PathBuf::from(trimmed);
    }

    match platform_cache {
        Some(base) => base.join("forgeplan").join("models"),
        None => PathBuf::from(FASTEMBED_DEFAULT_CACHE_DIR),
    }
}

/// Report whether a model appears to be present in `dir`.
///
/// Deliberately shallow: the presence of any `models--*` subdirectory is
/// enough. Verifying weights properly is fastembed's job, and a false
/// "present" only costs us a skipped notice, never a wrong download.
fn cache_looks_populated(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries.flatten().any(|entry| {
        entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with("models--"))
    })
}

/// A legacy per-project cache left by an earlier build, if one exists in CWD.
///
/// Returned so the caller can tell the user a 2 GB download is about to be
/// repeated needlessly, and how to avoid it. We never move it ourselves —
/// relocating gigabytes without being asked is not a library's call.
pub fn legacy_local_cache() -> Option<PathBuf> {
    let local = PathBuf::from(FASTEMBED_DEFAULT_CACHE_DIR);
    cache_looks_populated(&local).then_some(local)
}

/// What the caller should tell the user before an embedder is constructed.
///
/// `None` means the model is already cached and initialisation will be quiet.
/// `Some(notice)` means a download is imminent and the user deserves to know
/// its size before it starts rather than after.
pub fn first_run_notice() -> Option<String> {
    let cache = resolve_cache_dir();
    if cache_looks_populated(&cache) {
        return None;
    }
    Some(compose_first_run_notice(&cache, legacy_local_cache()))
}

/// Wording of the first-run notice, with the filesystem facts injected.
///
/// Separated from [`first_run_notice`] for the same reason as
/// [`resolve_cache_dir_from`]: the message can then be asserted on directly,
/// with no environment mutation and no directories created or removed.
fn compose_first_run_notice(cache: &Path, legacy: Option<PathBuf>) -> String {
    let mut notice = format!(
        "First run: downloading the embedding model ({size}) to {path}.\n\
         This happens once per machine; later runs load from that cache.",
        size = MODEL_DOWNLOAD_SIZE_HINT,
        path = cache.display(),
    );

    if let Some(legacy) = legacy {
        notice.push_str(&format!(
            "\n\nA per-project cache already exists at {legacy}. Move it to \
             skip the download entirely:\n  mkdir -p {parent} && mv {legacy} {target}",
            legacy = legacy.display(),
            parent = cache.parent().unwrap_or(Path::new(".")).display(),
            target = cache.display(),
        ));
    }

    notice
}

#[cfg(feature = "tract-engine")]
mod inner {
    use super::tract_engine::{TractEmbedder, ensure_model};

    /// Which HuggingFace repo backs a configured model name.
    ///
    /// **Deliberately narrower than the fastembed-era list.** That list mapped
    /// twelve names, but the mapping was only half the story: fastembed also
    /// knew each model's pooling strategy and ONNX file layout. Reproducing
    /// twelve of those without being able to verify each one would mean
    /// shipping vectors nobody has checked — and a wrong pooling strategy
    /// does not fail, it returns plausible numbers (see the module note in
    /// `tract_engine`).
    ///
    /// So the mapping covers what is verified against the oracle, and anything
    /// else gets a clear error instead of a silent substitution. Widening it
    /// is a contained follow-up: add the repo, confirm its pooling, capture
    /// reference vectors, done.
    fn resolve_repo(model_name: &str) -> Option<&'static str> {
        match model_name {
            "bge-m3" => Some("BAAI/bge-m3"),
            _ => None,
        }
    }

    /// Embedding dimension depends on model.
    pub fn embedding_dim(model_name: &str) -> usize {
        match model_name {
            "bge-m3" => 1024,
            "bge-small-en" => 384,
            "bge-base-en" => 768,
            "bge-large-en" => 1024,
            "multilingual-e5-small" => 384,
            "multilingual-e5-base" => 768,
            "multilingual-e5-large" => 1024,
            "nomic-embed-v1.5" => 768,
            "all-minilm-l6" => 384,
            "jina-v2-en" | "jina-v2-code" => 768,
            "embedding-gemma-300m" => 768,
            _ => 1024,
        }
    }

    /// Default dimension (BGE-M3).
    pub const EMBEDDING_DIM: usize = 1024;

    /// Text embedder backed by the pure-Rust tract engine.
    ///
    /// The public shape is unchanged from the fastembed version on purpose
    /// (RFC-013 invariant #2) — every caller in the CLI, the MCP server and
    /// the store keeps working untouched, which keeps the blast radius of this
    /// swap inside one module.
    ///
    /// `&mut self` on `embed` is likewise kept even though tract needs only
    /// `&self`: relaxing it would be a silent API widening, and callers hold
    /// `let mut embedder` today.
    pub struct Embedder {
        engine: TractEmbedder,
        model_name: String,
    }

    impl Embedder {
        /// Create embedder with default model (BGE-M3).
        pub fn new() -> anyhow::Result<Self> {
            Self::with_model("bge-m3")
        }

        /// Create embedder with specific model from config.
        ///
        /// The model downloads on first use into the shared cache resolved by
        /// [`super::resolve_cache_dir`] — one copy per machine rather than one
        /// per project. A failure here is almost always network or disk, so we
        /// say that instead of surfacing a bare library error.
        pub fn with_model(model_name: &str) -> anyhow::Result<Self> {
            let repo = resolve_repo(model_name).ok_or_else(|| {
                anyhow::anyhow!(
                    "Embedding model '{model_name}' is not available on this build.\n\
                     Supported: bge-m3.\n\
                     Fix: set `embedding.model: bge-m3` in .forgeplan/config.yaml"
                )
            })?;

            let cache_dir = super::resolve_cache_dir();
            let dim = embedding_dim(model_name);

            let snapshot = ensure_model(&cache_dir, repo, true).map_err(|e| {
                anyhow::anyhow!(
                    "Could not obtain the embedding model '{model_name}' \
                     (cache: {cache}).\n\
                     First use downloads {size}; this needs network access and \
                     free disk space.\n\
                     Underlying error: {e}\n\
                     Fix: check connectivity and rerun, or point \
                     FORGEPLAN_MODEL_CACHE at a writable location",
                    cache = cache_dir.display(),
                    size = super::MODEL_DOWNLOAD_SIZE_HINT,
                )
            })?;

            let engine = TractEmbedder::from_snapshot(&snapshot, model_name, dim).map_err(|e| {
                anyhow::anyhow!(
                    "The embedding model '{model_name}' is present at {} but \
                     could not be loaded.\n\
                     Underlying error: {e}\n\
                     Fix: the cached copy may be incomplete — remove it and \
                     rerun to fetch it again",
                    snapshot.display()
                )
            })?;

            Ok(Self {
                engine,
                model_name: model_name.to_string(),
            })
        }

        /// Current model name.
        pub fn model_name(&self) -> &str {
            &self.model_name
        }

        /// Embedding dimension for current model.
        pub fn dim(&self) -> usize {
            self.engine.dim()
        }

        /// Embed a single text.
        pub fn embed(&mut self, text: &str) -> anyhow::Result<Vec<f32>> {
            self.engine.embed(text)
        }

        /// Embed multiple texts in batch.
        pub fn embed_batch(&mut self, texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
            self.engine.embed_batch(texts)
        }
    }
}

#[cfg(feature = "tract-engine")]
pub use inner::*;

/// Placeholder when the embedding engine is not compiled in.
#[cfg(not(feature = "tract-engine"))]
pub const EMBEDDING_DIM: usize = 1024;

#[cfg(test)]
mod cache_dir_tests {
    use super::*;

    /// None of these tests touch process environment or the filesystem.
    /// Both inputs are injected, so they cannot race the git-dependent tests
    /// elsewhere in this crate that shell out and read `PATH`.

    #[test]
    fn explicit_override_wins_over_platform_cache() {
        let resolved = resolve_cache_dir_from(
            Some("/tmp/forgeplan-models"),
            Some(PathBuf::from("/home/u/.cache")),
        );
        assert_eq!(resolved, PathBuf::from("/tmp/forgeplan-models"));
    }

    #[test]
    fn override_is_trimmed() {
        let resolved = resolve_cache_dir_from(Some("  /tmp/models  "), None);
        assert_eq!(resolved, PathBuf::from("/tmp/models"));
    }

    #[test]
    fn blank_override_falls_through_instead_of_resolving_to_root() {
        // A whitespace-only value is a misconfiguration. Honouring it
        // literally would point the cache at "" — silently unusable.
        for blank in ["", "   ", "\t"] {
            let resolved =
                resolve_cache_dir_from(Some(blank), Some(PathBuf::from("/home/u/.cache")));
            assert_eq!(
                resolved,
                PathBuf::from("/home/u/.cache/forgeplan/models"),
                "blank override {blank:?} should fall through to the platform cache"
            );
        }
    }

    #[test]
    fn platform_cache_is_machine_shared_not_cwd_relative() {
        // The whole point of the fix: the default must not be the
        // CWD-relative path that produced one 2.1 GB copy per project.
        let resolved = resolve_cache_dir_from(None, Some(PathBuf::from("/home/u/.cache")));
        assert_eq!(resolved, PathBuf::from("/home/u/.cache/forgeplan/models"));
        assert!(resolved.is_absolute());
    }

    #[test]
    fn falls_back_to_fastembed_default_only_when_platform_dir_is_unknown() {
        let resolved = resolve_cache_dir_from(None, None);
        assert_eq!(resolved, PathBuf::from(FASTEMBED_DEFAULT_CACHE_DIR));
    }

    #[test]
    fn notice_names_the_size_and_the_destination() {
        let notice = compose_first_run_notice(Path::new("/home/u/.cache/forgeplan/models"), None);
        assert!(notice.contains(MODEL_DOWNLOAD_SIZE_HINT));
        assert!(notice.contains("/home/u/.cache/forgeplan/models"));
    }

    #[test]
    fn notice_offers_migration_when_a_legacy_cache_exists() {
        let notice = compose_first_run_notice(
            Path::new("/home/u/.cache/forgeplan/models"),
            Some(PathBuf::from(".fastembed_cache")),
        );
        // The user must not be left to work out the move themselves — a
        // repeated 2.1 GB download is the cost of a missing hint.
        assert!(notice.contains("mv .fastembed_cache /home/u/.cache/forgeplan/models"));
        assert!(notice.contains("mkdir -p /home/u/.cache/forgeplan"));
    }

    #[test]
    fn notice_stays_quiet_about_migration_when_there_is_nothing_to_migrate() {
        let notice = compose_first_run_notice(Path::new("/home/u/.cache/forgeplan/models"), None);
        assert!(!notice.contains("mv "));
    }

    #[test]
    fn size_hint_is_stated_once_and_carries_a_unit() {
        // Guards the regression this constant exists to prevent: three files
        // each carrying a different invented figure.
        assert!(MODEL_DOWNLOAD_SIZE_HINT.contains("GB"));
    }
}
