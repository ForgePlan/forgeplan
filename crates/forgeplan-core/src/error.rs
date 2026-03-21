use thiserror::Error;

/// Core error types for Forgeplan operations.
#[derive(Error, Debug)]
pub enum ForgeplanError {
    #[error("workspace not found: run `forgeplan init` first")]
    WorkspaceNotFound,

    #[error("workspace already exists at {0}")]
    WorkspaceExists(String),

    #[error("artifact not found: {0}")]
    ArtifactNotFound(String),

    #[error("invalid artifact kind: {0}")]
    InvalidKind(String),

    #[error("invalid relation type: {0} (valid: informs, based_on, supersedes, contradicts, refines)")]
    InvalidRelation(String),

    #[error("link already exists: {from} --{relation}--> {to}")]
    LinkExists {
        from: String,
        relation: String,
        to: String,
    },

    #[error("frontmatter error: {0}")]
    Frontmatter(String),

    #[error("validation failed: {0} error(s) found")]
    ValidationFailed(usize),

    #[error("template error: {0}")]
    Template(String),

    #[error("config error: {0}")]
    Config(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Yaml(#[from] serde_yaml::Error),
}

pub type Result<T> = std::result::Result<T, ForgeplanError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_not_found_display() {
        let err = ForgeplanError::WorkspaceNotFound;
        assert_eq!(err.to_string(), "workspace not found: run `forgeplan init` first");
    }

    #[test]
    fn workspace_exists_display() {
        let err = ForgeplanError::WorkspaceExists("/some/path".to_string());
        assert_eq!(err.to_string(), "workspace already exists at /some/path");
    }

    #[test]
    fn artifact_not_found_display() {
        let err = ForgeplanError::ArtifactNotFound("PRD-042".to_string());
        assert_eq!(err.to_string(), "artifact not found: PRD-042");
    }

    #[test]
    fn invalid_kind_display() {
        let err = ForgeplanError::InvalidKind("banana".to_string());
        assert_eq!(err.to_string(), "invalid artifact kind: banana");
    }

    #[test]
    fn invalid_relation_display_contains_input_and_valid_list() {
        let err = ForgeplanError::InvalidRelation("depends_on".to_string());
        let msg = err.to_string();
        assert!(msg.contains("depends_on"));
        assert!(msg.contains("informs"));
        assert!(msg.contains("based_on"));
        assert!(msg.contains("supersedes"));
        assert!(msg.contains("contradicts"));
        assert!(msg.contains("refines"));
    }

    #[test]
    fn link_exists_display() {
        let err = ForgeplanError::LinkExists {
            from: "PRD-001".to_string(),
            relation: "informs".to_string(),
            to: "RFC-001".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("PRD-001"));
        assert!(msg.contains("informs"));
        assert!(msg.contains("RFC-001"));
    }

    #[test]
    fn frontmatter_display() {
        let err = ForgeplanError::Frontmatter("missing closing ---".to_string());
        assert_eq!(err.to_string(), "frontmatter error: missing closing ---");
    }

    #[test]
    fn validation_failed_display() {
        let err = ForgeplanError::ValidationFailed(3);
        assert_eq!(err.to_string(), "validation failed: 3 error(s) found");
    }

    #[test]
    fn template_display() {
        let err = ForgeplanError::Template("template not found".to_string());
        assert_eq!(err.to_string(), "template error: template not found");
    }

    #[test]
    fn config_display() {
        let err = ForgeplanError::Config("invalid yaml".to_string());
        assert_eq!(err.to_string(), "config error: invalid yaml");
    }

    #[test]
    fn io_error_transparent() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err = ForgeplanError::Io(io_err);
        assert!(err.to_string().contains("file missing"));
    }
}
