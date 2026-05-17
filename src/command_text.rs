fn run_direct_command(
    base_dir: &Path,
    command: &str,
    effective_env: &RuntimeEnv,
    output_mode: OutputMode,
) -> CjResult<CommandResult> {
    let argv = interpolate_argv(command, &effective_env.vars)?;
    let Some(program) = argv.first() else {
        return Ok(CommandResult::default());
    };

    let mut child = Command::new(program);
    child
        .args(&argv[1..])
        .current_dir(base_dir)
        .env_clear()
        .envs(&effective_env.exports)
        .stdin(Stdio::inherit())
        .stderr(Stdio::inherit());

    let result = run_child(child, output_mode)
        .map_err(|err| CjError::new(format!("failed to run command '{command}': {err}")))?;
    Ok(result)
}

fn run_shell_command(
    base_dir: &Path,
    command: &str,
    effective_env: &RuntimeEnv,
    output_mode: OutputMode,
) -> CjResult<CommandResult> {
    let mut child = Command::new("/bin/sh");
    child
        .arg("-c")
        .arg(command)
        .current_dir(base_dir)
        .env_clear()
        .envs(&effective_env.exports)
        .stdin(Stdio::inherit())
        .stderr(Stdio::inherit());

    let result = run_child(child, output_mode)
        .map_err(|err| CjError::new(format!("failed to run shell command '{command}': {err}")))?;
    Ok(result)
}

fn run_child(mut child: Command, output_mode: OutputMode) -> io::Result<CommandResult> {
    match output_mode {
        OutputMode::Inherit => {
            let status = child.stdout(Stdio::inherit()).status()?;
            Ok(CommandResult {
                status: status.code().unwrap_or(1),
                output: String::new(),
            })
        }
        OutputMode::Capture => {
            let output = child.stdout(Stdio::piped()).output()?;
            let text = String::from_utf8_lossy(&output.stdout).to_string();
            append_captured_output(&text);
            Ok(CommandResult {
                status: output.status.code().unwrap_or(1),
                output: text,
            })
        }
    }
}

fn interpolate_argv(
    command: &str,
    effective_env: &HashMap<String, String>,
) -> CjResult<Vec<String>> {
    split_words(command)?
        .into_iter()
        .map(|word| interpolate_text(&word, effective_env, QuoteMode::None))
        .collect()
}

fn interpolate_shell_text(command: &str, effective_env: &RuntimeEnv) -> CjResult<String> {
    interpolate_text(command, &effective_env.vars, QuoteMode::Shell)
}

fn interpolate_text(
    input: &str,
    effective_env: &HashMap<String, String>,
    quote_mode: QuoteMode,
) -> CjResult<String> {
    let mut output = String::new();
    let mut chars = input.char_indices().peekable();
    while let Some((_, ch)) = chars.next() {
        if ch == '\\' {
            if let Some((_, '$')) = chars.peek().copied() {
                chars.next();
                output.push('$');
            } else {
                output.push(ch);
            }
            continue;
        }
        if ch != '$' {
            output.push(ch);
            continue;
        }

        let Some((_, next)) = chars.peek().copied() else {
            output.push('$');
            continue;
        };
        if next == '{' {
            chars.next();
            let mut expression = String::new();
            let mut closed = false;
            for (_, expr_ch) in chars.by_ref() {
                if expr_ch == '}' {
                    closed = true;
                    break;
                }
                expression.push(expr_ch);
            }
            if !closed {
                return Err(CjError::new("unterminated variable interpolation"));
            }
            let value = expand_braced(&expression, effective_env)?;
            output.push_str(&quote_value(&value, quote_mode));
            continue;
        }
        if !is_env_start(next) {
            output.push('$');
            continue;
        }
        let mut name = String::new();
        while let Some((_, name_ch)) = chars.peek().copied() {
            if is_env_continue(name_ch) {
                chars.next();
                name.push(name_ch);
            } else {
                break;
            }
        }
        let value = effective_env.get(&name).cloned().unwrap_or_default();
        output.push_str(&quote_value(&value, quote_mode));
    }
    Ok(output)
}

fn expand_braced(expression: &str, effective_env: &HashMap<String, String>) -> CjResult<String> {
    if let Some((name, fallback)) = expression.split_once(":-") {
        validate_env_name(name).map_err(|err| {
            CjError::new(format!(
                "invalid variable interpolation '{expression}': {err}"
            ))
        })?;
        Ok(match effective_env.get(name) {
            Some(value) if !value.is_empty() => value.clone(),
            _ => fallback.to_string(),
        })
    } else {
        validate_env_name(expression).map_err(|err| {
            CjError::new(format!(
                "invalid variable interpolation '{expression}': {err}"
            ))
        })?;
        effective_env
            .get(expression)
            .cloned()
            .ok_or_else(|| CjError::new(format!("missing variable: {expression}")))
    }
}

fn quote_value(value: &str, quote_mode: QuoteMode) -> String {
    match quote_mode {
        QuoteMode::None => value.to_string(),
        QuoteMode::Shell => shlex::try_quote(value)
            .map(|quoted| quoted.into_owned())
            .unwrap_or_else(|_| "''".to_string()),
    }
}

fn is_env_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_env_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn split_words(command: &str) -> CjResult<Vec<String>> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut chars = command.chars().peekable();
    let mut quote: Option<char> = None;
    let mut in_word = false;

    while let Some(ch) = chars.next() {
        match quote {
            Some(active) if ch == active => {
                quote = None;
                in_word = true;
            }
            Some('\'') => {
                current.push(ch);
                in_word = true;
            }
            Some('"') if ch == '\\' => {
                if let Some(next) = chars.next() {
                    if next == '$' {
                        current.push('\\');
                    }
                    current.push(next);
                    in_word = true;
                } else {
                    current.push(ch);
                    in_word = true;
                }
            }
            Some(_) => {
                current.push(ch);
                in_word = true;
            }
            None if ch == '\'' || ch == '"' => {
                quote = Some(ch);
                in_word = true;
            }
            None if ch.is_whitespace() => {
                if in_word {
                    words.push(std::mem::take(&mut current));
                    in_word = false;
                }
            }
            None if ch == '\\' => {
                if let Some(next) = chars.next() {
                    if next == '$' {
                        current.push('\\');
                    }
                    current.push(next);
                } else {
                    current.push(ch);
                }
                in_word = true;
            }
            None => {
                current.push(ch);
                in_word = true;
            }
        }
    }

    if let Some(active) = quote {
        return Err(CjError::new(format!("unterminated {active} quote")));
    }
    if in_word {
        words.push(current);
    }
    Ok(words)
}
