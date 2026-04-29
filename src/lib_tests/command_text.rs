#[cfg(unix)]
use crate::command_text::should_isolate_child_process_group;
use crate::command_text::{
    contains_variable_interpolation, open_command_spec, unescape_variable_literals,
};

#[test]
fn detects_unescaped_variable_interpolation() {
    for input in [
        "$NAME",
        "${NAME}",
        "${NAME?fallback}",
        "prefix $NAME suffix",
        "prefix ${NAME} suffix",
    ] {
        assert!(contains_variable_interpolation(input), "{input}");
    }

    for input in ["$", "$5", r"\$NAME", r"\${NAME}", "plain text"] {
        assert!(!contains_variable_interpolation(input), "{input}");
    }
}

#[test]
fn unescapes_literal_variable_markers() {
    assert_eq!(
        unescape_variable_literals(r"Literal \$NAME and \${NAME}"),
        "Literal $NAME and ${NAME}"
    );
}

#[test]
fn open_command_uses_platform_opener() {
    let command = open_command_spec("https://example.com");
    #[cfg(target_os = "macos")]
    {
        assert_eq!(command.program, "open");
        assert_eq!(command.args, vec!["https://example.com"]);
    }
    #[cfg(target_os = "windows")]
    {
        assert_eq!(command.program, "cmd");
        assert_eq!(command.args, vec!["/C", "start", "", "https://example.com"]);
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        assert_eq!(command.program, "xdg-open");
        assert_eq!(command.args, vec!["https://example.com"]);
    }
}

#[cfg(unix)]
#[test]
fn terminal_commands_stay_in_the_foreground_process_group() {
    assert!(!should_isolate_child_process_group(true, true));
    assert!(should_isolate_child_process_group(true, false));
    assert!(should_isolate_child_process_group(false, true));
}
