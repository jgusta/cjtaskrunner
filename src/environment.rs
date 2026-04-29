use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

use crate::task_file::EnvEntries;
use crate::{CjError, CjResult};

pub(crate) fn build_effective_env(
    base_dir: &Path,
    entries: &EnvEntries,
) -> CjResult<HashMap<String, String>> {
    let mut effective: HashMap<String, String> = env::vars().collect();

    for (key, value) in &entries.fallbacks {
        effective
            .entry(key.clone())
            .or_insert_with(|| value.clone());
    }

    for (key, value) in &entries.overrides {
        effective.insert(key.clone(), value.clone());
    }

    apply_python_venv(base_dir, &mut effective)?;

    Ok(effective)
}

pub(crate) fn apply_python_venv(
    base_dir: &Path,
    effective: &mut HashMap<String, String>,
) -> CjResult<()> {
    let selected = if let Some(path) = non_empty_env(effective, "VIRTUAL_ENV") {
        Some(PathBuf::from(path))
    } else if let Some(path) = non_empty_env(effective, "CJ_VENV") {
        Some(PathBuf::from(path))
    } else {
        let local = base_dir.join(".venv");
        local.is_dir().then_some(local)
    };

    let Some(venv) = selected else {
        return Ok(());
    };

    let executable_dir = venv.join("bin");
    if !executable_dir.is_dir() {
        return Err(CjError::new(format!(
            "python virtualenv executable directory does not exist: {}",
            executable_dir.display()
        )));
    }

    let executable_dir = executable_dir.to_string_lossy().to_string();
    let path = match effective.get("PATH") {
        Some(existing) if !existing.is_empty() => format!("{executable_dir}:{existing}"),
        _ => executable_dir,
    };
    effective.insert("PATH".to_string(), path);
    effective.insert(
        "VIRTUAL_ENV".to_string(),
        venv.to_string_lossy().to_string(),
    );

    Ok(())
}

fn non_empty_env(effective: &HashMap<String, String>, key: &str) -> Option<String> {
    effective
        .get(key)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}
