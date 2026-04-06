# Rust Architecture Patterns — Research Report

> Extracted from 3 reference projects: edgequake, jan, graphrag-rs
> Date: 2026-03-22
> Purpose: Inform Forgeplan LanceDB integration (RFC-003)

---

## Synthesis: Best Patterns for Forgeplan

### 1. Storage Abstraction: Trait-Based (from graphrag-rs)

```rust
#[async_trait]
pub trait ArtifactStore: Send + Sync {
    async fn create(&self, artifact: &NewArtifact) -> Result<ArtifactSummary>;
    async fn get(&self, id: &str) -> Result<Option<Artifact>>;
    async fn list(&self, filter: &ArtifactFilter) -> Result<Vec<ArtifactSummary>>;
    async fn update(&self, id: &str, update: &ArtifactUpdate) -> Result<()>;
    async fn delete(&self, id: &str) -> Result<()>;
    async fn search(&self, query: &str, kind: Option<&str>) -> Result<Vec<SearchHit>>;
}
```

**Source**: graphrag-rs VectorStore trait (42 lines) — clean, async, Send + Sync bounds.
**Why**: Allows swapping LanceDB for in-memory store in tests, or future SQLite fallback.

### 2. LanceDB Patterns (from graphrag-rs)

#### Table Init: try open, else create
```rust
let table = match db.open_table("artifacts").execute().await {
    Ok(table) => table,
    Err(_) => {
        let empty = create_empty_batch(schema.clone())?;
        db.create_table("artifacts", empty).execute().await?
    }
};
```

#### RecordBatch Construction
```rust
let batch = RecordBatch::try_new(
    schema.clone(),
    vec![
        Arc::new(StringArray::from(vec![id])),
        Arc::new(StringArray::from(vec![title])),
        Arc::new(Float64Array::from(vec![r_eff_score])),
        Arc::new(FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
            vec![Some(embedding.iter().map(|&v| Some(v)).collect())],
            384, // dimension as i32
        )),
    ],
)?;
```

#### Query Patterns
```rust
// By ID (SQL filter)
table.query().only_if(format!("id = '{}'", id)).execute().await?

// Vector search
table.query().limit(k).nearest_to(query_vec).execute().await?

// Stream collection
.try_collect::<Vec<RecordBatch>>().await?
```

#### Gotchas
- `.limit()` BEFORE `.nearest_to()` (order matters!)
- FixedSizeList size is **i32**, not usize
- Schema from `.schema().await` (not hardcoded per operation)
- `.only_if()` vulnerable to injection — validate IDs!

### 3. Error Handling: thiserror Enum (from edgequake)

```rust
#[derive(Error, Debug)]
pub enum ForgeplanError {
    #[error("workspace not found: run `forgeplan init` first")]
    WorkspaceNotFound,
    #[error("artifact not found: {0}")]
    ArtifactNotFound(String),
    #[error("storage error: {0}")]
    Storage(String),
    #[error("validation failed: {0} error(s)")]
    ValidationFailed(usize),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Arrow(#[from] arrow::error::ArrowError),
    #[error(transparent)]
    Lance(#[from] lancedb::Error),
}

impl ForgeplanError {
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Storage(_))
    }
}
```

### 4. Client/Store: Arc<Inner> + Builder (from edgequake + jan)

```rust
#[derive(Clone)]
pub struct LanceStore {
    inner: Arc<LanceStoreInner>,
}

struct LanceStoreInner {
    db: lancedb::Database,
    artifacts: lancedb::Table,
    evidence: lancedb::Table,
    relations: lancedb::Table,
    workspace: PathBuf,
}

impl LanceStore {
    pub fn builder() -> LanceStoreBuilder { ... }
}

pub struct LanceStoreBuilder {
    workspace: Option<PathBuf>,
    embedding_dim: usize,
}

impl LanceStoreBuilder {
    pub fn workspace(mut self, path: impl Into<PathBuf>) -> Self { ... }
    pub fn embedding_dim(mut self, dim: usize) -> Self { ... }
    pub async fn build(self) -> Result<LanceStore> { ... }
}
```

