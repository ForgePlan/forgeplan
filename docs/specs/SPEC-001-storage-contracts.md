---
id: SPEC-001
title: "Storage Contracts — LanceDB schema, ArtifactStore API, DTOs"
status: Draft
author: explosovebit
created: 2026-03-22
updated: 2026-03-22
prd: PRD-001
rfc: RFC-003
depth: deep
spec_type: data_model
---

# SPEC-001: Storage Contracts

## Summary

Формальная спецификация контрактов между слоями Forgeplan: Arrow schemas для LanceDB таблиц, `ArtifactStore` trait API, domain types (DTOs), conversion rules. Это контракт — изменения только через delta-spec.

---

## Data Models

### 1. Domain Types (DTOs)

#### ArtifactId (Newtype)

```rust
/// Type-safe artifact identifier. Format: "{KIND}-{NNN}" (e.g., "PRD-001", "RFC-002").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArtifactId(String);

impl ArtifactId {
    pub fn new(kind: &ArtifactKind, num: u32, digits: u32) -> Self;
    pub fn parse(s: &str) -> Result<Self>;  // validates format
    pub fn as_str(&self) -> &str;
    pub fn kind(&self) -> Option<ArtifactKind>;  // extract from prefix
    pub fn number(&self) -> Option<u32>;          // extract numeric part
}

impl Display for ArtifactId { /* "PRD-001" */ }
impl From<String> for ArtifactId { /* unchecked */ }
impl From<&str> for ArtifactId { /* unchecked */ }
```

**Validation**: Uppercase prefix + dash + zero-padded digits. Regex: `^[A-Z]+-\d{3,}$`

#### ArtifactRecord (full domain object)

```rust
/// Complete artifact with all fields. Used for get() and storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactRecord {
    pub id: ArtifactId,
    pub kind: ArtifactKind,
    pub status: Status,
    pub title: String,
    pub body: String,

    #[serde(default)]
    pub depth: Option<Mode>,

    #[serde(default)]
    pub author: Option<String>,

    #[serde(default)]
    pub parent_epic: Option<String>,

    #[serde(default)]
    pub r_eff_score: f64,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<String>,

    pub created_at: String,  // ISO 8601
    pub updated_at: String,  // ISO 8601

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,  // 384-dim, None until Phase 4
}
```

#### NewArtifact (create DTO)

```rust
/// Input for creating a new artifact. ID is auto-generated.
pub struct NewArtifact {
    pub kind: ArtifactKind,
    pub title: String,
    pub body: String,
    pub depth: Option<Mode>,
    pub author: Option<String>,
    pub parent_epic: Option<String>,
}
```

#### ArtifactSummary (list DTO)

```rust
/// Lightweight projection for list/search results. No body, no embedding.
#[derive(Debug, Clone)]
pub struct ArtifactSummary {
    pub id: ArtifactId,
    pub title: String,
    pub kind: String,
    pub status: String,
    pub depth: Option<String>,
    pub r_eff_score: f64,
    pub created_at: String,
    pub updated_at: String,
}
```

#### ArtifactFilter (query DTO)

```rust
/// Filter criteria for list queries.
#[derive(Debug, Clone, Default)]
pub struct ArtifactFilter {
    pub kind: Option<String>,
    pub status: Option<String>,
    pub parent_epic: Option<String>,
}
```

#### ArtifactUpdate (update DTO)

```rust
/// Partial update. Only non-None fields are applied.
pub struct ArtifactUpdate {
    pub status: Option<Status>,
    pub title: Option<String>,
    pub body: Option<String>,
    pub r_eff_score: Option<f64>,
    pub valid_until: Option<String>,
}
```

---

### 2. Arrow Schemas (LanceDB)

#### artifacts table

| # | Column | Arrow Type | Nullable | Description |
|---|--------|-----------|----------|-------------|
| 0 | `id` | `Utf8` | NO | PK: "PRD-001" |
| 1 | `kind` | `Utf8` | NO | "prd", "rfc", "adr" |
| 2 | `status` | `Utf8` | NO | "draft", "active", "superseded" |
| 3 | `title` | `Utf8` | NO | Human-readable title |
| 4 | `body` | `LargeUtf8` | NO | Markdown content |
| 5 | `depth` | `Utf8` | YES | "tactical", "standard", "deep" |
| 6 | `author` | `Utf8` | YES | Author name |
| 7 | `parent_epic` | `Utf8` | YES | FK: parent epic ID |
| 8 | `r_eff_score` | `Float64` | NO | Cached R_eff (default 0.0) |
| 9 | `valid_until` | `Utf8` | YES | ISO date "2027-03-22" |
| 10 | `created_at` | `Utf8` | NO | ISO datetime |
| 11 | `updated_at` | `Utf8` | NO | ISO datetime |
| 12 | `embedding` | `FixedSizeList(384, Float32)` | YES | Semantic vector (None until Phase 4) |

