const assert = require("assert");
const fs = require("fs");
const path = require("path");
const oniguruma = require("vscode-oniguruma");
const textmate = require("vscode-textmate");

const source = `@env:
  PORT?: 3000
  MESSAGE: "hello $USER"
@help:
  Warning: "quoted" @task build
  Home is documented
build:client:
  @desc Build target quickly
  @set RESULT:
    @echo "captured $VALUE"
  @cd editors/vscode-cjtaskrunner
  @task build:assets
  cargo build --target $TARGET # not a comment
  # full-line comment`;

const invalidMetadataVariableSource = `@help:
  Home is $HOME
build:
  @desc Build $TARGET quickly`;

const stringEdgeSource = String.raw`strings:
  @echo "unterminated
next:task:
  @echo "escaped \"quote\" remains"
after:quotes:`;

const semicolonSource = `flow:
  false; @or
  @task build; @and`;

const escapedSemicolonSource = String.raw`run:
  @echo x\; @task fake`;

const invalidDirectiveSource = `invalid:
  @desc-extra prose
  @task-extra build
  @if-version-extra 1.2.3`;

const variableEdgeSource = [
  "variables:",
  "  @echo ${A?fallback} then ${B}",
  "  @echo \\$NAME \\${NAME}",
].join("\n");

const taskArgumentSource = `deploy (TARGET, TAG):
  true
release:
  publish (REGISTRY):
    true`;

const interpolatedSetCaptureSource = `run:
  @set $CAPTURE_NAME:
    @echo captured`;

async function loadGrammar() {
  const wasmPath = require.resolve("vscode-oniguruma/release/onig.wasm");
  await oniguruma.loadWASM(fs.readFileSync(wasmPath).buffer);

  const grammarPath = path.join(
    __dirname,
    "..",
    "syntaxes",
    "cjtasks.tmLanguage.json",
  );
  const registry = new textmate.Registry({
    onigLib: Promise.resolve({
      createOnigScanner: (patterns) => new oniguruma.OnigScanner(patterns),
      createOnigString: (value) => new oniguruma.OnigString(value),
    }),
    loadGrammar: async (scopeName) => {
      if (scopeName !== "source.cjtasks") {
        return null;
      }
      return textmate.parseRawGrammar(
        fs.readFileSync(grammarPath, "utf8"),
        grammarPath,
      );
    },
  });

  return registry.loadGrammar("source.cjtasks");
}

function tokenizeLines(grammar, text) {
  let ruleStack = textmate.INITIAL;
  return text.split("\n").map((line) => {
    const result = grammar.tokenizeLine(line, ruleStack);
    ruleStack = result.ruleStack;
    return { line, tokens: result.tokens };
  });
}

function findToken(lines, lineNumber, substring, occurrence = 1) {
  const { line, tokens } = lines[lineNumber];
  let startIndex = -1;
  let searchFrom = 0;

  for (let found = 0; found < occurrence; found += 1) {
    startIndex = line.indexOf(substring, searchFrom);
    assert.notStrictEqual(
      startIndex,
      -1,
      `Line ${lineNumber + 1} does not contain occurrence ${occurrence} of ${JSON.stringify(substring)}`,
    );
    searchFrom = startIndex + substring.length;
  }

  const endIndex = startIndex + substring.length;
  const token = tokens.find(
    (candidate) =>
      candidate.startIndex <= startIndex && candidate.endIndex >= endIndex,
  );
  assert.ok(
    token,
    `No token covers ${JSON.stringify(substring)} on line ${lineNumber + 1}`,
  );
  return token;
}

function assertHasScope(lines, lineNumber, substring, scope, occurrence = 1) {
  const token = findToken(lines, lineNumber, substring, occurrence);
  assert.ok(
    token.scopes.includes(scope),
    `Expected ${JSON.stringify(substring)} on line ${lineNumber + 1} to have scope ${scope}; got ${token.scopes.join(", ")}`,
  );
}

function assertLacksScope(lines, lineNumber, substring, scope, occurrence = 1) {
  const token = findToken(lines, lineNumber, substring, occurrence);
  assert.ok(
    !token.scopes.includes(scope),
    `Expected ${JSON.stringify(substring)} on line ${lineNumber + 1} not to have scope ${scope}; got ${token.scopes.join(", ")}`,
  );
}

