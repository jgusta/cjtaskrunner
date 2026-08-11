const assert = require("assert");
const fs = require("fs");
const path = require("path");

const extensionRoot = path.join(__dirname, "..");
const packageJson = JSON.parse(
  fs.readFileSync(path.join(extensionRoot, "package.json"), "utf8")
);
const extensionSource = fs.readFileSync(
  path.join(extensionRoot, "src", "extension.ts"),
  "utf8"
);

assert.deepStrictEqual(
  packageJson.activationEvents.filter((event) => event.startsWith("workspaceContains:")),
  [
    "workspaceContains:cjtasks",
    "workspaceContains:production.cjtasks",
    "workspaceContains:staging.cjtasks",
    "workspaceContains:development.cjtasks",
    "workspaceContains:local.cjtasks"
  ],
  "extension activation must check only recognized taskfiles at workspace roots"
);

const taskView = packageJson.contributes.views.explorer.find(
  (view) => view.id === "cjtaskrunner.tasks"
);
assert.strictEqual(
  taskView.when,
  "cjtaskrunner.hasRootTaskfile",
  "task view must stay hidden without a recognized root taskfile"
);
assert.strictEqual(taskView.name, "CJTASKS");

const configuration = packageJson.contributes.configuration.properties;
assert.strictEqual(configuration["cjtaskrunner.showTaskfileCascade"].default, false);
assert.strictEqual(
  configuration["cjtaskrunner.showTaskfileCascade"].description,
  "Show the taskfile cascade in the CJTASKS panel."
);
assert.strictEqual(
  configuration["cjtaskrunner.showOverriddenTasksInCascade"].default,
  false
);
assert.strictEqual(
  configuration["cjtaskrunner.showOverriddenTasksInCascade"].description,
  "Show overridden tasks as disabled entries in the taskfile cascade."
);

const language = packageJson.contributes.languages.find(
  (entry) => entry.id === "cjtasks"
);
assert.deepStrictEqual(language.filenames, [
  "cjtasks",
  "production.cjtasks",
  "staging.cjtasks",
  "development.cjtasks",
  "local.cjtasks"
]);
assert.ok(
  !Object.hasOwn(language, "extensions"),
  "arbitrary .cjtasks filenames must not activate language support"
);

assert.ok(
  !/from\s+["']\.\/languageServer["']/.test(extensionSource),
  "extension.ts must not import the language server eagerly"
);
assert.ok(
  /import\(["']\.\/languageServer\.js["']\)/.test(extensionSource),
  "extension.ts must load the emitted language server bundle with a dynamic import"
);
assert.ok(
  !/await\s+[^;\n]*\.start\(\)/.test(extensionSource),
  "activate() must not await language server startup"
);
assert.ok(
  extensionSource.includes('"cjtaskrunner.hasRootTaskfile"')
    && extensionSource.includes('"setContext"'),
  "extension must update the task view visibility context"
);
assert.ok(
  !extensionSource.includes("*.cjtasks"),
  "extension discovery must not use wildcard .cjtasks matching"
);
assert.ok(
  extensionSource.includes('command: "cjtaskrunner.openTask"'),
  "active task rows must open their definitions"
);
assert.ok(
  extensionSource.includes("let taskTerminal: vscode.Terminal | undefined"),
  "task runs must retain an extension-owned terminal for reuse"
);
assert.ok(
  extensionSource.includes("if (!taskTerminal)")
    && extensionSource.includes('name: "CJTaskrunner"'),
  "task runs must reuse one stable CJTaskrunner terminal"
);
assert.ok(
  extensionSource.includes("vscode.window.onDidCloseTerminal")
    && extensionSource.includes("taskTerminal = undefined"),
  "closing the shared task terminal must allow it to be recreated"
);
