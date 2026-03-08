# jjj TUI Demo — interactive terminal interface
# Records: demo/tui.gif

Output demo/tui.gif

Require jjj
Require jj

Set Shell "bash"
Set FontSize 16
Set Width 960
Set Height 600
Set Padding 16
Set Theme "Catppuccin Mocha"
Set TypingSpeed 30ms
Set WindowBar Colorful
Set CursorBlink false

# --- Hidden setup: create repo with sample data ---
Hide
Type "export TMPDIR=$(mktemp -d)" Enter
Sleep 500ms
Type "cd $TMPDIR && jj git init myproject && cd myproject" Enter
Sleep 1s
Type "jjj init" Enter
Sleep 500ms
Type "jjj problem new 'Search is slow' --priority P1 --tags backend,performance" Enter
Sleep 300ms
Type "jjj problem new 'Login page redesign' --priority P2 --tags frontend,ux" Enter
Sleep 300ms
Type "jjj problem new 'API rate limiting' --priority P0 --tags backend,security" Enter
Sleep 300ms
Type "jjj solution new 'Add search index' --problem 'Search is slow'" Enter
Sleep 300ms
Type "jjj solution new 'Token bucket algorithm' --problem 'API rate'" Enter
Sleep 300ms
Type "jjj critique new 'Add search' 'Missing error handling' --severity high" Enter
Sleep 300ms
Type "jjj critique new 'Token bucket' 'No distributed lock' --severity medium" Enter
Sleep 500ms
Type "clear" Enter
Sleep 500ms
Show

# --- Launch TUI ---
Sleep 500ms
Type "jjj ui"
Sleep 500ms
Enter
Sleep 2s

# Navigate down through next actions
Down
Sleep 700ms
Down
Sleep 700ms
Down
Sleep 700ms

# Switch to project tree
Tab
Sleep 1s

# Navigate in project tree
Down
Sleep 600ms
Down
Sleep 600ms

# Expand a problem to see its solution
Right
Sleep 800ms
Down
Sleep 600ms

# Expand solution to see critique
Right
Sleep 800ms
Down
Sleep 600ms

# Scroll detail pane
Type "j"
Sleep 400ms
Type "j"
Sleep 400ms
Type "k"
Sleep 400ms

Sleep 1s

# Show help
Type "?"
Sleep 3s

# Close help
Type "?"
Sleep 1s

# Quit
Type "q"
Sleep 1s
