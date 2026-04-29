use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::command_text::{
    contains_variable_interpolation, split_words, unescape_variable_literals,
};
use crate::directives::split_directive;
use crate::taskfile_discovery::{base_taskfile_path, layer_paths, BASE_TASKFILE_NAME};
use crate::version::{validate_semver, version_env_key};
use crate::{CjError, CjResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskFile {
    pub(crate) env: EnvEntries,
    pub(crate) versions: HashMap<String, VersionEntry>,
    pub(crate) source_path: Option<PathBuf>,
    pub(crate) tasks: HashMap<String, Vec<TaskLine>>,
    pub(crate) task_arguments: HashMap<String, Vec<String>>,
    pub(crate) awaits: HashMap<String, Vec<AwaitTask>>,
    pub(crate) descriptions: HashMap<String, String>,
    pub(crate) help: Option<String>,
    pub(crate) task_help: HashMap<String, String>,
    pub(crate) task_order: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskLine {
    pub(crate) line_number: usize,
    pub(crate) indent: usize,
    pub(crate) text: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct EnvEntries {
    pub(crate) overrides: HashMap<String, String>,
    pub(crate) fallbacks: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VersionEntry {
    pub(crate) name: String,
    pub(crate) env_key: String,
    pub(crate) value: String,
    pub(crate) line_number: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AwaitTask {
    pub(crate) name: String,
    pub(crate) line_number: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ParserMode {
    Top,
    Env,
    Task,
    TopHelp,
    TaskHelp {
        task: String,
        base_indent: usize,
        lines: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TaskContext {
    name: String,
    header_indent: usize,
}

#[derive(Debug, Clone, Copy)]
struct SourceLine<'a> {
    number: usize,
    text: &'a str,
    trimmed: &'a str,
}

#[derive(Debug, Clone, Copy, Default)]
struct LeadingIndent {
    width: usize,
    bytes: usize,
    has_spaces: bool,
    has_tabs: bool,
}

#[derive(Debug, Clone, Copy)]
enum LineEntry<'a> {
    TopLevel(&'a str),
    Indented { indent: usize, text: &'a str },
}

impl<'a> SourceLine<'a> {
    fn new(number: usize, raw: &'a str) -> Self {
        let text = raw.strip_suffix('\r').unwrap_or(raw);
        Self {
            number,
            text,
            trimmed: text.trim(),
        }
    }

    fn indent(self) -> usize {
        leading_indent(self.text).width
    }

    fn indent_bytes(self) -> usize {
        leading_indent(self.text).bytes
    }

    fn is_indented(self) -> bool {
        leading_indent(self.text).bytes > 0
    }
}

struct TaskFileParser<'a> {
    path: &'a Path,
    env: EnvEntries,
    versions: HashMap<String, VersionEntry>,
    tasks: HashMap<String, Vec<TaskLine>>,
    task_arguments: HashMap<String, Vec<String>>,
    awaits: HashMap<String, Vec<AwaitTask>>,
    descriptions: HashMap<String, String>,
    help_lines: Vec<String>,
    task_help: HashMap<String, String>,
    task_order: Vec<String>,
    mode: ParserMode,
    task_stack: Vec<TaskContext>,
    seen_env: bool,
    seen_help: bool,
    seen_task: bool,
}

pub(crate) fn parse_task_file_layers(dir: &Path) -> CjResult<TaskFile> {
    let base = base_taskfile_path(dir)
        .is_file()
        .then(|| base_taskfile_path(dir));
    let paths = layer_paths(dir);
    if paths.is_empty() {
        return Err(CjError::new(format!(
            "no recognized taskfile found in {}",
            dir.display()
        )));
    }

    let mut flattened = empty_task_file(base.clone());
    for path in paths {
        let source = fs::read_to_string(&path)
            .map_err(|err| CjError::new(format!("failed to read {}: {err}", path.display())))?;
        let layer = TaskFileParser::new(&path).parse_unvalidated(&source)?;
        let is_base = base.as_ref().is_some_and(|base| base == &path);
        merge_layer(&mut flattened, layer, &path, is_base)?;
    }
    validate_awaits(
        &flattened.tasks,
        &flattened.task_arguments,
        &flattened.awaits,
        base.as_deref().unwrap_or(dir),
    )?;
    Ok(flattened)
}

pub fn parse_task_file(source: &str, path: &Path) -> CjResult<TaskFile> {
    TaskFileParser::new(path).parse(source)
}

impl<'a> TaskFileParser<'a> {
    fn new(path: &'a Path) -> Self {
        Self {
            path,
            env: EnvEntries::default(),
            versions: HashMap::new(),
            tasks: HashMap::new(),
            task_arguments: HashMap::new(),
            awaits: HashMap::new(),
            descriptions: HashMap::new(),
            help_lines: Vec::new(),
            task_help: HashMap::new(),
            task_order: Vec::new(),
            mode: ParserMode::Top,
            task_stack: Vec::new(),
            seen_env: false,
            seen_help: false,
            seen_task: false,
        }
    }

    fn parse(self, source: &str) -> CjResult<TaskFile> {
        let path = self.path;
        let task_file = self.parse_unvalidated(source)?;
        validate_awaits(
            &task_file.tasks,
            &task_file.task_arguments,
            &task_file.awaits,
            path,
        )?;
        Ok(task_file)
    }

    fn parse_unvalidated(mut self, source: &str) -> CjResult<TaskFile> {
        reject_mixed_indentation_styles(source, self.path)?;
        for (index, raw_line) in source.lines().enumerate() {
            self.parse_line(SourceLine::new(index + 1, raw_line))?;
        }
        self.finish()
    }

    fn parse_line(&mut self, line: SourceLine<'_>) -> CjResult<()> {
        if self.capture_help_line(line)? {
            return Ok(());
        }

        let Some(entry) = self.classify_line(line)? else {
            return Ok(());
        };

        match entry {
            LineEntry::TopLevel(text) => self.handle_top_level(text, line.number),
            LineEntry::Indented { indent, text } => self.handle_indented(indent, text, line.number),
        }
    }

    fn capture_help_line(&mut self, line: SourceLine<'_>) -> CjResult<bool> {
        match &mut self.mode {
            ParserMode::TopHelp => {
                if line.trimmed.is_empty() {
                    self.help_lines.push(String::new());
                    return Ok(true);
                }
                if line.is_indented() {
                    let indent = line.indent();
                    if indent < 2 {
                        return Err(line_error(
                            self.path,
                            line.number,
                            "help entries must use at least one indentation level",
                        ));
                    }
                    let text = strip_help_indent(line.text, 2);
                    reject_metadata_variables(self.path, line.number, "@help:", text)?;
                    self.help_lines.push(unescape_variable_literals(text));
                    return Ok(true);
                }
                self.mode = ParserMode::Top;
                Ok(false)
            }
            ParserMode::TaskHelp {
                base_indent, lines, ..
            } => {
                if line.trimmed.is_empty() {
                    lines.push(String::new());
                    return Ok(true);
                }
                let indent = line.indent();
                if line.is_indented() && indent > *base_indent {
                    let text = strip_help_indent(line.text, *base_indent + 2);
                    reject_metadata_variables(self.path, line.number, "@help:", text)?;
                    lines.push(unescape_variable_literals(text));
                    return Ok(true);
                }
                self.finish_task_help();
                Ok(false)
            }
            ParserMode::Top | ParserMode::Env | ParserMode::Task => Ok(false),
        }
    }

    fn classify_line<'b>(&self, line: SourceLine<'b>) -> CjResult<Option<LineEntry<'b>>> {
        if line.trimmed.is_empty() || line.trimmed.starts_with('#') {
            return Ok(None);
        }
        if !line.is_indented() {
            return Ok(Some(LineEntry::TopLevel(line.text)));
        }

        let indent = line.indent();
        if indent < 2 || !indent.is_multiple_of(2) {
            return Err(line_error(
                self.path,
                line.number,
                "indented entries must use full indentation levels",
            ));
        }

        Ok(Some(LineEntry::Indented {
            indent,
            text: &line.text[line.indent_bytes()..],
        }))
    }

    fn handle_top_level(&mut self, text: &str, line_number: usize) -> CjResult<()> {
        self.task_stack.clear();

        if let Some(version) = parse_version_header(text, self.path, line_number)? {
            return self.add_version(version, line_number);
        }

        match text {
            "@help:" => return self.start_top_help(line_number),
            "@env:" => return self.start_env(line_number),
            _ => {}
        }

        let (key, arguments) = parse_task_header(text)
            .map_err(|message| line_error(self.path, line_number, message))?;
        self.seen_task = true;
        self.add_task(&key, arguments, line_number)?;
        self.task_stack.push(TaskContext {
            name: key,
            header_indent: 0,
        });
        self.mode = ParserMode::Task;
        Ok(())
    }

    fn handle_indented(&mut self, indent: usize, text: &str, line_number: usize) -> CjResult<()> {
        match self.mode {
            ParserMode::Env => self.handle_env_line(indent, text, line_number),
            ParserMode::Task => self.handle_task_line(indent, text, line_number),
            ParserMode::Top => Err(line_error(
                self.path,
                line_number,
                "indented entry is not under env or a task",
            )),
            ParserMode::TopHelp | ParserMode::TaskHelp { .. } => {
                unreachable!("help modes are consumed before normal line dispatch")
            }
        }
    }

    fn handle_env_line(&mut self, indent: usize, text: &str, line_number: usize) -> CjResult<()> {
        if indent != 2 {
            return Err(line_error(
                self.path,
                line_number,
                "env entries must use exactly one indentation level",
            ));
        }
        parse_env_entry(text, &mut self.env, self.path, line_number)
    }

    fn handle_task_line(&mut self, indent: usize, text: &str, line_number: usize) -> CjResult<()> {
        self.pop_finished_task_contexts(indent);

        let context = self
            .task_stack
            .last()
            .ok_or_else(|| line_error(self.path, line_number, "command without a task"))?;
        let active_task = context.name.clone();
        let logical_indent = indent.checked_sub(context.header_indent).ok_or_else(|| {
            line_error(
                self.path,
                line_number,
                "command indentation is outside current task",
            )
        })?;

        if logical_indent == 2 {
            if let Some(header) = parse_nested_task_header(text) {
                let (child_name, arguments) =
                    header.map_err(|message| line_error(self.path, line_number, message))?;
                let nested_name = format!("{active_task}:{child_name}");
                self.add_task(&nested_name, arguments, line_number)?;
                self.task_stack.push(TaskContext {
                    name: nested_name,
                    header_indent: indent,
                });
                return Ok(());
            }
            if let Some(parsed_awaits) = parse_awaits(text, self.path, line_number)? {
                self.awaits
                    .entry(active_task.clone())
                    .or_default()
                    .extend(parsed_awaits);
            }
        }

        validate_directive_syntax(text, self.path, line_number)?;

        if logical_indent == 2 {
            if let Some(description) = parse_description(text) {
                reject_metadata_variables(self.path, line_number, "@desc", description)?;
                self.descriptions
                    .insert(active_task.clone(), unescape_variable_literals(description));
                return Ok(());
            }
            if is_help_directive(text) {
                self.mode = ParserMode::TaskHelp {
                    task: active_task,
                    base_indent: indent,
                    lines: Vec::new(),
                };
                return Ok(());
            }
        }

        let task = self
            .tasks
            .get_mut(&active_task)
            .expect("current task must exist");
        for text in split_line_expressions(text) {
            validate_task_expression_syntax(&text, self.path, line_number)?;
            task.push(TaskLine {
                line_number,
                indent: logical_indent,
                text,
            });
        }
        Ok(())
    }

    fn pop_finished_task_contexts(&mut self, indent: usize) {
        while self.task_stack.len() > 1
            && indent <= self.task_stack.last().expect("task context").header_indent
        {
            self.task_stack.pop();
        }
    }

    fn start_top_help(&mut self, line_number: usize) -> CjResult<()> {
        if self.seen_help {
            return Err(line_error(
                self.path,
                line_number,
                "multiple @help: sections are not allowed in the same block",
            ));
        }
        self.seen_help = true;
        self.mode = ParserMode::TopHelp;
        Ok(())
    }

    fn start_env(&mut self, line_number: usize) -> CjResult<()> {
        if self.seen_task {
            return Err(line_error(
                self.path,
                line_number,
                "@env: must appear before tasks",
            ));
        }
        if self.seen_env {
            return Err(line_error(
                self.path,
                line_number,
                "multiple @env: sections are not allowed in the same block",
            ));
        }
        self.seen_env = true;
        self.mode = ParserMode::Env;
        Ok(())
    }

    fn add_version(&mut self, version: VersionEntry, line_number: usize) -> CjResult<()> {
        if self.seen_task {
            return Err(line_error(
                self.path,
                line_number,
                "@version must appear before tasks",
            ));
        }
        if self.versions.contains_key(&version.name) {
            return Err(line_error(
                self.path,
                line_number,
                format!("duplicate version '{}'", version.name),
            ));
        }
        if self.env.overrides.contains_key(&version.env_key)
            || self.env.fallbacks.contains_key(&version.env_key)
        {
            return Err(line_error(
                self.path,
                line_number,
                format!("duplicate env entry '{}'", version.env_key),
            ));
        }
        self.env
            .overrides
            .insert(version.env_key.clone(), version.value.clone());
        self.versions.insert(version.name.clone(), version);
        self.mode = ParserMode::Top;
        Ok(())
    }

    fn add_task(&mut self, key: &str, arguments: Vec<String>, line_number: usize) -> CjResult<()> {
        validate_task_name(key).map_err(|err| {
            line_error(
                self.path,
                line_number,
                format!("invalid task name '{key}': {err}"),
            )
        })?;
        validate_task_nesting_depth(key).map_err(|err| {
            line_error(
                self.path,
                line_number,
                format!("invalid task name '{key}': {err}"),
            )
        })?;
        if self.tasks.contains_key(key) {
            return Err(line_error(
                self.path,
                line_number,
                format!("duplicate task '{key}'"),
            ));
        }
        self.task_order.push(key.to_string());
        self.tasks.insert(key.to_string(), Vec::new());
        self.task_arguments.insert(key.to_string(), arguments);
        Ok(())
    }

    fn finish_task_help(&mut self) {
        let mode = std::mem::replace(&mut self.mode, ParserMode::Task);
        if let ParserMode::TaskHelp { task, lines, .. } = mode {
            self.task_help.insert(task, finish_help(lines));
        }
    }

    fn finish(mut self) -> CjResult<TaskFile> {
        self.finish_task_help();
        Ok(TaskFile {
            env: self.env,
            versions: self.versions,
            source_path: None,
            tasks: self.tasks,
            task_arguments: self.task_arguments,
            awaits: self.awaits,
            descriptions: self.descriptions,
            help: self.seen_help.then(|| finish_help(self.help_lines)),
            task_help: self.task_help,
            task_order: self.task_order,
        })
    }
}

fn empty_task_file(source_path: Option<PathBuf>) -> TaskFile {
    TaskFile {
        env: EnvEntries::default(),
        versions: HashMap::new(),
        source_path,
        tasks: HashMap::new(),
        task_arguments: HashMap::new(),
        awaits: HashMap::new(),
        descriptions: HashMap::new(),
        help: None,
        task_help: HashMap::new(),
        task_order: Vec::new(),
    }
}

fn merge_layer(
    target: &mut TaskFile,
    mut layer: TaskFile,
    path: &Path,
    is_base: bool,
) -> CjResult<()> {
    if !is_base {
        // check to make sure we aren't using any version tools anywhere
        if let Some(version) = layer.versions.values().next() {
            return Err(line_error(
                path,
                version.line_number,
                format!("@version is only allowed in {BASE_TASKFILE_NAME}"),
            ));
        }
        for lines in layer.tasks.values() {
            for line in lines {
                if line
                    .text
                    .strip_prefix('@')
                    .is_some_and(|rest| is_version_bump_directive(split_directive(rest).0))
                {
                    return Err(line_error(
                        path,
                        line.line_number,
                        format!("version bump directives are only allowed in {BASE_TASKFILE_NAME}"),
                    ));
                }
            }
        }
    } else {
        target.versions = std::mem::take(&mut layer.versions);
    }

    //wrangle the environment entries
    for (name, value) in layer.env.overrides {
        target.env.fallbacks.remove(&name);
        target.env.overrides.insert(name, value);
    }
    for (name, value) in layer.env.fallbacks {
        target.env.overrides.remove(&name);
        target.env.fallbacks.insert(name, value);
    }
    if layer.help.is_some() {
        target.help = layer.help;
    }

    // check the arity of each to make sure they match.
    // Other than that, just replace the whole thing.
    for name in layer.task_order {
        let new_arity = layer.task_arguments.get(&name).map_or(0, Vec::len);
        if let Some(arguments) = target.task_arguments.get(&name) {
            if arguments.len() != new_arity {
                return Err(CjError::new(format!(
                    "{}: task '{name}' overrides arity {} with arity {new_arity}",
                    path.display(),
                    arguments.len()
                )));
            }
        } else {
            target.task_order.push(name.clone());
        }
        target.tasks.insert(
            name.clone(),
            layer.tasks.remove(&name).expect("layer task must exist"),
        );
        target.task_arguments.insert(
            name.clone(),
            layer.task_arguments.remove(&name).unwrap_or_default(),
        );
        replace_optional_entry(&mut target.awaits, &mut layer.awaits, &name);
        replace_optional_entry(&mut target.descriptions, &mut layer.descriptions, &name);
        replace_optional_entry(&mut target.task_help, &mut layer.task_help, &name);
    }
    Ok(())
}

fn replace_optional_entry<T>(
    target: &mut HashMap<String, T>,
    layer: &mut HashMap<String, T>,
    name: &str,
) {
    target.remove(name);
    if let Some(value) = layer.remove(name) {
        target.insert(name.to_string(), value);
    }
}

fn reject_metadata_variables(
    path: &Path,
    line_number: usize,
    directive: &str,
    text: &str,
) -> CjResult<()> {
    if contains_variable_interpolation(text) {
        return Err(line_error(
            path,
            line_number,
            format!("{directive} text cannot contain variables"),
        ));
    }
    Ok(())
}

pub(crate) fn parse_nested_task_header(
    text: &str,
) -> Option<std::result::Result<(String, Vec<String>), String>> {
    if text.starts_with('@') || !text.ends_with(':') {
        return None;
    }
    let key = text.strip_suffix(':').expect("checked suffix");
    let looks_like_plain_task = !key.is_empty() && !key.contains(char::is_whitespace);
    let looks_like_argument_task =
        key.split_once(char::is_whitespace)
            .is_some_and(|(name, rest)| {
                valid_task_name_part(name) && rest.trim_start().starts_with('(')
            });
    (looks_like_plain_task || looks_like_argument_task).then(|| parse_task_header(text))
}

fn parse_description(text: &str) -> Option<&str> {
    let args = text.strip_prefix("@desc")?;
    if !args.is_empty() && !args.starts_with(char::is_whitespace) {
        return None;
    }
    Some(args.trim())
}

fn parse_awaits(text: &str, path: &Path, line_number: usize) -> CjResult<Option<Vec<AwaitTask>>> {
    let Some(rest) = text.strip_prefix("@await") else {
        return Ok(None);
    };
    if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
        return Ok(None);
    }
    let names = split_words(rest.trim_start()).map_err(|err| {
        line_error(
            path,
            line_number,
            format!("@await has invalid arguments: {err}"),
        )
    })?;
    if names.is_empty() {
        return Err(line_error(
            path,
            line_number,
            "@await expects at least one task name",
        ));
    }
    for name in &names {
        validate_task_name(name).map_err(|err| {
            line_error(
                path,
                line_number,
                format!("invalid awaited task name '{name}': {err}"),
            )
        })?;
    }
    Ok(Some(
        names
            .into_iter()
            .map(|name| AwaitTask { name, line_number })
            .collect(),
    ))
}

fn is_help_directive(text: &str) -> bool {
    text == "@help:"
}

// enforce an extreme bias towards spaces.
fn reject_mixed_indentation_styles(source: &str, path: &Path) -> CjResult<()> {
    let mut space_indent_line = None;
    let mut tab_indent_line = None;

    for (index, raw_line) in source.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        let indent = leading_indent(line);
        if indent.has_spaces {
            space_indent_line.get_or_insert(line_number);
        }
        if indent.has_tabs {
            tab_indent_line.get_or_insert(line_number);
        }
        if indent.has_spaces && indent.has_tabs {
            return Err(mixed_indentation_error(path, line_number));
        }
        if space_indent_line.is_some() && tab_indent_line.is_some() {
            return Err(mixed_indentation_error(path, line_number));
        }
    }

    Ok(())
}

fn mixed_indentation_error(path: &Path, line_number: usize) -> CjError {
    line_error(
        path,
        line_number,
        "taskfile uses both leading spaces and tabs for indentation; use one indentation style per file or run `cj --format` to normalize indentation to spaces",
    )
}

fn leading_indent(line: &str) -> LeadingIndent {
    let mut indent = LeadingIndent::default();
    for (index, ch) in line.char_indices() {
        match ch {
            ' ' => {
                indent.width += 1;
                indent.bytes = index + 1;
                indent.has_spaces = true;
            }
            '\t' => {
                indent.width += 2;
                indent.bytes = index + 1;
                indent.has_tabs = true;
            }
            _ => break,
        }
    }
    indent
}

fn strip_help_indent(line: &str, width: usize) -> &str {
    let mut remaining = width;
    let mut byte_index = 0;
    for (index, ch) in line.char_indices() {
        if remaining == 0 {
            byte_index = index;
            break;
        }
        let Some(indent_width) = indent_char_width(ch) else {
            byte_index = index;
            break;
        };
        remaining = remaining.saturating_sub(indent_width);
        byte_index = index + 1;
    }
    &line[byte_index..]
}

fn indent_char_width(ch: char) -> Option<usize> {
    match ch {
        ' ' => Some(1),
        '\t' => Some(2),
        _ => None,
    }
}

fn finish_help(mut lines: Vec<String>) -> String {
    while lines.first().is_some_and(|line| line.is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

pub(crate) fn split_line_expressions(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;

    for ch in text.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            current.push(ch);
            escaped = true;
            continue;
        }
        match quote {
            Some(active) if ch == active => {
                quote = None;
                current.push(ch);
            }
            Some(_) => current.push(ch),
            None if ch == '\'' || ch == '"' => {
                quote = Some(ch);
                current.push(ch);
            }
            None if ch == ';' => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    parts.push(trimmed.to_string());
                }
                current.clear();
            }
            None => current.push(ch),
        }
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() {
        parts.push(trimmed.to_string());
    }
    parts
}

pub(crate) fn parse_task_header(line: &str) -> Result<(String, Vec<String>), String> {
    if !line.ends_with(':') {
        return Err("top-level entries must be a key followed by ':'".to_string());
    }

    let key = &line[..line.len() - 1];
    if key.trim() != key || key.is_empty() {
        return Err("invalid top-level key".to_string());
    }

    let Some(open) = key.find(" (") else {
        if key.contains(['(', ')']) || key.contains(char::is_whitespace) {
            return Err("invalid task argument declaration".to_string());
        }
        return Ok((key.to_string(), Vec::new()));
    };
    if !key.ends_with(')') || key[open + 2..key.len() - 1].contains(['(', ')']) {
        return Err("invalid task argument declaration".to_string());
    }

    let name = &key[..open];
    if name.is_empty() || name.contains(char::is_whitespace) {
        return Err("invalid task argument declaration".to_string());
    }

    let raw_arguments = &key[open + 2..key.len() - 1];
    if raw_arguments.trim().is_empty() {
        return Err("task argument list cannot be empty".to_string());
    }

    let mut seen = HashSet::new();
    let mut arguments = Vec::new();
    for raw_argument in raw_arguments.split(',') {
        let argument = raw_argument.trim();
        if argument.is_empty() {
            return Err("invalid or blank task argument declaration".to_string());
        }
        if argument.contains(char::is_whitespace) {
            return Err(
                "task arguments must be separated by commas and optional spaces".to_string(),
            );
        }
        validate_env_name(argument)
            .map_err(|err| format!("invalid task argument '{argument}': {err}"))?;
        if !seen.insert(argument) {
            return Err(format!("duplicate task argument '{argument}'"));
        }
        arguments.push(argument.to_string());
    }

    Ok((name.to_string(), arguments))
}

fn parse_version_header(
    line: &str,
    path: &Path,
    line_number: usize,
) -> CjResult<Option<VersionEntry>> {
    let Some(rest) = line.strip_prefix('@') else {
        return Ok(None);
    };
    let (directive, args) = split_directive(rest);
    if directive != "version" {
        return Ok(None);
    }

    let mut parts = args.split_whitespace();
    let Some(name) = parts.next() else {
        return Err(line_error(
            path,
            line_number,
            format!("@{directive} expects name and value"),
        ));
    };
    let Some(value) = parts.next() else {
        return Err(line_error(
            path,
            line_number,
            format!("@{directive} expects name and value"),
        ));
    };
    if parts.next().is_some() {
        return Err(line_error(
            path,
            line_number,
            format!("@{directive} expects exactly name and value"),
        ));
    }

    let env_key = version_env_key(name).map_err(|err| {
        line_error(
            path,
            line_number,
            format!("invalid version name '{name}': {err}"),
        )
    })?;
    validate_semver(value, line_number)
        .map_err(|err| line_error(path, line_number, err.to_string()))?;
    Ok(Some(VersionEntry {
        name: name.to_string(),
        env_key,
        value: value.to_string(),
        line_number,
    }))
}

fn validate_directive_syntax(text: &str, path: &Path, line_number: usize) -> CjResult<()> {
    if let Some(message) = directive_syntax_error(text) {
        return Err(line_error(path, line_number, message));
    }
    Ok(())
}

fn validate_task_expression_syntax(text: &str, path: &Path, line_number: usize) -> CjResult<()> {
    if let Some(message) = task_expression_syntax_error(text) {
        return Err(line_error(path, line_number, message));
    }
    Ok(())
}

pub(crate) fn directive_syntax_error(text: &str) -> Option<&'static str> {
    let rest = text.strip_prefix('@')?;
    let (name, args) = split_directive(rest);
    let colon_block_directive = matches!(
        name,
        "if" | "if-not"
            | "if-in"
            | "if-not-in"
            | "if-exists"
            | "if-not-exists"
            | "if-set"
            | "if-not-set"
            | "if-version"
            | "if-not-version"
            | "if-bumped"
            | "if-not-bumped"
            | "if-patch"
            | "if-minor"
            | "if-major"
            | "if-pre"
            | "if-release"
            | "if-not-patch"
            | "if-not-minor"
            | "if-not-major"
            | "if-not-pre"
            | "if-not-release"
            | "else"
            | "switch"
            | "case"
            | "default"
    ) && args.trim_end().ends_with(':');
    if name == "help:" {
        return None;
    }
    if name == "help" {
        return Some("@help must use trailing ':'");
    }
    if name.ends_with(':') || colon_block_directive {
        return Some("CJTaskrunner directives do not use trailing ':'");
    }
    None
}

pub(crate) fn task_expression_syntax_error(text: &str) -> Option<&'static str> {
    let trimmed = text.trim();
    if has_shell_line_continuation(trimmed)
        || (!trimmed.starts_with('@') && starts_with_shell_env_assignment(trimmed))
    {
        Some(
            "task lines do not run through a shell, so line continuations and NAME=value command prefixes are not supported; use @shell with the command on one line, or @export NAME value before the command",
        )
    } else {
        None
    }
}

