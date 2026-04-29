use std::fs;
use std::path::Path;

use crate::command_text::{
    interpolate_argv, interpolate_shell_text, interpolate_text, run_open_url, run_shell_command,
};
use crate::help_output::format_task_help;
use crate::runner::{
    execute_await_tasks, execute_block, execute_block_capture, run_task_inner, ExecutionContext,
};
use crate::runtime::{BumpKind, QuoteMode, RuntimeEnv};
use crate::task_file::{validate_env_name, TaskLine};
use crate::version::bump_taskfile_version;
use crate::{CjError, CjResult};

mod conditions;
mod filesystem;
mod values;
mod watch;

use conditions::evaluate_condition;
pub(crate) use conditions::if_condition_error;
pub(crate) use conditions::if_in_condition_error;
use filesystem::{copy_dirs, copy_files, remove_path, rename_path};
pub(crate) use values::parse_variable_name_token;
use values::{
    is_set_capture_args, parse_env_mutation, parse_export_mutation,
    parse_set_capture_name_with_env, return_value_status, write_output_line,
};
use watch::execute_watch_directive;

pub(crate) fn execute_directive(
    ctx: &mut ExecutionContext<'_>,
    lines: &[TaskLine],
    index: &mut usize,
    end: usize,
    indent: usize,
    directive: &str,
) -> CjResult<i32> {
    let (name, args) = split_directive(directive);
    match name {
        "shell" => {
            let command = interpolate_shell_text(args, ctx.env)?;
            *index += 1;
            run_shell_command(ctx.cwd.current(), &command, ctx.env, ctx.output_mode)
                .map(|result| result.status)
        }
        "open" => {
            let argv = interpolate_argv(args, &ctx.env.vars)?;
            if argv.len() != 1 {
                return Err(CjError::new(format!(
                    "line {}: @open expects exactly one URL",
                    lines[*index].line_number
                )));
            }
            validate_open_url(&argv[0], lines[*index].line_number)?;
            *index += 1;
            run_open_url(&argv[0], ctx.env, ctx.output_mode).map(|result| result.status)
        }
        "task" => {
            let argv = interpolate_argv(args, &ctx.env.vars)?;
            if argv.is_empty() {
                return Err(CjError::new(format!(
                    "line {}: @task expects a task name",
                    lines[*index].line_number
                )));
            }
            *index += 1;
            run_task_inner(ctx, &argv[0], &argv[1..])
        }
        "await" => execute_await_directive(ctx, lines, index, end, indent, args),
        "watch" => execute_watch_directive(ctx, lines, index, end, indent, args),
        "cd" => {
            let argv = interpolate_argv(args, &ctx.env.vars)?;
            if argv.len() != 1 {
                return Err(CjError::new(format!(
                    "line {}: @cd expects exactly one path",
                    lines[*index].line_number
                )));
            }
            let next = ctx.cwd.current().join(&argv[0]);
            if !next.is_dir() {
                return Err(CjError::new(format!(
                    "line {}: @cd path is not a directory: {}",
                    lines[*index].line_number,
                    next.display()
                )));
            }
            ctx.cwd.cd(next);
            *index += 1;
            Ok(0)
        }
        "back" => {
            if !args.trim().is_empty() {
                return Err(CjError::new(format!(
                    "line {}: @back does not take arguments",
                    lines[*index].line_number
                )));
            }
            ctx.cwd.back();
            *index += 1;
            Ok(0)
        }
        "desc" => {
            *index += 1;
            Ok(0)
        }
        "selfhelp" => {
            if !args.trim().is_empty() {
                return Err(CjError::new(format!(
                    "line {}: @selfhelp does not take arguments",
                    lines[*index].line_number
                )));
            }
            let task_name = ctx.stack.last().ok_or_else(|| {
                CjError::new(format!(
                    "line {}: @selfhelp must run inside a task",
                    lines[*index].line_number
                ))
            })?;
            let task_path = ctx
                .task_file
                .source_path
                .as_deref()
                .unwrap_or_else(|| Path::new("cjtasks"));
            let help = format_task_help(ctx.task_file, task_name, task_path)?;
            write_output_line(&help, ctx.output_mode);
            *index = end;
            Ok(0)
        }
        "echo" => {
            let value = interpolate_text(args, &ctx.env.vars, QuoteMode::None)?;
            write_output_line(&value, ctx.output_mode);
            *index += 1;
            Ok(0)
        }
        "clean" => {
            let argv = interpolate_argv(args, &ctx.env.vars)?;
            remove_path(
                ctx.cwd.current(),
                ctx.cwd.scope_base(),
                &argv,
                lines[*index].line_number,
            )?;
            *index += 1;
            Ok(0)
        }
        "mkdir" => {
            let argv = interpolate_argv(args, &ctx.env.vars)?;
            if argv.is_empty() {
                return Err(CjError::new(format!(
                    "line {}: @mkdir expects at least one path",
                    lines[*index].line_number
                )));
            }
            for path in argv {
                fs::create_dir_all(ctx.cwd.current().join(path))?;
            }
            *index += 1;
            Ok(0)
        }
        "cp" => {
            let argv = interpolate_argv(args, &ctx.env.vars)?;
            copy_files(ctx.cwd.current(), &argv, lines[*index].line_number)?;
            *index += 1;
            Ok(0)
        }
        "cpdir" => {
            let argv = interpolate_argv(args, &ctx.env.vars)?;
            copy_dirs(ctx.cwd.current(), &argv, lines[*index].line_number)?;
            *index += 1;
            Ok(0)
        }
        "rename" => {
            let argv = interpolate_argv(args, &ctx.env.vars)?;
            rename_path(ctx.cwd.current(), &argv, lines[*index].line_number)?;
            *index += 1;
            Ok(0)
        }
        "stop" => {
            if !args.trim().is_empty() {
                let value = interpolate_text(args, &ctx.env.vars, QuoteMode::None)?;
                write_output_line(&value, ctx.output_mode);
            }
            *index += 1;
            Ok(1)
        }
        "success" => {
            *index += 1;
            Ok(0)
        }
        "fail" => {
            *index += 1;
            Ok(1)
        }
        "return" => {
            let block_start = *index + 1;
            let block_end = find_block_end(lines, block_start, end, indent);
            if block_start < block_end {
                let status = execute_block(ctx, lines, block_start, block_end, indent + 2)?;
                *index = block_end;
                Ok(status)
            } else {
                let status = return_value_status(args, ctx.env)?;
                *index += 1;
                Ok(status)
            }
        }
        "set" | "export" => {
            if name == "set" {
                let block_start = *index + 1;
                let block_end = find_block_end(lines, block_start, end, indent);
                if block_start < block_end && is_set_capture_args(args) {
                    let key =
                        parse_set_capture_name_with_env(args, ctx.env, lines[*index].line_number)?;
                    let value =
                        execute_block_capture(ctx, lines, block_start, block_end, indent + 2)?;
                    ctx.env.vars.insert(key, value);
                    *index = block_end;
                    return Ok(0);
                }
                let (key, value) = parse_env_mutation(args, ctx.env, lines[*index].line_number)?;
                ctx.env.vars.insert(key, value);
            } else {
                let (key, value) = parse_export_mutation(args, ctx.env, lines[*index].line_number)?;
                ctx.env.export(key, value);
            }
            *index += 1;
            Ok(0)
        }
        "unset" => {
            let argv = interpolate_argv(args, &ctx.env.vars)?;
            if argv.len() != 1 {
                return Err(CjError::new(format!(
                    "line {}: @unset expects exactly one variable name",
                    lines[*index].line_number
                )));
            }
            validate_env_name(&argv[0]).map_err(|err| {
                CjError::new(format!(
                    "line {}: invalid env name '{}': {err}",
                    lines[*index].line_number, argv[0]
                ))
            })?;
            ctx.env.unset(&argv[0]);
            *index += 1;
            Ok(0)
        }
        "version" => Err(CjError::new(format!(
            "line {}: @version can only be used as a top-level header",
            lines[*index].line_number
        ))),
        "patch" | "minor" | "major" | "pre" | "release" => {
            execute_version_bump_directive(ctx, name, args, lines[*index].line_number)?;
            *index += 1;
            Ok(0)
        }
        "if" | "if-not" | "if-in" | "if-not-in" | "if-exists" | "if-not-exists" | "if-set"
        | "if-not-set" | "if-version" | "if-not-version" | "if-bumped" | "if-not-bumped"
        | "if-patch" | "if-minor" | "if-major" | "if-pre" | "if-release" | "if-not-patch"
        | "if-not-minor" | "if-not-major" | "if-not-pre" | "if-not-release" => {
            execute_if_directive(ctx, lines, index, end, indent, name, args)
        }
        "else" => Err(CjError::new(format!(
            "line {}: @else without matching @if",
            lines[*index].line_number
        ))),
        "switch" => execute_switch_directive(ctx, lines, index, end, indent, args),
        "case" | "default" => Err(CjError::new(format!(
            "line {}: @{name} without matching @switch",
            lines[*index].line_number
        ))),
        "" => Err(CjError::new(format!(
            "line {}: empty directive",
            lines[*index].line_number
        ))),
        _ => Err(CjError::new(format!(
            "line {}: unknown directive @{name}",
            lines[*index].line_number
        ))),
    }
}