```rust
pub fn artifacts_schema() -> Schema {
    Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("kind", DataType::Utf8, false),
        Field::new("status", DataType::Utf8, false),
        Field::new("title", DataType::Utf8, false),
        Field::new("body", DataType::LargeUtf8, false),
        Field::new("depth", DataType::Utf8, true),
        Field::new("author", DataType::Utf8, true),
        Field::new("parent_epic", DataType::Utf8, true),
        Field::new("r_eff_score", DataType::Float64, false),
        Field::new("valid_until", DataType::Utf8, true),
        Field::new("created_at", DataType::Utf8, false),
        Field::new("updated_at", DataType::Utf8, false),
        Field::new("embedding",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                384  // i32!
            ),
            true,  // nullable — None until ONNX embeddings added
        ),
    ])
}
```

#### evidence table

| # | Column | Arrow Type | Nullable | Description |
|---|--------|-----------|----------|-------------|
| 0 | `id` | `Utf8` | NO | PK: "EVID-001" |
| 1 | `artifact_id` | `Utf8` | NO | FK: linked artifact |
| 2 | `evidence_type` | `Utf8` | NO | "measurement", "test", "benchmark", "audit" |
| 3 | `verdict` | `Utf8` | NO | "supports", "weakens", "refutes" |
| 4 | `congruence_level` | `Int32` | NO | 0-3 |
| 5 | `valid_until` | `Utf8` | YES | ISO date |
| 6 | `content` | `Utf8` | NO | Description |
| 7 | `created_at` | `Utf8` | NO | ISO datetime |

#### relations table

| # | Column | Arrow Type | Nullable | Description |
|---|--------|-----------|----------|-------------|
| 0 | `source_id` | `Utf8` | NO | FK: source artifact |
| 1 | `target_id` | `Utf8` | NO | FK: target artifact |
| 2 | `relation_type` | `Utf8` | NO | "informs", "based_on", "supersedes", "contradicts", "refines" |
| 3 | `created_at` | `Utf8` | NO | ISO datetime |

---

## API Contracts

### ArtifactStore Trait

```rust
/// Storage abstraction — swappable backend (LanceDB, in-memory, file-based).
#[async_trait]
pub trait ArtifactStore: Send + Sync {
    /// Create artifact, auto-generate ID. Returns the generated ID.
    async fn create(&self, artifact: &NewArtifact) -> Result<ArtifactId>;

    /// Get full artifact by ID. Returns None if not found.
    async fn get(&self, id: &str) -> Result<Option<ArtifactRecord>>;

    /// List artifacts with optional filter. Returns summaries (no body).
    async fn list(&self, filter: &ArtifactFilter) -> Result<Vec<ArtifactSummary>>;

    /// Update artifact fields. Only non-None fields in ArtifactUpdate are applied.
    async fn update(&self, id: &str, update: &ArtifactUpdate) -> Result<()>;

    /// Delete artifact by ID.
    async fn delete(&self, id: &str) -> Result<()>;

    /// Add typed relationship between two artifacts.
    async fn add_link(&self, source: &str, target: &str, relation: &str) -> Result<()>;

    /// Get all outgoing links from an artifact. Returns Vec<(target_id, relation)>.
    async fn get_links(&self, id: &str) -> Result<Vec<(String, String)>>;

    /// Get next sequential ID for a kind. E.g., "PRD-004" if 3 PRDs exist.
    async fn next_id(&self, kind: &ArtifactKind) -> Result<ArtifactId>;

    /// Count artifacts by kind and status.
    async fn count(&self, filter: &ArtifactFilter) -> Result<usize>;
}
```

**Invariants**:
- `create()` MUST auto-generate monotonically increasing IDs per kind
- `create()` MUST write markdown projection after LanceDB insert
- `get()` returns from LanceDB, NOT from markdown files
- `list()` returns `ArtifactSummary` (no body, no embedding) for performance
- `add_link()` MUST normalize target to uppercase
- `add_link()` MUST reject duplicates (case-insensitive)
- All methods MUST validate ID format before `.only_if()` (SQL injection prevention)

