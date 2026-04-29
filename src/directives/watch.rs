use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use notify::event::EventKind;
use notify::{RecursiveMode, Watcher};

use crate::command_text::{interpolate_argv, interrupted, split_words, terminate_active_children};
use crate::directives::{find_block_end, split_directive};
use crate::runner::{execute_block, ExecutionContext};
use crate::task_file::{split_line_expressions, TaskFile, TaskLine};
use crate::{CjError, CjResult};

const WATCH_DEBOUNCE: Duration = Duration::from_secs(3);
const INTERRUPT_POLL: Duration = Duration::from_millis(100);

enum WatchDecision {
    Restart,
    Finished,
    Interrupted,
}

pub(super) fn execute_watch_directive(
    ctx: &mut ExecutionContext<'_>,
    lines: &[TaskLine],
    index: &mut usize,
    end: usize,
    indent: usize,
    args: &str,
) -> CjResult<i32> {
    let line_number = lines[*index].line_number;
    let block_start = *index + 1;
    let block_end = find_block_end(lines, block_start, end, indent);
    let action = watch_action_line(lines, block_start, block_end, indent, line_number)?;
    validate_watch_line(ctx.task_file, &action)?;

    let argv = interpolate_argv(args, &ctx.env.vars)?;
    if argv.is_empty() {
        return Err(CjError::new(format!(
            "line {line_number}: @watch expects at least one path"
        )));
    }

    let paths = argv
        .into_iter()
        .map(|path| ctx.cwd.current().join(path))
        .collect::<Vec<_>>();
    for path in &paths {
        if !path.exists() {
            return Err(CjError::new(format!(
                "line {line_number}: @watch path does not exist: {}",
                path.display()
            )));
        }
    }

    let result = run_watched_line(ctx, &paths, action, indent + 2)?;
    *index = block_end;
    Ok(result)
}

fn watch_action_line(
    lines: &[TaskLine],
    block_start: usize,
    block_end: usize,
    indent: usize,
    line_number: usize,
) -> CjResult<TaskLine> {
    if block_start == block_end {
        return Err(CjError::new(format!(
            "line {line_number}: @watch expects an indented line"
        )));
    }
    if block_end != block_start + 1 {
        return Err(CjError::new(format!(
            "line {line_number}: @watch expects exactly one indented line"
        )));
    }

    let action = lines[block_start].clone();
    if action.indent != indent + 2 {
        return Err(CjError::new(format!(
            "line {}: @watch action must be indented exactly one level",
            action.line_number
        )));
    }
    let expressions = split_line_expressions(&action.text);
    if expressions.len() != 1 {
        return Err(CjError::new(format!(
            "line {}: @watch action must be one expression",
            action.line_number
        )));
    }
    Ok(action)
}

fn run_watched_line(
    ctx: &mut ExecutionContext<'_>,
    paths: &[PathBuf],
    action: TaskLine,
    action_indent: usize,
) -> CjResult<i32> {
    thread::scope(|scope| {
        let mut last_status = 0;
        while !interrupted() {
            let action_lines = vec![action.clone()];
            let mut action_env = ctx.env.clone();
            let mut action_cwd = ctx.cwd.clone();
            let mut action_stack = ctx.stack.clone();
            let task_file = ctx.task_file;
            let output_mode = ctx.output_mode;
            let (done_tx, done_rx) = mpsc::channel();

            let handle = scope.spawn(move || {
                let mut action_ctx = ExecutionContext {
                    task_file,
                    env: &mut action_env,
                    cwd: &mut action_cwd,
                    stack: &mut action_stack,
                    output_mode,
                };
                let result = execute_block(&mut action_ctx, &action_lines, 0, 1, action_indent);
                let _ = done_tx.send(());
                result
            });

            let (watcher, event_rx) = create_watcher(paths)?;
            let decision = wait_for_restart_or_finish(&event_rx, &done_rx)?;
            drop(watcher);

            match decision {
                WatchDecision::Restart => {
                    terminate_active_children();
                    let result = handle
                        .join()
                        .unwrap_or_else(|_| Err(CjError::new("@watch action panicked")))?;
                    last_status = result;
                }
                WatchDecision::Finished => {
                    return handle
                        .join()
                        .unwrap_or_else(|_| Err(CjError::new("@watch action panicked")));
                }
                WatchDecision::Interrupted => {
                    terminate_active_children();
                    let _ = handle.join();
                    return Ok(130);
                }
            }

            if interrupted() {
                return Ok(130);
            }
        }

        Ok(last_status)
    })
}

