//! Metadata-only tokenizer loading and the immutable section layout.
//!
//! ADR-029 §2A.1–§2A.4. This module exists because the tokenizer is needed by
//! capture **without** the inference engine: section windows are measured in
//! tokens, so capture must know where token boundaries fall. Loading them
//! through [`crate::embed::CachedSnapshot`] would have made every capture of a
//! `no-semantic` build depend on 2.1 GB of ONNX weights and would build a tract
//! plan nobody asked for.
//!
//! # What this module may read
//!
//! Exactly three files:
//!
//! * `tokenizer.json` — the vocabulary and pre-tokenizer;
//! * `tokenizer_config.json` — `model_max_length`, `pad_token`;
//! * `config.json` — `pad_token_id`.
//!
//! Not `onnx/model.onnx`, not `onnx/model.onnx_data`, not
//! `special_tokens_map.json`. No network, no download, no
//! [`crate::embed::CachedSnapshot`], no tract plan. Weights and download stay
//! `semantic-search`-only; this loader is compiled in **both** feature
//! configurations.
//!
//! # What a half-fetched directory is
//!
//! Not a snapshot. [`find_metadata_snapshot`] only accepts a directory that
//! holds all three metadata files, and [`MetadataTokenizer::load`] re-reads
//! and hashes them itself. A directory with one stray `tokenizer.json` from an
//! interrupted fetch resolves to the fingerprinted `semantic_unavailable`
//! state, never to a half-usable layout.
//!
//! # Why one `Arc` per capture
//!
//! Parsing `tokenizer.json` costs tens of milliseconds and produces an
//! immutable value. Re-parsing it per artifact or per job would make capture
//! time scale with corpus size for no reason, so a capture holds one
//! [`Arc<MetadataTokenizer>`] and the resolved [`SectionLayout`] beside it,
//! and every window/builder call borrows that.
//!
//! # The layout fingerprint
//!
//! A length-prefixed, domain-separated SHA-256 over exactly five things:
//! the algorithm version, the three metadata checksums, the **actual** token
//! limit, and the header-context limit. It deliberately contains no weights
//! and no raw source digest — those are separate quantities with separate
//! purposes (`profileId` and `sourceDigest`/`inputDigest`), and folding them
//! in here would make a layout change look like a source change.
//!
//! Absence of metadata is a **separate fingerprinted state**, not a missing
//! field: [`SectionLayout::unavailable`] carries its own fingerprint, so a
//! capture without a tokenizer can never hash equal to a capture with one.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
// `added_tokens` is a method of the trait, not an inherent one on `Tokenizer`;
// the handle `get_post_processor` returns is `&dyn PostProcessor`, so without
// this the call does not resolve.
use tokenizers::tokenizer::PostProcessor as _;

/// Version tag of the windowing + input-building algorithm.
///
/// Part of the layout fingerprint, so changing how windows are cut changes the
/// identity of the layout rather than silently reusing `#sN` keys under a
/// previous one. Bump this on any change to window boundaries, header
/// budgeting, or the assembled input shape.
pub const SECTION_LAYOUT_ALGORITHM: &str = "s1-token-layout-v2";

/// Wire string for the versioned domain separator of the layout fingerprint.
pub const SECTION_LAYOUT_DOMAIN: &str = "forgeplan.constellation.section-layout.v1";

/// Ceiling on the **actual** token limit.
///
/// ADR-029 §3: the fingerprint records `actual max_tokens` and it is never
/// above the historical cap of 512. BGE-M3 declares 8192, so 512 wins; a model
/// with a smaller own limit wins over it.
pub const MAX_TOKENS_CEILING: usize = 512;

/// Token budget for the summed `title + heading` context of one section input.
///
/// Only the **embedding input** is capped by this. The display title and
/// heading stored in a section row are the full captured strings and are never
/// shortened.
pub const HEADER_CONTEXT_TOKENS: usize = 128;

/// Head-room reserved for boundary merges.
///
/// A tokenizer that merges across the header/body join can need one more token
/// than the two parts need separately, and padding to a boundary can cost
/// another. This reserve absorbs that instead of letting the final check
/// fail on inputs that are within budget on paper.
const BOUNDARY_RESERVE_TOKENS: usize = 8;

/// The three metadata files a tokenizer-only load requires.
///
/// `onnx/*` is deliberately absent. `special_tokens_map.json` is absent
/// because nothing here reads it — `tokenizer_config.json` and `config.json`
/// carry the two values the loader needs (`model_max_length`, `pad_token`,
/// `pad_token_id`).
pub const REQUIRED_METADATA_FILES: [&str; 3] =
    ["tokenizer.json", "tokenizer_config.json", "config.json"];

/// SHA-256 of one metadata file's bytes, lowercase hex.
fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push(char::from_digit((byte >> 4) as u32, 16).expect("nibble is a hex digit"));
        out.push(char::from_digit((byte & 0x0f) as u32, 16).expect("nibble is a hex digit"));
    }
    out
}

/// The immutable token layout a capture was built under.
///
/// Five fields, all of them hashed. `max_tokens` is the **actual** limit in
/// force (`min(model_max_length, 512)`), not a constant restated at each call
/// site — that divergence is what let a 2000-character window lose its tail at
/// encode time while the capture believed it was covered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionLayout {
    /// [`SECTION_LAYOUT_ALGORITHM`].
    pub algorithm: String,
    pub tokenizer_checksum: String,
    pub tokenizer_config_checksum: String,
    pub config_checksum: String,
    /// The effective token limit. Never above [`MAX_TOKENS_CEILING`].
    pub max_tokens: usize,
    /// The summed title+heading context budget.
    pub header_context_tokens: usize,
    /// Number of tokens the tokenizer adds around any sequence (BOS/EOS).
    /// Part of every window's arithmetic, so it is resolved once here rather
    /// than re-derived per artifact.
    pub special_tokens: usize,
}

impl SectionLayout {
    /// Build a layout from verified metadata, resolving the actual limit.
    ///
    /// `model_max_length` is the model's own declared ceiling. A value that is
    /// absent, non-numeric or absurdly large falls back to the ceiling; a
    /// value smaller than the special-token budget plus one body token is
    /// **not** silently accepted — [`resolve_token_budgets`] reports it, and
    /// the caller records `semantic_unavailable` rather than producing
    /// windows that cannot hold a single token.
    pub fn from_metadata(
        tokenizer_checksum: String,
        tokenizer_config_checksum: String,
        config_checksum: String,
        model_max_length: Option<usize>,
        special_tokens: usize,
    ) -> Self {
        let model_limit = model_max_length.filter(|limit| *limit > 0);
        let max_tokens = match model_limit {
            Some(limit) => limit.min(MAX_TOKENS_CEILING),
            None => MAX_TOKENS_CEILING,
        };
        Self {
            algorithm: SECTION_LAYOUT_ALGORITHM.to_string(),
            tokenizer_checksum,
            tokenizer_config_checksum,
            config_checksum,
            max_tokens,
            header_context_tokens: HEADER_CONTEXT_TOKENS,
            special_tokens,
        }
    }