fn validate_open_url(url: &str, line_number: usize) -> CjResult<()> {
    if url.starts_with("http://") || url.starts_with("https://") {
        Ok(())
    } else {
        Err(CjError::new(format!(
            "line {line_number}: @open URL must begin with http:// or https://"
        )))
    }
}

fn execute_await_directive(
    ctx: &mut ExecutionContext<'_>,
    lines: &[TaskLine],
    index: &mut usize,
    end: usize,
    indent: usize,
    args: &str,
) -> CjResult<i32> {
    let awaited_tasks = interpolate_argv(args, &ctx.env.vars)?;
    if awaited_tasks.is_empty() {
        return Err(CjError::new(format!(
            "line {}: @await expects at least one task name",
            lines[*index].line_number
        )));
    }
    let block_start = *index + 1;
    let block_end = find_block_end(lines, block_start, end, indent);
    *index = if block_start < block_end {
        block_end
    } else {
        *index + 1
    };

    let status = if ctx.env.await_blocks_satisfied {
        0
    } else {
        execute_await_tasks(ctx.task_file, &awaited_tasks, ctx.env, ctx.cwd)?
    };
    ctx.env.sync_exports();
    if status != 0 || block_start == block_end {
        return Ok(status);
    }
    execute_block(ctx, lines, block_start, block_end, indent + 2)
}

