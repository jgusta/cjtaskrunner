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

  if (findOnPath(executable.command, env.PATH ?? "", platform)) {
    return undefined;
  }

  return `Could not find ${executable.displayPath} on PATH. Install CJTaskrunner or set cjtaskrunner.path to the cj executable.`;
}

function findOnPath(command: string, pathValue: string, platform: string): boolean {
  if (command.includes(path.sep) || (path.sep === "\\" && command.includes("/"))) {
    return isExecutableFile(command, platform);
  }

  const extensions = executableExtensions(command, platform);
  for (const directory of pathValue.split(path.delimiter)) {
    if (!directory) {
      continue;
    }
    for (const extension of extensions) {
      if (isExecutableFile(path.join(directory, `${command}${extension}`), platform)) {
        return true;
      }
    }
  }
  return false;
}

function executableExtensions(command: string, platform: string): string[] {
  if (platform !== "win32" || path.extname(command)) {
    return [""];
  }
  return [".exe", ".cmd", ".bat", ""];
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
  const command = `${status.executable.displayPath} lsp`;
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