    /// The layout of a workspace with no usable tokenizer metadata.
    ///
    /// A **value**, not `None`: the absence has its own fingerprint, so a
    /// capture taken while metadata was missing can never compare equal to one
    /// taken with metadata present, and its absence can never be mistaken for
    /// a second windowing of the same bytes.
    pub fn unavailable(reason: &str) -> Self {
        Self {
            algorithm: SECTION_LAYOUT_ALGORITHM.to_string(),
            tokenizer_checksum: format!("semantic_unavailable:{reason}"),
            tokenizer_config_checksum: format!("semantic_unavailable:{reason}"),
            config_checksum: format!("semantic_unavailable:{reason}"),
            max_tokens: 0,
            header_context_tokens: HEADER_CONTEXT_TOKENS,
            special_tokens: 0,
        }
    }

    /// Whether a tokenizer actually backs this layout.
    ///
    /// `false` means the fingerprinted `semantic_unavailable` state: there are
    /// no ready semantic sections, and the publisher's section set is exactly
    /// empty.
    pub fn is_available(&self) -> bool {
        self.max_tokens > 0
    }

    /// Domain-separated, length-prefixed identity of this layout.
    pub fn fingerprint(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update((SECTION_LAYOUT_DOMAIN.len() as u64).to_be_bytes());
        hasher.update(SECTION_LAYOUT_DOMAIN.as_bytes());
        let mut field = |bytes: &[u8]| {
            hasher.update((bytes.len() as u64).to_be_bytes());
            hasher.update(bytes);
        };
        field(self.algorithm.as_bytes());
        field(self.tokenizer_checksum.as_bytes());
        field(self.tokenizer_config_checksum.as_bytes());
        field(self.config_checksum.as_bytes());
        field(self.max_tokens.to_string().as_bytes());
        field(self.header_context_tokens.to_string().as_bytes());
        hex(&hasher.finalize())
    }
}

/// The token budgets a window and an assembled input may use.
///
/// Derived once from a [`SectionLayout`], so capture, digest, admission and
/// worker recovery all read the same numbers instead of each re-deriving them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenBudgets {
    /// The full admissible input, special tokens included.
    pub max_tokens: usize,
    /// How many of them the tokenizer spends on special tokens.
    pub special_tokens: usize,
    /// Ceiling on the summed title+heading context.
    pub header_context_tokens: usize,
    /// Tokens available to the body once the header cap is reserved.
    pub body_tokens: usize,
}

impl TokenBudgets {
    /// Resolve the budgets for `layout`, or explain why none exist.
    ///
    /// The header cap shrinks when `max_tokens` is small, so a profile with a
    /// smaller model limit still advances through the body instead of
    /// spending the whole budget on the title. What it never does is return a
    /// body budget of zero: a layout that cannot hold one body token has no
    /// honest windowing, and the caller records `semantic_unavailable` rather
    /// than emitting empty windows forever.
    pub fn resolve(layout: &SectionLayout) -> Result<Self> {
        let special_tokens = layout.special_tokens;
        let max_tokens = layout.max_tokens;
        if !layout.is_available() {
            anyhow::bail!(
                "layout {} has no usable tokenizer metadata, so it has no token budget",
                layout.fingerprint()
            );
        }
        let available = max_tokens.saturating_sub(special_tokens + BOUNDARY_RESERVE_TOKENS);
        if available == 0 {
            anyhow::bail!(
                "actual max_tokens {max_tokens} leaves no room after {special_tokens} special \
                 tokens; this profile cannot hold a single body token"
            );
        }
        // At least one token must always remain for the body.
        let header_context_tokens = layout
            .header_context_tokens
            .min(available.saturating_sub(1));
        let body_tokens = available - header_context_tokens;
        Ok(Self {
            max_tokens,
            special_tokens,
            header_context_tokens,
            body_tokens,
        })
    }
}

/// One contiguous UTF-8 byte range of a captured body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteRange {
    pub start: usize,
    pub end: usize,
}

impl ByteRange {
    /// The range as a half-open slice of `body`, or `None` if it is not a
    /// valid boundary pair.
    pub fn slice<'a>(&self, body: &'a str) -> Option<&'a str> {
        body.get(self.start..self.end)
    }

    /// The number of bytes covered.
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Whether the range covers no bytes.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// One loaded tokenizer plus the checksums that identify it.
///
/// Immutable after construction and shared by `Arc` for the whole capture: the
/// tokenizer is parsed once, and every window and input builder borrows this.
#[derive(Debug)]
pub struct MetadataTokenizer {
    tokenizer: tokenizers::Tokenizer,
    layout: SectionLayout,
    snapshot_root: PathBuf,
}

impl MetadataTokenizer {
    /// Load the three metadata files under `snapshot` and resolve the layout.
    ///
    /// No weights, no graph, no network. Every failure names the file that
    /// failed rather than collapsing into "model unavailable".
    pub fn load(snapshot: &Path) -> Result<Self> {
        let mut checksums: Vec<(String, String)> = Vec::with_capacity(3);
        for relative in REQUIRED_METADATA_FILES {
            let path = snapshot.join(relative);
            let bytes = std::fs::read(&path)
                .with_context(|| format!("cannot read tokenizer metadata {}", path.display()))?;
            checksums.push((relative.to_string(), sha256_hex(&bytes)));
        }
        let checksum_of = |name: &str| {
            checksums
                .iter()
                .find(|(file, _)| file == name)
                .map(|(_, sum)| sum.clone())
                .unwrap_or_default()
        };

        let tokenizer_config = read_json(&snapshot.join("tokenizer_config.json"))?;
        let config = read_json(&snapshot.join("config.json"))?;

        // Read the special-token budget straight off the post-processor, because
        // it is part of the layout's identity: the tokens the processor adds
        // around every sequence are exactly what the window budget must
        // reserve.
        //
        // Read from the template rather than measured by encoding a probe
        // string. An encode-and-count probe reports the wrapper **plus the
        // probe's own tokens** — a single space costs one ordinary token, so
        // the probe reported one token more than the wrapper actually adds,
        // and every window was understated by that much. It also conflated two
        // questions: how many tokens does the post-processor add, and how does
        // this pre-tokenizer treat a particular character. Only the first is the
        // budget.
        //
        // A tokenizer with no post-processor adds nothing, and `map_or` gives
        // that the same honest `0` the old probe gave a degenerate tokenizer.
        let tokenizer = tokenizers::Tokenizer::from_file(snapshot.join("tokenizer.json"))
            .map_err(|e| anyhow::anyhow!("cannot load tokenizer.json: {e}"))?;
        let special_tokens = tokenizer
            .get_post_processor()
            .map_or(0, |processor| processor.added_tokens(false));

        let model_max_length = tokenizer_config
            .get("model_max_length")
            .and_then(|value| value.as_f64())
            .filter(|value| value.is_finite() && *value > 0.0)
            .map(|value| value as usize);

        let layout = SectionLayout::from_metadata(
            checksum_of("tokenizer.json"),
            checksum_of("tokenizer_config.json"),
            checksum_of("config.json"),
            model_max_length,
            special_tokens,
        );

        // Pad configuration is not applied here: this tokenizer is only ever
        // used to measure offsets and count tokens, never to produce a batch.
        // The engine's own loader applies padding and truncation for the one
        // place that actually runs inference.
        let _ = config;

        Ok(Self {
            tokenizer,
            layout,
            snapshot_root: snapshot.to_path_buf(),
        })
    }

    /// The layout this tokenizer produces.
    pub fn layout(&self) -> &SectionLayout {
        &self.layout
    }

    /// The snapshot directory the metadata was read from.
    pub fn snapshot_root(&self) -> &Path {
        &self.snapshot_root
    }

