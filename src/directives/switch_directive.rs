use crate::command_text::interpolate_argv;
use crate::runner::{execute_block, ExecutionContext};
use crate::task_file::TaskLine;
use crate::{CjError, CjResult};

use super::{find_block_end, split_directive};

pub(super) fn execute_switch_directive(
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

fn find_case_body_end(lines: &[TaskLine], start: usize, end: usize, case_indent: usize) -> usize {
    let mut cursor = start;
    while cursor < end && lines[cursor].indent > case_indent {
        cursor += 1;
    }
    cursor
}
