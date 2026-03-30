//! Factory for creating StorageDriver instances from config.

use std::path::Path;
use std::sync::Arc;

use crate::config::types::StorageConfig;
use crate::driver::in_memory::InMemoryStore;
use crate::driver::lance::LanceDriver;
use crate::driver::StorageDriver;

/// Create a StorageDriver based on config.
/// Returns Arc<dyn StorageDriver> for shared ownership.
pub async fn create_storage(
    config: &StorageConfig,
    workspace_path: &Path,
) -> anyhow::Result<Arc<dyn StorageDriver>> {
    match config.driver.as_str() {
        "lancedb" | "lance" => {
            let driver = LanceDriver::open(workspace_path).await?;
            Ok(Arc::new(driver))
        }
        "memory" | "in_memory" => Ok(Arc::new(InMemoryStore::new())),
        other => {
            anyhow::bail!(
                "Unknown storage driver: '{}'. Supported: lancedb, memory",
                other
            )
        }
    }
}

/// Create and initialize a NEW storage (for forgeplan init).
pub async fn init_storage(
    config: &StorageConfig,
    workspace_path: &Path,
) -> anyhow::Result<Arc<dyn StorageDriver>> {
    match config.driver.as_str() {
        "lancedb" | "lance" => {
            let driver = LanceDriver::init(workspace_path).await?;
            Ok(Arc::new(driver))
        }
        "memory" | "in_memory" => Ok(Arc::new(InMemoryStore::new())),
        other => {
            anyhow::bail!(
                "Unknown storage driver: '{}'. Supported: lancedb, memory",
                other
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_lance_driver() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = tmp.path().join("lance");
        std::fs::create_dir_all(&ws).unwrap();

        let config = StorageConfig {
            driver: "lancedb".to_string(),
        };

        // First init to create tables
        let _driver = init_storage(&config, &ws).await.unwrap();

        // Then open existing
        let driver = create_storage(&config, &ws).await.unwrap();
        assert!(Arc::strong_count(&driver) == 1);
    }

    #[tokio::test]
    async fn test_create_memory_driver() {
        let config = StorageConfig {
            driver: "memory".to_string(),
        };

        let driver = create_storage(&config, Path::new("/unused")).await.unwrap();
        assert!(!driver.supports_vectors());
    }

    #[tokio::test]
    async fn test_unknown_driver_returns_error() {
        let config = StorageConfig {
            driver: "postgres".to_string(),
        };

        let result = create_storage(&config, Path::new("/unused")).await;
        let err = match result {
            Ok(_) => panic!("expected error for unknown driver"),
            Err(e) => e.to_string(),
        };
        assert!(err.contains("Unknown storage driver"));
        assert!(err.contains("postgres"));
    }
}
