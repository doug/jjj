# JJJ VS Code Extension - UX Plan

This document outlines the user experience (UX) design for the JJJ VS Code extension. The goal is to provide a seamless, intuitive interface for the `jjj` command-line tool's features, deeply integrated into the developer's daily workflow within VS Code.

## 1. High-Level Vision

The extension will translate the powerful, git-native task management and code review capabilities of `jjj` into a rich graphical interface. It will feel like a natural part of VS Code, using its standard UI components to expose `jjj`'s core concepts: tasks, reviews, the Kanban board, and the personal dashboard.

## 2. Core UX Components

### 2.1. Activity Bar Icon

-   **Icon:** A dedicated "JJJ" icon will be added to the Activity Bar.
-   **Functionality:** Clicking this icon will open the JJJ Sidebar, which will contain the main views for tasks, reviews, and the dashboard.

### 2.2. JJJ Sidebar Views

The JJJ Sidebar will be the primary interaction point. It will feature several views, which can be expanded or collapsed.

#### 2.2.1. Welcome/Init View

-   **Context:** When the workspace is not a `jjj` repository.
-   **UI:** A simple view with a button: `[ Initialize JJJ Repository ]`.
-   **Action:** Clicking the button will run `jjj init`.

#### 2.2.2. Task Management View (`My Tasks` and `Board`)

This view will provide two ways of looking at tasks: by personal assignment and by Kanban board status.

-   **`My Tasks` Tree View:**
    -   A tree view showing tasks assigned to the user, grouped by status (e.g., "In Progress", "To Do").
    -   Each task item will be clickable, opening a detailed view.
    -   Context menu on each task: `Edit Task`, `Move Task`, `Attach Change`, `Delete Task`.
    -   A "+" icon in the view header to run the `task new` command.

-   **`Board` Tree View:**
    -   A tree view representing the Kanban board (`jjj board`).
    -   Top-level nodes are the board columns (e.g., "TODO", "IN PROGRESS", "DONE").
    -   Tasks are nested under their respective columns.
    -   Drag-and-drop functionality to move tasks between columns (`jjj task move`).
    -   Context menu on each task: `Edit Task`, `Show Details`.

#### 2.2.3. Code Review View (`My Reviews`)

This view is for managing code reviews.

-   **Tree View Grouping:**
    -   `Pending My Review`: Reviews where the user is a reviewer.
    -   `My Open Reviews`: Reviews requested by the user.
    -   `Recently Closed`: Recently approved or abandoned reviews.
-   **Review Items:**
    -   Each item shows the change ID, title, and author.
    -   Clicking a review item will open a diff view and a dedicated "Review" webview.
    -   Context menu: `Start Review`, `Approve`, `Request Changes`.

#### 2.2.4. Dashboard View (Webview)

-   **Trigger:** A "Show Dashboard" command or a dedicated icon.
-   **UI:** A webview that visualizes the output of `jjj dashboard`.
    -   Displays a summary of pending reviews and active tasks.
    -   Could include quick links to start reviews or view tasks.

### 2.3. Command Palette Integration

All `jjj` commands will be accessible through the Command Palette (`Ctrl/Cmd+Shift+P`), prefixed with "JJJ:". This provides a power-user interface and an alternative to the GUI.

-   Examples: `JJJ: Create New Task`, `JJJ: List Reviews`, `JJJ: Show Kanban Board`.
-   Commands will use quick picks and input boxes to gather arguments (e.g., prompting for a task title when creating a new task).

### 2.4. Editor and SCM Integration

-   **Source Control View:** The current Jujutsu change in the SCM view will have an associated "JJJ Task" field.
    -   Buttons to `Attach Task` or `Request Review` for the current change.
-   **CodeLens:**
    -   In files with active review comments, CodeLens annotations will appear above the commented lines, showing a summary of the comment thread. Clicking it would open the review panel.
-   **Gutter Icons:**
    -   Lines with review comments will have an icon in the gutter. Hovering over the icon will show the comment.

## 3. User Workflows

### 3.1. Creating and Managing a Task

1.  **Creation:**
    -   User clicks the "+" icon in the "Task Management" view or uses the `JJJ: Create New Task` command.
    -   An input box prompts for the task title.
    -   A quick pick prompts to select a feature.
    -   An input box prompts for tags.
    -   A new task is created and appears in the `My Tasks` and `Board` views.
2.  **Working on a Task:**
    -   User finds the task in the `Board`'s "TODO" column.
    -   They drag-and-drop it to "IN PROGRESS". This runs `jjj task move`.
    -   They work on their code.
    -   From the SCM view, they click `Attach Task` on their current change, and select the task from a quick pick list. This runs `jjj task attach`.
3.  **Completion:**
    -   When done, the user drags the task to the "DONE" column.

### 3.2. Requesting and Performing a Code Review

1.  **Requesting a Review:**
    -   With a change ready in the SCM view, the user clicks `Request Review`.
    -   A quick pick asks for reviewers.
    -   The review is created and appears in the requestor's `My Open Reviews` view and the reviewers' `Pending My Review` view.
2.  **Performing a Review:**
    -   A reviewer sees the new review in their `Pending My Review` list.
    -   They click the review item. This opens a diff view of the changes.
    -   They can add comments directly in the diff view (which will trigger `jjj review comment`).
    -   When finished, they use the buttons in the review panel or the context menu to `Approve` or `Request Changes`.

## 4. Future Enhancements

-   **Customizable Dashboards:** Allow users to configure their dashboard view.
-   **Conflict Resolution UI:** A dedicated UI for walking through the `jjj resolve` process.
-   **Timeline View:** A visual timeline of a task or review's history.
-   **Deeper Git/Jujutsu Integration:** Show `jjj` information directly in the git history views.
