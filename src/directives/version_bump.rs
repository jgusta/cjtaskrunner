use crate::command_text::interpolate_argv;
use crate::runner::ExecutionContext;
use crate::runtime::{BumpKind, RuntimeEnv};
use crate::version::bump_taskfile_version;
use crate::{CjError, CjResult};

pub(super) fn execute_version_bump_directive(
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
