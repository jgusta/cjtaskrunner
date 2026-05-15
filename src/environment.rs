fn build_effective_env(base_dir: &Path, entries: &EnvEntries) -> CjResult<HashMap<String, String>> {
    let mut effective: HashMap<String, String> = env::vars().collect();

    load_dot_env_absent_only(base_dir, &mut effective)?;

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

fn load_dot_env_absent_only(
    base_dir: &Path,
    effective: &mut HashMap<String, String>,
) -> CjResult<()> {
    let path = base_dir.join(".env");
    if !path.exists() {
        return Ok(());
    }

    let source = fs::read_to_string(&path)
        .map_err(|err| CjError::new(format!("failed to read {}: {err}", path.display())))?;
    for (index, raw_line) in source.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let Some(eq_index) = line.find('=') else {
            return Err(line_error(
                &path,
                line_number,
                ".env entry must contain '='",
            ));
        };
        let key = &line[..eq_index];
        validate_env_name(key).map_err(|err| {
            line_error(
                &path,
                line_number,
                format!("invalid .env name '{key}': {err}"),
            )
        })?;
        let value = strip_matching_quotes(&line[eq_index + 1..]);
        effective.entry(key.to_string()).or_insert(value);
    }

    Ok(())
}

fn apply_python_venv(base_dir: &Path, effective: &mut HashMap<String, String>) -> CjResult<()> {
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
