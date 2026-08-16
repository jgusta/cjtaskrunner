import * as fs from "fs/promises";
import * as path from "path";
import * as vscode from "vscode";
import {
  lspConfigurationChanged,
  resolveConfiguredExecutablePath
} from "./extensionConfiguration";
import { registerDocumentSymbols } from "./documentSymbols";
import { parseTaskOutline, type TaskEntry } from "./taskOutline";
import {
  isRecognizedTaskfileName,
  rootTaskfileCandidates,
  selectPreferredTaskfilePaths,
  TASKFILE_NAMES,
  taskfileLayerPaths
} from "./taskfileDiscovery";

type CjLanguageServerManager = import("./languageServer.js").CjLanguageServerManager;

let languageServer: LazyLanguageServer | undefined;
let taskProvider: CjTaskProvider | undefined;
let taskTerminal: vscode.Terminal | undefined;
const HAS_ROOT_TASKFILE_CONTEXT = "cjtaskrunner.hasRootTaskfile";

type TaskFileEntry = {
  uri: vscode.Uri;
  workspaceFolder?: vscode.WorkspaceFolder;
  tasks: TaskDefinition[];
  layers: TaskLayerEntry[];
};

type TaskDefinition = TaskEntry & {
  uri: vscode.Uri;
  overridden: boolean;
};

type TaskLayerEntry = {
  uri: vscode.Uri;
  tasks: TaskDefinition[];
};

type TreeEntry =
  | { kind: "file"; file: TaskFileEntry }
  | { kind: "layer"; file: TaskFileEntry; layer: TaskLayerEntry }
  | { kind: "task"; file: TaskFileEntry; tasks: TaskDefinition[]; task: TaskDefinition };

export function activate(context: vscode.ExtensionContext): void {
  const server = new LazyLanguageServer();
  languageServer = server;
  context.subscriptions.push(server);

  taskProvider = new CjTaskProvider(context.extensionUri);
  const watcher = vscode.workspace.createFileSystemWatcher(
    "**/{cjtasks,production.cjtasks,staging.cjtasks,development.cjtasks,local.cjtasks}"
  );
  const refreshWorkspaceState = (): void => {
    void updateWorkspaceState();
  };
  context.subscriptions.push(
    vscode.window.createTreeView("cjtaskrunner.tasks", {
      treeDataProvider: taskProvider,
      showCollapseAll: true
    }),
    vscode.window.onDidCloseTerminal((terminal) => {
      if (terminal === taskTerminal) {
        taskTerminal = undefined;
      }
    }),
    vscode.commands.registerCommand("cjtaskrunner.refreshTasks", () => taskProvider?.refresh()),
    vscode.commands.registerCommand("cjtaskrunner.runTask", (entry?: TreeEntry) => {
      void runTreeEntry(context, entry);
    }),
    vscode.commands.registerCommand("cjtaskrunner.openTask", (entry: TreeEntry) => {
      void openTreeEntry(entry);
    }),
    vscode.commands.registerCommand("cjtaskrunner.restartLanguageServer", () => {
      languageServer?.restart();
    }),
    vscode.commands.registerCommand("cjtaskrunner.languageServerStatus", () => {
      languageServer?.showStatus();
    }),
    vscode.commands.registerCommand("cjtaskrunner.showLanguageServerOutput", () => {
      languageServer?.showOutput();
    }),
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (lspConfigurationChanged(event)) {
        languageServer?.restart();
      }
      if (
        event.affectsConfiguration("cjtaskrunner.showTaskfileCascade") ||
        event.affectsConfiguration("cjtaskrunner.showOverriddenTasksInCascade")
      ) {
        taskProvider?.refresh();
      }
    }),
    vscode.workspace.onDidSaveTextDocument((document) => {
      if (isTaskfileUri(document.uri)) {
        taskProvider?.refresh();
      }
    }),
    watcher,
    watcher.onDidCreate(refreshWorkspaceState),
    watcher.onDidDelete(refreshWorkspaceState),
    vscode.workspace.onDidChangeWorkspaceFolders(refreshWorkspaceState),
    registerDocumentSymbols()
  );

  refreshWorkspaceState();
  const startTimer = setTimeout(() => server.start(), 0);
  context.subscriptions.push({
    dispose: () => clearTimeout(startTimer)
  });
}

export async function deactivate(): Promise<void> {
  const server = languageServer;
  languageServer = undefined;
  if (server) {
    await server.stop();
  }
}

class LazyLanguageServer implements vscode.Disposable {
  private manager: CjLanguageServerManager | undefined;
  private loading: Promise<CjLanguageServerManager> | undefined;
  private disposed = false;

