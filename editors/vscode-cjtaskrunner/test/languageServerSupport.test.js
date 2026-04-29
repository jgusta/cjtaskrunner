const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const {
  executableResolutionError,
  renderLanguageServerStatus,
  resolveExecutablePath
} = require("../out/languageServerSupport");

const workspaceRoot = path.join(path.sep, "repo", "workspace");

assert.deepStrictEqual(
  resolveExecutablePath({
    configuredPath: "",
    defaultBinaryName: "cj",
    workspaceRoot
  }),
  {
    command: "cj",
    configuredPath: "",
    displayPath: "cj",
    isConfigured: false,
    source: "PATH"
  }
);

assert.deepStrictEqual(
  resolveExecutablePath({
    configuredPath: "target/debug/cj",
    defaultBinaryName: "cj",
    workspaceRoot
  }),
  {
    command: path.join(workspaceRoot, "target/debug/cj"),
    configuredPath: "target/debug/cj",
    displayPath: path.join(workspaceRoot, "target/debug/cj"),
    isConfigured: true,
    source: "workspace setting"
  }
);

assert.deepStrictEqual(
  resolveExecutablePath({
    configuredPath: path.join(path.sep, "usr", "local", "bin", "cj"),
    defaultBinaryName: "cj",
    workspaceRoot
  }).command,
  path.join(path.sep, "usr", "local", "bin", "cj")
);

const status = renderLanguageServerStatus({
  clientState: "running",
  executable: resolveExecutablePath({
    configuredPath: "target/debug/cj",
    defaultBinaryName: "cj",
    workspaceRoot
  }),
  trace: "messages",
  outlineMode: "Language server with extension fallback",
  lastStartedAt: "2026-05-31T12:00:00.000Z",
  lastStoppedAt: undefined,
  lastError: undefined
});

assert.match(status, /^# CJTaskrunner Language Server/m);
assert.match(status, /State: running/);
assert.match(status, /Command: .*target[/\\]debug[/\\]cj lsp/);
assert.match(status, /Trace: messages/);
assert.match(status, /Outline: Language server with extension fallback/);

const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "cjtaskrunner-lsp-path-"));
const executableName = process.platform === "win32" ? "cj.exe" : "cj";
const executablePath = path.join(tempDir, executableName);
fs.writeFileSync(executablePath, process.platform === "win32" ? "" : "#!/bin/sh\n");
if (process.platform !== "win32") {
  fs.chmodSync(executablePath, 0o755);
}

assert.strictEqual(
  executableResolutionError(
    resolveExecutablePath({
      configuredPath: "",
      defaultBinaryName: "cj",
      workspaceRoot
    }),
    { PATH: tempDir },
    process.platform
  ),
  undefined
);

assert.match(
  executableResolutionError(
    resolveExecutablePath({
      configuredPath: "",
      defaultBinaryName: "cj",
      workspaceRoot
    }),
    { PATH: "" },
    process.platform
  ),
  /Could not find cj on PATH/
);

assert.match(
  executableResolutionError(
    resolveExecutablePath({
      configuredPath: "missing/cj",
      defaultBinaryName: "cj",
      workspaceRoot
    }),
    { PATH: tempDir },
    process.platform
  ),
  /Configured cjtaskrunner\.path is not executable/
);

fs.rmSync(tempDir, { recursive: true, force: true });