// line continuation with backslash is a shell feature.
fn has_shell_line_continuation(text: &str) -> bool {
    let trimmed = text.trim_end();
    let trailing_backslashes = trimmed
        .as_bytes()
        .iter()
        .rev()
        .take_while(|byte| **byte == b'\\')
        .count();
    if trailing_backslashes == 0 || trailing_backslashes.is_multiple_of(2) {
        return false;
    }

    let prefix = &trimmed[..trimmed.len() - trailing_backslashes];
    prefix.chars().next_back().is_none_or(char::is_whitespace)
}

fn starts_with_shell_env_assignment(text: &str) -> bool {
    let Some(first_word) = text.split_whitespace().next() else {
        return false;
    };
    let Some((name, _)) = first_word.split_once('=') else {
        return false;
    };
    validate_env_name(name).is_ok()
}

fn is_version_bump_directive(name: &str) -> bool {
    matches!(name, "patch" | "minor" | "major" | "pre" | "release")
}

fn validate_awaits(
    tasks: &HashMap<String, Vec<TaskLine>>,
    task_arguments: &HashMap<String, Vec<String>>,
    awaits: &HashMap<String, Vec<AwaitTask>>,
    path: &Path,
) -> CjResult<()> {
    for (task, entries) in awaits {
        for awaited in entries {
            if task_arguments
                .get(&awaited.name)
                .is_some_and(|arguments| !arguments.is_empty())
            {
                return Err(line_error(
                    path,
                    awaited.line_number,
                    format!("awaited task '{}' requires arguments", awaited.name),
                ));
            }
        }
        let mut visiting = Vec::new();
        validate_await_cycles(task, task, entries, tasks, awaits, path, &mut visiting)?;
    }

    Ok(())
}