### 5. Newtype IDs (from graphrag-rs)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArtifactId(String);

impl ArtifactId {
    pub fn new(kind: &ArtifactKind, num: u32, digits: u32) -> Self {
        Self(format!("{}-{:0>width$}", kind.prefix().trim_end_matches('-').to_uppercase(), num, width = digits as usize))
    }
    pub fn as_str(&self) -> &str { &self.0 }
}

impl Display for ArtifactId { ... }
impl From<String> for ArtifactId { ... }
impl From<&str> for ArtifactId { ... }
```

### 6. Feature Flags (from jan + graphrag-rs)

```toml
[features]
default = ["file-store"]
file-store = []                    # Current file-based storage
lancedb = ["dep:lancedb", "dep:arrow", "dep:arrow-array", "dep:arrow-schema"]
semantic-search = ["lancedb", "dep:ort"]  # Phase 4
desktop = ["dep:tauri"]            # Phase 4
mcp = ["dep:rmcp"]                 # Phase 5
```

### 7. Serde Patterns (from edgequake)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactRecord {
    pub id: ArtifactId,
    pub kind: ArtifactKind,
    pub status: Status,
    pub title: String,
    pub body: String,

    #[serde(default)]
    pub author: Option<String>,

    #[serde(default)]
    pub depth: Option<Mode>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<Link>,
}
```

### 8. Async Test Patterns (from all 3)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn setup_store() -> (TempDir, LanceStore) {
        let tmp = TempDir::new().unwrap();
        let store = LanceStore::builder()
            .workspace(tmp.path())
            .embedding_dim(384)
            .build()
            .await
            .unwrap();
        (tmp, store)
    }

    #[tokio::test]
    async fn test_create_and_get() {
        let (_tmp, store) = setup_store().await;
        let id = store.create(&NewArtifact { ... }).await.unwrap();
        let artifact = store.get(id.as_str()).await.unwrap();
        assert!(artifact.is_some());
    }
}
```

### 9. Markdown Projection (Forgeplan-specific)

```rust
impl LanceStore {
    /// After writing to LanceDB, render markdown projection
    async fn project_to_markdown(&self, artifact: &Artifact) -> Result<()> {
        let dir = self.workspace.join(kind_dir(&artifact.kind));
        let filename = format!("{}-{}.md", artifact.id, slugify(&artifact.title));
        let content = render_frontmatter_and_body(artifact)?;
        tokio::fs::write(dir.join(filename), content).await?;
        Ok(())
    }
}
```

---

## Decision Matrix: What to Adopt

| Pattern | Source | Adopt? | Priority |
|---------|--------|--------|----------|
| ArtifactStore trait (async) | graphrag-rs | YES | P0 |
| LanceDB RecordBatch patterns | graphrag-rs | YES | P0 |
| thiserror enum + is_retryable | edgequake | YES | P0 |
| Arc<Inner> + Builder | edgequake | YES | P0 |
| Newtype ArtifactId | graphrag-rs | YES | P1 |
| Feature flags (lancedb/desktop) | jan + graphrag | YES | P1 |
| Serde skip_serializing_if | edgequake | YES | P1 |
| TypedBuilder (compile-time) | graphrag-rs | NO (over-eng) | — |
| Plugin architecture | jan | NO (Phase 4) | — |
| wiremock for HTTP tests | edgequake | NO (no HTTP) | — |

---

## References

- edgequake SDK: `sources/edgequake/sdks/rust/` (~1800 LOC)
- jan: `sources/jan/` (monolithic + plugins)
- graphrag-rs: `sources/graphrag-rs/` (8 crates, `graphrag-core/src/persistence/lance.rs` = 521 LOC)
