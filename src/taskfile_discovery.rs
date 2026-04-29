use std::path::{Path, PathBuf};

pub(crate) const BASE_TASKFILE_NAME: &str = "cjtasks";

// there are total of five allowed taskfile names.
// the base taskfile is a special case because versioning is only supported for the base taskfile. If it doesn't exist, none of the version tools will work.
// The ordering of files goes from base, to production, then up to local, with local being the highest priority but last to load.
pub(crate) const OVERLAY_LOAD_ORDER: &[&str] = &[
    "production.cjtasks",
    "staging.cjtasks",
    "development.cjtasks",
    "local.cjtasks",
];

pub(crate) const OVERLAY_DISPLAY_ORDER: &[&str] = &[
    "local.cjtasks",
    "development.cjtasks",
    "staging.cjtasks",
    "production.cjtasks",
];

pub(crate) const TASKFILE_NAMES: &[&str] = &[
    BASE_TASKFILE_NAME,
    "production.cjtasks",
    "staging.cjtasks",
    "development.cjtasks",
    "local.cjtasks",
];

pub(crate) fn is_recognized_taskfile_name(name: &str) -> bool {
    TASKFILE_NAMES.contains(&name)
}

pub(crate) fn is_recognized_taskfile(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(is_recognized_taskfile_name)
}

pub(crate) fn base_taskfile_path(dir: &Path) -> PathBuf {
    dir.join(BASE_TASKFILE_NAME)
}

pub(crate) fn existing_taskfile_path(dir: &Path) -> Option<PathBuf> {
    let base = base_taskfile_path(dir);
    if base.is_file() {
        return Some(base);
    }
    OVERLAY_DISPLAY_ORDER
        .iter()
        .map(|name| dir.join(name))
        .find(|path| path.is_file())
}

pub(crate) fn layer_paths(dir: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let base = base_taskfile_path(dir);
    if base.is_file() {
        paths.push(base);
    }
    paths.extend(
        OVERLAY_LOAD_ORDER
            .iter()
            .map(|name| dir.join(name))
            .filter(|path| path.is_file()),
    );
    paths
}
