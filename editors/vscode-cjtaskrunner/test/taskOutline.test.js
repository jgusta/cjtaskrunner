const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { parseTaskOutline } = require("../out/taskOutline");

const fixture = fs.readFileSync(
  path.join(__dirname, "..", "..", "..", "tests", "fixtures", "outline.cjtasks"),
  "utf8",
);
const outline = parseTaskOutline(fixture);

assert.deepStrictEqual(
  outline.tasks.map((task) => task.name),
  ["build", "build:dev", "help", "env"]
);
assert.strictEqual(outline.tasks[0].description, "build tasks");
assert.strictEqual(outline.tasks[1].description, "build dev assets");
assert.strictEqual(outline.tasks[0].line, 6);
assert.strictEqual(outline.tasks[1].line, 8);
assert.deepStrictEqual(
  outline.symbols.map((symbol) => symbol.name),
  ["build", "help", "env"]
);
assert.deepStrictEqual(
  outline.symbols[0].children.map((symbol) => symbol.name),
  ["build:dev"]
);
assert.strictEqual(outline.symbols[0].startLine, 6);
assert.strictEqual(outline.symbols[0].endLine, 12);
assert.strictEqual(outline.symbols[1].name, "help");
assert.strictEqual(outline.symbols[2].name, "env");

const hiddenOutline = parseTaskOutline(`visible:
  public:
    true
  _private:
    child:
      true
_internal:
  child:
    true
group:_hidden:
  child:
    true
`);

assert.deepStrictEqual(
  hiddenOutline.tasks.map((task) => task.name),
  ["visible", "visible:public"],
);
assert.deepStrictEqual(
  hiddenOutline.symbols.map((symbol) => symbol.name),
  ["visible", "_internal", "group:_hidden"],
);
assert.deepStrictEqual(
  hiddenOutline.symbols[0].children.map((symbol) => symbol.name),
  ["visible:public", "visible:_private"],
);

const tabOutline = parseTaskOutline(`build:
\t@desc build tasks
\tcli (PROFILE):
\t\t@desc build cli
\t\ttrue
`);

assert.deepStrictEqual(
  tabOutline.tasks.map((task) => task.name),
  ["build", "build:cli"],
);
assert.strictEqual(tabOutline.tasks[0].description, "build tasks");
assert.strictEqual(tabOutline.tasks[1].description, "build cli");
assert.deepStrictEqual(
  tabOutline.symbols[0].children.map((symbol) => symbol.name),
  ["build:cli"],
);

const selfHelpOutline = parseTaskOutline(`cli:
  @desc cli command group
  @help:
    CLI help.
  @selfhelp
  build:
    true
`);

assert.strictEqual(selfHelpOutline.tasks[0].name, "cli");
assert.strictEqual(selfHelpOutline.tasks[0].selfHelp, true);
assert.strictEqual(selfHelpOutline.tasks[1].name, "cli:build");
assert.strictEqual(selfHelpOutline.tasks[1].selfHelp, undefined);
