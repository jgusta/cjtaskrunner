import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  State,
  Trace
} from "vscode-languageclient/node";
import {
  ExecutableResolution,
  executableResolutionError,
  LanguageServerLifecycleState,
  renderLanguageServerStatus
} from "./languageServerSupport";
import {
  currentTraceSetting,
  resolveConfiguredExecutablePath
} from "./extensionConfiguration";

const DOCUMENT_SELECTOR = [
  { scheme: "file", language: "cjtasks" },
  { scheme: "untitled", language: "cjtasks" }
];
const STATUS_URI = vscode.Uri.parse("cjtaskrunner:/language-server-status.md");

export class CjLanguageServerManager implements vscode.Disposable {
  private readonly output = vscode.window.createOutputChannel("CJTaskrunner");
  private readonly statusProvider = new CjStatusDocumentProvider(() => this.renderStatus());
  private readonly disposables: vscode.Disposable[] = [];
  private client: LanguageClient | undefined;
  private clientStateListener: vscode.Disposable | undefined;
  private state: LanguageServerLifecycleState = "stopped";
  private lastExecutable: ExecutableResolution | undefined;
  private lastStartedAt: string | undefined;
  private lastStoppedAt: string | undefined;
  private lastError: string | undefined;

  constructor() {
    this.disposables.push(
      this.output,
      this.statusProvider,
      vscode.workspace.registerTextDocumentContentProvider("cjtaskrunner", this.statusProvider)
    );
  }

  async start(): Promise<void> {
    if (this.state === "starting" || this.state === "running") {
      return;
    }

    const executable = resolveConfiguredExecutablePath();
    this.lastExecutable = executable;
    this.lastError = undefined;
    const resolutionError = executableResolutionError(executable);
    if (resolutionError) {
      this.state = "failed";
      this.lastError = resolutionError;
      this.output.appendLine(`[${new Date().toISOString()}] Language server not started: ${resolutionError}`);
      this.refreshStatus();
      const selection = await vscode.window.showWarningMessage(
        `CJTaskrunner language server did not start. ${resolutionError}`,
        "Open Settings",
        "Open Status",
        "Show Output"
      );
      if (selection === "Open Settings") {
        await vscode.commands.executeCommand("workbench.action.openSettings", "cjtaskrunner.path");
      } else if (selection === "Open Status") {
        await this.showStatus();
      } else if (selection === "Show Output") {
        this.showOutput();
      }
      return;
    }
    this.state = "starting";
    this.refreshStatus();
    this.output.appendLine(`[${new Date().toISOString()}] Starting ${executable.displayPath} lsp`);

    const nextClient = this.createClient(executable);
    this.client = nextClient;

    try {
      await nextClient.start();
      this.state = "running";
      this.lastStartedAt = new Date().toISOString();
      this.lastStoppedAt = undefined;
      this.output.appendLine(`[${this.lastStartedAt}] Language server started`);
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      this.state = "failed";
      this.lastError = detail;
      this.output.appendLine(`[${new Date().toISOString()}] Language server failed to start: ${detail}`);
      this.clientStateListener?.dispose();
      this.clientStateListener = undefined;
      this.client = undefined;
      const selection = await vscode.window.showWarningMessage(
        `CJTaskrunner language server did not start. Outline symbols are running in extension-only mode. ${detail}`,
        "Show Output",
        "Open Status"
      );
      if (selection === "Show Output") {
        this.showOutput();
      } else if (selection === "Open Status") {
        await this.showStatus();
      }
    } finally {
      this.refreshStatus();
    }
  }

  async stop(): Promise<void> {
    if (!this.client) {
      this.state = "stopped";
      this.refreshStatus();
      return;
    }

    const stoppingClient = this.client;
    this.state = "stopping";
    this.refreshStatus();
    this.output.appendLine(`[${new Date().toISOString()}] Stopping language server`);

    try {
      await stoppingClient.stop();
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      this.lastError = detail;
      this.output.appendLine(`[${new Date().toISOString()}] Language server stop failed: ${detail}`);
    } finally {
      this.clientStateListener?.dispose();
      this.clientStateListener = undefined;
      if (this.client === stoppingClient) {
        this.client = undefined;
      }
      this.state = "stopped";
      this.lastStoppedAt = new Date().toISOString();
      this.refreshStatus();
    }
  }

  async restart(): Promise<void> {
    this.output.appendLine(`[${new Date().toISOString()}] Restart requested`);
    await this.stop();
    await this.start();
  }

  async showStatus(): Promise<void> {
    this.refreshStatus();
    const document = await vscode.workspace.openTextDocument(STATUS_URI);
    await vscode.window.showTextDocument(document, { preview: false });
  }

  showOutput(): void {
    this.output.show(true);
  }

  dispose(): void {
    this.clientStateListener?.dispose();
    this.clientStateListener = undefined;
    for (const disposable of this.disposables.reverse()) {
      disposable.dispose();
    }
  }

  private createClient(executable: ExecutableResolution): LanguageClient {
    const serverOptions: ServerOptions = {
      command: executable.command,
      args: ["lsp"]
    };
    const clientOptions: LanguageClientOptions = {
      documentSelector: DOCUMENT_SELECTOR,
      outputChannel: this.output,
      middleware: {
        provideDocumentSymbols: () => undefined
      }
    };
    const client = new LanguageClient(
      "cjtaskrunner",
      "CJTaskrunner",
      serverOptions,
      clientOptions
    );
    client.setTrace(toTrace(currentTraceSetting()));
    this.clientStateListener?.dispose();
    this.clientStateListener = client.onDidChangeState((event) => {
      this.recordClientState(event.newState);
    });
    return client;
  }

  private recordClientState(nextState: State): void {
    if (nextState === State.Running) {
      this.state = "running";
      this.lastStartedAt = new Date().toISOString();
      this.lastError = undefined;
    } else if (nextState === State.Stopped && this.state !== "failed") {
      this.state = "stopped";
      this.lastStoppedAt = new Date().toISOString();
    } else if (nextState === State.Starting) {
      this.state = "starting";
    }
    this.refreshStatus();
  }

  private renderStatus(): string {
    const executable = this.lastExecutable ?? resolveConfiguredExecutablePath();
    const outlineMode = this.state === "running"
      ? "Language server with extension fallback"
      : "Extension fallback";
    return renderLanguageServerStatus({
      clientState: this.state,
      executable,
      trace: currentTraceSetting(),
      outlineMode,
      lastStartedAt: this.lastStartedAt,
      lastStoppedAt: this.lastStoppedAt,
      lastError: this.lastError
    });
  }

  private refreshStatus(): void {
    this.statusProvider.refresh();
  }
}

function toTrace(trace: string): Trace {
  if (trace === "verbose") {
    return Trace.Verbose;
  }
  if (trace === "messages") {
    return Trace.Messages;
  }
  return Trace.Off;
}

class CjStatusDocumentProvider implements vscode.TextDocumentContentProvider, vscode.Disposable {
  private readonly changed = new vscode.EventEmitter<vscode.Uri>();
  readonly onDidChange = this.changed.event;

  constructor(private readonly render: () => string) {}

  provideTextDocumentContent(): string {
    return this.render();
  }

  refresh(): void {
    this.changed.fire(STATUS_URI);
  }

  dispose(): void {
    this.changed.dispose();
  }
}
