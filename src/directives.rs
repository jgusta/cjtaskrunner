use std::fs;
use std::path::Path;

use crate::command_text::{
    interpolate_argv, interpolate_shell_text, interpolate_text, run_open_url, run_shell_command,
};
use crate::help_output::format_task_help;
use crate::runner::{execute_block, execute_block_capture, run_task_inner, ExecutionContext};
use crate::runtime::QuoteMode;
use crate::task_file::{validate_env_name, TaskLine};
use crate::{CjError, CjResult};

mod await_directive;
mod conditions;
mod filesystem;
mod if_directive;
mod regex_replace;
mod switch_directive;
mod values;
mod version_bump;
mod watch;

use await_directive::execute_await_directive;
pub(crate) use conditions::if_condition_error;
pub(crate) use conditions::if_in_condition_error;
use filesystem::{copy_dirs, copy_files, remove_path, rename_path, unzip_path, zip_paths};
use if_directive::execute_if_directive;
use regex_replace::execute_regex_replace_directive;
use switch_directive::execute_switch_directive;
pub(crate) use values::parse_variable_name_token;
use values::{
    is_set_capture_args, parse_env_mutation, parse_export_mutation,
    parse_set_capture_name_with_env, return_value_status, write_output_line,
};
use version_bump::execute_version_bump_directive;
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
        "zip" => {
            let argv = interpolate_argv(args, &ctx.env.vars)?;
            zip_paths(ctx.cwd.current(), &argv, lines[*index].line_number)?;
            *index += 1;
            Ok(0)
        }
        "unzip" => {
            let argv = interpolate_argv(args, &ctx.env.vars)?;
            unzip_path(ctx.cwd.current(), &argv, lines[*index].line_number)?;
            *index += 1;
            Ok(0)
        }
        "rename" => {
            let argv = interpolate_argv(args, &ctx.env.vars)?;
            rename_path(ctx.cwd.current(), &argv, lines[*index].line_number)?;
            *index += 1;
            Ok(0)
        }
        "regex-replace" => {
            execute_regex_replace_directive(ctx, args, lines[*index].line_number)?;
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

pub(crate) fn split_directive(directive: &str) -> (&str, &str) {
    let trimmed = directive.trim_start();
    match trimmed.find(char::is_whitespace) {
        Some(index) => (&trimmed[..index], trimmed[index..].trim_start()),
        None => (trimmed, ""),
    }
}