fn validate_await_cycles(
    root: &str,
    current: &str,
    entries: &[AwaitTask],
    tasks: &HashMap<String, Vec<TaskLine>>,
    awaits: &HashMap<String, Vec<AwaitTask>>,
    path: &Path,
    visiting: &mut Vec<String>,
) -> CjResult<()> {
    visiting.push(current.to_string());
    for awaited in entries {
        if !tasks.contains_key(&awaited.name) {
            return Err(line_error(
                path,
                awaited.line_number,
                format!("awaited task not found: {}", awaited.name),
            ));
        }
        if let Some(index) = visiting.iter().position(|task| task == &awaited.name) {
            let mut cycle = visiting[index..].to_vec();
            cycle.push(awaited.name.clone());
            return Err(line_error(
                path,
                awaited.line_number,
                format!("task await cycle detected: {}", cycle.join(" -> ")),
            ));
        }
        if let Some(next) = awaits.get(&awaited.name) {
            validate_await_cycles(root, &awaited.name, next, tasks, awaits, path, visiting)?;
        }
    }
    visiting.pop();
    if visiting.is_empty() && root != current {
        unreachable!("root await traversal should finish at root");
    }
    Ok(())
}

fn parse_env_entry(
    entry: &str,
    env: &mut EnvEntries,
    path: &Path,
    line_number: usize,
) -> CjResult<()> {
    let Some(colon_index) = entry.find(':') else {
        return Err(line_error(path, line_number, "env entry must contain ':'"));
    };
    let raw_key = &entry[..colon_index];
    let fallback = raw_key.ends_with('?');
    let key = if fallback {
        &raw_key[..raw_key.len() - 1]
    } else {
        raw_key
    };

    validate_env_name(key).map_err(|err| {
        line_error(
            path,
            line_number,
            format!("invalid env name '{key}': {err}"),
        )
    })?;

    if env.overrides.contains_key(key) || env.fallbacks.contains_key(key) {
        return Err(line_error(
            path,
            line_number,
            format!("duplicate env entry '{key}'"),
        ));
    }

    let value = strip_matching_quotes(strip_one_leading_space(&entry[colon_index + 1..]));
    if fallback {
        env.fallbacks.insert(key.to_string(), value);
    } else {
        env.overrides.insert(key.to_string(), value);
    }
    Ok(())
}

