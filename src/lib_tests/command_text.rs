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

#[test]
fn variable_interpolation_distinguishes_absent_empty_and_present_values() {
    use crate::command_text::interpolate_text;
    use crate::runtime::QuoteMode;
    use std::collections::HashMap;

    for input in ["$NAME", "${NAME}", "${NAME?}", "${NAME?fallback}"] {
        for value in ["", "current value"] {
            let vars = HashMap::from([("NAME".to_string(), value.to_string())]);
            assert_eq!(
                interpolate_text(input, &vars, QuoteMode::None).unwrap(),
                value,
                "{input}"
            );
        }
    }

    let vars = HashMap::new();
    for input in ["$NAME", "${NAME}"] {
        for mode in [QuoteMode::None, QuoteMode::Shell] {
            let error = interpolate_text(input, &vars, mode).unwrap_err();
            assert_eq!(error.to_string(), "missing variable: NAME");
        }
    }
    for (input, expected) in [
        ("${NAME?}", ""),
        ("${NAME?fallback}", "fallback"),
        ("${NAME?\"fallback value\"}", "fallback value"),
    ] {
        assert_eq!(
            interpolate_text(input, &vars, QuoteMode::None).unwrap(),
            expected
        );
    }
    assert_eq!(
        interpolate_text("${NAME?}", &vars, QuoteMode::Shell).unwrap(),
        "''"
    );
}
