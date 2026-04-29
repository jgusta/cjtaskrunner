mod common;

use common::temp_path;
use std::fs;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

fn run_count(path: &std::path::Path) -> usize {
    fs::read_to_string(path).unwrap_or_default().lines().count()
}

#[test]
fn watch_starts_one_line_then_restarts_after_debounced_changes() {
    let dir = temp_path("watch-debounce");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(dir.join("watched.txt"), "start").expect("watched");
    fs::write(
        dir.join("serve.sh"),
        "#!/bin/sh\nprintf 'run\\n' >> runs.txt\nwhile true; do sleep 1; done\n",
    )
    .expect("write serve script");
    fs::write(
        dir.join("cjtasks"),
        r#"run:
  @watch watched.txt
    sh serve.sh
"#,
    )
    .expect("write cjtasks");

    let mut child = Command::new(env!("CARGO_BIN_EXE_cj"))
        .arg("run")
        .current_dir(&dir)
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn cj watch");

    let runs = dir.join("runs.txt");
    let start = Instant::now();
    while run_count(&runs) < 1 && start.elapsed() < Duration::from_secs(5) {
        thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(run_count(&runs), 1, "initial watch block should run once");

    fs::write(dir.join("watched.txt"), "change 1").expect("change 1");
    thread::sleep(Duration::from_millis(250));
    fs::write(dir.join("watched.txt"), "change 2").expect("change 2");
    thread::sleep(Duration::from_millis(250));
    fs::write(dir.join("watched.txt"), "change 3").expect("change 3");

    thread::sleep(Duration::from_secs(2));
    assert_eq!(
        run_count(&runs),
        1,
        "watch line should not restart before the debounce window ends"
    );

    let start = Instant::now();
    while run_count(&runs) < 2 && start.elapsed() < Duration::from_secs(5) {
        thread::sleep(Duration::from_millis(100));
    }
    assert_eq!(
        run_count(&runs),
        2,
        "change burst should collapse to one run"
    );

    child.kill().expect("kill watcher");
    let _ = child.wait();

    assert_eq!(
        run_count(&runs),
        2,
        "no extra run should be scheduled for the collapsed burst"
    );
    fs::remove_dir_all(dir).expect("cleanup");
}