async function main() {
  const grammar = await loadGrammar();
  assert.ok(grammar, "CJTasks grammar failed to load");
  const lines = tokenizeLines(grammar, source);
  const invalidMetadataVariableLines = tokenizeLines(
    grammar,
    invalidMetadataVariableSource,
  );
  const taskArgumentLines = tokenizeLines(grammar, taskArgumentSource);
  const interpolatedSetCaptureLines = tokenizeLines(
    grammar,
    interpolatedSetCaptureSource,
  );

  assertHasScope(
    taskArgumentLines,
    0,
    "deploy",
    "entity.name.function.task.cjtasks",
  );
  assertHasScope(taskArgumentLines, 0, "TARGET", "variable.other.cjtasks");
  assertHasScope(taskArgumentLines, 0, "TAG", "variable.other.cjtasks");
  assertHasScope(
    taskArgumentLines,
    3,
    "publish",
    "entity.name.function.task.cjtasks",
  );
  assertHasScope(taskArgumentLines, 3, "REGISTRY", "variable.other.cjtasks");
  assertHasScope(
    taskArgumentLines,
    3,
    ":",
    "punctuation.separator.colon.cjtasks",
  );

  for (const lineNumber of [0, 3]) {
    assertHasScope(
      lines,
      lineNumber,
      lineNumber === 0 ? "@env" : "@help",
      "keyword.control.directive.block.cjtasks",
    );
    assertHasScope(
      lines,
      lineNumber,
      ":",
      "punctuation.separator.colon.cjtasks",
    );
  }

  assertHasScope(lines, 1, "PORT", "variable.other.cjtasks");
  assertHasScope(lines, 1, "?:", "punctuation.separator.colon.cjtasks");
  assertHasScope(lines, 1, "3000", "meta.task-line.cjtasks");
  assertHasScope(lines, 2, "MESSAGE", "variable.other.cjtasks");
  assertHasScope(lines, 2, ":", "punctuation.separator.colon.cjtasks");
  assertHasScope(lines, 2, "hello", "string.quoted.double.cjtasks");
  assertHasScope(lines, 2, "hello", "meta.task-line.cjtasks");
  assertHasScope(lines, 2, "$USER", "variable.other.cjtasks");
  assertHasScope(lines, 2, "$USER", "meta.task-line.cjtasks");

  for (const substring of ["Warning", "\"quoted\"", "@task"]) {
    assertHasScope(
      lines,
      4,
      substring,
      "comment.block.documentation.cjtasks",
    );
    for (const scope of [
      "entity.name.function.task.cjtasks",
      "keyword.control.directive.cjtasks",
      "string.quoted.double.cjtasks",
      "punctuation.separator.colon.cjtasks",
    ]) {
      assertLacksScope(lines, 4, substring, scope);
    }
  }
  assertHasScope(
    lines,
    4,
    ":",
    "comment.block.documentation.cjtasks",
  );
  assertLacksScope(
    lines,
    4,
    ":",
    "punctuation.separator.colon.cjtasks",
  );
  assertHasScope(
    invalidMetadataVariableLines,
    1,
    "$HOME",
    "variable.other.cjtasks",
  );

  assertHasScope(lines, 6, "build", "entity.name.function.task.cjtasks");
  assertHasScope(lines, 6, "client", "entity.name.function.task.cjtasks");
  assertHasScope(lines, 6, ":", "punctuation.separator.colon.cjtasks", 1);
  assertHasScope(lines, 6, ":", "punctuation.separator.colon.cjtasks", 2);

  assertHasScope(lines, 7, "@desc", "keyword.control.directive.cjtasks");
  assertHasScope(
    lines,
    7,
    "Build",
    "comment.block.documentation.cjtasks",
  );
  assertHasScope(
    invalidMetadataVariableLines,
    3,
    "$TARGET",
    "variable.other.cjtasks",
  );

  assertHasScope(
    lines,
    8,
    "@set",
    "keyword.control.directive.block.cjtasks",
  );
  assertHasScope(lines, 8, "RESULT", "variable.other.cjtasks");
  assertHasScope(lines, 8, ":", "punctuation.separator.colon.cjtasks");
  assertHasScope(
    interpolatedSetCaptureLines,
    1,
    "@set",
    "keyword.control.directive.block.cjtasks",
  );
  assertHasScope(
    interpolatedSetCaptureLines,
    1,
    "$CAPTURE_NAME",
    "variable.other.cjtasks",
  );
  assertHasScope(
    interpolatedSetCaptureLines,
    1,
    ":",
    "punctuation.separator.colon.cjtasks",
  );

  for (const [lineNumber, directive] of [
    [9, "@echo"],
    [10, "@cd"],
  ]) {
    assertHasScope(
      lines,
      lineNumber,
      directive,
      "keyword.control.directive.cjtasks",
    );
  }
  assertHasScope(lines, 9, "captured", "string.quoted.double.cjtasks");
  assertHasScope(lines, 9, "captured", "meta.task-line.cjtasks");
  assertHasScope(lines, 9, "$VALUE", "variable.other.cjtasks");
  assertHasScope(lines, 9, "$VALUE", "meta.task-line.cjtasks");
  assertHasScope(
    lines,
    10,
    "editors/vscode-cjtaskrunner",
    "meta.task-line.cjtasks",
  );

  assertHasScope(lines, 11, "@task", "keyword.control.directive.cjtasks");
  assertHasScope(
    lines,
    11,
    "build",
    "entity.name.function.task.cjtasks",
  );
  assertHasScope(
    lines,
    11,
    "assets",
    "entity.name.function.task.cjtasks",
  );
  assertHasScope(lines, 11, ":", "punctuation.separator.colon.cjtasks");

  assertHasScope(lines, 12, "cargo", "meta.task-line.cjtasks");
  assertHasScope(lines, 12, "--target", "meta.task-line.cjtasks");
  assertHasScope(lines, 12, "$TARGET", "variable.other.cjtasks");
  assertHasScope(lines, 12, "$TARGET", "meta.task-line.cjtasks");
  assertHasScope(lines, 12, "# not a comment", "meta.task-line.cjtasks");
  assertLacksScope(
    lines,
    12,
    "# not a comment",
    "comment.line.number-sign.cjtasks",
  );
  assertHasScope(
    lines,
    13,
    "# full-line comment",
    "comment.line.number-sign.cjtasks",
  );

  const stringEdgeLines = tokenizeLines(grammar, stringEdgeSource);
  assertHasScope(
    stringEdgeLines,
    1,
    "unterminated",
    "string.quoted.double.cjtasks",
  );
  assertHasScope(
    stringEdgeLines,
    2,
    "next",
    "entity.name.function.task.cjtasks",
  );
  assertLacksScope(
    stringEdgeLines,
    2,
    "next",
    "string.quoted.double.cjtasks",
  );
  assertHasScope(
    stringEdgeLines,
    3,
    "quote",
    "string.quoted.double.cjtasks",
  );
  assertHasScope(
    stringEdgeLines,
    3,
    "remains",
    "string.quoted.double.cjtasks",
  );
  assertHasScope(
    stringEdgeLines,
    4,
    "after",
    "entity.name.function.task.cjtasks",
  );
  assertLacksScope(
    stringEdgeLines,
    4,
    "after",
    "string.quoted.double.cjtasks",
  );

  const semicolonLines = tokenizeLines(grammar, semicolonSource);
  assertHasScope(
    semicolonLines,
    1,
    "@or",
    "keyword.control.directive.cjtasks",
  );
  assertHasScope(
    semicolonLines,
    2,
    "@task",
    "keyword.control.directive.cjtasks",
  );
  assertHasScope(
    semicolonLines,
    2,
    "build",
    "entity.name.function.task.cjtasks",
  );
  assertHasScope(
    semicolonLines,
    2,
    "@and",
    "keyword.control.directive.cjtasks",
  );
  assertLacksScope(
    semicolonLines,
    2,
    "and",
    "entity.name.function.task.cjtasks",
  );

  const escapedSemicolonLines = tokenizeLines(grammar, escapedSemicolonSource);
  assertHasScope(
    escapedSemicolonLines,
    1,
    "x",
    "meta.task-line.cjtasks",
  );
  assertHasScope(
    escapedSemicolonLines,
    1,
    ";",
    "meta.task-line.cjtasks",
  );
  assertHasScope(
    escapedSemicolonLines,
    1,
    "@task",
    "meta.task-line.cjtasks",
  );
  assertLacksScope(
    escapedSemicolonLines,
    1,
    "@task",
    "keyword.control.directive.cjtasks",
  );
  assertLacksScope(
    escapedSemicolonLines,
    1,
    "fake",
    "entity.name.function.task.cjtasks",
  );

  const invalidDirectiveLines = tokenizeLines(grammar, invalidDirectiveSource);
  for (const [lineNumber, directive] of [
    [1, "@desc-extra"],
    [2, "@task-extra"],
    [3, "@if-version-extra"],
  ]) {
    assertHasScope(
      invalidDirectiveLines,
      lineNumber,
      directive,
      "keyword.control.directive.cjtasks",
    );
    assertLacksScope(
      invalidDirectiveLines,
      lineNumber,
      directive,
      "keyword.control.directive.block.cjtasks",
    );
  }

  const variableEdgeLines = tokenizeLines(grammar, variableEdgeSource);
  assertHasScope(
    variableEdgeLines,
    1,
    "${A?fallback}",
    "variable.other.cjtasks",
  );
  assertLacksScope(
    variableEdgeLines,
    1,
    " then ",
    "variable.other.cjtasks",
  );
  assertHasScope(
    variableEdgeLines,
    1,
    "${B}",
    "variable.other.cjtasks",
  );
  assertLacksScope(
    variableEdgeLines,
    2,
    "$NAME",
    "variable.other.cjtasks",
  );
  assertLacksScope(
    variableEdgeLines,
    2,
    "${NAME}",
    "variable.other.cjtasks",
  );
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
