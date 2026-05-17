fn parse_task_file_path(path: &Path) -> CjResult<TaskFile> {
    let source = fs::read_to_string(path)
        .map_err(|err| CjError::new(format!("failed to read {}: {err}", path.display())))?;
    parse_task_file(&source, path)
}

pub fn parse_task_file(source: &str, path: &Path) -> CjResult<TaskFile> {
    let mut env = EnvEntries::default();
    let mut tasks: HashMap<String, Vec<TaskLine>> = HashMap::new();
    let mut descriptions: HashMap<String, String> = HashMap::new();
    let mut help_lines = Vec::new();
    let mut task_help: HashMap<String, String> = HashMap::new();
    let mut task_order = Vec::new();
    let mut section = Section::Top;
    let mut current_task: Option<String> = None;
    let mut seen_env = false;
    let mut seen_help = false;
    let mut active_task_help: Option<(String, usize, Vec<String>)> = None;

    for (index, raw_line) in source.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        let trimmed = line.trim();

        if let Some((_, help_indent, lines)) = active_task_help.as_mut() {
            if trimmed.is_empty() {
                lines.push(String::new());
                continue;
            }
            let indent = line.chars().take_while(|ch| *ch == ' ').count();
            if line.starts_with(' ') && indent > *help_indent {
                lines.push(strip_help_indent(line, *help_indent + 2).to_string());
                continue;
            }
        }
        if let Some((task, _, lines)) = active_task_help.take() {
            task_help.insert(task, finish_help(lines));
        }

        if section == Section::Help {
            if trimmed.is_empty() {
                help_lines.push(String::new());
                continue;
            }
            if line.starts_with(' ') {
                let indent = line.chars().take_while(|ch| *ch == ' ').count();
                if indent < 2 {
                    return Err(line_error(
                        path,
                        line_number,
                        "help entries must use at least two leading spaces",
                    ));
                }
                help_lines.push(strip_help_indent(line, 2).to_string());
                continue;
            }
            section = Section::Top;
        }

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if !line.starts_with(' ') {
            current_task = None;
            let key = parse_top_level_key(line, path, line_number)?;
            if key == "env" {
                if seen_env {
                    return Err(line_error(
                        path,
                        line_number,
                        "multiple env sections are not allowed",
                    ));
                }
                seen_env = true;
                section = Section::Env;
            } else if key == "help" {
                if seen_help {
                    return Err(line_error(
                        path,
                        line_number,
                        "multiple help sections are not allowed",
                    ));
                }
                seen_help = true;
                section = Section::Help;
            } else {
                validate_task_name(&key).map_err(|err| {
                    line_error(
                        path,
                        line_number,
                        format!("invalid task name '{key}': {err}"),
                    )
                })?;
                if tasks.contains_key(&key) {
                    return Err(line_error(
                        path,
                        line_number,
                        format!("duplicate task '{key}'"),
                    ));
                }
                task_order.push(key.clone());
                tasks.insert(key.clone(), Vec::new());
                current_task = Some(key);
                section = Section::Task;
            }
            continue;
        }

        let indent = line.chars().take_while(|ch| *ch == ' ').count();
        if indent < 2 || indent % 2 != 0 {
            return Err(line_error(
                path,
                line_number,
                "indented entries must use an even number of spaces, at least two",
            ));
        }

        match section {
            Section::Env => {
                if indent != 2 {
                    return Err(line_error(
                        path,
                        line_number,
                        "env entries must use exactly two leading spaces",
                    ));
                }
                parse_env_entry(&line[2..], &mut env, path, line_number)?;
            }
            Section::Task => {
                let task_name = current_task
                    .as_ref()
                    .ok_or_else(|| line_error(path, line_number, "command without a task"))?;
                let text = &line[indent..];
                if text.is_empty() {
                    continue;
                }
                validate_directive_syntax(text, path, line_number)?;
                if indent == 2 {
                    if let Some(description) = parse_description(text) {
                        descriptions.insert(task_name.clone(), description.to_string());
                        continue;
                    }
                    if is_help_directive(text) {
                        active_task_help = Some((task_name.clone(), indent, Vec::new()));
                        continue;
                    }
                }
                let task = tasks.get_mut(task_name).expect("current task must exist");
                for text in split_line_expressions(text) {
                    task.push(TaskLine {
                        line_number,
                        indent,
                        text,
                    });
                }
            }
            Section::Top => {
                return Err(line_error(
                    path,
                    line_number,
                    "indented entry is not under env or a task",
                ));
            }
            Section::Help => unreachable!("top-level help handled before section dispatch"),
        }
    }

    if let Some((task, _, lines)) = active_task_help.take() {
        task_help.insert(task, finish_help(lines));
    }

    Ok(TaskFile {
        env,
        tasks,
        descriptions,
        help: seen_help.then(|| finish_help(help_lines)),
        task_help,
        task_order,
    })
}

fn parse_description(text: &str) -> Option<&str> {
    let args = text.strip_prefix("@desc")?;
    if !args.is_empty() && !args.starts_with(char::is_whitespace) {
        return None;
    }
    Some(args.trim())
}

fn is_help_directive(text: &str) -> bool {
    text == "@help:"
}

