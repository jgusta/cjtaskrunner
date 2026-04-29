mod common;

use common::{assert_success, temp_path};
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn run_install(
    shell: &str,
    home: &Path,
    data_home: Option<&Path>,
    config_home: Option<&Path>,
) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_cj"));
    command
        .env_clear()
        .env("HOME", home)
        .arg("--install-completions")
        .arg(shell);

    if let Some(data_home) = data_home {
        command.env("XDG_DATA_HOME", data_home);
    }
    if let Some(config_home) = config_home {
        command.env("XDG_CONFIG_HOME", config_home);
    }

    command.output().expect("run cj --install-completions")
}

fn run_completions(shell: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cj"))
        .arg("--completions")
        .arg(shell)
        .output()
        .expect("run cj --completions")
}

#[test]
fn fish_completions_include_task_descriptions() {
    let stdout = assert_success(&run_completions("fish"));

    assert!(stdout.contains("print task \"\\t\" desc"));
    assert!(stdout.contains("complete -c cj -f -n '__fish_use_subcommand' -a '(__cj_tasks)'"));
}

#[test]
fn completions_include_marked_tasks_subtasks_and_help_tasks() {
    let bash = assert_success(&run_completions("bash"));
    assert!(bash.contains("if ($1 == \"+\") print $2; else print $1"));
    assert!(bash.contains("compgen -W \"$(_cj_tasks)\""));
    assert!(!bash.contains("_cj_help_sections"));

    let zsh = assert_success(&run_completions("zsh"));
    assert!(zsh.contains("_describe 'task help' tasks"));
    assert!(!zsh.contains("_cj_help_sections"));

    let fish = assert_success(&run_completions("fish"));
    assert!(fish.contains("task = marked ? $2 : $1"));
    assert!(fish.contains("-a '(__cj_tasks)' -d 'Task help'"));
    assert!(!fish.contains("__cj_help_sections"));

    for script in [bash, zsh, fish] {
        assert!(script.contains("(:[A-Za-z0-9_-]+)*"));
    }
}

#[test]
fn completions_include_project_setup_options() {
    let bash = assert_success(&run_completions("bash"));
    assert!(bash.contains("--init"));
    assert!(bash.contains("--auto"));
    assert!(bash.contains("-e"));
    assert!(!bash.contains("-el"));
    assert!(!bash.contains("-ep"));
    assert!(!bash.contains("-ed"));
    assert!(!bash.contains("-es"));

    let fish = assert_success(&run_completions("fish"));
    assert!(fish.contains("-l init"));
    assert!(fish.contains("-l auto"));
    assert!(fish.contains("-s e"));
    assert!(!fish.contains("-s el"));
    assert!(!fish.contains("-s ep"));
    assert!(!fish.contains("-s ed"));
    assert!(!fish.contains("-s es"));
}

#[test]
fn install_completions_observes_xdg_shell_locations() {
    let dir = temp_path("install-completions-xdg");
    let home = dir.join("home");
    let data_home = dir.join("data");
    let config_home = dir.join("config");
    fs::create_dir_all(&home).expect("mkdir home");

    let bash = run_install("bash", &home, Some(&data_home), Some(&config_home));
    let bash_stdout = assert_success(&bash);
    let bash_path = data_home.join("bash-completion/completions/cj");
    assert!(bash_path.is_file(), "missing {}", bash_path.display());
    assert!(fs::read_to_string(&bash_path)
        .expect("read bash completion")
        .contains("NO_COLOR=1 cj"));
    assert!(bash_stdout.contains(&bash_path.display().to_string()));

    let fish = run_install("fish", &home, Some(&data_home), Some(&config_home));
    let fish_stdout = assert_success(&fish);
    let fish_path = config_home.join("fish/completions/cj.fish");
    assert!(fish_path.is_file(), "missing {}", fish_path.display());
    assert!(fs::read_to_string(&fish_path)
        .expect("read fish completion")
        .contains("NO_COLOR=1 cj"));
    assert!(fish_stdout.contains(&fish_path.display().to_string()));

    let zsh = run_install("zsh", &home, Some(&data_home), Some(&config_home));
    let zsh_stdout = assert_success(&zsh);
    let zsh_path = data_home.join("zsh/site-functions/_cj");
    assert!(zsh_path.is_file(), "missing {}", zsh_path.display());
    assert!(fs::read_to_string(&zsh_path)
        .expect("read zsh completion")
        .contains("NO_COLOR=1 cj"));
    assert!(zsh_stdout.contains(&zsh_path.display().to_string()));
    assert!(zsh_stdout.contains("fpath"));
    assert!(zsh_stdout.contains(&zsh_path.parent().unwrap().display().to_string()));

    assert!(!home.join(".zfunc/_cj").exists());
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn install_completions_falls_back_to_home_xdg_defaults() {
    let dir = temp_path("install-completions-defaults");
    let home = dir.join("home");
    fs::create_dir_all(&home).expect("mkdir home");

    let bash = run_install("bash", &home, None, None);
    assert_success(&bash);
    assert!(home
        .join(".local/share/bash-completion/completions/cj")
        .is_file());

    let fish = run_install("fish", &home, None, None);
    assert_success(&fish);
    assert!(home.join(".config/fish/completions/cj.fish").is_file());

    let zsh = run_install("zsh", &home, None, None);
    assert_success(&zsh);
    assert!(home.join(".local/share/zsh/site-functions/_cj").is_file());
    assert!(!home.join(".zfunc/_cj").exists());

    fs::remove_dir_all(dir).expect("cleanup");
}