### LanceStoreBuilder

```rust
pub struct LanceStoreBuilder {
    workspace: Option<PathBuf>,
    embedding_dim: i32,  // default: 384
}

impl LanceStoreBuilder {
    pub fn new() -> Self;
    pub fn workspace(self, path: impl Into<PathBuf>) -> Self;
    pub fn embedding_dim(self, dim: i32) -> Self;
    pub async fn build(self) -> Result<LanceStore>;
    pub async fn open_existing(self) -> Result<LanceStore>;  // fails if tables don't exist
}
```

**Invariants**:
- `build()` creates tables if they don't exist (idempotent)
- `open_existing()` fails with `ForgeplanError::WorkspaceNotFound` if no lance/ directory
- `.forgeplan/lance/` directory is created by `build()`, not by workspace init

### LanceStore

```rust
#[derive(Clone)]  // Clone = Arc bump, cheap
pub struct LanceStore {
    inner: Arc<LanceStoreInner>,
}

struct LanceStoreInner {
    db: lancedb::Database,
    artifacts: lancedb::Table,
    evidence: lancedb::Table,
    relations: lancedb::Table,
    workspace: PathBuf,
    embedding_dim: i32,
}

impl ArtifactStore for LanceStore { /* ... */ }
```

---

## Validation Rules

### Field Constraints

| Field | Constraint |
|-------|-----------|
| `id` | Regex: `^[A-Z]+-\d{3,}$` |
| `kind` | One of: prd, epic, spec, rfc, adr, note, problem, solution, evidence, refresh |
| `status` | One of: draft, active, superseded, deprecated, refresh_due |
| `depth` | One of: note, tactical, standard, deep (nullable) |
| `congruence_level` | 0..=3 |
| `relation_type` | One of: informs, based_on, supersedes, contradicts, refines |
| `r_eff_score` | 0.0..=1.0 |
| `embedding` | Exactly 384 f32 values, or None |

### SQL Injection Prevention

IDs used in `.only_if()` MUST be sanitized:

```rust
fn sanitize_id(id: &str) -> Result<&str> {
    if id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        Ok(id)
    } else {
        Err(ForgeplanError::ArtifactNotFound(id.to_string()))
    }
}
```

---

## Events / Side Effects

| Operation | Side Effect |
|-----------|-------------|
| `create()` | Insert into `artifacts` table → render markdown → write to `.forgeplan/{kind}s/{id}-{slug}.md` |
| `update()` | Update LanceDB record → re-render markdown → overwrite `.md` file |
| `delete()` | Delete from LanceDB → delete `.md` file |
| `add_link()` | Insert into `relations` table → update source artifact's `.md` frontmatter `links:` |

### Markdown Projection Format

```yaml
---
id: PRD-001
title: "Feature X"
kind: prd
status: Draft
depth: standard
author: explosovebit
created: 2026-03-22
updated: 2026-03-22
links:
  - target: RFC-001
    relation: informs
---

[body content from LanceDB]
```

---

## Conversion Rules

### Domain → Arrow (write path)

| Domain Type | Arrow Type | Conversion |
|-------------|-----------|------------|
| `ArtifactId` | `StringArray` | `.as_str().to_string()` |
| `ArtifactKind` | `StringArray` | serde snake_case: `Prd` → `"prd"` |
| `Status` | `StringArray` | serde snake_case: `Draft` → `"draft"` |
| `Option<String>` | `StringArray` (nullable) | `Some(s)` → value, `None` → null |
| `f64` | `Float64Array` | direct |
| `Option<Vec<f32>>` | `FixedSizeListArray` (nullable) | `None` → null row |

### Arrow → Domain (read path)

| Arrow Column | Domain Type | Conversion |
|--------------|-------------|------------|
| `StringArray[i]` | `String` | `.value(i).to_string()` |
| `StringArray[i]` (nullable) | `Option<String>` | `.is_null(i)` check |
| `Float64Array[i]` | `f64` | `.value(i)` |
| `FixedSizeListArray[i]` | `Option<Vec<f32>>` | `.is_null(i)`, then `.as_primitive::<Float32Type>().values().to_vec()` |

---

## Versioning

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-03-22 | Initial: 3 tables, ArtifactStore trait, 6 DTOs |

---

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| PRD-001 | PRD | based_on |
| RFC-003 | RFC | implements |
| ADR-002 | ADR | informs (LanceDB decision) |
| ADR-004 | ADR | informs (async decision) |
| EPIC-001 | Epic | parent |
