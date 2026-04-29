import * as vscode from "vscode";
import { parseTaskOutline, type TaskSymbolEntry } from "./taskOutline";

const DOCUMENT_SELECTOR = [
  { scheme: "file", language: "cjtasks" },
  { scheme: "untitled", language: "cjtasks" }
];

export function registerDocumentSymbols(): vscode.Disposable {
  return vscode.languages.registerDocumentSymbolProvider(
    DOCUMENT_SELECTOR,
    new CjDocumentSymbolProvider()
  );
}

class CjDocumentSymbolProvider implements vscode.DocumentSymbolProvider {
  provideDocumentSymbols(document: vscode.TextDocument): vscode.DocumentSymbol[] {
    return parseTaskOutline(document.getText()).symbols.map(toDocumentSymbol);
  }
}

function toDocumentSymbol(symbol: TaskSymbolEntry): vscode.DocumentSymbol {
  const documentSymbol = new vscode.DocumentSymbol(
    symbol.name,
    symbol.description ?? "task",
    vscode.SymbolKind.Function,
    new vscode.Range(symbol.startLine, symbol.startCharacter, symbol.endLine, symbol.endCharacter),
    new vscode.Range(
      symbol.startLine,
      symbol.selectionStartCharacter,
      symbol.startLine,
      symbol.selectionEndCharacter
    )
  );
  documentSymbol.children = symbol.children.map(toDocumentSymbol);
  return documentSymbol;
}
