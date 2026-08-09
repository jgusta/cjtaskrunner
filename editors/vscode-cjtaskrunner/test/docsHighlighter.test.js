const assert = require("assert");
const path = require("path");

const root = path.join(__dirname, "..", "..", "..");
const hljs = require(path.join(root, "docs", "theme", "highlight.js"));
const cjTasksLanguage = require(path.join(root, "docs", "theme", "cjtaskrunner.js"));

if (!hljs.getLanguage("cjtasks")) {
  hljs.registerLanguage("cjtasks", cjTasksLanguage);
}

function highlight(source) {
  return hljs.highlight("cjtasks", source).value;
}

function assertHighlighted(source, expected) {
  const result = highlight(source);
  assert.ok(
    result.includes(expected),
    `Expected ${JSON.stringify(source)} to include ${JSON.stringify(expected)}; got ${result}`,
  );
}

assertHighlighted(
  "run:\n  @set $CAPTURE_NAME:\n    @echo captured",
  '<span class="hljs-meta">@set</span> <span class="hljs-variable">$CAPTURE_NAME</span><span class="hljs-symbol">:</span>',
);

assertHighlighted(
  "run:\n  @set ${CAPTURE_NAME?RESULT}:\n    @echo captured",
  '<span class="hljs-meta">@set</span> <span class="hljs-variable">${CAPTURE_NAME?RESULT}</span><span class="hljs-symbol">:</span>',
);

assertHighlighted(
  "build (PROFILE):\n  @watch src\n    @task build",
  '<span class="hljs-title">build</span> (<span class="hljs-variable">PROFILE</span>)<span class="hljs-symbol">:</span>',
);

assertHighlighted(
  "run:\n  @desc Build $TARGET",
  '<span class="hljs-variable">$TARGET</span>',
);

assertHighlighted(
  "run:\n  @task build:cli",
  '<span class="hljs-title">build:cli</span>',
);
