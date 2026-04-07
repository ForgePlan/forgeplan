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
fn load_config_rejects_bad_integrity_threshold() {
    // Bug 1 regression: IntegrityConfig::validate() must run on every load_config call.
    let dir = tempdir().unwrap();
    let ws = workspace::init_workspace(dir.path(), "test").unwrap();
    // Overwrite config.yaml with bad integrity values.
    std::fs::write(
        ws.join("config.yaml"),
        "integrity:\n  duplicate_threshold: 5.0\n",
    )
    .unwrap();
    let result = workspace::load_config(&ws);
    assert!(result.is_err(), "expected validation error, got Ok");
    let msg = format!("{}", result.unwrap_err());
    assert!(
        msg.contains("duplicate_threshold"),
        "error should mention duplicate_threshold: {msg}"
    );
}

#[test]
fn load_config_rejects_zero_body_limit() {
    // Bug 1 regression: zero body limit must be rejected.
    let dir = tempdir().unwrap();
    let ws = workspace::init_workspace(dir.path(), "test").unwrap();
    std::fs::write(
        ws.join("config.yaml"),
        "integrity:\n  mcp_max_body_len: 0\n",
    )
    .unwrap();
    let result = workspace::load_config(&ws);
    assert!(result.is_err());
}

#[test]
fn load_config_accepts_partial_config_without_version() {
    // Bug 2 regression: partial config.yaml without top-level `version` must parse.
    // All Config top-level fields have serde defaults, so missing fields fall back.
    let dir = tempdir().unwrap();
    let fp = dir.path().join(".forgeplan");
    std::fs::create_dir_all(&fp).unwrap();
    std::fs::write(
        fp.join("config.yaml"),
        "integrity:\n  duplicate_threshold: 0.7\n",
    )
    .unwrap();
    let config = workspace::load_config(&fp).expect("partial config should parse");
    // Missing `version` falls back to default.
    assert_eq!(config.version, 1);
    assert!((config.integrity.duplicate_threshold - 0.7).abs() < f64::EPSILON);
}

#[test]
fn load_config_accepts_empty_config() {
    // Bug 2 regression: empty config.yaml must parse using defaults.
    let dir = tempdir().unwrap();
    let fp = dir.path().join(".forgeplan");
    std::fs::create_dir_all(&fp).unwrap();
    std::fs::write(fp.join("config.yaml"), "").unwrap();
    let config = workspace::load_config(&fp).expect("empty config should parse");
    assert_eq!(config.version, 1);
    assert_eq!(config.default_depth, "standard");
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
