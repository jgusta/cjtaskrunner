#![cfg(unix)]

mod common;

use common::temp_path;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

fn wait_for_file(path: &Path) {
    for _ in 0..100 {
        if path.exists() {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("timed out waiting for {}", path.display());
}

fn process_exists(pid: i32) -> bool {
    unsafe { libc::kill(pid, 0) == 0 }
}

#[test]
fn sigint_to_runner_kills_parallel_await_children() {
    let dir = temp_path("signals");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(
        dir.join("cjtasks"),
        r#"run:
  @await left right
left:
  @shell echo $$ > left.pid && sleep 30
right:
  @shell echo $$ > right.pid && sleep 30
"#,
    )
    .expect("write cjtasks");

    let mut child = Command::new(env!("CARGO_BIN_EXE_cj"))
        .arg("run")
        .current_dir(&dir)
        .stdin(Stdio::null())
        .spawn()
        .expect("spawn cj");

    wait_for_file(&dir.join("left.pid"));
    wait_for_file(&dir.join("right.pid"));
    let left_pid = fs::read_to_string(dir.join("left.pid"))
        .expect("left pid")
        .trim()
        .parse::<i32>()
        .expect("left pid int");
    let right_pid = fs::read_to_string(dir.join("right.pid"))
        .expect("right pid")
        .trim()
        .parse::<i32>()
        .expect("right pid int");

    unsafe {
        libc::kill(child.id() as i32, libc::SIGINT);
    }
    let _ = child.wait().expect("wait cj");
    thread::sleep(Duration::from_millis(250));

    assert!(!process_exists(left_pid), "left awaited shell should exit");
    assert!(
        !process_exists(right_pid),
        "right awaited shell should exit"
    );

    fs::remove_dir_all(dir).expect("cleanup");
}