    /// A copy of the underlying tokenizer, for the one caller that has to
    /// configure it for inference.
    ///
    /// A clone on purpose: `with_padding` / `with_truncation` consume and
    /// return a **new** tokenizer, and mutating the shared measurement value
    /// would make a capture's window arithmetic and the engine's encode path
    /// use different configurations from the same object.
    pub fn into_tokenizer(self) -> tokenizers::Tokenizer {
        self.tokenizer
    }

    /// Token counts and offsets for `text` with **no** truncation, **no**
    /// padding and **no** special tokens.
    ///
    /// This is the measurement the whole layout rests on: window boundaries
    /// come from these offsets, and the assembled input's final check counts
    /// with special tokens. Asking for a truncated encoding here would make
    /// the measured window and the embedded window different things.
    pub fn measure(&self, text: &str) -> Result<MeasuredEncoding> {
        let encoding = self
            .tokenizer
            .encode(text, false)
            .map_err(|e| anyhow::anyhow!("tokenization failed: {e}"))?;
        Ok(MeasuredEncoding {
            ids: encoding.get_ids().to_vec(),
            offsets: encoding.get_offsets().to_vec(),
        })
    }

    /// Total token count of `text` as the engine will see it: special tokens
    /// included.
    pub fn count_with_special_tokens(&self, text: &str) -> Result<usize> {
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| anyhow::anyhow!("tokenization failed: {e}"))?;
        Ok(encoding.get_ids().len())
    }

    /// Contiguous token windows covering `content` exactly.
    ///
    /// Every byte of `content` lands in exactly one window: the windows are
    /// cut at token boundaries but **extended** so they tile the input with no
    /// gap and no overlap, which is what keeps inter-window whitespace and
    /// metaspaces from being dropped. The `(0, 0)` offsets a tokenizer emits
    /// for inserted special tokens are never used as a boundary.
    ///
    /// An empty input yields exactly one empty window, so "the body was
    /// empty" stays distinguishable from "no windows were produced".
    pub fn token_windows(&self, content: &str, budgets: &TokenBudgets) -> Result<Vec<ByteRange>> {
        if content.is_empty() {
            return Ok(vec![ByteRange { start: 0, end: 0 }]);
        }
        let measured = self.measure(content)?;
        let body_tokens = budgets.body_tokens.max(1);

        // Real boundaries only. An inserted special token reports `(0, 0)`,
        // which is not a place in this text; keeping it would cut a window at
        // offset 0 and re-emit the head of the section.
        let mut starts: Vec<usize> = Vec::with_capacity(measured.offsets.len());
        let mut cursor = 0usize;
        for (start, end) in &measured.offsets {
            if *start == 0 && *end == 0 {
                continue;
            }
            // Monotonic by construction, and clamped into the text, so a
            // tokenizer reporting a shrinking offset cannot produce windows
            // that walk backwards and silently drop bytes.
            let byte = (*start).clamp(cursor, content.len());
            cursor = cursor.max(*end).min(content.len());
            starts.push(byte);
        }
        if starts.is_empty() {
            // Content that tokenizes to nothing usable (pure control bytes, or
            // a normalizer that removed everything). It still has to be
            // covered exactly once.
            return Ok(vec![ByteRange {
                start: 0,
                end: content.len(),
            }]);
        }

        let mut windows: Vec<ByteRange> = Vec::new();
        let mut index = 0usize;
        while index < starts.len() {
            let next = (index + body_tokens).min(starts.len());
            // The **first** window starts at 0, not at the first token's
            // offset: a tokenizer that drops leading whitespace — or a normalizer
            // that rewrites it — would otherwise leave the gap between byte 0
            // and `starts[0]` owned by no window at all, and the body could not
            // be reproduced.
            let start = if index == 0 { 0 } else { starts[index] };
            // Each window ends where the next one begins, so the windows tile
            // the input exactly: no gap loses the whitespace or metaspaces
            // between them, and no overlap double-embeds a byte.
            let end = if next >= starts.len() {
                content.len()
            } else {
                starts[next].max(start)
            };
            windows.push(ByteRange { start, end });
            index = next;
        }
        Ok(windows)
    }

    /// Assemble the one embedding input for a section window.
    ///
    /// `title` and `heading` are the **full display strings**; only the copy
    /// that enters this text is token-budgeted. The result is verified against
    /// the actual limit with special tokens included, and if the join costs
    /// more than the reserve allowed, the body part is trimmed at its last
    /// token boundary until it fits. Trimming loses no corpus content: the
    /// trimmed bytes are the first bytes of the next window, which is itself
    /// embedded with the same header.
    pub fn build_section_input(
        &self,
        title: &str,
        heading: Option<&str>,
        window: &str,
        budgets: &TokenBudgets,
    ) -> Result<String> {
        let header = self.build_header(title, heading, budgets);
        let mut body = window.to_string();
        loop {
            let candidate = if header.is_empty() {
                body.clone()
            } else {
                format!("{header} {body}")
            };
            let count = self.count_with_special_tokens(&candidate)?;
            if count <= budgets.max_tokens {
                return Ok(candidate);
            }
            match self.trim_tail(&body) {
                Some(trimmed) => body = trimmed,
                None => anyhow::bail!(
                    "a single body token does not fit the actual token limit {} with the header \
                     context; this profile cannot embed this section",
                    budgets.max_tokens
                ),
            }
        }
    }

    /// The token-budgeted `title + heading` context, without the separator.
    fn build_header(&self, title: &str, heading: Option<&str>, budgets: &TokenBudgets) -> String {
        let full = match heading {
            Some(heading) if !heading.is_empty() => format!("{title} {heading}"),
            _ => title.to_string(),
        };
        if budgets.header_context_tokens == 0 {
            return String::new();
        }
        self.truncate_to_tokens(&full, budgets.header_context_tokens)
            .unwrap_or(full)
    }

    /// The longest prefix of `text` that tokenizes to at most `limit` tokens.
    fn truncate_to_tokens(&self, text: &str, limit: usize) -> Option<String> {
        if text.is_empty() {
            return Some(String::new());
        }
        let measured = self.measure(text).ok()?;
        if measured.ids.len() <= limit {
            return Some(text.to_string());
        }
        let end = boundary_start(&measured.offsets, limit, text.len());
        if end == 0 {
            return None;
        }
        text.get(..end).map(str::to_string)
    }

    /// `text` without its last token, or `None` when a single token is all
    /// that is left.
    fn trim_tail(&self, text: &str) -> Option<String> {
        if text.is_empty() {
            return None;
        }
        let measured = self.measure(text).ok()?;
        if measured.ids.len() <= 1 {
            return None;
        }
        let end = boundary_start(&measured.offsets, measured.ids.len() - 1, text.len());
        if end == 0 {
            return None;
        }
        text.get(..end).map(str::to_string)
    }
}

/// One untruncated, unpadded, special-token-free encoding.
#[derive(Debug, Clone)]
pub struct MeasuredEncoding {
    /// Token ids in order.
    pub ids: Vec<u32>,
    /// `(start, end)` UTF-8 byte offsets into the measured text.
    pub offsets: Vec<(usize, usize)>,
}

