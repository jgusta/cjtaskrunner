use std::collections::HashSet;
use std::env;
use std::thread;

use crate::command_text::split_words;
use crate::command_text::{interrupted, run_direct_command};
use crate::directives::{execute_directive, find_block_end, split_directive};
use crate::runtime::{CwdState, OutputMode, RuntimeEnv, CAPTURED_OUTPUT};
use crate::task_file::{validate_task_name, TaskFile, TaskLine};
use crate::{CjError, CjResult};

pub(crate) const MAX_EXECUTION_STEPS: usize = 100_000;

pub(crate) struct ExecutionContext<'a> {
    pub(crate) task_file: &'a TaskFile,
    pub(crate) env: &'a mut RuntimeEnv,
    pub(crate) cwd: &'a mut CwdState,
    pub(crate) stack: &'a mut Vec<String>,
    pub(crate) output_mode: OutputMode,
}

pub(crate) fn run_task(
    task_file: &TaskFile,
    task_name: &str,
    arguments: &[String],
    effective_env: &mut RuntimeEnv,
    cwd: &mut CwdState,
    stack: &mut Vec<String>,
) -> CjResult<i32> {
    let mut ctx = ExecutionContext {
        task_file,
        env: effective_env,
        cwd,
        stack,
        output_mode: OutputMode::Inherit,
    };
    run_task_inner(&mut ctx, task_name, arguments)
}

pub(crate) fn run_task_inner(
    ctx: &mut ExecutionContext<'_>,
    task_name: &str,
    arguments: &[String],
) -> CjResult<i32> {
    validate_task_name(task_name)
        .map_err(|err| CjError::new(format!("invalid task name '{task_name}': {err}")))?;
    if let Some(index) = ctx.stack.iter().position(|active| active == task_name) {
        let mut cycle = ctx.stack[index..].to_vec();
        cycle.push(task_name.to_string());
        return Err(CjError::new(format!(
            "recursive @task cycle detected: {}",
            cycle.join(" -> ")
        )));
    }

    let lines = ctx
        .task_file
        .tasks
        .get(task_name)
        .ok_or_else(|| CjError::new(format!("task not found: {task_name}")))?;
    let parameters = ctx
        .task_file
        .task_arguments
        .get(task_name)
        .map(Vec::as_slice)
        .unwrap_or_default();
    if parameters.len() != arguments.len() {
        let mut message = format!(
            "task '{task_name}' expects {} arguments, received {}",
            parameters.len(),
            arguments.len()
        );
        if parameters.is_empty() && arguments.len() == 1 {
            let nested_name = format!("{task_name}:{}", arguments[0]);
            if ctx.task_file.tasks.contains_key(&nested_name) {
                message.push_str(&format!("\nDid you mean `{nested_name}`?"));
            }
        }
        return Err(CjError::new(message));
    }

    let task_vars = ctx.env.vars.clone();
    let task_exports = ctx.env.exported_values();
    for (name, value) in parameters.iter().zip(arguments) {
        ctx.env.vars.insert(name.clone(), value.clone());
    }
    ctx.stack.push(task_name.to_string());
    let result = execute_block(ctx, lines, 0, lines.len(), 2);
    ctx.stack.pop();
    ctx.env.restore_task_vars(task_vars, task_exports);
    result
}

pub(crate) fn execute_await_tasks(
    task_file: &TaskFile,
    task_names: &[String],
    effective_env: &RuntimeEnv,
    cwd: &CwdState,
) -> CjResult<i32> {
    let mut pending = HashSet::new();
    for task_name in task_names {
        if !task_file.tasks.contains_key(task_name) {
            return Err(CjError::new(format!("awaited task not found: {task_name}")));
        }
        validate_await_safe_task(task_file, task_name, &mut HashSet::new())?;
        if pending.insert(task_name.clone()) {
            collect_await_tasks(task_file, task_name, &mut pending);
        }
    }
    let mut completed = HashSet::new();
    let jobs = await_jobs();

    while !pending.is_empty() {
        let mut ready: Vec<String> = pending
            .iter()
            .filter(|name| {
                task_file.awaits.get(*name).is_none_or(|awaits| {
                    awaits
                        .iter()
                        .all(|awaited| completed.contains(&awaited.name))
                })
            })
            .cloned()
            .collect();
        ready.sort();
        if ready.is_empty() {
            return Err(CjError::new("task await cycle detected while running"));
        }

        for chunk in ready.chunks(jobs) {
            if interrupted() {
                return Ok(130);
            }
            let mut results = Vec::new();
            thread::scope(|scope| {
                let mut handles = Vec::new();
                for awaited in chunk {
                    let mut await_env = effective_env.clone();
                    await_env.await_blocks_satisfied = true;
                    let mut await_cwd = cwd.clone();
                    let mut await_stack = Vec::new();
                    let await_name = awaited.clone();
                    handles.push(scope.spawn(move || {
                        let mut ctx = ExecutionContext {
                            task_file,
                            env: &mut await_env,
                            cwd: &mut await_cwd,
                            stack: &mut await_stack,
                            output_mode: OutputMode::Inherit,
                        };
                        run_task_inner(&mut ctx, &await_name, &[])
                    }));
                }
                for handle in handles {
                    results.push(handle.join().unwrap_or_else(|_| {
                        Err(CjError::new("awaited task panicked while running"))
                    }));
                }
            });

            for result in results {
                let status = result?;
                if status != 0 {
                    return Ok(status);
                }
            }
        }

        for awaited in ready {
            pending.remove(&awaited);
            completed.insert(awaited);
        }
    }

    Ok(0)
}

