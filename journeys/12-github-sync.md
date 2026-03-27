---
title: "GitHub Sync"
description: "GitHub issue import, status, solve with auto-close, push reconciliation, and custom label mapping"
replaces: "uxr/scenarios/12-github-sync.sh"
covers:
  - "Issue import (single and --all)"
  - "Priority mapping from GitHub labels"
  - "Idempotent reimport"
  - "GitHub status shows linked problems"
  - "Solve with --github-close"
  - "Dissolve with --github-close"
  - "GitHub push reconciles issue state"
  - "Solution comment (critique reply)"
  - "Custom label_priority config"
tags: [github, sync, import, label-priority]
---

# GitHub Sync

## Setup

```shell:setup
mkdir -p $REPO/fake-bin
cat > $REPO/fake-bin/gh << 'GHEOF'
#!/usr/bin/env bash
# Minimal fake gh CLI for UXR testing.
# Match on first two positional args, then check the rest.

CMD="$1 $2"

case "$CMD" in
  "auth status")
    echo "Logged in to github.com account testuser (keyring)"
    exit 0
    ;;

  "api user")
    # gh api user --jq .login
    echo "testuser"
    exit 0
    ;;

  "repo view")
    # gh repo view --json owner,name --jq ...
    echo "testowner/testrepo"
    exit 0
    ;;

  "issue create")
    echo "42"
    exit 0
    ;;

  "issue view")
    # gh issue view N --json ...
    NUM="$3"
    ARGS="$*"
    if [[ "$ARGS" == *"state"* && "$ARGS" == *".state"* ]]; then
      echo "OPEN"
    elif [[ "$NUM" == "42" ]]; then
      cat <<'JSON'
{"number":42,"title":"Login is slow when session expires","body":"Users are logged out after 30 minutes of inactivity.","state":"OPEN","labels":[{"name":"high"}],"author":{"login":"octocat"}}
JSON
    else
      cat <<'JSON'
{"number":43,"title":"Memory leak in worker thread","body":"Worker process grows to 2 GB then crashes.","state":"OPEN","labels":[{"name":"critical"}],"author":{"login":"bob"}}
JSON
    fi
    exit 0
    ;;

  "issue list")
    cat <<'JSON'
[{"number":42,"title":"Login is slow when session expires","state":"OPEN","labels":[{"name":"high"}]},{"number":43,"title":"Memory leak in worker thread","state":"OPEN","labels":[{"name":"critical"}]}]
JSON
    exit 0
    ;;

  "issue close")
    echo "Closed issue #$3."
    exit 0
    ;;

  "issue reopen")
    echo "Reopened issue #$3."
    exit 0
    ;;

  "pr view")
    # Determine what field is requested
    ARGS="$*"
    if [[ "$ARGS" == *"reviewThreads"* ]]; then
      cat <<'JSON'
[{"isResolved":false,"isOutdated":false,"comments":[{"databaseId":99001,"author":{"login":"alice"},"body":"Missing null check on line 42","path":"src/session.rs","line":42,"originalLine":42}]}]
JSON
    elif [[ "$ARGS" == *"reviews"* ]]; then
      cat <<'JSON'
[{"id":9001,"author":{"login":"alice"},"state":"CHANGES_REQUESTED","body":"The session timeout logic needs error handling for network failures."}]
JSON
    elif [[ "$ARGS" == *"state"* ]]; then
      echo "OPEN"
    else
      echo '{}'
    fi
    exit 0
    ;;

  "pr create")
    echo "101"
    exit 0
    ;;

  "pr edit")
    echo "Updated PR #$3."
    exit 0
    ;;

  "pr merge")
    echo "Merged PR #$3."
    exit 0
    ;;

  *)
    echo "fake-gh: unhandled: $*" >&2
    exit 1
    ;;
esac
GHEOF
chmod +x $REPO/fake-bin/gh
```

```jjj:setup
init
```

## Issue Import

```jjj
github import 42
> Login is slow
> 42
```

```jjj
problem list
> Login is slow
```

Priority from the "high" label is set on the problem:

```jjj
problem list --json
> "high"
```

Reimporting the same issue reports it is already linked:

```jjj
github import 42
> already linked
```

## Import All Issues

```jjj
github import --all
> Memory leak
```

Priority for issue 43 (critical label) appears in the problem JSON:

