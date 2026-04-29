use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QuoteMode {
    None,
    Shell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputMode {
    Inherit,
    Capture,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CommandResult {
    pub(crate) status: i32,
    pub(crate) output: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BumpKind {
    Major,
    Minor,
    Patch,
    Pre,
    Release,
}

impl BumpKind {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "major" => Some(Self::Major),
            "minor" => Some(Self::Minor),
            "patch" => Some(Self::Patch),
            "pre" | "prerelease" => Some(Self::Pre),
            "release" => Some(Self::Release),
            _ => None,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Major => "major",
            Self::Minor => "minor",
            Self::Patch => "patch",
            Self::Pre => "pre",
            Self::Release => "release",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeEnv {
    pub(crate) vars: HashMap<String, String>,
    exports: Arc<Mutex<HashMap<String, String>>>,
    pub(crate) steps: usize,
    pub(crate) await_blocks_satisfied: bool,
    pub(crate) bumped_versions: HashMap<String, BumpKind>,
}

impl RuntimeEnv {
    pub(crate) fn new(initial: HashMap<String, String>) -> Self {
        Self {
            vars: initial.clone(),
            exports: Arc::new(Mutex::new(initial)),
            steps: 0,
            await_blocks_satisfied: false,
            bumped_versions: HashMap::new(),
        }
    }

    pub(crate) fn export(&mut self, key: String, value: String) {
        self.vars.insert(key.clone(), value.clone());
        self.exports.lock().expect("exports lock").insert(key, value);
    }

    pub(crate) fn unset(&mut self, key: &str) {
        self.vars.remove(key);
        self.exports.lock().expect("exports lock").remove(key);
    }

    pub(crate) fn exported_values(&self) -> HashMap<String, String> {
        self.exports.lock().expect("exports lock").clone()
    }

    pub(crate) fn restore_task_vars(
        &mut self,
        snapshot: HashMap<String, String>,
        previous_exports: HashMap<String, String>,
    ) {
        self.vars = snapshot;
        let current_exports = self.exported_values();
        for key in previous_exports.keys() {
            if !current_exports.contains_key(key) {
                self.vars.remove(key);
            }
        }
        self.sync_exports();
    }

    pub(crate) fn sync_exports(&mut self) {
        for (key, value) in self.exported_values() {
            self.vars.insert(key, value);
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CwdState {
    current: PathBuf,
    history: Vec<PathBuf>,
    scopes: Vec<CwdScope>,
}

#[derive(Debug, Clone)]
struct CwdScope {
    start: PathBuf,
    floor: usize,
}

impl CwdState {
    pub(crate) fn new(base_dir: &Path) -> Self {
        Self {
            current: base_dir.to_path_buf(),
            history: Vec::new(),
            scopes: Vec::new(),
        }
    }

    pub(crate) fn current(&self) -> &Path {
        &self.current
    }

    pub(crate) fn scope_base(&self) -> &Path {
        self.scopes
            .last()
            .map_or(self.current.as_path(), |scope| scope.start.as_path())
    }

    pub(crate) fn push_scope(&mut self) {
        self.scopes.push(CwdScope {
            start: self.current.clone(),
            floor: self.history.len(),
        });
    }

    pub(crate) fn pop_scope(&mut self) {
        if let Some(scope) = self.scopes.pop() {
            self.current = scope.start;
            self.history.truncate(scope.floor);
        }
    }

    pub(crate) fn cd(&mut self, path: PathBuf) {
        self.history.push(self.current.clone());
        self.current = path;
    }

    pub(crate) fn back(&mut self) {
        let floor = self.scopes.last().map_or(0, |scope| scope.floor);
        if self.history.len() > floor {
            if let Some(previous) = self.history.pop() {
                self.current = previous;
            }
        }
    }
}

thread_local! {
    pub(crate) static CAPTURED_OUTPUT: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn append_captured_output(value: &str) {
    CAPTURED_OUTPUT.with(|captured| {
        let mut captured = captured.borrow_mut();
        if let Some(active) = captured.last_mut() {
            active.push_str(value);
        }
    });
}
