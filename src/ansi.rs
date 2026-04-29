use std::env;
use std::fmt;

#[derive(Debug, Clone, Copy)]
pub(crate) enum Style {
    Header,
    Section,
    Task,
    Directive,
    Description,
    SummaryDescription,
}

pub(crate) fn paint(value: impl fmt::Display, style: Style) -> String {
    if !color_enabled() {
        return value.to_string();
    }

    let code = match style {
        Style::Header => "1;36",
        Style::Section => "1;33",
        Style::Task => "1;32",
        Style::Directive => "36",
        Style::Description => "2",
        Style::SummaryDescription => "36",
    };
    format!("\x1b[{code}m{value}\x1b[0m")
}

fn color_enabled() -> bool {
    env::var_os("NO_COLOR").is_none_or(|value| value.is_empty())
}