fn collect_await_tasks(task_file: &TaskFile, task_name: &str, collected: &mut HashSet<String>) {
    if let Some(awaits) = task_file.awaits.get(task_name) {
        for awaited in awaits {
            if collected.insert(awaited.name.clone()) {
                collect_await_tasks(task_file, &awaited.name, collected);
            }
        }
    }
}

fn validate_await_safe_task(
    task_file: &TaskFile,
    task_name: &str,
    visited: &mut HashSet<String>,
) -> CjResult<()> {
    if !visited.insert(task_name.to_string()) {
        return Ok(());
    }
    let Some(lines) = task_file.tasks.get(task_name) else {
        return Ok(());
    };
    for line in lines {
        let Some(rest) = line.text.strip_prefix('@') else {
            continue;
        };
        let (name, args) = split_directive(rest);
        if matches!(name, "patch" | "minor" | "major" | "pre" | "release") {
            return Err(CjError::new(format!(
                "line {}: task {task_name} is used by @await and cannot use @{name}",
                line.line_number
            )));
        }
        if name == "task" {
            if let Ok(argv) = split_words(args) {
                if let Some(task_name) = argv.first() {
                    if task_file.tasks.contains_key(task_name) {
                        validate_await_safe_task(task_file, task_name, visited)?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn await_jobs() -> usize {
    env::var("CJ_JOBS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .or_else(|| thread::available_parallelism().ok().map(usize::from))
        .unwrap_or(1)
}

pub(crate) fn execute_block(
    ctx: &mut ExecutionContext<'_>,
    lines: &[TaskLine],
    start: usize,
    end: usize,
    indent: usize,
) -> CjResult<i32> {
    ctx.cwd.push_scope();
    let result = (|| {
        let mut index = start;
        let mut previous_status = 0;
        loop {
            if index >= end {
                break Ok(previous_status);
            }
            let line = &lines[index];
            if line.indent < indent {
                break Ok(previous_status);
            }
            if line.indent > indent {
                break Err(CjError::new(format!(
                    "line {}: unexpected indentation",
                    line.line_number
                )));
            }
            ctx.env.steps += 1;
            if ctx.env.steps > MAX_EXECUTION_STEPS {
                break Err(CjError::new(format!(
                    "possible infinite loop detected after {MAX_EXECUTION_STEPS} task steps"
                )));
            }
            if interrupted() {
                break Ok(130);
            }

            if let Some(rest) = line.text.strip_prefix('@') {
                let (name, _) = split_directive(rest);
                if name == "and" || name == "or" {
                    let status = execute_chain_directive(
                        ctx,
                        lines,
                        &mut index,
                        end,
                        indent,
                        name,
                        previous_status,
                    )?;
                    previous_status = status;
                    if status != 0 && !next_directive_is(lines, index, end, indent, "or") {
                        break Ok(status);
                    }
                    continue;
                }
                let status = execute_directive(ctx, lines, &mut index, end, indent, rest)?;
                previous_status = status;
                if status != 0 {
                    if next_directive_is_chain(lines, index, end, indent) {
                        continue;
                    }
                    break Ok(status);
                }
            } else {
                let result =
                    run_direct_command(ctx.cwd.current(), &line.text, ctx.env, ctx.output_mode)?;
                index += 1;
                previous_status = result.status;
                if result.status != 0 {
                    if next_directive_is_chain(lines, index, end, indent) {
                        continue;
                    }
                    break Ok(result.status);
                }
            }
        }
    })();

    ctx.cwd.pop_scope();
    result
}

fn execute_chain_directive(
    ctx: &mut ExecutionContext<'_>,
    lines: &[TaskLine],
    index: &mut usize,
    end: usize,
    indent: usize,
    name: &str,
    previous_status: i32,
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
        execute_block(ctx, lines, block_start, block_end, indent + 2)
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

fn next_directive_is_chain(lines: &[TaskLine], index: usize, end: usize, indent: usize) -> bool {
    if index >= end || lines[index].indent != indent {
        return false;
    }
    lines[index]
        .text
        .strip_prefix('@')
        .map(split_directive)
        .is_some_and(|(name, _)| matches!(name, "and" | "or"))
}

pub(crate) fn execute_block_capture(
    parent: &mut ExecutionContext<'_>,
    lines: &[TaskLine],
    start: usize,
    end: usize,
    indent: usize,
) -> CjResult<String> {
    CAPTURED_OUTPUT.with(|captured| captured.borrow_mut().push(String::new()));
    let original_output_mode = parent.output_mode;
    parent.output_mode = OutputMode::Capture;
    let status = execute_block(parent, lines, start, end, indent);
    parent.output_mode = original_output_mode;
    let status = status?;
    let output = CAPTURED_OUTPUT.with(|captured| captured.borrow_mut().pop().unwrap_or_default());
    if status == 0 {
        Ok(output.trim_end_matches(['\r', '\n']).to_string())
    } else {
        Err(CjError::new(format!(
            "captured @set block failed with status {status}"
        )))
    }
}