pub(crate) fn validate_task_name(name: &str) -> Result<(), &'static str> {
    if name.is_empty() {
        return Err("task name cannot be empty");
    }
    for part in name.split(':') {
        if !valid_task_name_part(part) {
            return Err(
                "task name parts must contain ASCII letters, digits, hyphens, and underscores",
            );
        }
    }
    Ok(())
}

pub(crate) fn validate_task_nesting_depth(name: &str) -> Result<(), &'static str> {
    if name.split(':').count() > 2 {
        return Err("task nesting is limited to one level");
    }
    Ok(())
}

fn valid_task_name_part(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch == '-' || ch == '_' || ch.is_ascii_alphanumeric())
}

pub(crate) fn validate_env_name(name: &str) -> Result<(), &'static str> {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Err("env name cannot be empty");
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return Err("env names must start with a letter or underscore");
    }
    if chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric()) {
        Ok(())
    } else {
        Err("env names must contain only ASCII letters, digits, and underscores")
    }
}

fn strip_one_leading_space(value: &str) -> &str {
    value.strip_prefix(' ').unwrap_or(value)
}

pub(crate) fn strip_matching_quotes(value: &str) -> String {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'\'' && last == b'\'') || (first == b'"' && last == b'"') {
            return value[1..value.len() - 1].to_string();
        }
    }
    value.to_string()
}

pub(crate) fn line_error(path: &Path, line_number: usize, message: impl Into<String>) -> CjError {
    CjError::new(format!(
        "{}:{line_number}: {}",
        path.display(),
        message.into()
    ))
}