fn strip_help_indent(line: &str, width: usize) -> &str {
    let mut remaining = width;
    let mut byte_index = 0;
    for (index, ch) in line.char_indices() {
        if remaining == 0 {
            byte_index = index;
            break;
        }
        if ch != ' ' {
            byte_index = index;
            break;
        }
        remaining -= 1;
        byte_index = index + 1;
    }
    &line[byte_index..]
}

fn finish_help(mut lines: Vec<String>) -> String {
    while lines.first().is_some_and(|line| line.is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

fn split_line_expressions(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;

    for ch in text.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            current.push(ch);
            escaped = true;
            continue;
        }
        match quote {
            Some(active) if ch == active => {
                quote = None;
                current.push(ch);
            }
            Some(_) => current.push(ch),
            None if ch == '\'' || ch == '"' => {
                quote = Some(ch);
                current.push(ch);
            }
            None if ch == ';' => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    parts.push(trimmed.to_string());
                }
                current.clear();
            }
            None => current.push(ch),
        }
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() {
        parts.push(trimmed.to_string());
    }
    parts
}

fn parse_top_level_key(line: &str, path: &Path, line_number: usize) -> CjResult<String> {
    if !line.ends_with(':') {
        return Err(line_error(
            path,
            line_number,
            "top-level entries must be a key followed by ':'",
        ));
    }

    let key = &line[..line.len() - 1];
    if key.trim() != key || key.is_empty() {
        return Err(line_error(path, line_number, "invalid top-level key"));
    }
    Ok(key.to_string())
}

fn validate_directive_syntax(text: &str, path: &Path, line_number: usize) -> CjResult<()> {
    if let Some(rest) = text.strip_prefix('@') {
        let (name, args) = split_directive(rest);
        let colon_block_directive = matches!(
            name,
            "if" | "if-exists"
                | "if-missing"
                | "if-set"
                | "if-unset"
                | "else"
                | "switch"
                | "case"
                | "default"
        ) && args.trim_end().ends_with(':');
        if name == "help:" {
            return Ok(());
        }
        if name == "help" {
            return Err(line_error(path, line_number, "@help must use trailing ':'"));
        }
        if name.ends_with(':') || colon_block_directive {
            return Err(line_error(
                path,
                line_number,
                "CJTaskrunner directives do not use trailing ':'",
            ));
        }
    }
    Ok(())
}

fn parse_env_entry(
    entry: &str,
    env: &mut EnvEntries,
    path: &Path,
    line_number: usize,
) -> CjResult<()> {
    let Some(colon_index) = entry.find(':') else {
        return Err(line_error(path, line_number, "env entry must contain ':'"));
    };
    let raw_key = &entry[..colon_index];
    let fallback = raw_key.ends_with('?');
    let key = if fallback {
        &raw_key[..raw_key.len() - 1]
    } else {
        raw_key
    };

    validate_env_name(key).map_err(|err| {
        line_error(
            path,
            line_number,
            format!("invalid env name '{key}': {err}"),
        )
    })?;

    if env.overrides.contains_key(key) || env.fallbacks.contains_key(key) {
        return Err(line_error(
            path,
            line_number,
            format!("duplicate env entry '{key}'"),
        ));
    }

    let value = strip_matching_quotes(strip_one_leading_space(&entry[colon_index + 1..]));
    if fallback {
        env.fallbacks.insert(key.to_string(), value);
    } else {
        env.overrides.insert(key.to_string(), value);
    }
    Ok(())
}

fn validate_task_name(name: &str) -> Result<(), &'static str> {
    if name.is_empty() {
        return Err("task name cannot be empty");
    }
    if name == "env" || name == "help" {
        return Err("'env' and 'help' are reserved");
    }
    let mut parts = name.split(':');
    let Some(first) = parts.next() else {
        return Err("task name cannot be empty");
    };
    if !valid_task_name_part(first) {
        return Err("task names must contain ASCII letters, digits, hyphens, and underscores");
    }
    match (parts.next(), parts.next()) {
        (None, None) => Ok(()),
        (Some(second), None) if valid_task_name_part(second) => Ok(()),
        (Some(_), None) => Err("task name group cannot be empty"),
        _ => Err("task names may contain at most one colon"),
    }
}

fn valid_task_name_part(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch == '-' || ch == '_' || ch.is_ascii_alphanumeric())
}

fn validate_env_name(name: &str) -> Result<(), &'static str> {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Err("env name cannot be empty");
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return Err("env names must start with a letter or underscore");
    }
    if chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric()) {
        Ok(())
    } else {
        Err("env names must contain only ASCII letters, digits, and underscores")
    }
}

fn strip_one_leading_space(value: &str) -> &str {
    value.strip_prefix(' ').unwrap_or(value)
}

fn strip_matching_quotes(value: &str) -> String {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'\'' && last == b'\'') || (first == b'"' && last == b'"') {
            return value[1..value.len() - 1].to_string();
        }
    }
    value.to_string()
}

fn line_error(path: &Path, line_number: usize, message: impl Into<String>) -> CjError {
    CjError::new(format!(
        "{}:{line_number}: {}",
        path.display(),
        message.into()
    ))
}
