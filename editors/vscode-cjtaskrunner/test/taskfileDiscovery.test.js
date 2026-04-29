const assert = require("assert");
const {
  isRecognizedTaskfileName,
  rootTaskfileCandidates,
  selectPreferredTaskfilePaths,
  taskfileLayerPaths
} = require("../out/taskfileDiscovery");

assert.strictEqual(isRecognizedTaskfileName("cjtasks"), true);
assert.strictEqual(isRecognizedTaskfileName("unknown.cjtasks"), false);
assert.strictEqual(isRecognizedTaskfileName("local.cjtasks"), true);
assert.strictEqual(isRecognizedTaskfileName("misc.cjtasks"), false);
assert.strictEqual(isRecognizedTaskfileName("tasks"), false);

assert.deepStrictEqual(rootTaskfileCandidates("/workspace"), [
  "/workspace/cjtasks",
  "/workspace/production.cjtasks",
  "/workspace/staging.cjtasks",
  "/workspace/development.cjtasks",
  "/workspace/local.cjtasks"
]);

assert.deepStrictEqual(
  selectPreferredTaskfilePaths([
    "/workspace/cjtasks",
    "/workspace/apps/api/local.cjtasks",
    "/workspace/apps/web/cjtasks",
    "/workspace/apps/web/local.cjtasks",
    "/workspace/apps/ignored/unknown.cjtasks"
  ]),
  [
    "/workspace/cjtasks",
    "/workspace/apps/api/local.cjtasks",
    "/workspace/apps/web/local.cjtasks"
  ]
);

assert.deepStrictEqual(
  taskfileLayerPaths([
    "/workspace/cjtasks",
    "/workspace/production.cjtasks",
    "/workspace/local.cjtasks"
  ], "/workspace"),
  [
    "/workspace/cjtasks",
    "/workspace/production.cjtasks",
    "/workspace/local.cjtasks"
  ]
);
