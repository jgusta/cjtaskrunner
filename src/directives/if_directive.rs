use crate::runner::{execute_block, ExecutionContext};
use crate::task_file::TaskLine;
use crate::CjResult;

use super::conditions::evaluate_condition;
use super::find_block_end;

pub(super) fn execute_if_directive(
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
