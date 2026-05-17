import * as fs from "fs/promises";
import * as path from "path";
import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  Trace
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;
let taskProvider: CjTaskProvider | undefined;

type TaskFileEntry = {
  uri: vscode.Uri;
  workspaceFolder?: vscode.WorkspaceFolder;
  tasks: TaskEntry[];
};

type TaskEntry = {
  name: string;
  description?: string;
};

type TreeEntry =
  | { kind: "file"; file: TaskFileEntry }
  | { kind: "task"; file: TaskFileEntry; task: TaskEntry };

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  const config = vscode.workspace.getConfiguration("cjtaskrunner");
  const serverPath = await resolveToolPath(
    context,
    config.get<string>("lsp.path", "").trim(),
    process.platform === "win32" ? "cjtaskrunner-lsp.exe" : "cjtaskrunner-lsp"
  );

  const serverOptions: ServerOptions = {
    command: serverPath,
    args: []
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: "file", language: "cjtasks" },
      { scheme: "untitled", language: "cjtasks" }
    ],
    outputChannelName: "CJTaskrunner LSP"
  };

  client = new LanguageClient(
    "cjtaskrunner",
    "CJTaskrunner",
    serverOptions,
    clientOptions
  );

  const trace = config.get<string>("lsp.trace.server", "off");
  client.setTrace(trace === "verbose" ? Trace.Verbose : trace === "messages" ? Trace.Messages : Trace.Off);

  context.subscriptions.push({
    dispose: () => {
      void client?.stop();
    }
  });

  taskProvider = new CjTaskProvider();
  const watcher = vscode.workspace.createFileSystemWatcher("**/{cjtasks,*.cjtasks}");
  context.subscriptions.push(
    vscode.window.createTreeView("cjtaskrunner.tasks", {
      treeDataProvider: taskProvider,
      showCollapseAll: true
    }),
    vscode.commands.registerCommand("cjtaskrunner.refreshTasks", () => taskProvider?.refresh()),
    vscode.commands.registerCommand("cjtaskrunner.runTask", (entry?: TreeEntry) => {
      void runTreeEntry(context, entry);
    }),
    vscode.workspace.onDidSaveTextDocument((document) => {
      if (isTaskfileUri(document.uri)) {
        taskProvider?.refresh();
      }
    }),
    watcher,
    watcher.onDidCreate(() => taskProvider?.refresh()),
    watcher.onDidDelete(() => taskProvider?.refresh())
  );

  await client.start();
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
    client = undefined;
  }
}

function resolveToolPath(
  context: vscode.ExtensionContext,
  configuredPath: string,
  binaryName: string
): Promise<string> {
  if (configuredPath.length > 0) {
    return Promise.resolve(configuredPath);
  }
  return resolveDefaultToolPath(context, binaryName);
}

async function resolveDefaultToolPath(
  context: vscode.ExtensionContext,
  binaryName: string
): Promise<string> {
  const bundled = path.join(context.extensionPath, "bin", binaryName);
  if (await pathExists(bundled)) {
    return bundled;
  }
  for (const folder of vscode.workspace.workspaceFolders ?? []) {
    const candidate = path.join(folder.uri.fsPath, "target", "debug", binaryName);
    if (await pathExists(candidate)) {
      return candidate;
    }
  }
  return binaryName;
}

async function pathExists(filePath: string): Promise<boolean> {
  try {
    await fs.access(filePath);
    return true;
  } catch {
    return false;
  }
}

class CjTaskProvider implements vscode.TreeDataProvider<TreeEntry> {
  private readonly changed = new vscode.EventEmitter<TreeEntry | undefined | null | void>();
  readonly onDidChangeTreeData = this.changed.event;
  private files: TaskFileEntry[] = [];

  refresh(): void {
    this.files = [];
    this.changed.fire();
  }

  getTreeItem(entry: TreeEntry): vscode.TreeItem {
    if (entry.kind === "file") {
      const item = new vscode.TreeItem(fileLabel(entry.file), vscode.TreeItemCollapsibleState.Expanded);
      item.resourceUri = entry.file.uri;
      item.description = entry.file.tasks.length === 1 ? "1 task" : `${entry.file.tasks.length} tasks`;
      item.contextValue = "cjtaskrunner.taskfile";
      item.iconPath = new vscode.ThemeIcon("file-code");
      return item;
    }

    const item = new vscode.TreeItem(entry.task.name, vscode.TreeItemCollapsibleState.None);
    item.description = entry.task.description ?? fileLabel(entry.file);
    item.contextValue = "cjtaskrunner.task";
    item.iconPath = new vscode.ThemeIcon("play");
    item.command = {
      command: "cjtaskrunner.runTask",
      title: "Run Task",
      arguments: [entry]
    };
    return item;
  }

