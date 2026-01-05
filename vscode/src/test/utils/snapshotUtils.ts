import * as fs from 'fs';
import * as path from 'path';
import * as vscode from 'vscode';

export interface SerializedTreeItem {
  label: string;
  description?: string;
  tooltip?: string;
  contextValue?: string;
  collapsibleState: string;
  iconPath?: {
    id: string;
    color?: string;
  };
  children?: SerializedTreeItem[];
}

export function serializeTreeItem(item: vscode.TreeItem): SerializedTreeItem {
  const serialized: SerializedTreeItem = {
    label: typeof item.label === 'string' ? item.label : item.label?.label || '',
    description: typeof item.description === 'string' ? item.description : undefined,
    tooltip: typeof item.tooltip === 'string' ? item.tooltip : undefined,
    contextValue: item.contextValue,
    collapsibleState: vscode.TreeItemCollapsibleState[item.collapsibleState!] || 'None',
  };

  if (item.iconPath && typeof item.iconPath === 'object' && 'id' in item.iconPath) {
    const themeIcon = item.iconPath as vscode.ThemeIcon;
    serialized.iconPath = {
      id: themeIcon.id,
      color: themeIcon.color?.id,
    };
  }

  return serialized;
}

export async function serializeTree<T extends vscode.TreeItem>(
  provider: vscode.TreeDataProvider<T>,
  parent?: T,
): Promise<SerializedTreeItem[]> {
  const children = await provider.getChildren(parent);
  if (!children) {
    return [];
  }

  const serialized: SerializedTreeItem[] = [];

  for (const child of children) {
    const itemOrThenable = provider.getTreeItem(child);
    const item = await Promise.resolve(itemOrThenable);
    const serializedItem = serializeTreeItem(item);

    // Recursively serialize children if collapsible
    if (item.collapsibleState !== vscode.TreeItemCollapsibleState.None) {
      serializedItem.children = await serializeTree(provider, child);
    }

    serialized.push(serializedItem);
  }

  return serialized;
}

export function saveGoldenSnapshot(name: string, data: any): void {
  const goldenDir = path.join(__dirname, '../goldens');
  if (!fs.existsSync(goldenDir)) {
    fs.mkdirSync(goldenDir, { recursive: true });
  }

  const filePath = path.join(goldenDir, `${name}.json`);
  fs.writeFileSync(filePath, JSON.stringify(data, null, 2), 'utf-8');
}

export function loadGoldenSnapshot(name: string): any {
  const filePath = path.join(__dirname, '../goldens', `${name}.json`);
  if (!fs.existsSync(filePath)) {
    throw new Error(`Golden snapshot not found: ${filePath}`);
  }

  const content = fs.readFileSync(filePath, 'utf-8');
  return JSON.parse(content);
}

export function compareSnapshots(
  actual: any,
  expected: any,
): {
  matches: boolean;
  differences: string[];
} {
  const differences: string[] = [];

  function compare(path: string, actualValue: any, expectedValue: any): void {
    if (typeof actualValue !== typeof expectedValue) {
      differences.push(
        `Type mismatch at ${path}: expected ${typeof expectedValue}, got ${typeof actualValue}`,
      );
      return;
    }

    if (actualValue === null || expectedValue === null) {
      if (actualValue !== expectedValue) {
        differences.push(
          `Value mismatch at ${path}: expected ${expectedValue}, got ${actualValue}`,
        );
      }
      return;
    }

    if (typeof actualValue === 'object') {
      if (Array.isArray(actualValue) !== Array.isArray(expectedValue)) {
        differences.push(`Array mismatch at ${path}`);
        return;
      }

      if (Array.isArray(actualValue)) {
        if (actualValue.length !== expectedValue.length) {
          differences.push(
            `Array length mismatch at ${path}: expected ${expectedValue.length}, got ${actualValue.length}`,
          );
        }

        const minLength = Math.min(actualValue.length, expectedValue.length);
        for (let i = 0; i < minLength; i++) {
          compare(`${path}[${i}]`, actualValue[i], expectedValue[i]);
        }
      } else {
        const allKeys = new Set([...Object.keys(actualValue), ...Object.keys(expectedValue)]);

        for (const key of allKeys) {
          if (!(key in actualValue)) {
            differences.push(`Missing key at ${path}.${key}`);
          } else if (!(key in expectedValue)) {
            differences.push(`Extra key at ${path}.${key}`);
          } else {
            compare(`${path}.${key}`, actualValue[key], expectedValue[key]);
          }
        }
      }
    } else if (actualValue !== expectedValue) {
      differences.push(`Value mismatch at ${path}: expected ${expectedValue}, got ${actualValue}`);
    }
  }

  compare('root', actual, expected);

  return {
    matches: differences.length === 0,
    differences,
  };
}

export interface SnapshotTestOptions {
  updateGoldens?: boolean;
}

export async function snapshotTest<T extends vscode.TreeItem>(
  name: string,
  provider: vscode.TreeDataProvider<T>,
  options: SnapshotTestOptions = {},
): Promise<{ matches: boolean; differences: string[] }> {
  const actualTree = await serializeTree(provider);

  if (options.updateGoldens) {
    saveGoldenSnapshot(name, actualTree);
    return { matches: true, differences: [] };
  }

  const expectedTree = loadGoldenSnapshot(name);
  return compareSnapshots(actualTree, expectedTree);
}
