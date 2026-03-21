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
