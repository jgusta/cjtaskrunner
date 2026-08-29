const assert = require("assert");
const fs = require("fs");
const path = require("path");

const extensionRoot = path.join(__dirname, "..");
const packageJson = JSON.parse(
  fs.readFileSync(path.join(extensionRoot, "package.json"), "utf8")
);
const iconPath = path.join(extensionRoot, packageJson.icon ?? "");
const root = path.join(extensionRoot, "..", "..");
const canonicalLogoPath = path.join(root, "logo", "cj-logo-color-d.svg");
const panelIcons = [
  "images/cjdocicon-light.svg",
  "images/cjdocicon-dark.svg",
  "images/cdjtaskicon-light.svg",
  "images/cdjtaskicon-dark.svg"
];
const tasksView = packageJson.contributes?.views?.explorer?.find(
  (view) => view.id === "cjtaskrunner.tasks"
);
const vscodeIgnoreLines = fs
  .readFileSync(path.join(extensionRoot, ".vscodeignore"), "utf8")
  .split(/\r?\n/)
  .map((line) => line.trim())
  .filter((line) => line.length > 0 && !line.startsWith("#"));

assert.ok(
  packageJson.devDependencies?.["vscode-languageclient"],
  "vscode-languageclient must be a build-time dependency"
);
assert.ok(
  !packageJson.dependencies?.["vscode-languageclient"],
  "the bundled extension must not ship vscode-languageclient as a runtime dependency"
);
assert.ok(
  packageJson.devDependencies?.esbuild,
  "esbuild must bundle the extension runtime"
);
assert.match(
  packageJson.scripts?.compile ?? "",
  /\besbuild\b/,
  "compile must create the bundled extension entry point"
);
assert.ok(
  vscodeIgnoreLines.includes("node_modules/**"),
  ".vscodeignore must exclude bundled build dependencies"
);
assert.ok(packageJson.icon, "the extension must declare a Marketplace icon");
assert.ok(fs.statSync(iconPath).isFile(), "the Marketplace icon must exist");
assert.strictEqual(
  packageJson.icon,
  "icon.png",
  "the Marketplace icon must use the checked-in rendered PNG"
);
assert.ok(tasksView, "the CJTASKS Explorer view must be contributed");
assert.strictEqual(
  tasksView.icon,
  "$(checklist)",
  "the CJTASKS Explorer view must declare its themed view icon"
);

const icon = fs.readFileSync(iconPath);
assert.ok(
  fs.statSync(canonicalLogoPath).isFile(),
  "the canonical SVG logo must exist"
);
assert.ok(
  icon.length > 5000,
  "the Marketplace icon must be the rendered 256px asset"
);
assert.strictEqual(icon.readUInt32BE(16), 256, "icon width must be 256px");
assert.strictEqual(icon.readUInt32BE(20), 256, "icon height must be 256px");
assert.strictEqual(icon[25], 6, "icon must be an RGBA PNG");

for (const panelIcon of panelIcons) {
  assert.ok(
    fs.statSync(path.join(extensionRoot, panelIcon)).isFile(),
    `missing panel icon ${panelIcon}`
  );
  assert.ok(
    !vscodeIgnoreLines.includes(panelIcon) && !vscodeIgnoreLines.includes("images/**"),
    `panel icon must be included in the extension package: ${panelIcon}`
  );
}

for (const scriptName of ["package:ci", "publish:dry-run"]) {
  const script = packageJson.scripts?.[scriptName] ?? "";
  assert.ok(
    !script.includes("--no-dependencies"),
    `${scriptName} must use the standard package command`
  );
}

const bundle = fs.readFileSync(
  path.join(extensionRoot, packageJson.main),
  "utf8"
);
assert.ok(
  Buffer.byteLength(bundle) < 100 * 1024,
  "the activation bundle must stay below 100 KiB"
);
assert.ok(
  !/\brequire\(["']vscode-languageclient(?:\/node)?["']\)/.test(bundle),
  "the extension bundle must not contain vscode-languageclient"
);

const languageServerBundle = fs.readFileSync(
  path.join(extensionRoot, "out", "languageServer.js"),
  "utf8"
);
assert.ok(
  !/\brequire\(["']vscode-languageclient(?:\/node)?["']\)/.test(languageServerBundle),
  "the language server bundle must not contain vscode-languageclient"
);
