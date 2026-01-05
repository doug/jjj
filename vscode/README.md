# JJJ VS Code Extension

This VS Code extension provides a rich user interface within Visual Studio Code for task management and code review, leveraging the powerful `jjj` command-line tool. It aims to streamline your development workflow by integrating `jjj`'s capabilities directly into your IDE.

## Features

- **Task Management UI**: View, create, update, and manage your development tasks directly within VS Code.
- **Code Review Workflow**: Facilitate code review processes with dedicated UI elements for submitting, reviewing, and tracking code changes.
- **`jjj` Integration**: Seamlessly execute `jjj` commands from the VS Code interface, providing a graphical front-end to `jjj`'s robust backend.
- **Real-time Updates**: Stay informed with real-time feedback and status updates from your `jjj` projects.

## Requirements

- Visual Studio Code (version X.Y.Z or later)
- The `jjj` command-line tool installed and configured on your system.

## Installation

1.  **Install `jjj` CLI**: Ensure you have the `jjj` command-line tool installed and properly configured. Refer to the main `jjj` project documentation for installation instructions.
2.  **Clone the Repository**:
    ```bash
    git clone https://github.com/your-repo/jjj.git
    cd jjj/vscode
    ```
3.  **Open in VS Code**:
    ```bash
    code .
    ```
4.  **Install Dependencies**: Open the terminal in VS Code (`Ctrl+`` or `Cmd+``) and run:
    ```bash
    npm install
    ```
5.  **Run the Extension**: Press `F5` to open a new Extension Development Host window with the extension loaded.

## Usage

Once installed and running, the JJJ extension will add new views and commands to your VS Code sidebar and command palette.

-   **Sidebar Views**: Look for a new icon in the Activity Bar (left sidebar) that will open the JJJ panel. Here you can interact with your tasks and review items.
-   **Command Palette**: Access various `jjj` functions by opening the Command Palette (`Ctrl+Shift+P` or `Cmd+Shift+P`) and searching for `JJJ:` commands.

More detailed usage instructions will be provided in future updates.

## Contributing

We welcome contributions! Please see the main `jjj` project's `CONTRIBUTING.md` for guidelines on how to contribute.
