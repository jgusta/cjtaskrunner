use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::task_file::{parse_task_file, validate_task_name};
use crate::taskfile_discovery::base_taskfile_path;
use crate::{CjError, CjResult};

const INITIAL_TASKFILE: &str = "";

struct ImportGroup {
    source: String,
    tasks: Vec<ImportedTask>,
}

struct ImportedTask {
    base_name: String,
    command: String,
}

pub(crate) fn init_taskfile(cwd: &Path) -> CjResult<i32> {
    if let Some(existing) = existing_taskfile(cwd) {
        return Err(CjError::new(format!(
            "taskfile already exists: {}",
            existing.display()
        )));
    }

    let path = base_taskfile_path(cwd);
    fs::write(&path, INITIAL_TASKFILE)
        .map_err(|err| CjError::new(format!("failed to write {}: {err}", path.display())))?;
    println!("created {}", path.display());
    Ok(0)
}

pub(crate) fn auto_import_tasks(cwd: &Path) -> CjResult<i32> {
    let groups = discover_imports(cwd)?;
    if groups.iter().all(|group| group.tasks.is_empty()) {
        return Err(CjError::new(
            "no importable tasks found in package.json, deno.json, Makefile, or Justfile",
        ));
    }

    let path = existing_taskfile(cwd).unwrap_or_else(|| base_taskfile_path(cwd));
    let existed = path.is_file();
    let source = if existed {
        fs::read_to_string(&path)
            .map_err(|err| CjError::new(format!("failed to read {}: {err}", path.display())))?
    } else {
        INITIAL_TASKFILE.to_string()
    };
    let parsed = parse_task_file(&source, &path)?;
    let mut known = parsed.task_order.iter().cloned().collect::<HashSet<_>>();
    let mut imported_commands = parsed
        .tasks
        .values()
        .filter(|lines| lines.len() == 1 && !lines[0].text.starts_with('@'))
        .map(|lines| lines[0].text.clone())
        .collect::<HashSet<_>>();
    let mut additions = String::new();
    let mut added = 0usize;
    let mut skipped = 0usize;

    for group in groups {
        let mut block = String::new();
        for task in group.tasks {
            if imported_commands.contains(&task.command) {
                skipped += 1;
                continue;
            }
            let name = available_task_name(&task.base_name, &known, cwd);
            known.insert(name.clone());
            imported_commands.insert(task.command.clone());
            block.push_str(&format!("{name}:\n  {}\n", task.command));
            added += 1;
        }
        if !block.is_empty() {
            additions.push_str(&format!(
                "# Imported from {} by cj --auto\n{}\n",
                group.source, block
            ));
        }
    }

    if added == 0 {
        println!("no new tasks to add to {}", path.display());
        return Ok(0);
    }

    let mut updated = source;
    if !updated.is_empty() {
        if !updated.ends_with('\n') {
            updated.push('\n');
        }
        if !updated.ends_with("\n\n") {
            updated.push('\n');
        }
    }
    updated.push_str(&additions);
    parse_task_file(&updated, &path)?;
    fs::write(&path, updated)
        .map_err(|err| CjError::new(format!("failed to write {}: {err}", path.display())))?;

    let action = if existed { "updated" } else { "created" };
    println!("{action} {} with {added} imported tasks", path.display());
    if skipped > 0 {
        println!("skipped {skipped} existing or conflicting tasks");
    }
    Ok(0)
}

fn existing_taskfile(cwd: &Path) -> Option<PathBuf> {
    let path = base_taskfile_path(cwd);
    path.is_file().then_some(path)
}

fn discover_imports(cwd: &Path) -> CjResult<Vec<ImportGroup>> {
    let mut groups = Vec::new();
    if let Some(group) = package_json_tasks(cwd)? {
        groups.push(group);
    }
    if let Some(group) = json_tasks(&cwd.join("deno.json"), "tasks", "deno task")? {
        groups.push(group);
    }
    if let Some(path) = first_file(cwd, &["Makefile", "makefile"]) {
        groups.push(makefile_tasks(&path)?);
    }
    if let Some(path) = first_file(cwd, &["Justfile", "justfile"]) {
        groups.push(justfile_tasks(&path)?);
    }
    Ok(groups)
}

fn package_json_tasks(cwd: &Path) -> CjResult<Option<ImportGroup>> {
    let path = cwd.join("package.json");
    if !path.is_file() {
        return Ok(None);
    }
    let root = read_json_object(&path)?;
    let manager = package_manager(cwd, &root);
    let tasks = json_entries(&path, &root, "scripts")?
        .into_iter()
        .filter_map(|name| imported_task(&name, &format!("{manager} run")))
        .collect();
    Ok(Some(ImportGroup {
        source: "package.json".to_string(),
        tasks,
    }))
}

fn json_tasks(path: &Path, key: &str, command: &str) -> CjResult<Option<ImportGroup>> {
    if !path.is_file() {
        return Ok(None);
    }
    let root = read_json_object(path)?;
    let tasks = json_entries(path, &root, key)?
        .into_iter()
        .filter_map(|name| imported_task(&name, command))
        .collect();
    Ok(Some(ImportGroup {
        source: path
            .file_name()
            .expect("task source filename")
            .to_string_lossy()
            .into_owned(),
        tasks,
    }))
}

