import * as vscode from "vscode";
import {
  ExecutableResolution,
  resolveExecutablePath
} from "./languageServerSupport";

export function resolveConfiguredExecutablePath(resource?: vscode.Uri): ExecutableResolution {
  const config = vscode.workspace.getConfiguration("cjtaskrunner", resource);
  return resolveExecutablePath({
    configuredPath: config.get<string>("path", "").trim(),
    defaultBinaryName: process.platform === "win32" ? "cj.exe" : "cj",
    workspaceRoot: workspaceRootFor(resource)
  });
}

export function lspConfigurationChanged(event: vscode.ConfigurationChangeEvent): boolean {
  return event.affectsConfiguration("cjtaskrunner.path")
    || event.affectsConfiguration("cjtaskrunner.lsp.trace.server");
}

export function currentTraceSetting(): string {
  return vscode.workspace
    .getConfiguration("cjtaskrunner")
    .get<string>("lsp.trace.server", "off");
}

function workspaceRootFor(resource?: vscode.Uri): string | undefined {
  const folder = resource
    ? vscode.workspace.getWorkspaceFolder(resource)
    : vscode.workspace.workspaceFolders?.[0];
  return folder?.uri.fsPath;
}
