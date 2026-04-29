#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArgumentRule {
    Any,
    None,
    AtLeast(usize),
    Exactly(usize),
    Variable,
    Set,
    Export,
    IfCondition,
    IfInCondition,
    VersionCondition,
    BumpedCondition,
    BumpKindCondition,
    VersionBump,
    PreBump,
}

pub(crate) struct DirectiveInfo {
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
    pub(crate) arguments: ArgumentRule,
}

macro_rules! directives {
    ($(($name:literal, $description:literal, $arguments:expr)),+ $(,)?) => {
        pub(crate) const DIRECTIVES: &[DirectiveInfo] = &[
            $(DirectiveInfo {
                name: $name,
                description: $description,
                arguments: $arguments,
            }),+
        ];
    };
}

use ArgumentRule::{
    Any, AtLeast, BumpKindCondition, BumpedCondition, Exactly, Export, IfCondition, IfInCondition,
    None, PreBump, Set, Variable, VersionBump, VersionCondition,
};

directives!(
    ("shell", "Run command through /bin/sh -c on Unix.", Any),
    ("open", "Open a URL with the system browser.", Exactly(1)),
    ("task", "Run another task from same taskfile.", AtLeast(1)),
    (
        "await",
        "Run tasks in parallel, then optionally run a success block.",
        AtLeast(1)
    ),
    (
        "watch",
        "Run one line and restart it after watched paths change.",
        AtLeast(1)
    ),
    (
        "desc",
        "Describe task for listings and editor task views.",
        Any
    ),
    ("help:", "Document task help text.", None),
    (
        "env:",
        "Declare top-level taskfile environment entries.",
        None
    ),
    (
        "selfhelp",
        "Print this task's help output and stop the task.",
        None
    ),
    (
        "cd",
        "Change working directory for current scope.",
        Exactly(1)
    ),
    ("back", "Undo one @cd within current scope.", None),
    ("echo", "Write text plus newline to stdout.", Any),
    (
        "clean",
        "Remove file or directory relative to current working directory.",
        Exactly(1)
    ),
    ("mkdir", "Create one or more directories.", AtLeast(1)),
    ("cp", "Copy one or more files.", AtLeast(2)),
    ("cpdir", "Copy one or more directories.", AtLeast(2)),
    (
        "rename",
        "Rename a file or directory without moving it.",
        Exactly(2)
    ),
    ("stop", "Write optional text, then stop with status 1.", Any),
    (
        "set",
        "Set runtime variable, or capture block stdout with @set NAME:.",
        Set
    ),
    (
        "export",
        "Export variable to later child processes.",
        Export
    ),
    (
        "unset",
        "Remove runtime variable and export overlay.",
        Variable
    ),
    (
        "version",
        "Declare a top-level taskfile version header.",
        Exactly(2)
    ),
    ("patch", "Patch a taskfile @version header.", VersionBump),
    (
        "minor",
        "Minor-bump a taskfile @version header.",
        VersionBump
    ),
    (
        "major",
        "Major-bump a taskfile @version header.",
        VersionBump
    ),
    (
        "pre",
        "Prerelease-bump a taskfile @version header.",
        PreBump
    ),
    (
        "release",
        "Release a taskfile @version header.",
        VersionBump
    ),
    (
        "return",
        "Return a derived status, or return block status.",
        Any
    ),
    ("success", "Return status 0.", None),
    ("fail", "Return status 1.", None),
    (
        "and",
        "Run block when previous expression returned 0.",
        None
    ),
    (
        "or",
        "Run block when previous expression returned non-zero.",
        None
    ),
    (
        "if",
        "Run block when a value is truthy, equal, or unequal.",
        IfCondition
    ),
    (
        "if-not",
        "Run block when an @if condition would be false.",
        IfCondition
    ),
    (
        "if-in",
        "Run block when a value is in a list.",
        IfInCondition
    ),
    (
        "if-not-in",
        "Run block when a value is not in a list.",
        IfInCondition
    ),
    ("else", "Else block for matching @if.", None),
    ("if-exists", "Run block when path exists.", Exactly(1)),
    (
        "if-not-exists",
        "Run block when path does not exist.",
        Exactly(1)
    ),
    ("if-set", "Run block when variable is set.", Variable),
    (
        "if-not-set",
        "Run block when variable is not set.",
        Variable
    ),
    (
        "if-version",
        "Run block when a taskfile version condition matches.",
        VersionCondition
    ),
    (
        "if-not-version",
        "Run block when a taskfile version condition does not match.",
        VersionCondition
    ),
    (
        "if-bumped",
        "Run block when a version was bumped in this invocation.",
        BumpedCondition
    ),
    (
        "if-not-bumped",
        "Run block when a version was not bumped in this invocation.",
        BumpedCondition
    ),
    (
        "if-patch",
        "Run block when a version received a patch bump in this invocation.",
        BumpKindCondition
    ),
    (
        "if-not-patch",
        "Run block when a version did not receive a patch bump in this invocation.",
        BumpKindCondition
    ),
    (
        "if-minor",
        "Run block when a version received a minor bump in this invocation.",
        BumpKindCondition
    ),
    (
        "if-not-minor",
        "Run block when a version did not receive a minor bump in this invocation.",
        BumpKindCondition
    ),
    (
        "if-major",
        "Run block when a version received a major bump in this invocation.",
        BumpKindCondition
    ),
    (
        "if-not-major",
        "Run block when a version did not receive a major bump in this invocation.",
        BumpKindCondition
    ),
    (
        "if-pre",
        "Run block when a version received a prerelease bump in this invocation.",
        BumpKindCondition
    ),
    (
        "if-not-pre",
        "Run block when a version did not receive a prerelease bump in this invocation.",
        BumpKindCondition
    ),
    (
        "if-release",
        "Run block when a version received a release bump in this invocation.",
        BumpKindCondition
    ),
    (
        "if-not-release",
        "Run block when a version did not receive a release bump in this invocation.",
        BumpKindCondition
    ),
    ("switch", "Switch on one value.", Exactly(1)),
    ("case", "Case inside @switch.", Exactly(1)),
    ("default", "Default case inside @switch.", None),
);

pub(crate) fn directive(name: &str) -> Option<&'static DirectiveInfo> {
    DIRECTIVES.iter().find(|directive| directive.name == name)
}

pub(crate) fn directive_description(name: &str) -> Option<&'static str> {
    directive(name).map(|directive| directive.description)
}