fn validate_watch_line(task_file: &TaskFile, line: &TaskLine) -> CjResult<()> {
    let mut visited = std::collections::HashSet::new();
    validate_watch_expression(task_file, line.line_number, &line.text, &mut visited)
}

fn validate_watch_task(
    task_file: &TaskFile,
    task_name: &str,
    visited: &mut std::collections::HashSet<String>,
) -> CjResult<()> {
    if !visited.insert(task_name.to_string()) {
        return Ok(());
    }
    let Some(lines) = task_file.tasks.get(task_name) else {
        return Ok(());
    };
    for line in lines {
        for expression in split_line_expressions(&line.text) {
            validate_watch_expression(task_file, line.line_number, &expression, visited)?;
        }
    }
    Ok(())
}

fn validate_watch_expression(
    task_file: &TaskFile,
    line_number: usize,
    expression: &str,
    visited: &mut std::collections::HashSet<String>,
) -> CjResult<()> {
    let Some(rest) = expression.strip_prefix('@') else {
        return Ok(());
    };
    let (name, args) = split_directive(rest);
    if name == "await" {
        return Err(CjError::new(format!(
            "line {line_number}: @await cannot be used inside @watch"
        )));
    }
    if name == "task" {
        if let Ok(argv) = split_words(args) {
            if let Some(task_name) = argv.first() {
                validate_watch_task(task_file, task_name, visited)?;
            }
        }
    }
    Ok(())
}

fn create_watcher(
    paths: &[PathBuf],
) -> CjResult<(
    notify::RecommendedWatcher,
    Receiver<Result<notify::Event, notify::Error>>,
)> {
    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |result| {
        let _ = tx.send(result);
    })
    .map_err(|err| CjError::new(format!("failed to create file watcher: {err}")))?;

    for path in paths {
        let mode = if path.is_dir() {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        watcher
            .watch(path, mode)
            .map_err(|err| CjError::new(format!("failed to watch {}: {err}", path.display())))?;
    }

    Ok((watcher, rx))
}

fn wait_for_restart_or_finish(
    event_rx: &Receiver<Result<notify::Event, notify::Error>>,
    done_rx: &Receiver<()>,
) -> CjResult<WatchDecision> {
    while !interrupted() {
        if done_rx.try_recv().is_ok() {
            return Ok(WatchDecision::Finished);
        }
        match event_rx.recv_timeout(INTERRUPT_POLL) {
            Ok(Ok(event)) if rebuild_event(&event.kind) => {
                return wait_for_debounce(done_rx, event_rx);
            }
            Ok(Ok(_)) => {}
            Ok(Err(err)) => return Err(CjError::new(format!("file watch failed: {err}"))),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                return Err(CjError::new("file watcher stopped unexpectedly"));
            }
        }
    }
    Ok(WatchDecision::Interrupted)
}

fn wait_for_debounce(
    done_rx: &Receiver<()>,
    event_rx: &Receiver<Result<notify::Event, notify::Error>>,
) -> CjResult<WatchDecision> {
    let deadline = Instant::now() + WATCH_DEBOUNCE;
    while !interrupted() {
        let now = Instant::now();
        if now >= deadline {
            return Ok(WatchDecision::Restart);
        }
        let wait = (deadline - now).min(INTERRUPT_POLL);
        let _ = done_rx.try_recv();
        match event_rx.recv_timeout(wait) {
            Ok(Ok(_)) => {}
            Ok(Err(err)) => return Err(CjError::new(format!("file watch failed: {err}"))),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                return Err(CjError::new("file watcher stopped unexpectedly"));
            }
        }
    }
    Ok(WatchDecision::Interrupted)
}

fn rebuild_event(kind: &EventKind) -> bool {
    !matches!(kind, EventKind::Access(_))
}