  start(): void {
    this.run((manager) => manager.start());
  }

  restart(): void {
    this.run((manager) => manager.restart());
  }

  showStatus(): void {
    this.run((manager) => manager.showStatus());
  }

  showOutput(): void {
    this.run((manager) => manager.showOutput());
  }

  async stop(): Promise<void> {
    if (this.manager) {
      await this.manager.stop();
      return;
    }
    if (this.loading) {
      const manager = await this.loading;
      await manager.stop();
    }
  }

  dispose(): void {
    this.disposed = true;
    this.manager?.dispose();
    this.manager = undefined;
  }

  private run(action: (manager: CjLanguageServerManager) => void | Promise<void>): void {
    void this.load()
      .then(action)
      .catch((error) => {
        if (!this.disposed) {
          const detail = error instanceof Error ? error.message : String(error);
          void vscode.window.showErrorMessage(`CJTaskrunner language support failed to load: ${detail}`);
        }
      });
  }

  private load(): Promise<CjLanguageServerManager> {
    if (this.manager) {
      return Promise.resolve(this.manager);
    }
    if (!this.loading) {
      this.loading = import("./languageServer.js")
        .then(({ CjLanguageServerManager }) => {
          const manager = new CjLanguageServerManager();
          if (this.disposed) {
            manager.dispose();
            throw new Error("Extension was disposed while loading language support");
          }
          this.manager = manager;
          return manager;
        })
        .catch((error) => {
          this.loading = undefined;
          throw error;
        });
    }
    return this.loading;
  }
}

class CjTaskProvider implements vscode.TreeDataProvider<TreeEntry> {
  private readonly changed = new vscode.EventEmitter<TreeEntry | undefined | null | void>();
  readonly onDidChangeTreeData = this.changed.event;
  private files: TaskFileEntry[] | undefined;
  private loading: Promise<TaskFileEntry[]> | undefined;

  constructor(private readonly extensionUri: vscode.Uri) { }

  refresh(): void {
    this.files = undefined;
    this.changed.fire();
  }

  private async getFiles(): Promise<TaskFileEntry[]> {
    if (this.files) {
      return this.files;
    }

    if (!this.loading) {
      this.loading = discoverTaskfiles()
        .then((files) => {
          this.files = files;
          return files;
        })
        .finally(() => {
          this.loading = undefined;
        });
    }

    return this.loading;
  }

  getTreeItem(entry: TreeEntry): vscode.TreeItem {
    if (entry.kind === "file") {
      const item = new vscode.TreeItem(fileLabel(entry.file), vscode.TreeItemCollapsibleState.Expanded);
      item.resourceUri = entry.file.uri;
      item.description = entry.file.tasks.length === 1 ? "1 task" : `${entry.file.tasks.length} tasks`;
      item.contextValue = "cjtaskrunner.taskfile";
      item.iconPath = this.taskfileIcon();
      return item;
    }

    if (entry.kind === "layer") {
      const visibleTasks = layerTasks(entry.file, entry.layer);
      const item = new vscode.TreeItem(
        path.basename(entry.layer.uri.fsPath),
        vscode.TreeItemCollapsibleState.Expanded
      );
      item.resourceUri = entry.layer.uri;
      item.description = visibleTasks.length === 1 ? "1 task" : `${visibleTasks.length} tasks`;
      item.contextValue = "cjtaskrunner.taskfileLayer";
      item.iconPath = this.taskfileIcon();
      return item;
    }

    const item = new vscode.TreeItem(
      entry.task.name,
      hasChildTasks(entry.tasks, entry.task.name)
        ? vscode.TreeItemCollapsibleState.Collapsed
        : vscode.TreeItemCollapsibleState.None
    );
    const runnable = isRunnableTask(entry.task);
    item.description = entry.task.overridden
      ? "overridden"
      : entry.task.description ?? path.basename(entry.task.uri.fsPath);
    item.contextValue = runnable ? "cjtaskrunner.task" : "cjtaskrunner.nonRunnableTask";
    item.iconPath = taskIconPath(entry.task, this.extensionUri);
    if (entry.task.selfHelp && !entry.task.overridden) {
      item.tooltip = "Help-only task. Open it or run `cj help <task>`.";
    }
    if (!entry.task.overridden) {
      item.command = {
        command: "cjtaskrunner.openTask",
        title: "Open Task Definition",
        arguments: [entry]
      };
    }
    return item;
  }

  private taskfileIcon(): { light: vscode.Uri; dark: vscode.Uri } {
    return {
      light: vscode.Uri.joinPath(this.extensionUri, "images", "cjdocicon-light.svg"),
      dark: vscode.Uri.joinPath(this.extensionUri, "images", "cjdocicon-dark.svg")
    };
  }