/// The byte offset that starts token `index`, clamped into `text`.
///
/// `(0, 0)` — what a tokenizer reports for a token it inserted rather than
/// read — is never a window boundary, and neither is an offset that would walk
/// backwards. The result is non-decreasing, which is what lets the windows
/// tile the input exactly.
fn boundary_start(offsets: &[(usize, usize)], index: usize, text_len: usize) -> usize {
    let mut fallback = 0usize;
    for (position, (start, end)) in offsets.iter().enumerate() {
        if position >= index {
            if *start == 0 && *end == 0 {
                // Inserted special token: skip it and keep looking for a
                // real boundary, but never below what we have already passed.
                fallback = fallback.max(*end);
                continue;
            }
            return (*start).clamp(fallback, text_len);
        }
        fallback = fallback.max(*end);
    }
    text_len
}

/// Find a snapshot directory holding all three metadata files.
///
/// The HuggingFace cache layout is `models--ORG--NAME/snapshots/<rev>/`, and
/// `refs/main` names the resolved revision when it exists. A directory with
/// fewer than [`REQUIRED_METADATA_FILES`] is **not** a snapshot, whatever
/// else it contains — that is the "random half-downloaded directory" case the
/// old `find_snapshot` accepted because it only looked for `tokenizer.json`.
pub fn find_metadata_snapshot(cache_dir: &Path, repo: &str) -> Option<PathBuf> {
    let repo_dir = cache_dir.join(format!("models--{}", repo.replace('/', "--")));
    let revision = std::fs::read_to_string(repo_dir.join("refs").join("main"))
        .ok()
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty());
    if let Some(revision) = revision {
        let candidate = repo_dir.join("snapshots").join(&revision);
        if has_all_metadata(&candidate) {
            return Some(candidate);
        }
    }
    let snapshots = repo_dir.join("snapshots");
    std::fs::read_dir(snapshots)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| has_all_metadata(path))
}

/// Whether `dir` holds every metadata file.
pub fn has_all_metadata(dir: &Path) -> bool {
    REQUIRED_METADATA_FILES
        .iter()
        .all(|relative| dir.join(relative).is_file())
}

fn read_json(path: &Path) -> Result<serde_json::Value> {
    let raw =
        std::fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("cannot parse {}", path.display()))
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from_digit((byte >> 4) as u32, 16).expect("nibble is a hex digit"));
        out.push(char::from_digit((byte & 0x0f) as u32, 16).expect("nibble is a hex digit"));
    }
    out
}

/// How a capture's tokenizer resolution ended.
///
/// A workspace without usable metadata is a **state**, not an error: lexical
/// search, graph expansion and pure scoring all work, and the semantic channel
/// reports `semantic_unavailable` with this fingerprint rather than
/// substituting character windows under the previous layout identity.
#[derive(Debug, Clone)]
pub enum LayoutResolution {
    /// Metadata resolved; sections are measurable and embeddable.
    Available(Arc<MetadataTokenizer>),
    /// No usable metadata. The layout still has a fingerprint.
    Unavailable(SectionLayout),
}

impl LayoutResolution {
    /// The layout either way — always a value.
    pub fn layout(&self) -> SectionLayout {
        match self {
            Self::Available(tokenizer) => tokenizer.layout().clone(),
            Self::Unavailable(layout) => layout.clone(),
        }
    }

    /// The shared tokenizer, when one resolved.
    pub fn tokenizer(&self) -> Option<&Arc<MetadataTokenizer>> {
        match self {
            Self::Available(tokenizer) => Some(tokenizer),
            Self::Unavailable(_) => None,
        }
    }

    /// The fingerprint of the resolved (or explicitly unavailable) layout.
    pub fn fingerprint(&self) -> String {
        self.layout().fingerprint()
    }
}

/// Resolve the layout for `repo` from a local cache directory.
///
/// **Never downloads.** An absent `refs/main`, an absent snapshot, or a
/// snapshot missing any metadata file resolves to
/// [`LayoutResolution::Unavailable`] with the reason recorded in the
/// fingerprint, which is what a no-semantic build and a semantic build with no
/// cached model both legitimately report.
pub fn resolve_layout_in(cache_dir: &Path, repo: &str) -> LayoutResolution {
    let Some(snapshot) = find_metadata_snapshot(cache_dir, repo) else {
        return LayoutResolution::Unavailable(SectionLayout::unavailable(&format!(
            "no snapshot under {} holds all of {REQUIRED_METADATA_FILES:?}",
            cache_dir.display()
        )));
    };
    match MetadataTokenizer::load(&snapshot) {
        Ok(tokenizer) => LayoutResolution::Available(Arc::new(tokenizer)),
        Err(error) => LayoutResolution::Unavailable(SectionLayout::unavailable(&format!(
            "{snapshot:?}: {error:#}"
        ))),
    }
}

/// Resolve the layout from the process-wide model cache.
pub fn resolve_layout(repo: &str) -> LayoutResolution {
    // A test that must observe a *specific* layout sets one explicitly rather
    // than depending on whatever this machine happens to have downloaded.
    // Without this, a capture test that needs sections panics on a clean
    // machine and quietly changes meaning on one with a model cache — the
    // ambient-state dependency this seam exists to remove.
    #[cfg(any(test, all(feature = "test-helpers", debug_assertions)))]
    if let Some(override_dir) = test_layout_cache::get() {
        return resolve_layout_in(&override_dir, repo);
    }

    resolve_layout_in(&crate::embed::resolve_cache_dir(), repo)
}

/// A test-supplied model cache directory, consulted only by [`resolve_layout`].
///
/// Deliberately **not** an environment variable. The fault barrier needs an
/// env var because a *child process* has to be told where its parent's
/// listener is; here the caller is in the same process, and an env var would
/// be global state that leaks into every other test running on another thread
/// and that a panicking test leaves behind. A value a test sets and clears
/// under its own lock cannot do either.
///
/// Deliberately **not** consulted by [`resolve_layout_in`], which already
/// takes the directory as an argument. The absent/present/absent transition
/// tests need to resolve against directories they name explicitly, and must
/// keep working while some other test holds an override.
///
/// Compiled out of every production build, so the lookup does not exist in a
/// shipped binary.
#[cfg(any(test, all(feature = "test-helpers", debug_assertions)))]
mod test_layout_cache {
    use std::path::PathBuf;
    use std::sync::{LazyLock, Mutex};

    static OVERRIDE: LazyLock<Mutex<Option<PathBuf>>> = LazyLock::new(|| Mutex::new(None));

    /// A poisoned mutex means some earlier test panicked while holding it. The
    /// value is a plain `Option<PathBuf>`, so recovering is safe and refusing
    /// would only cascade one failure into every later test.
    fn lock() -> std::sync::MutexGuard<'static, Option<PathBuf>> {
        OVERRIDE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn get() -> Option<PathBuf> {
        lock().clone()
    }

    pub fn set(dir: Option<PathBuf>) {
        *lock() = dir;
    }
}

/// Point [`resolve_layout`] at `dir` until [`clear_test_layout_cache`].
///
/// Returns the previous value so a caller can restore it. Tests must hold
/// whatever lock serialises them, because this is process-global.
#[cfg(any(test, all(feature = "test-helpers", debug_assertions)))]
pub fn set_test_layout_cache(dir: Option<std::path::PathBuf>) -> Option<std::path::PathBuf> {
    let previous = test_layout_cache::get();
    test_layout_cache::set(dir);
    previous
}

/// Forget any directory installed by [`set_test_layout_cache`].
#[cfg(any(test, all(feature = "test-helpers", debug_assertions)))]
pub fn clear_test_layout_cache() {
    test_layout_cache::set(None);
}

