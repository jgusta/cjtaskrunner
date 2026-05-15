import * as path from "path";
import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  Trace
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  const config = vscode.workspace.getConfiguration("cjtaskrunner");
  const configuredPath = config.get<string>("lsp.path", "").trim();
  const defaultPath = path.resolve(
    context.extensionPath,
    "..",
    "..",
    "target",
    "debug",
    process.platform === "win32" ? "cjtaskrunner-lsp.exe" : "cjtaskrunner-lsp"
  );
  const serverPath = configuredPath || defaultPath;

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

  context.subscriptions.push(client.start());
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
    client = undefined;
  }
}
