export type TaskEntry = {
  name: string;
  line: number;
  description?: string;
  selfHelp?: boolean;
};

export type TaskSymbolEntry = {
  name: string;
  description?: string;
  startLine: number;
  startCharacter: number;
  endLine: number;
  endCharacter: number;
  selectionStartCharacter: number;
  selectionEndCharacter: number;
  children: TaskSymbolEntry[];
};

export type TaskOutline = {
  tasks: TaskEntry[];
  symbols: TaskSymbolEntry[];
};

type TaskContext = {
  name: string;
  headerIndent: number;
  task: TaskEntry;
  symbol: TaskSymbolEntry;
};

type LeadingIndent = {
  width: number;
  characters: number;
};

export function parseTaskOutline(source: string): TaskOutline {
  const tasks: TaskEntry[] = [];
  const symbols: TaskSymbolEntry[] = [];
  const contexts: TaskContext[] = [];
  const lines = source.split(/\r?\n/);

  for (let lineNumber = 0; lineNumber < lines.length; lineNumber += 1) {
    const line = lines[lineNumber];
    const trimmed = line.trim();
    if (trimmed.length === 0 || trimmed.startsWith("#")) {
      continue;
    }

    const indent = leadingIndent(line);
    closeFinishedContexts(contexts, indent.width, lineNumber);

    if (indent.width === 0) {
      closeAllContexts(contexts, lineNumber);
      const name = taskLabel(line);
      if (name) {
        const context = createTaskContext(name, lineNumber, 0, name.length, 0);
        if (isPanelVisibleTask(name)) {
          tasks.push(context.task);
        }
        symbols.push(context.symbol);
        contexts.push(context);
      }
      continue;
    }

    const parent = contexts[contexts.length - 1];
    if (!parent) {
      continue;
    }

    const logicalIndent = indent.width - parent.headerIndent;
    const text = line.slice(indent.characters);
    if (logicalIndent !== 2) {
      continue;
    }

    const childName = taskLabel(text);
    if (childName) {
      const name = `${parent.name}:${childName}`;
      const context = createTaskContext(
        name,
        lineNumber,
        indent.characters,
        indent.characters + childName.length,
        indent.width
      );
      if (isPanelVisibleTask(name)) {
        tasks.push(context.task);
      }
      parent.symbol.children.push(context.symbol);
      contexts.push(context);
      continue;
    }

    const description = taskDescription(text);
    if (description !== undefined) {
      parent.task.description = description;
      parent.symbol.description = description;
    } else if (isSelfHelpDirective(text)) {
      parent.task.selfHelp = true;
    }
  }

  closeAllContexts(contexts, lines.length);
  return { tasks, symbols };
}

function createTaskContext(
  name: string,
  lineNumber: number,
  selectionStartCharacter: number,
  selectionEndCharacter: number,
  headerIndent: number
): TaskContext {
  const task: TaskEntry = { name, line: lineNumber };
  const symbol: TaskSymbolEntry = {
    name,
    startLine: lineNumber,
    startCharacter: selectionStartCharacter,
    endLine: lineNumber,
    endCharacter: selectionEndCharacter,
    selectionStartCharacter,
    selectionEndCharacter,
    children: []
  };

  return { name, headerIndent, task, symbol };
}

function closeFinishedContexts(
  contexts: TaskContext[],
  indent: number,
  lineNumber: number
): void {
  while (
    contexts.length > 1 &&
    indent <= contexts[contexts.length - 1].headerIndent
  ) {
    const context = contexts.pop();
    if (context) {
      closeSymbol(context.symbol, lineNumber);
    }
  }
}

function closeAllContexts(contexts: TaskContext[], lineNumber: number): void {
  while (contexts.length > 0) {
    const context = contexts.pop();
    if (context) {
      closeSymbol(context.symbol, lineNumber);
    }
  }
}

function closeSymbol(symbol: TaskSymbolEntry, lineNumber: number): void {
  symbol.endLine = lineNumber;
  symbol.endCharacter = 0;
}

function taskDescription(text: string): string | undefined {
  if (text === "@desc") {
    return "";
  }
  if (!text.startsWith("@desc ")) {
    return undefined;
  }
  return text.slice("@desc".length).trim();
}

function isSelfHelpDirective(text: string): boolean {
  return /^@selfhelp(?:\s|;|$)/.test(text);
}

function taskLabel(text: string): string | undefined {
  if (text.startsWith("@")) {
    return undefined;
  }
  const match = text.match(
    /^([A-Za-z0-9_-]+(?::[A-Za-z0-9_-]+)*)(?:[ \t]+\([A-Za-z_][A-Za-z0-9_]*(?:\s*,\s*[A-Za-z_][A-Za-z0-9_]*)*\))?:$/
  );
  return match?.[1];
}

function isPanelVisibleTask(name: string): boolean {
  return name.split(":").every((part) => !part.startsWith("_"));
}

function leadingIndent(value: string): LeadingIndent {
  let width = 0;
  let characters = 0;
  for (const character of value) {
    if (character === " ") {
      width += 1;
    } else if (character === "\t") {
      width += 2;
    } else {
      break;
    }
    characters += 1;
  }
  return { width, characters };
}
