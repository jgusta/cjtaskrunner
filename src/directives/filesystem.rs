use std::fs;
use std::path::Path;

use crate::{CjError, CjResult};

pub(super) fn copy_files(base_dir: &Path, argv: &[String], line_number: usize) -> CjResult<()> {
    if argv.len() < 2 {
        return Err(CjError::new(format!(
            "line {line_number}: @cp expects one or more source files and a destination"
        )));
    }
    let destination = base_dir.join(argv.last().expect("argv has destination"));
    let sources = &argv[..argv.len() - 1];
    if sources.len() > 1 && !destination.is_dir() {
        return Err(CjError::new(format!(
            "line {line_number}: @cp destination must be a directory when copying multiple files"
        )));
    }
    for source in sources {
        let source_path = base_dir.join(source);
        if !source_path.is_file() {
            return Err(CjError::new(format!(
                "line {line_number}: @cp source is not a file: {}",
                source_path.display()
            )));
        }
        let target = if destination.is_dir() {
            destination.join(file_name(&source_path, line_number, "@cp source")?)
        } else {
            destination.clone()
        };
        fs::copy(&source_path, target)?;
    }
    Ok(())
}

pub(super) fn copy_dirs(base_dir: &Path, argv: &[String], line_number: usize) -> CjResult<()> {
    if argv.len() < 2 {
        return Err(CjError::new(format!(
            "line {line_number}: @cpdir expects one or more source directories and a destination"
        )));
    }
    let destination = base_dir.join(argv.last().expect("argv has destination"));
    let sources = &argv[..argv.len() - 1];
    if sources.len() > 1 && !destination.is_dir() {
        return Err(CjError::new(format!(
            "line {line_number}: @cpdir destination must be a directory when copying multiple directories"
        )));
    }
    for source in sources {
        let contents_only = has_trailing_separator(source);
        let source_path = base_dir.join(trim_trailing_separators(source));
        if !source_path.is_dir() {
            return Err(CjError::new(format!(
                "line {line_number}: @cpdir source is not a directory: {}",
                source_path.display()
            )));
        }
        if contents_only {
            fs::create_dir_all(&destination)?;
            copy_dir_contents(&source_path, &destination)?;
        } else {
            let target = if destination.is_dir() {
                destination.join(file_name(&source_path, line_number, "@cpdir source")?)
            } else {
                destination.clone()
            };
            copy_dir_recursive(&source_path, &target)?;
        }
    }
    Ok(())
}

pub(super) fn rename_path(base_dir: &Path, argv: &[String], line_number: usize) -> CjResult<()> {
    if argv.len() != 2 {
        return Err(CjError::new(format!(
            "line {line_number}: @rename expects source and destination"
        )));
    }
    let source = base_dir.join(&argv[0]);
    let destination = base_dir.join(&argv[1]);
    let source_parent = source.parent().unwrap_or(base_dir);
    let destination_parent = destination.parent().unwrap_or(base_dir);
    if source_parent != destination_parent {
        return Err(CjError::new(format!(
            "line {line_number}: @rename cannot move across directories"
        )));
    }
    fs::rename(source, destination)?;
    Ok(())
}

pub(super) fn remove_path(
    base_dir: &Path,
    scope_base: &Path,
    argv: &[String],
    line_number: usize,
) -> CjResult<()> {
    if argv.len() != 1 {
        return Err(CjError::new(format!(
            "line {line_number}: @clean expects exactly one path"
        )));
    }

    let path = base_dir.join(&argv[0]);
    if !path.exists() {
        return Ok(());
    }

    validate_removal_target(scope_base, &path, line_number)?;
    if path.is_dir() {
        fs::remove_dir_all(&path)?;
    } else {
        fs::remove_file(&path)?;
    }
    Ok(())
}

fn validate_removal_target(scope_base: &Path, path: &Path, line_number: usize) -> CjResult<()> {
    let scope_base = fs::canonicalize(scope_base)?;
    let path = fs::canonicalize(path)?;
    if path == scope_base || !path.starts_with(&scope_base) {
        return Err(CjError::new(format!(
            "line {line_number}: @clean cannot remove the current scope directory or any parent directory"
        )));
    }
    Ok(())
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> CjResult<()> {
    fs::create_dir_all(destination)?;
    copy_dir_contents(source, destination)
}

fn copy_dir_contents(source: &Path, destination: &Path) -> CjResult<()> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &destination_path)?;
        } else {
            fs::copy(&source_path, &destination_path)?;
        }
    }
    Ok(())
}

fn file_name<'a>(
    path: &'a Path,
    line_number: usize,
    context: &str,
) -> CjResult<&'a std::ffi::OsStr> {
    path.file_name().ok_or_else(|| {
        CjError::new(format!(
            "line {line_number}: {context} must have a file name"
        ))
    })
}

fn has_trailing_separator(path: &str) -> bool {
    path.ends_with('/') || path.ends_with('\\')
}

fn trim_trailing_separators(path: &str) -> &str {
    path.trim_end_matches(['/', '\\'])
}