fn execute_if_directive(
    ctx: &mut ExecutionContext<'_>,
    lines: &[TaskLine],
    index: &mut usize,
    end: usize,
    indent: usize,
    name: &str,
    args: &str,
) -> CjResult<i32> {
    let condition = evaluate_condition(ctx.cwd.current(), name, args, ctx.env)?;
    let then_start = *index + 1;
    let then_end = find_block_end(lines, then_start, end, indent);
    let mut else_range = None;

    if then_end < end && lines[then_end].indent == indent && lines[then_end].text == "@else" {
        let else_start = then_end + 1;
        let else_end = find_block_end(lines, else_start, end, indent);
        else_range = Some((else_start, else_end));
        *index = else_end;
    } else {
        *index = then_end;
    }

    if condition {
        execute_block(ctx, lines, then_start, then_end, indent + 2)
    } else if let Some((else_start, else_end)) = else_range {
        execute_block(ctx, lines, else_start, else_end, indent + 2)
    } else {
        Ok(0)
    }
}

fn execute_switch_directive(
    ctx: &mut ExecutionContext<'_>,
    lines: &[TaskLine],
    index: &mut usize,
    end: usize,
    indent: usize,
    args: &str,
) -> CjResult<i32> {
    let values = interpolate_argv(args, &ctx.env.vars)?;
    if values.len() != 1 {
        return Err(CjError::new(format!(
            "line {}: @switch expects exactly one value",
            lines[*index].line_number
        )));
    }
    let switch_value = &values[0];
    let switch_start = *index + 1;
    let switch_end = find_block_end(lines, switch_start, end, indent);
    let case_indent = indent + 2;
    let body_indent = indent + 4;
    let mut selected: Option<(usize, usize)> = None;
    let mut default: Option<(usize, usize)> = None;
    let mut cursor = switch_start;

    while cursor < switch_end {
        let line = &lines[cursor];
        if line.indent != case_indent {
            return Err(CjError::new(format!(
                "line {}: @switch body must contain @case or @default entries",
                line.line_number
            )));
        }
        let Some(rest) = line.text.strip_prefix('@') else {
            return Err(CjError::new(format!(
                "line {}: @switch body entries must use @case or @default",
                line.line_number
            )));
        };
        let (name, args) = split_directive(rest);
        if name != "case" && name != "default" {
            return Err(CjError::new(format!(
                "line {}: @switch body entries must use @case or @default",
                line.line_number
            )));
        }

        let body_start = cursor + 1;
        let body_end = find_case_body_end(lines, body_start, switch_end, case_indent);
        if name == "case" {
            let case_values = interpolate_argv(args, &ctx.env.vars)?;
            if case_values.len() != 1 {
                return Err(CjError::new(format!(
                    "line {}: @case expects exactly one value",
                    line.line_number
                )));
            }
            if selected.is_none() && case_values[0] == *switch_value {
                selected = Some((body_start, body_end));
            }
        } else {
            if !args.trim().is_empty() {
                return Err(CjError::new(format!(
                    "line {}: @default does not take arguments",
                    line.line_number
                )));
            }
            default.get_or_insert((body_start, body_end));
        }
        cursor = body_end;
    }

    *index = switch_end;
    if let Some((start, end)) = selected.or(default) {
        execute_block(ctx, lines, start, end, body_indent)
    } else {
        Ok(0)
    }
}

