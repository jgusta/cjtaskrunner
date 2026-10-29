use crate::command_text::interpolate_argv;
use crate::runner::{execute_await_tasks, execute_block, ExecutionContext};
use crate::task_file::TaskLine;
use crate::{CjError, CjResult};

use super::find_block_end;

pub(super) fn execute_await_directive(
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
