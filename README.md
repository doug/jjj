# **jjj: Jujutsu Juggler: Distributed Collaboration for Jujutsu**

**jjj** is a distributed project management and code review system built exclusively for the [Jujutsu (jj)](https://github.com/jj-vcs/jj) version control system.

It implements a **Kanban-style task manager** and a **resilient code review workflow** directly within your repository. It requires no central server, no database, and no browser. It functions entirely offline, synchronising via standard jj push/pull operations.

## **I. The Philosophy**

Previous attempts at distributed review (like git-appraise) suffered a fatal flaw: **The fragility of the Commit Hash.** In Git, if you rebase a branch to clean up history, every commit hash changes. Metadata attached to those hashes becomes orphaned, requiring complex heuristics to re-attach.

**Jujutsu solves this.** jj treats changes as first-class citizens with stable **Change IDs** that persist across rewrites, rebases, and squashes. jjj leverages this stability to anchor tasks and reviews to the *identity* of a change, not its momentary snapshot.

### **Core Principles**

1. **The Shadow Graph:** All metadata (tasks, comments, statuses) lives in an orphaned history root tracked by the jjj/meta bookmark. It never touches your working copy.  
2. **Change-Centricity:** Reviews attach to Change IDs. You can rebase your stack fifty times; the review comments remain attached.  
3. **Conflict as Data:** If two users update a task state simultaneously, jjj accepts both, creates a standard jj conflict, and exposes it in the UI for resolution.

## **II. The Architecture**

jjj maintains a parallel directory structure within the jjj/meta bookmark.

/ (root of jjj/meta)  
├── config.toml           # Project-wide settings (columns, tags)  
├── tasks/  
│   ├── T-1024.json       # Task metadata  
│   └── T-1025.json  
└── reviews/  
    └── kpzszn.../        # Directory named by Change ID  
        ├── manifest.toml # Review status (Pending, Approved)  
        └── comments/     # Individual comment objects  
            └── c-998.json

### **State Propagation**

To share tasks and reviews with your team, you must push the `jjj/meta` bookmark.

**Pushing changes:**
```bash
# Push your changes and the jjj metadata
jj git push -b jjj/meta
```

**Fetching updates:**
```bash
# Fetch updates from the team
jj git fetch
```

**One-time setup:**
If you haven't already, track the remote metadata bookmark:
```bash
jj bookmark track jjj/meta@origin
```

## **III. Workflow: Project Management (Kanban)**

The goal is a frictionless, terminal-based board that tracks work without context switching.

### **1. The Board View**

$ jjj board

**Output (TUI):**

```
 ┌── TODO ────────────────┐ ┌── IN PROGRESS ─────────┐ ┌── REVIEW ──────────────┐  
 │                        │ │                        │ │                        │  
 │ T-101: Db Schema       │ │ T-105: Auth API        │ │ T-99:  Login UI        │  
 │ #backend               │ │ @james  (yqosq...)     │ │ @sarah (zpmoz...)      │  
 │                        │ │                        │ │ ⚠ 2 comments           │  
 │                        │ │                        │ │                        │  
 └────────────────────────┘ └────────────────────────┘ └────────────────────────┘
```

### **2. Managing Tasks**

Tasks are independent entities. When you start working, you associate a task with your current jj change.


```
# Create a new task  
$ jjj task new "Refactor User Authentication" --tag backend
```

```
# Associate the current working change (Change ID: yqosq) with the task  
$ jjj task attach T-105
```

```
# Move the card  
$ jjj task move T-105 "In Progress"
```

### **3. Handling Conflicts**

If Alice moves T-105 to "Done" and Bob moves it to "Blocked" simultaneously, jj records a conflict in the file tasks/T-105.json.

jjj board renders this card in red: [ ! CONFLICT ].  
To resolve:  
$ jjj resolve T-105 --pick "Done"

This performs a standard jj merge on the hidden metadata file.

## **IV. Workflow: Code Review**

The review flow is designed for the "stacked diff" workflow that jj encourages.

### **1. Requesting Review**

The author requests a review on a specific change or a whole stack.

$ jjj review request @alice

*Effect:* Creates a review manifest in jjj/meta linked to the current Change ID.

### **2. The Reviewer's Experience**

Alice pulls the repo. She sees pending reviews in her dashboard.

```
$ jjj dashboard  
Review Requested:  
  - yqosq... "Refactor User Auth" (Author: James)
```

She enters the review mode:

```
$ jjj review start yqosq
```

This opens a TUI diff viewer (or launches your configured difftool).

### **3. Anchoring Comments**

Alice comments on src/auth.rs, line 42.  
jjj stores the comment with a Context Fingerprint:

* **Change ID:** yqosq...  
* **File:** src/auth.rs  
* **Line Context:** The code surrounding line 42.

### **4. The Author's Evolution**

James receives the feedback. He edits his code to fix the issue.

```
$ jj edit yqosq  
# ... fixes code ...  
$ jj squash
```

**Crucial Point:** In Git, the commit hash would change, and the comment might drift. In jj, the Change ID yqosq is constant.

When James runs jjj review status, jjj attempts to locate the comment. If line 42 has shifted to line 50, jjj uses the Context Fingerprint (fuzzy matching) to float the comment to the correct new location.

### **5. Approval**

Once satisfied, the reviewer stamps the change.

$ jjj review approve

This adds a signed "Approved" record to the review manifest. This approval can be configured to act as a gate for CI/CD pipelines.

## **V. Technical Specifics**

### **Storage Format**

We use **JSON** for machine readability and **TOML** for human readability where editing might occur manually.

**Example: Review Comment (c-998.json)**

{  
  "id": "c-998",  
  "author": "Alice <alice@example.com>",  
  "timestamp": "2023-10-27T10:00:00Z",  
  "target_change_id": "yqosq...",  
  "file_path": "src/lib.rs",  
  "location": {  
    "start_line": 42,  
    "end_line": 45,  
    "context_hash": "a1b2c3d4..."   
  },  
  "body": "This lock acquisition is not panic-safe."  
}

### **Integration with Forge (GitHub/GitLab)**

jjj is designed to replace the need for GitHub Pull Requests, but it can coexist. A server-side bridge could theoretically listen to jjj/meta pushes and generate HTML reports or sync status back to GitHub Issues if legacy compatibility is required.

## **VI. Summary**

`jjj` is not just "tasks in a text file." It is a distributed state machine piggybacking on the advanced cryptographic guarantees of Jujutsu. It enables a workflow where project management is as immutable, offline-capable, and branchable as the code itself.