  async getChildren(entry?: TreeEntry): Promise<TreeEntry[]> {
    if (entry?.kind === "file") {
      if (!showTaskfileCascade(entry.file.uri)) {
        return rootTaskEntries(entry.file, entry.file.tasks);
      }
      const [top, ...inherited] = entry.file.layers;
      const children: TreeEntry[] = top
        ? rootTaskEntries(entry.file, layerTasks(entry.file, top))
        : [];
      for (const layer of inherited) {
        if (layerTasks(entry.file, layer).length > 0) {
          children.push({ kind: "layer", file: entry.file, layer });
        }
      }
      return children;
    }
    if (entry?.kind === "layer") {
      return rootTaskEntries(entry.file, layerTasks(entry.file, entry.layer));
    }
    if (entry?.kind === "task") {
      return childTaskEntries(entry.file, entry.tasks, entry.task.name);
    }

    const files = await this.getFiles();
    return files.map((file) => ({ kind: "file", file }));
  }
}

async function discoverTaskfiles(): Promise<TaskFileEntry[]> {
  const workspaceFolders = await workspaceFoldersWithRootTaskfiles();
  const matches = await Promise.all(
    workspaceFolders.flatMap((folder) =>
      TASKFILE_NAMES.map((name) =>
        vscode.workspace.findFiles(
          new vscode.RelativePattern(folder, `**/${name}`),
          "**/{node_modules,.git}/**"
        )
      )
    )
  );
  const uris = matches.flat();
  const byPath = new Map<string, vscode.Uri>();
  for (const uri of uris) {
    byPath.set(uri.fsPath, uri);
  }

  const files: TaskFileEntry[] = [];
  for (const taskfilePath of selectPreferredTaskfilePaths(Array.from(byPath.keys()))) {
    const uri = byPath.get(taskfilePath);
    if (!uri) {
      continue;
    }
    const layers: TaskLayerEntry[] = [];
    for (const layerPath of taskfileLayerPaths(Array.from(byPath.keys()), path.dirname(taskfilePath))) {
      const layerUri = byPath.get(layerPath);
      if (!layerUri) {
        continue;
      }
      layers.push({
        uri: layerUri,
        tasks: (await readTasks(layerUri)).map((task) => ({
          ...task,
          uri: layerUri,
          overridden: false
        }))
      });
    }
    const winners = new Map<string, TaskDefinition>();
    for (const layer of layers) {
      for (const task of layer.tasks) {
        winners.set(task.name, task);
      }
    }
    for (const layer of layers) {
      for (const task of layer.tasks) {
        task.overridden = winners.get(task.name) !== task;
      }
    }
    const tasks = Array.from(winners.values());
    layers.reverse();
    if (tasks.length > 0) {
      files.push({
        uri,
        workspaceFolder: vscode.workspace.getWorkspaceFolder(uri),
        tasks,
        layers
      });
    }
  }
  return files;
}

async function updateWorkspaceState(): Promise<void> {
  const workspaceFolders = await workspaceFoldersWithRootTaskfiles();
  await vscode.commands.executeCommand(
    "setContext",
    HAS_ROOT_TASKFILE_CONTEXT,
    workspaceFolders.length > 0
  );
  taskProvider?.refresh();
}

async function workspaceFoldersWithRootTaskfiles(): Promise<vscode.WorkspaceFolder[]> {
  const workspaceFolders = vscode.workspace.workspaceFolders ?? [];
  const matches = await Promise.all(
    workspaceFolders.map(async (folder) => {
      for (const candidate of rootTaskfileCandidates(folder.uri.fsPath)) {
        try {
          if ((await fs.stat(candidate)).isFile()) {
            return folder;
          }
        } catch {
          // Check the lower-precedence name when this candidate is absent.
        }
      }
      return undefined;
    })
  );
  return matches.filter((folder): folder is vscode.WorkspaceFolder => folder !== undefined);
}

