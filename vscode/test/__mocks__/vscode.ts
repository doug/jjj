// Minimal VS Code API mock for unit tests

export class EventEmitter<T> {
  private listeners: Array<(e: T) => void> = [];
  event = (listener: (e: T) => void) => {
    this.listeners.push(listener);
    return { dispose: () => { this.listeners = this.listeners.filter(l => l !== listener); } };
  };
  fire(data: T): void {
    this.listeners.forEach(l => l(data));
  }
  dispose(): void {
    this.listeners = [];
  }
}

export enum TreeItemCollapsibleState {
  None = 0,
  Collapsed = 1,
  Expanded = 2,
}

export class TreeItem {
  label?: string;
  description?: string;
  tooltip?: string;
  iconPath?: unknown;
  command?: unknown;
  contextValue?: string;
  collapsibleState?: TreeItemCollapsibleState;
  constructor(label: string, collapsibleState?: TreeItemCollapsibleState) {
    this.label = label;
    this.collapsibleState = collapsibleState;
  }
}

export class ThemeIcon {
  constructor(public id: string, public color?: ThemeColor) {}
}

export class ThemeColor {
  constructor(public id: string) {}
}

export enum StatusBarAlignment {
  Left = 1,
  Right = 2,
}

export enum OverviewRulerLane {
  Left = 1,
  Center = 2,
  Right = 4,
  Full = 7,
}

export class Uri {
  scheme: string;
  path: string;
  constructor(scheme: string, path: string) {
    this.scheme = scheme;
    this.path = path;
  }
  static parse(value: string): Uri {
    const match = value.match(/^([^:]+):\/\/(.*)/);
    if (match) {
      return new Uri(match[1], match[2]);
    }
    return new Uri("", value);
  }
}

export class Range {
  constructor(
    public startLine: number,
    public startChar: number,
    public endLine: number,
    public endChar: number,
  ) {}
}

export class MarkdownString {
  constructor(public value: string) {}
}

export class DataTransferItem {
  constructor(public value: unknown) {}
}

export class DataTransfer {
  private items = new Map<string, DataTransferItem>();
  get(mimeType: string) { return this.items.get(mimeType); }
  set(mimeType: string, item: DataTransferItem) { this.items.set(mimeType, item); }
}

export const workspace = {
  getConfiguration: (_section?: string) => ({
    get: <T>(key: string, defaultValue?: T): T | undefined => defaultValue,
  }),
  workspaceFolders: [{ uri: { fsPath: "/mock/workspace" } }],
  textDocuments: [] as unknown[],
  registerTextDocumentContentProvider: () => ({ dispose: () => {} }),
  onDidSaveTextDocument: () => ({ dispose: () => {} }),
  asRelativePath: (uri: unknown) => String(uri),
};

export const window = {
  createStatusBarItem: () => ({
    text: "",
    command: "",
    color: undefined,
    backgroundColor: undefined,
    show: () => {},
    dispose: () => {},
  }),
  createTreeView: () => ({ dispose: () => {} }),
  registerTreeDataProvider: () => {},
  onDidChangeActiveTextEditor: () => ({ dispose: () => {} }),
  activeTextEditor: undefined,
  showInputBox: async () => undefined,
  showQuickPick: async () => undefined,
  showInformationMessage: async () => undefined,
  showErrorMessage: async () => undefined,
  createTextEditorDecorationType: () => ({ dispose: () => {} }),
};

export const commands = {
  registerCommand: (_id: string, _handler: (...args: unknown[]) => unknown) => ({ dispose: () => {} }),
};
