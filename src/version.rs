use std::cmp::Ordering;
use std::fs;

use crate::task_file::TaskFile;
use crate::{CjError, CjResult};

pub(crate) fn version_env_key(name: &str) -> Result<String, &'static str> {
    if name.is_empty() {
        return Err("version name cannot be empty");
    }
    if !name
        .chars()
        .all(|ch| ch == '-' || ch == '_' || ch.is_ascii_alphanumeric())
    {
        return Err("version names must contain ASCII letters, digits, hyphens, and underscores");
    }
    let normalized = name
        .chars()
        .map(|ch| {
            if ch == '-' {
                '_'
            } else {
                ch.to_ascii_uppercase()
            }
        })
        .collect::<String>();
    Ok(format!("VERSION_{normalized}"))
}

pub(crate) fn validate_semver(value: &str, line_number: usize) -> CjResult<()> {
    parse_semver(value, line_number).map(|_| ())
}

pub(crate) fn compare_semver(left: &str, right: &str, line_number: usize) -> CjResult<Ordering> {
    Ok(parse_semver(left, line_number)?.cmp(&parse_semver(right, line_number)?))
}

pub(crate) fn is_prerelease(value: &str, line_number: usize) -> CjResult<bool> {
    Ok(!parse_semver(value, line_number)?.prerelease.is_empty())
}

pub(crate) fn bump_taskfile_version(
    task_file: &TaskFile,
    name: &str,
    operation: &str,
    prerelease: Option<&str>,
    line_number: usize,
) -> CjResult<(String, String)> {
    let Some(entry) = task_file.versions.get(name) else {
        return Err(CjError::new(format!(
            "line {line_number}: unknown version '{name}'"
        )));
    };
    let Some(path) = task_file.source_path.as_ref() else {
        return Err(CjError::new(format!(
            "line {line_number}: version bump directives can only run from a taskfile path"
        )));
    };

    let source = fs::read_to_string(path)
        .map_err(|err| CjError::new(format!("failed to read {}: {err}", path.display())))?;
    let had_trailing_newline = source.ends_with('\n');
    let mut lines = source
        .lines()
        .map(|line| line.strip_suffix('\r').unwrap_or(line).to_string())
        .collect::<Vec<_>>();

    let header_index = entry.line_number.checked_sub(1).ok_or_else(|| {
        CjError::new(format!(
            "line {line_number}: invalid version header location for '{name}'"
        ))
    })?;
    let Some(current_line) = lines.get(header_index) else {
        return Err(CjError::new(format!(
            "line {line_number}: version header for '{name}' no longer exists"
        )));
    };
    let Some((directive, current_name, current_value)) = parse_current_header(current_line) else {
        return Err(CjError::new(format!(
            "line {line_number}: version header for '{name}' no longer matches @version syntax"
        )));
    };
    if current_name != entry.name {
        return Err(CjError::new(format!(
            "line {line_number}: version header changed from '{}' to '{current_name}'",
            entry.name
        )));
    }

    let new_value = bump_semver(current_value, operation, prerelease, line_number)?;
    lines[header_index] = format!("{directive} {} {new_value}", entry.name);

    let mut updated = lines.join("\n");
    if had_trailing_newline {
        updated.push('\n');
    }
    fs::write(path, updated)
        .map_err(|err| CjError::new(format!("failed to write {}: {err}", path.display())))?;

    Ok((entry.env_key.clone(), new_value))
}

fn parse_current_header(line: &str) -> Option<(&str, &str, &str)> {
    let mut parts = line.split_whitespace();
    let directive = parts.next()?;
    if directive != "@version" {
        return None;
    }
    let name = parts.next()?;
    let value = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    Some((directive, name, value))
}

fn bump_semver(
    value: &str,
    operation: &str,
    prerelease: Option<&str>,
    line_number: usize,
) -> CjResult<String> {
    let mut version = parse_semver(value, line_number)?;
    match operation {
        "major" => {
            version.major = increment_part(version.major, value, line_number)?;
            version.minor = 0;
            version.patch = 0;
            version.prerelease.clear();
        }
        "minor" => {
            version.minor = increment_part(version.minor, value, line_number)?;
            version.patch = 0;
            version.prerelease.clear();
        }
        "patch" => {
            version.patch = increment_part(version.patch, value, line_number)?;
            version.prerelease.clear();
        }
        "pre" => bump_prerelease(&mut version, prerelease, line_number)?,
        "release" => version.prerelease.clear(),
        _ => {
            return Err(CjError::new(format!(
                "line {line_number}: version bump operation must be major, minor, patch, pre, or release"
            )))
        }
    }
    Ok(version.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Semver {
    major: u64,
    minor: u64,
    patch: u64,
    prerelease: Vec<String>,
}

impl Ord for Semver {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then_with(|| self.minor.cmp(&other.minor))
            .then_with(|| self.patch.cmp(&other.patch))
            .then_with(|| compare_prerelease(&self.prerelease, &other.prerelease))
    }
}

impl PartialOrd for Semver {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl std::fmt::Display for Semver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if !self.prerelease.is_empty() {
            write!(f, "-{}", self.prerelease.join("."))?;
        }
        Ok(())
    }
}

