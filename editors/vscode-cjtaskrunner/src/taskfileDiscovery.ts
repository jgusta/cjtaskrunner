import * as path from "path";

export const TASKFILE_NAMES = [
  "cjtasks",
  "production.cjtasks",
  "staging.cjtasks",
  "development.cjtasks",
  "local.cjtasks",
] as const;

const DISPLAY_ORDER = [
  "local.cjtasks",
  "development.cjtasks",
  "staging.cjtasks",
  "production.cjtasks",
  "cjtasks"
] as const;

export function isRecognizedTaskfileName(name: string): boolean {
  return TASKFILE_NAMES.some((candidate) => candidate === name);
}

export function rootTaskfileCandidates(root: string): string[] {
  return TASKFILE_NAMES.map((name) => path.join(root, name));
}

export function selectPreferredTaskfilePaths(paths: readonly string[]): string[] {
  const selected = new Map<string, string>();

  for (const taskfilePath of paths) {
    const name = path.basename(taskfilePath);
    if (!isRecognizedTaskfileName(name)) {
      continue;
    }
    const directory = path.dirname(taskfilePath);
    const current = selected.get(directory);
    if (
      current === undefined ||
      DISPLAY_ORDER.indexOf(name as (typeof DISPLAY_ORDER)[number]) <
        DISPLAY_ORDER.indexOf(path.basename(current) as (typeof DISPLAY_ORDER)[number])
    ) {
      selected.set(directory, taskfilePath);
    }
  }

  return Array.from(selected.values()).sort((left, right) => {
    const depthDifference = directoryDepth(left) - directoryDepth(right);
    return depthDifference || left.localeCompare(right);
  });
}

function directoryDepth(taskfilePath: string): number {
  return path
    .dirname(taskfilePath)
    .slice(path.parse(taskfilePath).root.length)
    .split(/[\\/]/)
    .filter(Boolean).length;
}

export function taskfileLayerPaths(
  paths: readonly string[],
  directory: string
): string[] {
  const names = new Set(
    paths
      .filter((candidate) => path.dirname(candidate) === directory)
      .map((candidate) => path.basename(candidate))
  );
  return TASKFILE_NAMES.filter((name) => names.has(name)).map((name) => path.join(directory, name));
}
