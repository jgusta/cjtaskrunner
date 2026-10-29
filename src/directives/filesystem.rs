use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path};

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

pub(super) fn zip_paths(base_dir: &Path, argv: &[String], line_number: usize) -> CjResult<()> {
    if argv.len() < 2 {
        return Err(CjError::new(format!(
            "line {line_number}: @zip expects one or more paths and an output path"
        )));
    }
    let output_arg = Path::new(argv.last().unwrap());
    let mut output = base_dir.join(output_arg);
    let name = output.file_name().ok_or_else(|| {
        CjError::new(format!(
            "line {line_number}: @zip output must have a file name"
        ))
    })?;
    let zip_name = if name
        .to_string_lossy()
        .to_ascii_lowercase()
        .ends_with(".zip")
    {
        name.to_os_string()
    } else {
        let mut n = name.to_os_string();
        n.push(".zip");
        n
    };
    output.set_file_name(zip_name);
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let patterns = include_str!("../ignorelist.txt");
    let patterns: Vec<IgnorePattern> = patterns.lines().filter_map(IgnorePattern::parse).collect();
    let file = fs::File::create(&output)?;
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for source_arg in &argv[..argv.len() - 1] {
        let relative_source = Path::new(source_arg);
        let source = base_dir.join(relative_source);
        if !source.exists() {
            return Err(CjError::new(format!(
                "line {line_number}: @zip source does not exist: {}",
                source.display()
            )));
        }
        if source.is_dir() {
            zip_dir(&mut writer, &source, Path::new(""), &patterns, options)?;
        } else if !is_ignored(&source, base_dir, &patterns) {
            writer
                .start_file(
                    relative_source.to_string_lossy().replace('\\', "/"),
                    options,
                )
                .map_err(zip_error)?;
            let mut input = fs::File::open(source)?;
            std::io::copy(&mut input, &mut writer)?;
        }
    }
    writer.finish().map_err(zip_error)?;
    Ok(())
}

#[derive(Clone)]
struct IgnorePattern {
    text: String,
    negated: bool,
    directory: bool,
}
impl IgnorePattern {
    fn parse(line: &str) -> Option<Self> {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }
        let (negated, line) = line.strip_prefix('!').map_or((false, line), |s| (true, s));
        Some(Self {
            text: line.trim_end_matches('/').to_string(),
            negated,
            directory: line.ends_with('/'),
        })
    }
}
fn is_ignored(path: &Path, root: &Path, patterns: &[IgnorePattern]) -> bool {
    let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let mut ignored = false;
    for pattern in patterns {
        let candidate = if pattern.text.contains('/') {
            rel.as_str()
        } else {
            name.as_ref()
        };
        if glob_matches(&pattern.text, candidate)
            || (!pattern.directory
                && pattern.text.contains('/')
                && rel.split('/').any(|part| glob_matches(&pattern.text, part)))
        {
            ignored = !pattern.negated;
        }
    }
    ignored
}
fn glob_matches(pattern: &str, text: &str) -> bool {
    if pattern == text {
        return true;
    }
    if let Some(suffix) = pattern.strip_prefix("*") {
        return text.ends_with(suffix);
    }
    if let Some(prefix) = pattern.strip_suffix("*") {
        return text.starts_with(prefix);
    }
    pattern == "*"
}
fn zip_error(error: zip::result::ZipError) -> CjError {
    CjError::new(error.to_string())
}
fn zip_dir(
    writer: &mut zip::ZipWriter<fs::File>,
    dir: &Path,
    relative: &Path,
    patterns: &[IgnorePattern],
    options: zip::write::SimpleFileOptions,
) -> CjResult<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let rel = relative.join(entry.file_name());
        if is_ignored(&path, dir, patterns) {
            continue;
        }
        if path.is_dir() {
            zip_dir(writer, &path, &rel, patterns, options)?;
        } else {
            writer
                .start_file(rel.to_string_lossy().replace('\\', "/"), options)
                .map_err(zip_error)?;
            let mut input = fs::File::open(path)?;
            std::io::copy(&mut input, writer)?;
        }
    }
    Ok(())
}

pub(super) fn unzip_path(base_dir: &Path, argv: &[String], line_number: usize) -> CjResult<()> {
    if argv.is_empty() || argv.len() > 2 {
        return Err(CjError::new(format!(
            "line {line_number}: @unzip expects a zip file and optional destination"
        )));
    }
    let archive_path = base_dir.join(&argv[0]);
    let stem = archive_path
        .file_stem()
        .ok_or_else(|| CjError::new("@unzip zip path must have a file name"))?;
    let destination_base = if argv.len() == 2 {
        base_dir.join(&argv[1])
    } else {
        archive_path.parent().unwrap_or(base_dir).to_path_buf()
    };
    let destination = destination_base.join(stem);
    let archive_file = fs::File::open(&archive_path)?;
    let mut archive = zip::ZipArchive::new(archive_file).map_err(zip_error)?;
    let single_dir = if !archive.is_empty() {
        let first = archive.by_index(0).map_err(zip_error)?.name().to_string();
        let candidate = first.trim_end_matches('/').split('/').next().unwrap_or("");
        if !candidate.is_empty()
            && (0..archive.len()).all(|i| {
                archive
                    .by_index(i)
                    .map(|f| {
                        f.name().trim_end_matches('/') == candidate
                            || f.name().starts_with(&format!("{candidate}/"))
                    })
                    .unwrap_or(false)
            })
        {
            Some(candidate.to_string())
        } else {
            None
        }
    } else {
        None
    };
    fs::create_dir_all(&destination)?;
    for i in 0..archive.len() {
        let mut item = archive.by_index(i).map_err(zip_error)?;
        let original = Path::new(item.name());
        if original.components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        }) {
            return Err(CjError::new("@unzip archive contains an unsafe path"));
        }
        let stripped = if let Some(root) = &single_dir {
            original.strip_prefix(root).unwrap_or(original)
        } else {
            original
        };
        if stripped.as_os_str().is_empty() {
            continue;
        }
        let output = destination.join(stripped);
        if item.is_dir() {
            fs::create_dir_all(&output)?;
        } else {
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut out = fs::File::create(output)?;
            let mut buffer = Vec::new();
            item.read_to_end(&mut buffer)?;
            out.write_all(&buffer)?;
        }
    }
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