fn parse_semver(value: &str, line_number: usize) -> CjResult<Semver> {
    if value.contains('+') {
        return Err(invalid_semver(value, line_number));
    }
    let (core, prerelease) = if let Some((core, prerelease)) = value.split_once('-') {
        if prerelease.is_empty() {
            return Err(invalid_semver(value, line_number));
        }
        (core, Some(prerelease))
    } else {
        (value, None)
    };
    let mut parts = core.split('.');
    let major = parse_numeric_identifier(parts.next(), value, line_number)?;
    let minor = parse_numeric_identifier(parts.next(), value, line_number)?;
    let patch = parse_numeric_identifier(parts.next(), value, line_number)?;
    if parts.next().is_some() {
        return Err(invalid_semver(value, line_number));
    }

    let prerelease = if let Some(prerelease) = prerelease {
        parse_prerelease(prerelease, value, line_number)?
    } else {
        Vec::new()
    };

    Ok(Semver {
        major,
        minor,
        patch,
        prerelease,
    })
}

fn parse_numeric_identifier(part: Option<&str>, value: &str, line_number: usize) -> CjResult<u64> {
    let Some(part) = part else {
        return Err(invalid_semver(value, line_number));
    };
    if part.is_empty() || (part.len() > 1 && part.starts_with('0')) {
        return Err(invalid_semver(value, line_number));
    }
    part.parse::<u64>()
        .map_err(|_| invalid_semver(value, line_number))
}

fn parse_prerelease(prerelease: &str, value: &str, line_number: usize) -> CjResult<Vec<String>> {
    let mut identifiers = Vec::new();
    for identifier in prerelease.split('.') {
        if !valid_prerelease_identifier(identifier) {
            return Err(invalid_semver(value, line_number));
        }
        identifiers.push(identifier.to_string());
    }
    Ok(identifiers)
}

fn valid_prerelease_identifier(identifier: &str) -> bool {
    if identifier.is_empty()
        || !identifier
            .chars()
            .all(|ch| ch == '-' || ch.is_ascii_alphanumeric())
    {
        return false;
    }
    if identifier.chars().all(|ch| ch.is_ascii_digit()) {
        return identifier == "0" || !identifier.starts_with('0');
    }
    true
}

fn compare_prerelease(left: &[String], right: &[String]) -> Ordering {
    match (left.is_empty(), right.is_empty()) {
        (true, true) => return Ordering::Equal,
        (true, false) => return Ordering::Greater,
        (false, true) => return Ordering::Less,
        (false, false) => {}
    }
    for (left, right) in left.iter().zip(right.iter()) {
        let ordering = compare_prerelease_identifier(left, right);
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    left.len().cmp(&right.len())
}

fn compare_prerelease_identifier(left: &str, right: &str) -> Ordering {
    match (left.parse::<u64>(), right.parse::<u64>()) {
        (Ok(left), Ok(right)) => left.cmp(&right),
        (Ok(_), Err(_)) => Ordering::Less,
        (Err(_), Ok(_)) => Ordering::Greater,
        (Err(_), Err(_)) => left.cmp(right),
    }
}

fn bump_prerelease(
    version: &mut Semver,
    prerelease: Option<&str>,
    line_number: usize,
) -> CjResult<()> {
    match prerelease {
        Some(value) if value.ends_with('.') => {
            let base = value.trim_end_matches('.');
            let mut next = if base.is_empty() {
                Vec::new()
            } else {
                parse_prerelease(base, value, line_number)?
            };
            let number = if version.prerelease.len() == next.len() + 1
                && version.prerelease.starts_with(&next)
            {
                version
                    .prerelease
                    .last()
                    .and_then(|last| last.parse::<u64>().ok())
                    .map(|number| increment_part(number, value, line_number))
                    .transpose()?
                    .unwrap_or(0)
            } else {
                0
            };
            next.push(number.to_string());
            version.prerelease = next;
        }
        Some(value) => {
            version.prerelease = parse_prerelease(value, value, line_number)?;
        }
        None => {
            let current = version.to_string();
            if let Some(last) = version.prerelease.last_mut() {
                if let Ok(number) = last.parse::<u64>() {
                    *last = increment_part(number, &current, line_number)?.to_string();
                    return Ok(());
                }
            }
            version.prerelease.push("0".to_string());
        }
    }
    Ok(())
}

fn increment_part(part: u64, value: &str, line_number: usize) -> CjResult<u64> {
    part.checked_add(1).ok_or_else(|| {
        CjError::new(format!(
            "line {line_number}: version '{value}' is too large"
        ))
    })
}

fn invalid_semver(value: &str, line_number: usize) -> CjError {
    CjError::new(format!(
        "line {line_number}: version '{value}' must be semantic version MAJOR.MINOR.PATCH with optional prerelease"
    ))
}