/// Hermetic tokenizer fixtures.
///
/// A section window is a claim about **token** offsets, so testing it needs a
/// tokenizer. Requiring the 2.1 GB BGE-M3 snapshot for that would make every
/// windowing test a model-download test: it would pass or fail on whether this
/// machine had run `forgeplan index` first, and a failure would say nothing
/// about the code under test. These fixtures build a **real**
/// `tokenizers::Tokenizer`, write it out through the crate's own serializer,
/// and load it back through [`MetadataTokenizer::load`] — so the loader, the
/// checksums, the layout resolution and the window arithmetic are all
/// exercised, with no network, no weights and no ONNX plan.
///
/// The tokenizer is deliberately **character-splitting** (`Split` on `.`,
/// isolated): it produces a token per character for Latin, Cyrillic, CJK,
/// emoji and whitespace alike. That is the harshest case for window
/// boundaries, because the interesting bugs live in how a byte range is cut
/// around multi-byte characters and in the whitespace between tokens — and it
/// makes "these windows reproduce the body byte for byte" a statement about
/// many windows rather than a single one.
///
/// Gated exactly like this crate's other test-only surface, so a release build
/// never carries it and the `cargo test --workspace` baseline (no
/// `test-helpers`) skips the integration targets that use it instead of
/// failing to compile.
#[cfg(any(test, all(feature = "test-helpers", debug_assertions)))]
pub mod testing {
    use super::{MetadataTokenizer, SectionLayout, TokenBudgets};
    use std::path::Path;
    use tokenizers::SplitDelimiterBehavior;
    use tokenizers::models::wordlevel::WordLevel;
    use tokenizers::pre_tokenizers::split::{Split, SplitPattern};
    use tokenizers::pre_tokenizers::whitespace::WhitespaceSplit;
    use tokenizers::processors::template::TemplateProcessing;
    use tokenizers::tokenizer::{AddedToken, Tokenizer};

    /// Id the post-processor's `[CLS]` is emitted as.
    const CLS_ID: u32 = 1;
    /// Id the post-processor's `[SEP]` is emitted as.
    const SEP_ID: u32 = 2;

    /// The vocabulary a fixture tokenizer knows, one token per character.
    ///
    /// The three marker tokens come first so their ids are fixed; every other
    /// character follows in a fixed order, which makes the ids — and therefore
    /// the whole `tokenizer.json` — byte-identical between runs.
    fn fixture_vocabulary() -> Vec<String> {
        let mut tokens: Vec<String> = vec![
            "[UNK]".to_string(),
            "[CLS]".to_string(),
            "[SEP]".to_string(),
        ];
        // Not `mut`: the closure takes the vector as an argument and captures
        // nothing, so it never needs a mutable binding of its own.
        let push = |candidate: char, tokens: &mut Vec<String>| {
            let token = candidate.to_string();
            if !tokens.contains(&token) {
                tokens.push(token);
            }
        };

        // Printable ASCII, including the space, plus the two whitespace
        // characters `.` does not match — they are exactly the bytes a naive
        // splitter drops between windows.
        for byte in 0x20u32..=0x7Eu32 {
            if let Some(c) = char::from_u32(byte) {
                push(c, &mut tokens);
            }
        }
        for c in ['\n', '\t', '\r'] {
            push(c, &mut tokens);
        }

        // Cyrillic, which is what the project's own artifacts are written in.
        for c in 'а'..='я' {
            push(c, &mut tokens);
        }
        for c in 'А'..='Я' {
            push(c, &mut tokens);
        }
        for c in ['ё', 'Ё'] {
            push(c, &mut tokens);
        }

        // CJK: no whitespace to split on, so a character window is the only
        // thing that can produce more than one window at all.
        for c in "一二三四五六七八九十百千万漢字日本語世界時間分秒点".chars()
        {
            push(c, &mut tokens);
        }

        // Emoji: four-byte characters, the case where a byte offset and a
        // character offset disagree most.
        for c in "😀🚀✅🔥🎉🧭📌🛠️🧪🌍🌙⭐".chars() {
            push(c, &mut tokens);
        }

        tokens
    }

    /// Build the tokenizer a fixture snapshot is saved from.
    ///
    /// Named `build_fixture_tokenizer`, not `fixture_tokenizer`: the public
    /// entry point below returns a *loaded* `MetadataTokenizer` plus its
    /// budgets, and Rust does not overload free functions, so one name cannot
    /// mean both. The split is also the useful one — this builds the value that
    /// gets written to disk, the other loads it back through the production
    /// loader.
    fn build_fixture_tokenizer() -> anyhow::Result<Tokenizer> {
        let tokens = fixture_vocabulary();
        let mut vocab = serde_json::Map::with_capacity(tokens.len());
        for (id, token) in tokens.iter().enumerate() {
            vocab.insert(token.clone(), serde_json::Value::from(id as u32));
        }
        // `WordLevel` reads a vocabulary from a plain `{"token": id}` file,
        // which keeps the id assignment in one place instead of spread over a
        // builder call.
        //
        // A private `TempDir` per call, not a shared name keyed by pid: the
        // test harness runs tests on threads in one process, and two fixtures
        // being built at once would otherwise share a path — one deleting the
        // file the other was about to read. The model has fully consumed the
        // file by the time `build()` returns, so the directory is dropped here.
        let vocab_dir = tempfile::TempDir::new()?;
        let vocab_path = vocab_dir.path().join("vocab.json");
        std::fs::write(
            &vocab_path,
            serde_json::to_vec(&serde_json::Value::Object(vocab))?,
        )?;

        // `tokenizers` returns `Box<dyn Error + Send + Sync>`, which does not
        // convert into `anyhow::Error` through `?`. Mapped explicitly at each
        // step, with the step named, so a build-time failure is distinguishable
        // from a save-time one — they have completely different causes (an
        // unreadable vocabulary file versus an unwritable directory) and would
        // otherwise both read as "the fixture did not build".
        let model = WordLevel::builder()
            .files(vocab_path.to_string_lossy().into_owned())
            .unk_token("[UNK]".to_string())
            .build()
            .map_err(|e| {
                anyhow::anyhow!("cannot build the fixture WordLevel model from its vocabulary: {e}")
            })?;

        let pre_tokenizer = Split::new(
            SplitPattern::Regex(".".to_string()),
            SplitDelimiterBehavior::Isolated,
            false,
        )
        .map_err(|e| anyhow::anyhow!("cannot build the fixture pre-tokenizer: {e}"))?;

        // Two special tokens around every sequence, so the layout's
        // `special_tokens` budget is a real measurement rather than a
        // hard-coded zero — the arithmetic that reserves room for them is the
        // thing under test.
        let post_processor = TemplateProcessing::builder()
            .try_single("[CLS] $A [SEP]")
            .map_err(|e| anyhow::anyhow!("cannot build the fixture template: {e}"))?
            .special_tokens(vec![
                ("[CLS]".to_string(), CLS_ID),
                ("[SEP]".to_string(), SEP_ID),
            ])
            .build()
            .map_err(|e| anyhow::anyhow!("cannot build the fixture post-processor: {e}"))?;

        let mut tokenizer = Tokenizer::new(model);
        tokenizer
            .with_pre_tokenizer(Some(pre_tokenizer))
            .with_post_processor(Some(post_processor));
        tokenizer
            .add_special_tokens([
                AddedToken::from("[CLS]", true),
                AddedToken::from("[SEP]", true),
            ])
            .map_err(|e| anyhow::anyhow!("cannot register the fixture special tokens: {e}"))?;
        Ok(tokenizer)
    }