pub(crate) fn find_block_end(
    lines: &[TaskLine],
    start: usize,
    end: usize,
    parent_indent: usize,
) -> usize {
    let mut cursor = start;
    while cursor < end && lines[cursor].indent > parent_indent {
        cursor += 1;
    }
    cursor
}

fn find_case_body_end(lines: &[TaskLine], start: usize, end: usize, case_indent: usize) -> usize {
    let mut cursor = start;
    while cursor < end && lines[cursor].indent > case_indent {
        cursor += 1;
    }
    cursor
}

fn execute_version_bump_directive(
    ctx: &mut ExecutionContext<'_>,
    operation: &str,
    args: &str,
    line_number: usize,
) -> CjResult<()> {
    let argv = interpolate_argv(args, &ctx.env.vars)?;
    let (name, prerelease) = match (operation, argv.as_slice()) {
        ("pre", [name, prerelease]) => (name, Some(prerelease.as_str())),
        ("pre", _) => {
            return Err(CjError::new(format!(
                "line {line_number}: @pre expects '<name> <prerelease>'"
            )))
        }
        (_, [name]) => (name, None),
        _ => {
            return Err(CjError::new(format!(
                "line {line_number}: @{operation} expects exactly one version name"
            )))
        }
    };
    let kind = BumpKind::parse(operation).expect("version bump directive must be a bump kind");
    ensure_not_bumped(ctx.env, name, line_number)?;
    let (env_key, value) =
        bump_taskfile_version(ctx.task_file, name, operation, prerelease, line_number)?;
    ctx.env.bumped_versions.insert(name.to_string(), kind);
    ctx.env.export(env_key, value);
    Ok(())
}

fn ensure_not_bumped(effective_env: &RuntimeEnv, name: &str, line_number: usize) -> CjResult<()> {
    if let Some(previous) = effective_env.bumped_versions.get(name) {
        return Err(CjError::new(format!(
            "line {line_number}: version '{name}' was already bumped as {} in this run",
            previous.as_str()
        )));
    }
    Ok(())
}

pub(crate) fn split_directive(directive: &str) -> (&str, &str) {
    let trimmed = directive.trim_start();
    match trimmed.find(char::is_whitespace) {
        Some(index) => (&trimmed[..index], trimmed[index..].trim_start()),
        None => (trimmed, ""),
    }
}
