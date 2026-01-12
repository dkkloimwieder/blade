//! Path utilities

use std::path::{Path, PathBuf};

/// Normalize a path by removing . and .. components
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                if !components.is_empty() {
                    components.pop();
                }
            }
            std::path::Component::CurDir => {}
            c => components.push(c),
        }
    }
    components.iter().collect()
}

/// Check if a path is a descendant of another
pub fn is_descendant(path: &Path, ancestor: &Path) -> bool {
    path.starts_with(ancestor) && path != ancestor
}

/// Get the relative path from base to target
pub fn relative_path(base: &Path, target: &Path) -> Option<PathBuf> {
    let base = normalize_path(base);
    let target = normalize_path(target);

    if target.starts_with(&base) {
        target.strip_prefix(&base).ok().map(PathBuf::from)
    } else {
        None
    }
}