fn read_json_object(path: &Path) -> CjResult<Map<String, Value>> {
    let source = fs::read_to_string(path)
        .map_err(|err| CjError::new(format!("failed to read {}: {err}", path.display())))?;
    let value: Value = serde_json::from_str(&source)
        .map_err(|err| CjError::new(format!("failed to parse {}: {err}", path.display())))?;
    value.as_object().cloned().ok_or_else(|| {
        CjError::new(format!(
            "{} must contain a top-level JSON object",
            path.display()
        ))
    })
}

fn json_entries(path: &Path, root: &Map<String, Value>, key: &str) -> CjResult<Vec<String>> {
    let Some(value) = root.get(key) else {
        return Ok(Vec::new());
    };
    let entries = value
        .as_object()
        .ok_or_else(|| CjError::new(format!("{} '{key}' must be an object", path.display())))?;
    let mut names = Vec::new();
    for (name, command) in entries {
        if !command.is_string() {
            return Err(CjError::new(format!(
                "{} {key} entry '{name}' must be a string",
                path.display()
            )));
        }
        names.push(name.clone());
    }
    names.sort();
    Ok(names)
}

fn package_manager(cwd: &Path, root: &Map<String, Value>) -> String {
    if let Some(manager) = root
        .get("packageManager")
        .and_then(Value::as_str)
        .and_then(|value| value.split('@').next())
        .filter(|value| matches!(*value, "npm" | "pnpm" | "yarn" | "bun"))
    {
        return manager.to_string();
    }
    for (lockfile, manager) in [
        ("pnpm-lock.yaml", "pnpm"),
        ("yarn.lock", "yarn"),
        ("bun.lock", "bun"),
        ("bun.lockb", "bun"),
    ] {
        if cwd.join(lockfile).is_file() {
            return manager.to_string();
        }
    }
    "npm".to_string()
}

fn makefile_tasks(path: &Path) -> CjResult<ImportGroup> {
    let source = read_source(path)?;
    let mut names = HashSet::new();
    for line in source.lines() {
        if line.is_empty() || line.starts_with([' ', '\t', '#']) {
            continue;
        }
        let Some((targets, prerequisites)) = line.split_once(':') else {
            continue;
        };
        if targets.contains(['=', '$'])
            || prerequisites.trim_start().starts_with('=')
            || prerequisites.contains(':')
        {
            continue;
        }
        for target in targets.split_whitespace() {
            if !target.starts_with('.') && !target.contains(['%', '$', '(', ')', '&']) {
                names.insert(target.to_string());
            }
        }
    }
    Ok(import_group(path, "make", names))
}

fn justfile_tasks(path: &Path) -> CjResult<ImportGroup> {
    let source = read_source(path)?;
    let mut names = HashSet::new();
    for line in source.lines() {
        if line.is_empty() || line.starts_with([' ', '\t', '#', '[']) {
            continue;
        }
        let Some((header, body)) = line.split_once(':') else {
            continue;
        };
        if body.starts_with('=') || header.contains('=') {
            continue;
        }
        let mut words = header.split_whitespace();
        let Some(name) = words.next() else {
            continue;
        };
        if words.next().is_none() {
            names.insert(name.to_string());
        }
    }
    Ok(import_group(path, "just", names))
}

fn import_group(path: &Path, command: &str, names: HashSet<String>) -> ImportGroup {
    let mut names = names.into_iter().collect::<Vec<_>>();
    names.sort();
    ImportGroup {
        source: path
            .file_name()
            .expect("task source filename")
            .to_string_lossy()
            .into_owned(),
        tasks: names
            .into_iter()
            .filter_map(|name| imported_task(&name, command))
            .collect(),
    }
}

fn imported_task(source_name: &str, command: &str) -> Option<ImportedTask> {
    let base_name = normalize_task_name(source_name)?;
    validate_task_name(&base_name).ok()?;
    let escaped = source_name.replace('$', "\\$");
    let argument = shlex::try_quote(&escaped).ok()?.into_owned();
    Some(ImportedTask {
        base_name,
        command: format!("{command} {argument}"),
    })
}

fn normalize_task_name(source: &str) -> Option<String> {
    let mut normalized = String::new();
    for ch in source.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            normalized.push(ch);
        }
    }
    (!normalized.is_empty()).then_some(normalized)
}

fn available_task_name(base: &str, known: &HashSet<String>, cwd: &Path) -> String {
    if !known.contains(base) && !cwd.join(base).is_dir() {
        return base.to_string();
    }
    for suffix in 2usize.. {
        let candidate = format!("{base}{suffix}");
        if !known.contains(&candidate) && !cwd.join(&candidate).is_dir() {
            return candidate;
        }
    }
    unreachable!("usize task suffixes are exhaustive")
}

fn read_source(path: &Path) -> CjResult<String> {
    fs::read_to_string(path)
        .map_err(|err| CjError::new(format!("failed to read {}: {err}", path.display())))
}

fn first_file(cwd: &Path, names: &[&str]) -> Option<PathBuf> {
    names
        .iter()
        .map(|name| cwd.join(name))
        .find(|path| path.is_file())
}
