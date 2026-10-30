import * as fs from "fs";
import * as path from "path";

export type ExecutableResolution = {
  command: string;
  configuredPath: string;
  displayPath: string;
  isConfigured: boolean;
  source: "PATH" | "workspace setting";
};

export type ResolveExecutablePathOptions = {
  configuredPath: string;
  defaultBinaryName: string;
  workspaceRoot?: string;
};

export function languageServerEnvironment(
  env: NodeJS.ProcessEnv = process.env,
  platform = process.platform
): NodeJS.ProcessEnv {
  const pathKey = Object.keys(env).find((key) => key.toLowerCase() === "path") ?? "PATH";
  const separator = platform === "win32" ? ";" : path.delimiter;
  const inheritedPath = env[pathKey] ?? "";
  const userBinDirs = [env.XDG_BIN_HOME, env.HOME ? path.join(env.HOME, ".local", "bin") : undefined]
    .filter((dir): dir is string => Boolean(dir));
  const paths = [...new Set([...userBinDirs, ...inheritedPath.split(separator)].filter(Boolean))];

  return {
    ...env,
    [pathKey]: paths.join(separator)
  };
}

export type LanguageServerStatus = {
  clientState: string;
  executable: ExecutableResolution;
  trace: string;
  outlineMode: string;
  lastStartedAt?: string;
  lastStoppedAt?: string;
  lastError?: string;
};

export type LanguageServerLifecycleState = "stopped" | "starting" | "running" | "stopping" | "failed";

export function resolveExecutablePath(options: ResolveExecutablePathOptions): ExecutableResolution {
  const configuredPath = options.configuredPath.trim();
  if (configuredPath.length === 0) {
    return {
      command: options.defaultBinaryName,
      configuredPath: "",
      displayPath: options.defaultBinaryName,
      isConfigured: false,
      source: "PATH"
    };
  }

  const command = path.isAbsolute(configuredPath) || !options.workspaceRoot
    ? configuredPath
    : path.join(options.workspaceRoot, configuredPath);

  return {
    command,
    configuredPath,
    displayPath: command,
    isConfigured: true,
    source: "workspace setting"
  };
}

export function executableResolutionError(
  executable: ExecutableResolution,
  env: NodeJS.ProcessEnv = process.env,
  platform = process.platform
): string | undefined {
  if (executable.isConfigured) {
    if (isExecutableFile(executable.command, platform)) {
      return undefined;
    }
    return `Configured cjtaskrunner.path is not executable: ${executable.displayPath}`;
  }

  // PATH can differ between the extension host and VS Code's integrated
  // terminal. Let the language client attempt to spawn the bare command and
  // report a startup error only if that actual launch fails.
  return undefined;
}

function isExecutableFile(candidate: string, platform: string): boolean {
  try {
    fs.accessSync(candidate, platform === "win32" ? fs.constants.F_OK : fs.constants.X_OK);
    return fs.statSync(candidate).isFile();
  } catch {
    return false;
  }
}

export function renderLanguageServerStatus(status: LanguageServerStatus): string {
  const command = `${status.executable.displayPath} --lsp`;
  return [
    "# CJTaskrunner Language Server",
    "",
    `State: ${status.clientState}`,
    `Command: ${command}`,
    `Executable source: ${status.executable.source}`,
    `Configured path: ${status.executable.configuredPath || "(not set)"}`,
    `Trace: ${status.trace}`,
    `Outline: ${status.outlineMode}`,
    `Last started: ${status.lastStartedAt ?? "Never"}`,
    `Last stopped: ${status.lastStoppedAt ?? "Never"}`,
    `Last error: ${status.lastError ?? "None"}`,
    ""
  ].join("\n");
}