```jjj
problem list --json
> "critical"
```

```jjj
problem list
> Login is slow
> Memory leak
```

Running --all again finds nothing new:

```jjj
github import --all
> No unlinked
```

## GitHub Status

```jjj
github status
> testowner/testrepo
> testuser
> Linked problems
> 42
> 43
> Sync critiques: true
> Auto-close on solve: false
```

## Solve with --github-close

```jjj:setup
solution new "Add session keepalive" --problem "Login is slow" --force
```

```jjj:setup
solution submit "Add session keepalive"
```

```jjj:setup
solution approve "Add session keepalive" --force
```

```jjj
problem solve "Login is slow" --github-close
> marked as solved
> auto-closed GitHub issue #42
```

```jjj
problem list --status open
>! Login is slow
```

## Dissolve with --github-close

```jjj
problem dissolve "Memory leak" --reason "Turned out to be a test harness issue, not production code" --github-close
> dissolved
> auto-closed GitHub issue #43
```

## GitHub Push

The mock always reports issues as OPEN, so push will attempt to close them again (idempotent):

```jjj
github push
> Closed issue
```

## Solution Comment (critique reply)

```jjj:setup
problem new "API timeout on large payloads" --force
```

```jjj:setup
solution new "Stream large payloads" --problem "API timeout" --force
```

```jjj
critique new "Stream large payloads" "Backpressure not handled" --severity high
> Backpressure not handled
```

```jjj
solution comment "Stream large payloads" --critique "Backpressure" "Good point — I'll add flow control in the next commit"
> Replied to critique
```

```jjj
critique show "Backpressure not handled" --json
> Good point
> flow control
```

## Custom label_priority Config

Test that custom GitHub label-to-priority mappings work.

```shell:setup
# Create second repo for label_priority testing
mkdir -p $REPO/label-config
cd $REPO/label-config
git init -q .
git config user.name "Test User"
git config user.email "test@example.com"
git commit -q --allow-empty -m "initial"
jj git init --colocate 2>/dev/null || true
jj config set --repo user.name "Test User" 2>/dev/null
jj config set --repo user.email "test@example.com" 2>/dev/null
$JJJ init
```

```shell:setup
cat > $REPO/fake-bin/gh << 'GHEOF2'
#!/usr/bin/env bash
CMD="$1 $2"
case "$CMD" in
  "auth status")
    echo "Logged in to github.com account testuser (keyring)"
    exit 0
    ;;
  "api user")
    echo "testuser"
    exit 0
    ;;
  "repo view")
    echo "testowner/testrepo"
    exit 0
    ;;
  "issue view")
    NUM="$3"
    ARGS="$*"
    if [[ "$ARGS" == *"state"* && "$ARGS" == *".state"* ]]; then
      echo "OPEN"
    else
      cat <<'JSON'
{"number":50,"title":"Custom priority label test issue","body":"This issue has a custom team priority label.","state":"OPEN","labels":[{"name":"team-priority-1"}],"author":{"login":"testuser"}}
JSON
    fi
    exit 0
    ;;
  "issue list")
    cat <<'JSON'
[{"number":50,"title":"Custom priority label test issue","state":"OPEN","labels":[{"name":"team-priority-1"}]}]
JSON
    exit 0
    ;;
  "issue create")
    echo "50"
    exit 0
    ;;
  "issue close")
    echo "Closed issue #$3."
    exit 0
    ;;
  *)
    echo "fake-gh: unhandled: $*" >&2
    exit 1
    ;;
esac
GHEOF2
chmod +x $REPO/fake-bin/gh
```

Add the custom label-to-priority mapping to config:

```shell:setup
CONFIG_PATH="$REPO/label-config/.jj/jjj-meta/config.toml"
cat >> "$CONFIG_PATH" << 'TOMLEOF'
"team-priority-1" = "critical"
TOMLEOF
```

Import issue 50 with the custom label:

```shell
cd $REPO/label-config && $JJJ github import 50
> Custom priority label
```

Verify the priority was resolved to "critical" (not the default "medium"):

```shell
cd $REPO/label-config && $JJJ problem list --json
> "critical"
>! "medium"
```

The `label_priority` mapping in `[github]` config maps arbitrary GitHub labels to jjj priorities. Without the mapping, "team-priority-1" would not match any built-in label and priority would default to medium.
