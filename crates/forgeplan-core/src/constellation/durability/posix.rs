//! POSIX index-root ownership check.
//!
//! A symlinked root would give one physical index two owner-lock keys and two
//! owner processes, each believing it holds the only lock. The check walks
//! the existing prefix of the path and refuses any component that is a
//! symlink, rather than trusting `canonicalize` at one point: canonicalize
//! answers "where does this resolve to", not "is this the spelling we own".

use std::path::Path;

/// Verify every existing component of `root` is a real directory and not a
/// symlink.
///
/// Returns the offending component's path when one is.
pub(crate) fn root_is_own(root: &Path) -> Result<(), String> {
    let mut current = std::path::PathBuf::new();
    for component in root.components() {
        current.push(component);
        // A missing tail is bootstrap's job, not an alias problem.
        let Ok(meta) = std::fs::symlink_metadata(&current) else {
            return Ok(());
        };
        if meta.file_type().is_symlink() {
            return Err(format!(
                "index root component {} is a symlink; a symlinked alias would create a second physical owner",
                current.display()
            ));
        }
        if !meta.is_dir() {
            return Err(format!(
                "index root component {} is not a directory",
                current.display()
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn real_directory_tree_passes() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().canonicalize().unwrap();
        let root = base.join(".forgeplan").join("lance");
        std::fs::create_dir_all(&root).unwrap();
        assert!(root_is_own(&root).is_ok());
    }

    #[test]
    fn missing_tail_is_not_an_alias() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().canonicalize().unwrap();
        let root = base.join(".forgeplan").join("lance");
        std::fs::create_dir_all(base.join(".forgeplan")).unwrap();
        assert!(root_is_own(&root).is_ok());
    }

    #[test]
    fn symlink_anywhere_in_the_path_fails() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().canonicalize().unwrap();
        let real = base.join("real");
        std::fs::create_dir_all(&real).unwrap();
        let link = base.join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let root = link.join("lance");
        std::fs::create_dir_all(&root).unwrap();
        let err = root_is_own(&root).unwrap_err();
        assert!(err.contains("symlink"), "unexpected message: {err}");
    }

    #[test]
    fn file_component_fails() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().canonicalize().unwrap();
        let file = base.join("afile");
        std::fs::write(&file, b"x").unwrap();
        let root = file.join("lance");
        let err = root_is_own(&root).unwrap_err();
        assert!(err.contains("not a directory"), "unexpected message: {err}");
    }
}
