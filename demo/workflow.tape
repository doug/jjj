# jjj Workflow Demo — Popperian project management
# Records: demo/workflow.gif

Output demo/workflow.gif

Require jjj
Require jj

Set Shell "bash"
Set FontSize 16
Set Width 960
Set Height 540
Set Padding 16
Set Theme "Catppuccin Mocha"
Set TypingSpeed 30ms
Set WindowBar Colorful
Set CursorBlink false

# --- Hidden setup: create a temp jj repo ---
Hide
Type "export TMPDIR=$(mktemp -d)" Enter
Sleep 500ms
Type "cd $TMPDIR && jj git init myproject && cd myproject" Enter
Sleep 1s
Type "jjj init" Enter
Sleep 1s
Type "clear" Enter
Sleep 500ms
Show

# --- Demo starts ---
Sleep 500ms

# Create a problem with tags
Type "jjj problem new 'Search is too slow' --priority p1 --tags backend,performance"
Sleep 500ms
Enter
Sleep 2s

# Create a solution
Type "jjj solution new 'Add search index' --problem 'Search is too slow'"
Sleep 500ms
Enter
Sleep 2s

# Add a critique
Type "jjj critique new 'Add search' 'Missing error handling' --severity high"
Sleep 500ms
Enter
Sleep 2s

# Show the problem
Type "jjj problem show 'Search'"
Sleep 500ms
Enter
Sleep 3s

# Address the critique
Type "jjj critique address 'Missing error'"
Sleep 500ms
Enter
Sleep 2s

# Submit and approve
Type "jjj solution submit 'Add search'"
Sleep 500ms
Enter
Sleep 2s

Type "jjj solution approve 'Add search'"
Sleep 500ms
Enter
Sleep 2s

# Final status
Type "jjj status"
Sleep 500ms
Enter
Sleep 3s

Sleep 2s