    /// Write a complete, loadable tokenizer snapshot into `dir`.
    ///
    /// All three files [`MetadataTokenizer::load`] requires, and nothing else —
    /// no `onnx/`, no `refs/main`. That is deliberate: it proves the
    /// metadata-only loader is genuinely metadata-only, which is the property
    /// that keeps a `no-semantic` build from needing the weights.
    pub fn write_snapshot(dir: &Path, model_max_length: usize) -> anyhow::Result<()> {
        std::fs::create_dir_all(dir)?;
        // Same explicit mapping as `build_fixture_tokenizer`: the save target
        // is named so an unwritable directory is distinguishable from a model
        // that failed to build.
        build_fixture_tokenizer()?
            .save(dir.join("tokenizer.json"), false)
            .map_err(|e| anyhow::anyhow!("cannot write tokenizer.json: {e}"))?;

        write_snapshot_metadata(dir, model_max_length)
    }

    /// The two metadata files beside a saved `tokenizer.json`.
    ///
    /// Split out of [`write_snapshot`] so the whitespace-splitting variant
    /// below writes byte-identical metadata and differs from the per-character
    /// fixture in exactly one variable: its pre-tokenizer.
    fn write_snapshot_metadata(dir: &Path, model_max_length: usize) -> anyhow::Result<()> {
        std::fs::write(
            dir.join("tokenizer_config.json"),
            serde_json::to_vec(&serde_json::json!({
                "model_max_length": model_max_length,
                "pad_token": "[SEP]",
            }))?,
        )?;
        std::fs::write(
            dir.join("config.json"),
            serde_json::to_vec(&serde_json::json!({
                "pad_token_id": SEP_ID,
                "hidden_size": 8,
            }))?,
        )?;
        Ok(())
    }

    /// The vocabulary a whitespace-splitting fixture knows: whole words.
    ///
    /// `WordLevel` resolves a pre-token **whole** — a hit is one token spanning
    /// the word, a miss is `[UNK]` spanning the same bytes — so one word is
    /// always exactly one token. That is the property the window test needs:
    /// the boundaries are the word boundaries, and the only way a window can
    /// fail to start at byte 0 is if the leading whitespace is dropped.
    fn whitespace_fixture_vocabulary() -> Vec<String> {
        vec![
            "[UNK]".to_string(),
            "[CLS]".to_string(),
            "[SEP]".to_string(),
            "alpha".to_string(),
            "beta".to_string(),
        ]
    }

    /// The per-character fixture with its pre-tokenizer swapped for
    /// [`WhitespaceSplit`].
    ///
    /// Same model, same vocabulary shape, same post-processor, same ids — one
    /// variable changed, so a test that distinguishes them is attributing the
    /// difference to the pre-tokenizer and nothing else.
    fn build_whitespace_fixture_tokenizer() -> anyhow::Result<Tokenizer> {
        let tokens = whitespace_fixture_vocabulary();
        let mut vocab = serde_json::Map::with_capacity(tokens.len());
        for (id, token) in tokens.iter().enumerate() {
            vocab.insert(token.clone(), serde_json::Value::from(id as u32));
        }
        let vocab_dir = tempfile::TempDir::new()?;
        let vocab_path = vocab_dir.path().join("vocab.json");
        std::fs::write(
            &vocab_path,
            serde_json::to_vec(&serde_json::Value::Object(vocab))?,
        )?;

        let model = WordLevel::builder()
            .files(vocab_path.to_string_lossy().into_owned())
            .unk_token("[UNK]".to_string())
            .build()
            .map_err(|e| {
                anyhow::anyhow!("cannot build the whitespace fixture WordLevel model: {e}")
            })?;

        let post_processor = TemplateProcessing::builder()
            .try_single("[CLS] $A [SEP]")
            .map_err(|e| anyhow::anyhow!("cannot build the whitespace fixture template: {e}"))?
            .special_tokens(vec![
                ("[CLS]".to_string(), CLS_ID),
                ("[SEP]".to_string(), SEP_ID),
            ])
            .build()
            .map_err(|e| {
                anyhow::anyhow!("cannot build the whitespace fixture post-processor: {e}")
            })?;

        let mut tokenizer = Tokenizer::new(model);
        tokenizer
            .with_pre_tokenizer(Some(WhitespaceSplit))
            .with_post_processor(Some(post_processor));
        tokenizer
            .add_special_tokens([
                AddedToken::from("[CLS]", true),
                AddedToken::from("[SEP]", true),
            ])
            .map_err(|e| anyhow::anyhow!("cannot register the whitespace fixture tokens: {e}"))?;
        Ok(tokenizer)
    }

    /// Write a **whitespace-splitting** snapshot: the per-character fixture's
    /// twin whose pre-tokenizer drops whitespace between words.
    ///
    /// Kept beside [`write_snapshot`] rather than folded into it, because the
    /// per-character fixture is what every windowing test measures against and
    /// its byte-level boundaries must not move.
    pub fn write_whitespace_snapshot(dir: &Path, model_max_length: usize) -> anyhow::Result<()> {
        std::fs::create_dir_all(dir)?;
        build_whitespace_fixture_tokenizer()?
            .save(dir.join("tokenizer.json"), false)
            .map_err(|e| anyhow::anyhow!("cannot write whitespace tokenizer.json: {e}"))?;

        write_snapshot_metadata(dir, model_max_length)
    }

    /// A loaded whitespace-splitting fixture tokenizer and its budgets.
    ///
    /// Written to disk and read back through the production
    /// [`MetadataTokenizer::load`], exactly like the per-character fixture, so
    /// what the window test exercises is the value a real capture would hold.
    pub fn whitespace_fixture_tokenizer(
        model_max_length: usize,
    ) -> anyhow::Result<(tempfile::TempDir, MetadataTokenizer, TokenBudgets)> {
        let dir = tempfile::TempDir::new()?;
        write_whitespace_snapshot(dir.path(), model_max_length)?;
        let tokenizer = MetadataTokenizer::load(dir.path())?;
        let budgets = TokenBudgets::resolve(tokenizer.layout())?;
        Ok((dir, tokenizer, budgets))
    }

    /// The layout a fixture snapshot resolves to.
    pub fn fixture_layout(model_max_length: usize) -> anyhow::Result<SectionLayout> {
        let dir = tempfile::TempDir::new()?;
        write_snapshot(dir.path(), model_max_length)?;
        Ok(MetadataTokenizer::load(dir.path())?.layout().clone())
    }

