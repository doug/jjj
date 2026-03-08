# jjj + Jujutsu Integration Demo — change IDs, attach, resume
# Records: demo/jj-integration.gif

Output demo/jj-integration.gif

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

# --- Hidden setup ---
Hide
Type "export TMPDIR=$(mktemp -d)" Enter
Sleep 500ms
Type "cd $TMPDIR && jj git init myproject && cd myproject" Enter
Sleep 1s
Type "jjj init" Enter
Sleep 500ms
Type "jjj problem new 'Need user authentication' --priority P1" Enter
Sleep 300ms
Type "clear" Enter
Sleep 500ms
Show

Sleep 500ms

# Create solution — auto-creates a jj change
Type "jjj solution new 'Add JWT auth' --problem 'user authentication'"
Sleep 500ms
Enter
Sleep 2s

# Show the jj log — see the change ID
Type "jj log --limit 3"
Sleep 500ms
Enter
Sleep 3s

# Show solution — has change_id attached
Type "jjj solution show 'JWT auth'"
Sleep 500ms
Enter
Sleep 3s

# Resume working on the solution (switches to its change)
Type "jjj solution resume 'JWT auth'"
Sleep 500ms
Enter
Sleep 2s

# Show jj status
Type "jj status"
Sleep 500ms
Enter
Sleep 2s

# Show jjj status — links solution to current change
Type "jjj status"
Sleep 500ms
Enter
Sleep 3s

Sleep 2s