async function readTasks(uri: vscode.Uri): Promise<TaskEntry[]> {
  try {
    const source = await fs.readFile(uri.fsPath, "utf8");
    return parseTaskOutline(source).tasks;
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
  if (entry.kind !== "task" || entry.task.overridden) {
    return;
  }
  if (!isRunnableTask(entry.task)) {
    return;
  }

  const runnerPath = resolveConfiguredExecutablePath(entry.file.uri).command;
  if (!taskTerminal) {
    taskTerminal = vscode.window.createTerminal({
      name: "CJTaskrunner",
      cwd: entry.file.workspaceFolder?.uri.fsPath ?? path.dirname(entry.file.uri.fsPath)
    });
    context.subscriptions.push(taskTerminal);
  }
  taskTerminal.show();
  taskTerminal.sendText(`${shellQuote(runnerPath)} ${shellQuote(entry.file.uri.fsPath)} ${shellQuote(entry.task.name)}`);
}

async function openTreeEntry(entry: TreeEntry): Promise<void> {
  if (entry.kind !== "task" || entry.task.overridden) {
    return;
  }
  const document = await vscode.workspace.openTextDocument(entry.task.uri);
  const editor = await vscode.window.showTextDocument(document);
  const position = new vscode.Position(entry.task.line, 0);
  editor.selection = new vscode.Selection(position, position);
  editor.revealRange(new vscode.Range(position, position), vscode.TextEditorRevealType.InCenterIfOutsideViewport);
}

async function pickTask(): Promise<TreeEntry | undefined> {
  const files = await discoverTaskfiles();
  const picks = files.flatMap((file) =>
    file.tasks
      .filter((task) => isRunnableTask(task))
      .map((task) => ({
        label: task.name,
        description: task.description ?? fileLabel(file),
        entry: { kind: "task" as const, file, tasks: file.tasks, task }
      }))
  );
  const picked = await vscode.window.showQuickPick(picks, {
    placeHolder: "Run CJTaskrunner task"
  });
  return picked?.entry;
}

function taskEntries(file: TaskFileEntry, tasks: TaskDefinition[]): TreeEntry[] {
  return tasks.map((task) => ({ kind: "task", file, tasks, task }));
}

function taskIconPath(
  task: TaskDefinition,
  extensionUri: vscode.Uri
): vscode.ThemeIcon | { light: vscode.Uri; dark: vscode.Uri } {
  if (task.overridden) {
    return new vscode.ThemeIcon("circle-slash", new vscode.ThemeColor("disabledForeground"));
  }
  if (task.selfHelp) {
    return new vscode.ThemeIcon("book", new vscode.ThemeColor("descriptionForeground"));
  }
  return {
    light: vscode.Uri.joinPath(extensionUri, "images", "cdjtaskicon-light.svg"),
    dark: vscode.Uri.joinPath(extensionUri, "images", "cdjtaskicon-dark.svg")
  };
}

function isRunnableTask(task: TaskDefinition): boolean {
  return !task.overridden && !task.selfHelp;
}

function rootTaskEntries(file: TaskFileEntry, tasks: TaskDefinition[]): TreeEntry[] {
  const taskNames = new Set(tasks.map((task) => task.name));
  return taskEntries(
    file,
    tasks.filter((task) => {
      const parent = parentTaskName(task.name);
      return parent === undefined || !taskNames.has(parent);
    })
  );
}

function childTaskEntries(
  file: TaskFileEntry,
  tasks: TaskDefinition[],
  parentName: string
): TreeEntry[] {
  return taskEntries(
    file,
    tasks.filter((task) => parentTaskName(task.name) === parentName)
  );
}

function hasChildTasks(tasks: TaskDefinition[], parentName: string): boolean {
  return tasks.some((task) => parentTaskName(task.name) === parentName);
}

function parentTaskName(name: string): string | undefined {
  const separator = name.lastIndexOf(":");
  return separator === -1 ? undefined : name.slice(0, separator);
}

function layerTasks(file: TaskFileEntry, layer: TaskLayerEntry): TaskDefinition[] {
  return layer.tasks.filter(
    (task) => !task.overridden || showOverriddenTasks(file.uri)
  );
}

function showTaskfileCascade(uri: vscode.Uri): boolean {
  return vscode.workspace
    .getConfiguration("cjtaskrunner", uri)
    .get<boolean>("showTaskfileCascade", false);
}

function showOverriddenTasks(uri: vscode.Uri): boolean {
  return showTaskfileCascade(uri) && vscode.workspace
    .getConfiguration("cjtaskrunner", uri)
    .get<boolean>("showOverriddenTasksInCascade", false);
}

function fileLabel(file: TaskFileEntry): string {
  if (file.workspaceFolder) {
    return path.relative(file.workspaceFolder.uri.fsPath, file.uri.fsPath) || path.basename(file.uri.fsPath);
  }
  return path.basename(file.uri.fsPath);
}

function isTaskfileUri(uri: vscode.Uri): boolean {
  return isRecognizedTaskfileName(path.basename(uri.fsPath));
}

function shellQuote(value: string): string {
  if (process.platform === "win32") {
    return `"${value.replace(/"/g, '\\"')}"`;
  }
  return `'${value.replace(/'/g, "'\\''")}'`;
}
