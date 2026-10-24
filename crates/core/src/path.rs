//! Path-containment utility shared by the CLI and backends.
//!
//! Untrusted names from manifests or user input are accepted only as a
//! single path component beneath a trusted root. Absolute paths, parent
//! traversal, separators, NUL bytes, and empty names are rejected, and an
//! existing symlink that resolves outside the root is refused before any file
//! is written.

use std::path::{Path, PathBuf};

/// Why a candidate path was rejected.
#[derive(Debug, thiserror::Error)]
pub enum PathContainmentError {
    /// The component is empty.
    #[error("path component is empty")]
    Empty,
    /// The component contains a NUL byte.
    #[error("path component contains a NUL byte")]
    NulByte,
    /// The component is an absolute path.
    #[error("absolute path '{0}' is not allowed")]
    Absolute(String),
    /// The component contains a path separator.
    #[error("path separator in '{0}' is not allowed")]
    Separator(String),
    /// The component is `.` or `..`.
    #[error("parent traversal in '{0}' is not allowed")]
    ParentTraversal(String),
    /// The resolved path escapes the trusted root.
    #[error("path '{0}' resolves outside the destination directory")]
    EscapesRoot(String),
    /// The filesystem could not resolve the path.
    #[error("cannot resolve path: {0}")]
    Io(#[from] std::io::Error),
}

/// Join one validated component beneath `root`, refusing escapes.
pub fn safe_join(root: &Path, component: &str) -> Result<PathBuf, PathContainmentError> {
    validate_path_component(component)?;

    let root_canonical = canonicalise_deepest(root)?;
    let candidate = root_canonical.join(component);

    let resolved = if candidate.exists() {
        std::fs::canonicalize(&candidate)?
    } else {
        // Canonicalise the deepest existing parent, then reattach the
        // remaining (new) components so a symlinked directory cannot be used
        // to escape the root.
        canonicalise_deepest(&candidate)?
    };

    if resolved.starts_with(&root_canonical) {
        Ok(candidate)
    } else {
        Err(PathContainmentError::EscapesRoot(component.to_owned()))
    }
}

/// Validate one untrusted path component.
pub fn validate_path_component(component: &str) -> Result<(), PathContainmentError> {
    if component.is_empty() {
        return Err(PathContainmentError::Empty);
    }
    if component.contains('\0') {
        return Err(PathContainmentError::NulByte);
    }
    let path = Path::new(component);
    if path.is_absolute() {
        return Err(PathContainmentError::Absolute(component.to_owned()));
    }
    if component.contains('/') || component.contains('\\') {
        return Err(PathContainmentError::Separator(component.to_owned()));
    }
    if component == "." || component == ".." {
        return Err(PathContainmentError::ParentTraversal(component.to_owned()));
    }
    Ok(())
}

/// Canonicalise the deepest existing ancestor and reattach the missing
/// components without touching the filesystem for them.
fn canonicalise_deepest(path: &Path) -> Result<PathBuf, std::io::Error> {
    let mut existing = path.to_path_buf();
    let mut missing = Vec::new();
    while !existing.exists() {
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                missing.push(name.to_os_string());
                existing = parent.to_path_buf();
            }
            _ => break,
        }
    }
    let mut resolved = if existing.exists() {
        std::fs::canonicalize(&existing)?
    } else {
        existing
    };
    for component in missing.into_iter().rev() {
        resolved.push(component);
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    #[test]
    fn joins_a_simple_component() {
        let root = temp_root();
        let joined = safe_join(root.path(), "board.kicad_pcb").unwrap();
        assert_eq!(
            joined,
            root.path().canonicalize().unwrap().join("board.kicad_pcb")
        );
    }

    #[test]
    fn rejects_absolute_components() {
        let root = temp_root();
        let error = safe_join(root.path(), "/etc/passwd").unwrap_err();
        assert!(matches!(error, PathContainmentError::Absolute(_)));
    }

    #[test]
    fn rejects_parent_traversal() {
        let root = temp_root();
        assert!(matches!(
            safe_join(root.path(), "..").unwrap_err(),
            PathContainmentError::ParentTraversal(_)
        ));
        assert!(matches!(
            safe_join(root.path(), "../escape").unwrap_err(),
            PathContainmentError::Separator(_)
        ));
    }

    #[test]
    fn rejects_separators_and_empty_names() {
        let root = temp_root();
        assert!(matches!(
            safe_join(root.path(), "sub/file").unwrap_err(),
            PathContainmentError::Separator(_)
        ));
        assert!(matches!(
            safe_join(root.path(), "sub\\file").unwrap_err(),
            PathContainmentError::Separator(_)
        ));
        assert!(matches!(
            safe_join(root.path(), "").unwrap_err(),
            PathContainmentError::Empty
        ));
        assert!(matches!(
            safe_join(root.path(), "bad\0name").unwrap_err(),
            PathContainmentError::NulByte
        ));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_escape_for_existing_target() {
        let root = temp_root();
        let outside = temp_root();
        std::fs::write(outside.path().join("target"), "outside").unwrap();
        std::os::unix::fs::symlink(outside.path().join("target"), root.path().join("linked"))
            .unwrap();

        let error = safe_join(root.path(), "linked").unwrap_err();
        assert!(
            matches!(error, PathContainmentError::EscapesRoot(_)),
            "{error:?}"
        );

        // Nothing was created or modified outside the destination.
        assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 1);
    }
}
