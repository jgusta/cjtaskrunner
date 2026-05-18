use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_path(name: &str) -> PathBuf {
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    std::env::temp_dir().join(format!("cjtaskrunner-{name}-{id}"))
}

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

fn assert_success(output: &Output) -> String {
    assert!(
        output.status.success(),
        "command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone()).expect("stdout utf8")
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
    assert!(bash_stdout.contains(&bash_path.display().to_string()));

    let fish = run_install("fish", &home, Some(&data_home), Some(&config_home));
    let fish_stdout = assert_success(&fish);
    let fish_path = config_home.join("fish/completions/cj.fish");
    assert!(fish_path.is_file(), "missing {}", fish_path.display());
    assert!(fish_stdout.contains(&fish_path.display().to_string()));

    let zsh = run_install("zsh", &home, Some(&data_home), Some(&config_home));
    let zsh_stdout = assert_success(&zsh);
    let zsh_path = data_home.join("zsh/site-functions/_cj");
    assert!(zsh_path.is_file(), "missing {}", zsh_path.display());
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
