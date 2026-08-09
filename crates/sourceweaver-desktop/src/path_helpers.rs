//! Path display and project path helpers.

use super::*;

pub(crate) fn display_path(path: &Path) -> String {
    path.display().to_string()
}

pub(crate) fn remember_recent_path(recent: &mut Vec<PathBuf>, path: PathBuf) {
    recent.retain(|existing| existing != &path);
    recent.insert(0, path);
    recent.truncate(8);
}

pub(crate) fn project_relative_path(path: &Path, base_dir: &Path) -> String {
    if path.is_absolute() {
        match path.strip_prefix(base_dir) {
            Ok(relative) if !relative.as_os_str().is_empty() => return display_path(relative),
            _ => {}
        }
    }
    display_path(path)
}

pub(crate) fn resolve_project_path(base_dir: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        base_dir.join(path)
    }
}

pub(crate) fn file_name_or_path(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| display_path(path))
}