  async getChildren(entry?: TreeEntry): Promise<TreeEntry[]> {
    if (entry?.kind === "file") {
      return entry.file.tasks.map((task) => ({ kind: "task", file: entry.file, task }));
    }
    if (entry?.kind === "task") {
      return [];
    }
    if (this.files.length === 0) {
      this.files = await discoverTaskfiles();
    }
    return this.files.map((file) => ({ kind: "file", file }));
  }
}

async function discoverTaskfiles(): Promise<TaskFileEntry[]> {
  const [plain, extension] = await Promise.all([
    vscode.workspace.findFiles("**/cjtasks", "**/{node_modules,.git}/**"),
    vscode.workspace.findFiles("**/*.cjtasks", "**/{node_modules,.git}/**")
  ]);
  const uris = [...plain, ...extension];
  const byPath = new Map<string, vscode.Uri>();
  for (const uri of uris) {
    if (isTaskfileUri(uri)) {
      byPath.set(uri.fsPath, uri);
    }
  }

  const files: TaskFileEntry[] = [];
  for (const uri of Array.from(byPath.values()).sort((left, right) => left.fsPath.localeCompare(right.fsPath))) {
    const tasks = await readTasks(uri);
    if (tasks.length > 0) {
      files.push({
        uri,
        workspaceFolder: vscode.workspace.getWorkspaceFolder(uri),
        tasks
      });
    }
  }
  return files;
}

async function readTasks(uri: vscode.Uri): Promise<TaskEntry[]> {
  try {
    const source = await fs.readFile(uri.fsPath, "utf8");
    const tasks: TaskEntry[] = [];
    let currentTask: TaskEntry | undefined;
    for (const line of source.split(/\r?\n/)) {
      if (line.length === 0 || line.startsWith(" ") || line.trimStart().startsWith("#")) {
        if (currentTask && line.startsWith("  @desc")) {
          const description = line.slice("  @desc".length).trim();
          currentTask.description = description;
        }
        continue;
      }
      if (!line.endsWith(":")) {
        currentTask = undefined;
        continue;
      }
      const name = line.slice(0, -1);
      if (name !== "env" && name !== "help" && /^[A-Za-z0-9_.-]+(?::[A-Za-z0-9_.-]+)?$/.test(name)) {
        currentTask = { name };
        tasks.push(currentTask);
      } else {
        currentTask = undefined;
      }
    }
    return tasks;
  } catch {
    return [];
  }
}

async function runTreeEntry(context: vscode.ExtensionContext, entry?: TreeEntry): Promise<void> {
  if (!entry) {
    entry = await pickTask();
    if (!entry) {
      return;
    }
  }
  if (entry.kind !== "task") {
    return;
  }

  const config = vscode.workspace.getConfiguration("cjtaskrunner");
  const runnerPath = await resolveToolPath(
    context,
    config.get<string>("executable.path", "").trim(),
    process.platform === "win32" ? "cjtaskrunner.exe" : "cjtaskrunner"
  );
  const terminal = vscode.window.createTerminal({
    name: `CJTaskrunner: ${entry.task.name}`,
    cwd: entry.file.workspaceFolder?.uri.fsPath ?? path.dirname(entry.file.uri.fsPath)
  });
  terminal.show();
  terminal.sendText(`${shellQuote(runnerPath)} ${shellQuote(entry.file.uri.fsPath)} ${shellQuote(entry.task.name)}`);
}

async function pickTask(): Promise<TreeEntry | undefined> {
  const files = await discoverTaskfiles();
  const picks = files.flatMap((file) =>
    file.tasks.map((task) => ({
      label: task.name,
      description: task.description ?? fileLabel(file),
      entry: { kind: "task" as const, file, task }
    }))
  );
  const picked = await vscode.window.showQuickPick(picks, {
    placeHolder: "Run CJTaskrunner task"
  });
  return picked?.entry;
}

function fileLabel(file: TaskFileEntry): string {
  if (file.workspaceFolder) {
    return path.relative(file.workspaceFolder.uri.fsPath, file.uri.fsPath) || path.basename(file.uri.fsPath);
  }
  return path.basename(file.uri.fsPath);
}

function isTaskfileUri(uri: vscode.Uri): boolean {
  const base = path.basename(uri.fsPath);
  return base === "cjtasks" || base.endsWith(".cjtasks");
}

function shellQuote(value: string): string {
  if (process.platform === "win32") {
    return `"${value.replace(/"/g, '\\"')}"`;
  }
  return `'${value.replace(/'/g, "'\\''")}'`;
}
