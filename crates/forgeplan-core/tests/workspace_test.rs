use forgeplan_core::workspace;
use tempfile::tempdir;

#[test]
fn init_creates_workspace() {
    let dir = tempdir().unwrap();
    let result = workspace::init_workspace(dir.path(), "test-project");
    assert!(result.is_ok());
    let fp = dir.path().join(".forgeplan");
    assert!(fp.exists());
    assert!(fp.join("config.yaml").exists());
    assert!(fp.join("prds").is_dir());
    assert!(fp.join("adrs").is_dir());
}

#[test]
fn init_fails_if_exists() {
    let dir = tempdir().unwrap();
    workspace::init_workspace(dir.path(), "test").unwrap();
    let result = workspace::init_workspace(dir.path(), "test");
    assert!(result.is_err());
}

#[test]
fn find_workspace_walks_up() {
    let dir = tempdir().unwrap();
    workspace::init_workspace(dir.path(), "test").unwrap();
    let sub = dir.path().join("sub/deep");
    std::fs::create_dir_all(&sub).unwrap();
    let found = workspace::find_workspace(&sub);
    assert!(found.is_some());
}

#[test]
fn load_config_roundtrip() {
    let dir = tempdir().unwrap();
    let ws = workspace::init_workspace(dir.path(), "my-project").unwrap();
    let config = workspace::load_config(&ws).unwrap();
    assert_eq!(config.project_name, "my-project");
    assert_eq!(config.version, 1);
    assert_eq!(config.default_depth, "standard");
    assert_eq!(config.id_digits, 3);
}
