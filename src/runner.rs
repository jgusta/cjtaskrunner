fn run_task(
    base_dir: &Path,
    task_file: &TaskFile,
    task_name: &str,
    effective_env: &mut RuntimeEnv,
    stack: &mut Vec<String>,
) -> CjResult<i32> {
    validate_task_name(task_name)
        .map_err(|err| CjError::new(format!("invalid task name '{task_name}': {err}")))?;
    if let Some(index) = stack.iter().position(|active| active == task_name) {
        let mut cycle = stack[index..].to_vec();
        cycle.push(task_name.to_string());
        return Err(CjError::new(format!(
            "recursive @task cycle detected: {}",
            cycle.join(" -> ")
        )));
    }

    let lines = task_file
        .tasks
        .get(task_name)
        .ok_or_else(|| CjError::new(format!("task not found: {task_name}")))?;
    stack.push(task_name.to_string());
    let result = execute_block(
        base_dir,
        task_file,
        lines,
        0,
        lines.len(),
        2,
        effective_env,
        stack,
        OutputMode::Inherit,
    );
    stack.pop();
    result
}

fn execute_block(
    base_dir: &Path,
    task_file: &TaskFile,
    lines: &[TaskLine],
    start: usize,
    end: usize,
    indent: usize,
    effective_env: &mut RuntimeEnv,
    stack: &mut Vec<String>,
    output_mode: OutputMode,
) -> CjResult<i32> {
    let mut index = start;
    let mut previous_status = 0;
    while index < end {
        let line = &lines[index];
        if line.indent < indent {
            break;
        }
        if line.indent > indent {
            return Err(CjError::new(format!(
                "line {}: unexpected indentation",
                line.line_number
            )));
        }

        if let Some(rest) = line.text.strip_prefix('@') {
            let (name, _) = split_directive(rest);
            if name == "and" || name == "or" {
                let status = execute_chain_directive(
                    base_dir,
                    task_file,
                    lines,
                    &mut index,
                    end,
                    indent,
                    name,
                    previous_status,
                    effective_env,
                    stack,
                    output_mode,
                )?;
                previous_status = status;
                if status != 0 && !next_directive_is(lines, index, end, indent, "or") {
                    return Ok(status);
                }
                continue;
            }
            let status = execute_directive(
                base_dir,
                task_file,
                lines,
                &mut index,
                end,
                indent,
                rest,
                effective_env,
                stack,
                output_mode,
            )?;
            previous_status = status;
            if status != 0 {
                if next_directive_is(lines, index, end, indent, "or") {
                    continue;
                }
                return Ok(status);
            }
        } else {
            let result = run_direct_command(base_dir, &line.text, effective_env, output_mode)?;
            index += 1;
            previous_status = result.status;
            if result.status != 0 {
                if next_directive_is(lines, index, end, indent, "or") {
                    continue;
                }
                return Ok(result.status);
            }
        }
    }

    Ok(previous_status)
}

#[allow(clippy::too_many_arguments)]
fn execute_chain_directive(
    base_dir: &Path,
    task_file: &TaskFile,
    lines: &[TaskLine],
    index: &mut usize,
    end: usize,
    indent: usize,
    name: &str,
    previous_status: i32,
    effective_env: &mut RuntimeEnv,
    stack: &mut Vec<String>,
    output_mode: OutputMode,
) -> CjResult<i32> {
    let line_number = lines[*index].line_number;
    let block_start = *index + 1;
    let block_end = find_block_end(lines, block_start, end, indent);
    *index = block_end;

    if block_start == block_end {
        return Err(CjError::new(format!(
            "line {line_number}: @{name} expects an indented block"
        )));
    }

    let should_run =
        (name == "and" && previous_status == 0) || (name == "or" && previous_status != 0);
    if should_run {
        execute_block(
            base_dir,
            task_file,
            lines,
            block_start,
            block_end,
            indent + 2,
            effective_env,
            stack,
            output_mode,
        )
    } else if name == "and" {
        Ok(1)
    } else {
        Ok(0)
    }
}

fn next_directive_is(
    lines: &[TaskLine],
    index: usize,
    end: usize,
    indent: usize,
    expected: &str,
) -> bool {
    if index >= end || lines[index].indent != indent {
        return false;
    }
    lines[index]
        .text
        .strip_prefix('@')
        .map(split_directive)
        .is_some_and(|(name, _)| name == expected)
}

#[allow(clippy::too_many_arguments)]
fn execute_block_capture(
    base_dir: &Path,
    task_file: &TaskFile,
    lines: &[TaskLine],
    start: usize,
    end: usize,
    indent: usize,
    effective_env: &mut RuntimeEnv,
    stack: &mut Vec<String>,
) -> CjResult<String> {
    CAPTURED_OUTPUT.with(|captured| captured.borrow_mut().clear());
    let status = execute_block(
        base_dir,
        task_file,
        lines,
        start,
        end,
        indent,
        effective_env,
        stack,
        OutputMode::Capture,
    )?;
    let output = CAPTURED_OUTPUT.with(|captured| captured.borrow().clone());
    if status == 0 {
        Ok(output.trim_end_matches(['\r', '\n']).to_string())
    } else {
        Err(CjError::new(format!(
            "captured @set block failed with status {status}"
        )))
    }
}
