import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import { exec } from 'child_process';
import { JJJ } from './jjj';
import { TaskProvider } from './views/taskProvider';

// This function is called when your extension is activated.
export function activate(context: vscode.ExtensionContext) {
    console.log('Congratulations, your extension "jjj" is now active!');

    let jjj: JJJ | undefined;

    // Function to check for jjj initialization and set context
    const checkJjjInitialization = () => {
        const workspaceFolders = vscode.workspace.workspaceFolders;
        if (workspaceFolders) {
            const workspacePath = workspaceFolders[0].uri.fsPath;
            // This might need to be adjusted based on how jjj init works now
            // Actually, jjj init creates a bookmark, so we should check if jjj binary works or if we can detect the bookmark
            // For now, let's assume if we can run `jjj board --json` it's initialized.

            jjj = new JJJ(workspacePath);

            jjj.listTasks().then(() => {
                vscode.commands.executeCommand('setContext', 'jjj.initialized', true);
                vscode.window.registerTreeDataProvider('jjj-tasks', new TaskProvider(jjj!));
            }).catch(() => {
                vscode.commands.executeCommand('setContext', 'jjj.initialized', false);
            });
        } else {
            vscode.commands.executeCommand('setContext', 'jjj.initialized', false);
        }
    };

    // Initial check
    checkJjjInitialization();

    // Register a command to initialize the repository
    const initCommand = vscode.commands.registerCommand('jjj.init', async () => {
        const workspaceFolders = vscode.workspace.workspaceFolders;
        if (workspaceFolders) {
            const workspacePath = workspaceFolders[0].uri.fsPath;
            const tempJjj = new JJJ(workspacePath);
            try {
                await tempJjj.init();
                vscode.window.showInformationMessage('JJJ repository initialized successfully.');
                checkJjjInitialization();
            } catch (error) {
                vscode.window.showErrorMessage(`Failed to initialize JJJ repository: ${error}`);
            }
        } else {
            vscode.window.showErrorMessage('No workspace folder open to initialize JJJ.');
        }
    });

    context.subscriptions.push(initCommand);

    // Register command to open a task
    context.subscriptions.push(vscode.commands.registerCommand('jjj.openTask', (task) => {
        vscode.window.showInformationMessage(`Opening task: ${task.title}`);
    }));

    // Watch for changes in the workspace that might indicate initialization
    // This is tricky with bookmarks, maybe we just poll or rely on user action for now
}

// This function is called when your extension is deactivated
export function deactivate() { }