    /// A loaded fixture tokenizer and the budgets its layout resolves to.
    ///
    /// The two are returned together because a test that measures windows
    /// against budgets from a *different* layout would be testing nothing; the
    /// layout is the budget's only source.
    pub fn fixture_tokenizer(
        model_max_length: usize,
    ) -> anyhow::Result<(tempfile::TempDir, MetadataTokenizer, TokenBudgets)> {
        let dir = tempfile::TempDir::new()?;
        write_snapshot(dir.path(), model_max_length)?;
        let tokenizer = MetadataTokenizer::load(dir.path())?;
        let budgets = TokenBudgets::resolve(tokenizer.layout())?;
        Ok((dir, tokenizer, budgets))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(max_tokens: usize, special: usize) -> SectionLayout {
        SectionLayout {
            algorithm: SECTION_LAYOUT_ALGORITHM.to_string(),
            tokenizer_checksum: "t".into(),
            tokenizer_config_checksum: "tc".into(),
            config_checksum: "c".into(),
            max_tokens,
            header_context_tokens: HEADER_CONTEXT_TOKENS,
            special_tokens: special,
        }
    }

    #[test]
    fn a_fingerprint_binds_every_declared_layout_input() {
        let base = layout(512, 2);
        assert_eq!(base.fingerprint().len(), 64);

        let mut changed_algorithm = base.clone();
        changed_algorithm.algorithm = "other-v1".into();
        assert_ne!(
            base.fingerprint(),
            changed_algorithm.fingerprint(),
            "the algorithm version is part of the layout identity"
        );

        let mut changed_tokenizer = base.clone();
        changed_tokenizer.tokenizer_checksum = "different".into();
        assert_ne!(base.fingerprint(), changed_tokenizer.fingerprint());

        let mut changed_config = base.clone();
        changed_config.config_checksum = "different".into();
        assert_ne!(base.fingerprint(), changed_config.fingerprint());

        let mut changed_limit = base.clone();
        changed_limit.max_tokens = 256;
        assert_ne!(
            base.fingerprint(),
            changed_limit.fingerprint(),
            "the ACTUAL token limit is part of the layout identity, not a restated constant"
        );

        let mut changed_header = base.clone();
        changed_header.header_context_tokens = 64;
        assert_ne!(base.fingerprint(), changed_header.fingerprint());
    }

    #[test]
    fn an_absent_tokenizer_is_its_own_fingerprinted_state() {
        let unavailable = SectionLayout::unavailable("no snapshot");
        assert!(!unavailable.is_available());
        let with_metadata = layout(512, 2);
        assert_ne!(
            unavailable.fingerprint(),
            with_metadata.fingerprint(),
            "absence must never hash equal to a layout under the previous identity"
        );
        // Two different reasons are two different states too.
        assert_ne!(
            unavailable.fingerprint(),
            SectionLayout::unavailable("half-downloaded").fingerprint()
        );
    }

    #[test]
    fn the_model_limit_never_exceeds_the_ceiling_and_a_smaller_one_wins() {
        let generous =
            SectionLayout::from_metadata("a".into(), "b".into(), "c".into(), Some(8192), 2);
        assert_eq!(generous.max_tokens, MAX_TOKENS_CEILING);

        let modest = SectionLayout::from_metadata("a".into(), "b".into(), "c".into(), Some(256), 2);
        assert_eq!(modest.max_tokens, 256);

        let undeclared = SectionLayout::from_metadata("a".into(), "b".into(), "c".into(), None, 2);
        assert_eq!(undeclared.max_tokens, MAX_TOKENS_CEILING);
    }

    #[test]
    fn a_smaller_model_limit_shrinks_the_header_so_the_body_still_advances() {
        let mut small = layout(64, 2);
        small.max_tokens = 64;
        let budgets = TokenBudgets::resolve(&small).unwrap();
        assert!(
            budgets.header_context_tokens < HEADER_CONTEXT_TOKENS,
            "a 64-token limit cannot spend 128 tokens on the header"
        );
        assert!(budgets.body_tokens > 0);
        assert!(budgets.header_context_tokens + budgets.body_tokens <= 64);
    }

    #[test]
    fn a_profile_that_cannot_hold_one_body_token_is_refused_not_looped() {
        let impossible = layout(2, 2);
        let error = TokenBudgets::resolve(&impossible).unwrap_err().to_string();
        assert!(error.contains("single body token"), "got: {error}");
    }

    #[test]
    fn an_empty_body_still_yields_exactly_one_empty_window() {
        // Through the real `token_windows`, not a hand-built `ByteRange`: the
        // contract is what the function returns, and constructing the answer
        // by hand would pass even if the function stopped returning it.
        let (_dir, tokenizer, budgets) = testing::fixture_tokenizer(MAX_TOKENS_CEILING).unwrap();
        let windows = tokenizer
            .token_windows("", &budgets)
            .expect("empty windows");

        assert_eq!(windows.len(), 1, "an empty body yields exactly one window");
        assert!(windows[0].is_empty(), "and that window covers no bytes");
        assert_eq!(windows[0].start, 0);
        assert_eq!(windows[0].end, 0);

        // The range helpers the capture relies on, kept next to it because
        // they are what turns a window into body bytes.
        assert!(ByteRange { start: 0, end: 0 }.is_empty());
        assert_eq!(ByteRange { start: 0, end: 0 }.len(), 0);
        assert_eq!(
            ByteRange { start: 3, end: 9 }.slice("abcdefghij"),
            Some("defghi")
        );
        assert_eq!(ByteRange { start: 0, end: 99 }.slice("abc"), None);
    }

    /// The first window starts at byte **0**, not at the first token's offset.
    ///
    /// The per-character fixture cannot catch this: it splits on `.` with
    /// `Isolated`, so it preserves leading whitespace and its first token
    /// already begins at byte 0. A fixture whose pre-tokenizer *drops*
    /// whitespace is the only one where `starts[0] != 0`, and that is the
    /// whole difference the assertion turns on — with the old
    /// `start = starts[index]`, the two leading spaces here would have been
    /// owned by no window and the body could not have been reproduced.
    #[test]
    fn a_tokenizer_that_drops_leading_whitespace_still_windows_from_byte_zero() {
        // 64 leaves a one-token body budget (64 − 2 special − 8 reserve = 54
        // available, header takes 53), so each word becomes exactly one window
        // and the boundaries below are the word boundaries.
        let (_dir, tokenizer, budgets) = testing::whitespace_fixture_tokenizer(64).unwrap();
        assert!(
            tokenizer.layout().is_available(),
            "fixture precondition: the whitespace snapshot resolves a real layout"
        );
        assert_eq!(
            budgets.body_tokens, 1,
            "the fixture must give a one-token body budget, or the expected boundaries would not \
             be the word boundaries"
        );

        // The premise, asserted rather than assumed: this tokenizer really does
        // start its first token at byte 2, which is what makes the window
        // boundaries below a real test instead of a tautology.
        let measured = tokenizer.measure("  alpha  beta \n").expect("measure");
        assert_eq!(
            measured.offsets,
            vec![(2, 7), (9, 13)],
            "`WhitespaceSplit` must drop the leading spaces and start at `alpha`"
        );

        let windows = tokenizer
            .token_windows("  alpha  beta \n", &budgets)
            .expect("windows");
        let actual: Vec<(usize, usize)> = windows.iter().map(|w| (w.start, w.end)).collect();
        assert_eq!(
            actual,
            vec![(0, 9), (9, 15)],
            "the first window must begin at byte 0; `2..9` would leave the two leading spaces \
             owned by nobody"
        );
        assert_eq!(
            windows[0].end, windows[1].start,
            "the windows must abut exactly — no gap, no overlap"
        );

        // A whitespace-only input tokenizes to nothing usable, and still has to
        // be covered exactly once, from 0 to its end.
        let blank = tokenizer
            .token_windows("\n \t\n", &budgets)
            .expect("blank windows");
        assert_eq!(
            blank.iter().map(|w| (w.start, w.end)).collect::<Vec<_>>(),
            vec![(0, 4)],
            "content that tokenizes to nothing must still be covered in full"
        );

        // And an empty input stays one empty window, the case the sibling test
        // above pins through the per-character fixture.
        let empty = tokenizer
            .token_windows("", &budgets)
            .expect("empty windows");
        assert_eq!(
            empty.iter().map(|w| (w.start, w.end)).collect::<Vec<_>>(),
            vec![(0, 0)]
        );

        // The loaded tokenizer is measured, never reconfigured: two identical
        // reads give identical offsets, which a mutated one could not.
        assert_eq!(
            tokenizer
                .measure("  alpha  beta \n")
                .expect("measure")
                .offsets,
            measured.offsets,
            "measuring must not reconfigure the shared tokenizer"
        );
    }

    #[test]
    fn a_boundary_skips_an_inserted_special_token() {
        let offsets = vec![(0, 0), (0, 4), (5, 9)];
        assert_eq!(
            boundary_start(&offsets, 0, 12),
            0,
            "the (0,0) entry must not become the first boundary; index 0 has no real offset yet"
        );
        assert_eq!(boundary_start(&offsets, 1, 12), 0);
        assert_eq!(boundary_start(&offsets, 2, 12), 5);
        assert_eq!(
            boundary_start(&offsets, 3, 12),
            12,
            "past the last token the boundary is the end of the text"
        );
    }

    #[test]
    fn a_boundary_is_never_behind_one_already_passed() {
        // A tokenizer reporting a shrinking offset must not produce windows
        // that walk backwards and silently drop bytes.
        let offsets = vec![(6, 8), (2, 4)];
        let first = boundary_start(&offsets, 0, 10);
        let second = boundary_start(&offsets, 1, 10);
        assert!(first <= second, "{first} then {second}");
    }

    #[test]
    fn a_directory_missing_metadata_is_not_a_snapshot() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dir = tmp.path();
        // One stray file: exactly the half-fetched directory the old lookup
        // accepted.
        std::fs::write(dir.join("tokenizer.json"), b"{}").unwrap();
        assert!(!has_all_metadata(dir));

        for relative in REQUIRED_METADATA_FILES {
            std::fs::write(dir.join(relative), b"{}").unwrap();
        }
        assert!(has_all_metadata(dir));
    }

