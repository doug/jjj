# jjj Code Review Demo — critique, address, submit, approve
# Records: demo/code-review.gif

Output demo/code-review.gif

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
Type "jjj problem new 'Auth tokens expire silently' --priority p1" Enter
Sleep 300ms
Type "clear" Enter
Sleep 500ms
Show

Sleep 500ms

# Propose a solution
Type "jjj solution new 'Add token refresh with retry' --problem 'Auth tokens'"
Sleep 500ms
Enter
Sleep 2s

# Reviewer raises a critique
Type "# Reviewer spots an issue during code review"
Sleep 800ms
Enter
Sleep 500ms
Type "jjj critique new 'token refresh' 'No backoff on retry — could overwhelm auth server' --severity high"
Sleep 500ms
Enter
Sleep 2s

# Check what's blocking
Type "jjj critique list --solution 'token refresh'"
Sleep 500ms
Enter
Sleep 2s

# Author addresses the critique
Type "# Author adds exponential backoff, then addresses the critique"
Sleep 800ms
Enter
Sleep 500ms
Type "jjj critique address 'backoff'"
Sleep 500ms
Enter
Sleep 2s

# Submit for review
Type "jjj solution submit 'token refresh'"
Sleep 500ms
Enter
Sleep 2s

# Reviewer signs off
Type "jjj solution lgtm 'token refresh'"
Sleep 500ms
Enter
Sleep 2s

# Approve
Type "jjj solution approve 'token refresh'"
Sleep 500ms
Enter
Sleep 2s

Sleep 2s
