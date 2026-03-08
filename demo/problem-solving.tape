# jjj Problem Solving Demo — decomposition, hierarchy, tree view
# Records: demo/problem-solving.gif

Output demo/problem-solving.gif

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
Type "clear" Enter
Sleep 500ms
Show

Sleep 500ms

# Create root problem
Type "jjj problem new 'Search results include deleted items' --priority P1"
Sleep 500ms
Enter
Sleep 2s

# Decompose into sub-problems
Type "# Decompose into specific sub-problems"
Sleep 800ms
Enter
Sleep 500ms
Type "jjj problem new 'Soft-deleted records not filtered' --parent 'deleted items'"
Sleep 500ms
Enter
Sleep 2s

Type "jjj problem new 'Cache not invalidated on delete' --parent 'deleted items'"
Sleep 500ms
Enter
Sleep 2s

# View the hierarchy
Type "jjj problem tree"
Sleep 500ms
Enter
Sleep 3s

# Propose solutions for sub-problems
Type "jjj solution new 'Add deleted_at filter to query' --problem 'Soft-deleted'"
Sleep 500ms
Enter
Sleep 2s

Type "jjj solution new 'Invalidate cache on soft delete' --problem 'Cache not'"
Sleep 500ms
Enter
Sleep 2s

# View tree again with solutions
Type "jjj problem tree"
Sleep 500ms
Enter
Sleep 3s

Sleep 2s