    #[test]
    fn an_absent_cache_resolves_to_the_fingerprinted_unavailable_state() {
        let resolution = resolve_layout_in(
            Path::new("/nonexistent-forgeplan-model-cache"),
            "BAAI/bge-m3",
        );
        assert!(resolution.tokenizer().is_none());
        let layout = resolution.layout();
        assert!(!layout.is_available());
        assert_eq!(layout.fingerprint().len(), 64);
    }

    // -----------------------------------------------------------------------
    // The fixture itself.
    //
    // Everything above is decided by hand-built `SectionLayout` values, which
    // is correct: the fingerprint and the budget arithmetic are pure. The
    // fixture is the one piece that involves a real tokenizer, and a fixture
    // that silently failed to build would leave every windowing test passing
    // vacuously. So it is checked here, in the same module, rather than only
    // from the integration targets.
    // -----------------------------------------------------------------------

    #[test]
    fn the_fixture_loads_through_the_production_loader_and_wraps_its_input() {
        let (_dir, tokenizer, budgets) = testing::fixture_tokenizer(MAX_TOKENS_CEILING).unwrap();

        assert!(tokenizer.layout().is_available());
        assert_eq!(tokenizer.layout().max_tokens, MAX_TOKENS_CEILING);
        assert_eq!(budgets.max_tokens, MAX_TOKENS_CEILING);
        // Exactly the two the fixture's template adds. The loader reads this from the
        // post-processor's own `added_tokens`, so a value here that disagreed
        // with the template would mean the reserved budget and the real
        // wrapper had come apart — which is precisely the drift that made
        // every window one token too large.
        assert_eq!(
            tokenizer.layout().special_tokens,
            2,
            "the fixture template is `[CLS] $A [SEP]`: two added tokens"
        );
        // The special tokens really are added when asked for, and not when not
        // asked for: that difference is what makes `measure` and
        // `count_with_special_tokens` two different measurements on purpose.
        assert_eq!(
            tokenizer.count_with_special_tokens("").unwrap(),
            budgets.special_tokens
        );
        assert_eq!(tokenizer.measure("").unwrap().ids.len(), 0);
    }

    #[test]
    fn the_fixture_splits_every_script_into_several_tokens() {
        // See the two tests above for what the fixture is for.
        let (_dir, tokenizer, budgets) = testing::fixture_tokenizer(MAX_TOKENS_CEILING).unwrap();

        // One token per character is what makes this fixture useful: it forces
        // many windows out of text that has no whitespace to split on, which
        // is exactly the case a character-based scheme handled worst.
        for (label, text) in [
            ("english", "the body of an ordinary English section"),
            ("russian", "тело раздела для проверки покрытия"),
            ("cjk", "这是一段没有任何空格的中文正文内容"),
            ("emoji", "a section 🎉 with 🔥 emoji 🚀 inside"),
            (
                "no space",
                "averylongunbrokenruntokenwithnobreakswhatsoever",
            ),
        ] {
            let count = tokenizer.measure(text).unwrap().ids.len();
            assert!(
                count > 3,
                "{label} produced only {count} token(s); the fixture is not splitting per \
                 character, so window boundaries would not be exercised"
            );
        }

        // And windows over an unspaced run still tile it exactly.
        let long: String = "字".repeat(budgets.body_tokens + 50);
        let windows = tokenizer.token_windows(&long, &budgets).unwrap();
        assert!(windows.len() >= 2, "a body over the budget must window");
        let mut cursor = 0usize;
        for window in &windows {
            assert_eq!(window.start, cursor, "windows must tile with no gap");
            cursor = window.end;
        }
        assert_eq!(cursor, long.len(), "the last window runs to the end");
    }

    /// The override seam itself: it must win, and it must be removable.
    ///
    /// Both halves matter. A seam that did not win would leave the
    /// layout-dependent capture tests resolving whatever this machine has
    /// downloaded — the ambient-state dependency the seam exists to remove. A
    /// seam that could not be cleared would leak a fixture layout into every
    /// later test in the process, including the ones that must observe a
    /// genuinely absent cache.
    #[test]
    fn an_installed_layout_cache_overrides_resolution_and_is_removable() {
        const REPO: &str = crate::constellation::jobs::DEFAULT_MODEL_REPO;

        let cache = tempfile::TempDir::new().unwrap();
        let snapshot = cache
            .path()
            .join("models--BAAI--bge-m3")
            .join("snapshots/rev-override");
        std::fs::create_dir_all(&snapshot).unwrap();
        testing::write_snapshot(&snapshot, MAX_TOKENS_CEILING).unwrap();

        let previous = set_test_layout_cache(Some(cache.path().to_path_buf()));
        let overridden = resolve_layout(REPO);
        assert!(
            overridden.layout().is_available(),
            "the override must supply a layout even where the ambient cache has none"
        );

        set_test_layout_cache(previous);
        let restored = resolve_layout(REPO);
        assert_ne!(
            overridden.fingerprint(),
            restored.fingerprint(),
            "clearing the override must restore ambient resolution; a leaked fixture layout would \
             make every later test observe a tokenizer it did not ask for"
        );

        // And clearing twice is safe, which is what a panicking test leaves
        // behind.
        clear_test_layout_cache();
        assert_eq!(
            resolve_layout(REPO).fingerprint(),
            restored.fingerprint(),
            "a second clear must be a no-op, not a reset of ambient state"
        );
    }
}
