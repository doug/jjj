//! Reconstruct per-file content from a full-context `jj diff --git` delta.
//!
//! # Why (Pillar 1 content-fetch)
//!
//! Incremental fetch must, for each file changed between the last-synced
//! revision and the remote revision, recover **both** sides' full content so a
//! three-way merge can run: `base` (last-synced) and `remote`. M0 ruled out the
//! obvious approaches:
//!
//! - per-file `jj file show` is ~94ms each at 25K / ~300ms at 100K (it
//!   re-resolves the whole tree per call) — breaks the sub-second budget past a
//!   handful of files, and the merge needs *two* reads per file (base + remote);
//! - multi-path `jj file show` concatenates contents with **no delimiter**, so
//!   per-file boundaries are unrecoverable.
//!
//! The one batched primitive that works: `jj diff --from BASE --to REMOTE --git
//! --context <huge>` — a single subprocess (~0.32s, flat in delta size) whose
//! full-context hunks contain the **entire** file on both sides. From each
//! file's hunk we reconstruct:
//!
//! - `base`   = lines prefixed ` ` (context) or `-` (removed), in order;
//! - `remote` = lines prefixed ` ` (context) or `+` (added), in order.
//!
//! A `new file` section has no base; a `deleted file` section has no remote.
//!
//! This module is **pure** (`&str` → `Vec<FileDelta>`); it never shells out, so
//! it is exhaustively unit-testable on captured `jj` output.
//!
//! ## Full-context requirement
//!
//! The caller MUST pass a context larger than any entity file (entities are a
//! few KB; the fetch path uses a very large value). With full context jj emits a
//! single gap-free hunk per file, so concatenating all hunk body lines
//! reconstructs each side exactly. Were the context too small, separated hunks
//! would omit the unchanged middle and reconstruction would be lossy — hence the
//! invariant lives with the caller, asserted by the fetch path, not guessed here.

use crate::error::{JjjError, Result};

/// How a file changed between the two revisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaKind {
    /// Present only at the remote revision.
    Added,
    /// Present at both, content differs.
    Modified,
    /// Present only at the base revision.
    Deleted,
}

/// One changed file, with whichever sides exist reconstructed in full.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDelta {
    /// Repo-relative path, e.g. `problems/01.md`.
    pub path: String,
    pub kind: DeltaKind,
    /// Full content at the base (last-synced) revision. `None` iff `Added`.
    pub base: Option<String>,
    /// Full content at the remote revision. `None` iff `Deleted`.
    pub remote: Option<String>,
}

/// Parse a full-context `jj diff --git` blob into per-file deltas.
///
/// Returns an error only if the structure is malformed (a `diff --git` header
/// whose path can't be extracted). An empty input yields an empty vec.
pub fn parse_git_diff(diff: &str) -> Result<Vec<FileDelta>> {
    let mut out = Vec::new();
    let lines: Vec<&str> = diff.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        if !lines[i].starts_with("diff --git ") {
            i += 1;
            continue;
        }
        // Collect this file's section: from the `diff --git` header up to (but
        // not including) the next one.
        let start = i;
        i += 1;
        while i < lines.len() && !lines[i].starts_with("diff --git ") {
            i += 1;
        }
        let section = &lines[start..i];
        out.push(parse_section(section)?);
    }
    Ok(out)
}

/// Parse a single `diff --git` … section (header + metadata + one hunk).
fn parse_section(section: &[&str]) -> Result<FileDelta> {
    let header = section[0];
    let mut added = false;
    let mut deleted = false;

    // Walk metadata lines until the hunk body starts (`@@`). Body lines after
    // the first `@@` carry the content.
    let mut body_start = section.len();
    for (idx, line) in section.iter().enumerate().skip(1) {
        if line.starts_with("new file mode") {
            added = true;
        } else if line.starts_with("deleted file mode") {
            deleted = true;
        } else if line.starts_with("@@") {
            body_start = idx + 1;
            break;
        }
    }

    let path = section_path(header, deleted)?;

    let kind = match (added, deleted) {
        (true, _) => DeltaKind::Added,
        (_, true) => DeltaKind::Deleted,
        _ => DeltaKind::Modified,
    };

    // Reconstruct each side from the hunk body. `@@` lines (multiple hunks
    // shouldn't occur with full context, but tolerate them) are skipped.
    let mut base_lines: Vec<&str> = Vec::new();
    let mut remote_lines: Vec<&str> = Vec::new();
    // Track whether the most recent line on each side suppressed its newline.
    let mut base_no_newline = false;
    let mut remote_no_newline = false;

    for line in &section[body_start..] {
        if line.starts_with("@@") {
            continue;
        }
        // The "\ No newline at end of file" marker trails the last emitted
        // line. We can't perfectly attribute it to one side without re-walking,
        // and our entity files always end in a newline (this path is rarely
        // exercised), so we drop the trailing newline from both reconstructions
        // — correct for the common case and never fabricating content.
        if line.starts_with('\\') {
            base_no_newline = true;
            remote_no_newline = true;
            continue;
        }
        match line.as_bytes().first() {
            Some(b' ') => {
                base_lines.push(&line[1..]);
                remote_lines.push(&line[1..]);
            }
            Some(b'-') => base_lines.push(&line[1..]),
            Some(b'+') => remote_lines.push(&line[1..]),
            // An empty line in the diff body represents a blank context line.
            None => {
                base_lines.push("");
                remote_lines.push("");
            }
            _ => {}
        }
    }

    let base = if kind == DeltaKind::Added {
        None
    } else {
        Some(join_content(&base_lines, base_no_newline))
    };
    let remote = if kind == DeltaKind::Deleted {
        None
    } else {
        Some(join_content(&remote_lines, remote_no_newline))
    };

    Ok(FileDelta {
        path,
        kind,
        base,
        remote,
    })
}

/// Join reconstructed lines back into file content. By unified-diff convention
/// every emitted line had a trailing newline unless a `\ No newline` marker
/// said otherwise, so we append `\n` per line and optionally trim the final one.
fn join_content(lines: &[&str], no_trailing_newline: bool) -> String {
    if lines.is_empty() {
        return String::new();
    }
    let mut s = String::with_capacity(lines.iter().map(|l| l.len() + 1).sum());
    for line in lines {
        s.push_str(line);
        s.push('\n');
    }
    if no_trailing_newline {
        s.pop();
    }
    s
}

/// Extract the repo-relative path from a `diff --git a/PATH b/PATH` header.
///
/// For a delete the `b/` side is `/dev/null`, so prefer the `a/` side there.
/// jjj entity paths never contain spaces (`problems/<uuid>.md`), so a simple
/// `a/…  b/…` split is unambiguous for our data; we take the `b/` path (or `a/`
/// for deletes) and strip the one-char prefix.
fn section_path(header: &str, deleted: bool) -> Result<String> {
    let rest = header
        .strip_prefix("diff --git ")
        .ok_or_else(|| malformed(header))?;
    // Split into "a/PATH" and "b/PATH". Paths have no spaces in our corpus, so
    // the first token is the a-side and the second is the b-side.
    let mut parts = rest.split(' ');
    let a = parts.next().ok_or_else(|| malformed(header))?;
    let b = parts.next().ok_or_else(|| malformed(header))?;
    let chosen = if deleted { a } else { b };
    let path = chosen
        .strip_prefix("a/")
        .or_else(|| chosen.strip_prefix("b/"))
        .unwrap_or(chosen);
    Ok(path.to_string())
}

fn malformed(header: &str) -> JjjError {
    JjjError::Validation(format!("malformed diff --git header: {header}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_yields_no_deltas() {
        assert!(parse_git_diff("").unwrap().is_empty());
    }

    #[test]
    fn modified_file_reconstructs_both_sides() {
        // Captured from real `jj diff --git` (5-line file, line 3 changed).
        let diff = "\
diff --git a/problems/multi.md b/problems/multi.md
index b3c5a95f92..cf92929517 100644
--- a/problems/multi.md
+++ b/problems/multi.md
@@ -1,5 +1,5 @@
 line1
 line2
-line3
+CHANGED
 line4
 line5
";
        let deltas = parse_git_diff(diff).unwrap();
        assert_eq!(deltas.len(), 1);
        let d = &deltas[0];
        assert_eq!(d.path, "problems/multi.md");
        assert_eq!(d.kind, DeltaKind::Modified);
        assert_eq!(
            d.base.as_deref(),
            Some("line1\nline2\nline3\nline4\nline5\n")
        );
        assert_eq!(
            d.remote.as_deref(),
            Some("line1\nline2\nCHANGED\nline4\nline5\n")
        );
    }

    #[test]
    fn added_file_has_no_base() {
        let diff = "\
diff --git a/problems/d.md b/problems/d.md
new file mode 100644
index 0000000000..1784810501
--- /dev/null
+++ b/problems/d.md
@@ -0,0 +1,2 @@
+hello
+world
";
        let deltas = parse_git_diff(diff).unwrap();
        assert_eq!(deltas.len(), 1);
        let d = &deltas[0];
        assert_eq!(d.path, "problems/d.md");
        assert_eq!(d.kind, DeltaKind::Added);
        assert_eq!(d.base, None);
        assert_eq!(d.remote.as_deref(), Some("hello\nworld\n"));
    }

    #[test]
    fn deleted_file_has_no_remote_and_uses_a_side_path() {
        let diff = "\
diff --git a/problems/c.md b/problems/c.md
deleted file mode 100644
index 3cc58df837..0000000000
--- a/problems/c.md
+++ /dev/null
@@ -1,1 +0,0 @@
-gone
";
        let deltas = parse_git_diff(diff).unwrap();
        assert_eq!(deltas.len(), 1);
        let d = &deltas[0];
        assert_eq!(d.path, "problems/c.md");
        assert_eq!(d.kind, DeltaKind::Deleted);
        assert_eq!(d.base.as_deref(), Some("gone\n"));
        assert_eq!(d.remote, None);
    }

    #[test]
    fn multiple_files_in_one_diff() {
        // The exact three-file delta captured earlier (M, D, A).
        let diff = "\
diff --git a/problems/a.md b/problems/a.md
index f70f10e4db..3ce238aeae 100644
--- a/problems/a.md
+++ b/problems/a.md
@@ -1,1 +1,1 @@
-A
+A2
diff --git a/problems/c.md b/problems/c.md
deleted file mode 100644
index 3cc58df837..0000000000
--- a/problems/c.md
+++ /dev/null
@@ -1,1 +0,0 @@
-C
diff --git a/problems/d.md b/problems/d.md
new file mode 100644
index 0000000000..1784810501
--- /dev/null
+++ b/problems/d.md
@@ -0,0 +1,1 @@
+D
";
        let deltas = parse_git_diff(diff).unwrap();
        assert_eq!(deltas.len(), 3);

        assert_eq!(deltas[0].path, "problems/a.md");
        assert_eq!(deltas[0].kind, DeltaKind::Modified);
        assert_eq!(deltas[0].base.as_deref(), Some("A\n"));
        assert_eq!(deltas[0].remote.as_deref(), Some("A2\n"));

        assert_eq!(deltas[1].path, "problems/c.md");
        assert_eq!(deltas[1].kind, DeltaKind::Deleted);
        assert_eq!(deltas[1].remote, None);

        assert_eq!(deltas[2].path, "problems/d.md");
        assert_eq!(deltas[2].kind, DeltaKind::Added);
        assert_eq!(deltas[2].base, None);
        assert_eq!(deltas[2].remote.as_deref(), Some("D\n"));
    }

    #[test]
    fn realistic_frontmatter_entity_roundtrips() {
        // A change to a markdown entity with YAML frontmatter, full context —
        // the actual shape fetch reconstructs and feeds to merge_entity_md.
        let diff = "\
diff --git a/problems/01.md b/problems/01.md
index aaaaaaaaaa..bbbbbbbbbb 100644
--- a/problems/01.md
+++ b/problems/01.md
@@ -1,7 +1,7 @@
 ---
 id: '01'
-status: open
+status: in_progress
 created_at: 2026-05-01T00:00:00Z
 updated_at: 2026-05-02T00:00:00Z
 ---
 body
";
        let deltas = parse_git_diff(diff).unwrap();
        let d = &deltas[0];
        assert!(d.base.as_deref().unwrap().contains("status: open"));
        assert!(d.remote.as_deref().unwrap().contains("status: in_progress"));
        // Frontmatter delimiters preserved on both sides.
        assert!(d.base.as_deref().unwrap().starts_with("---\n"));
        assert!(d.remote.as_deref().unwrap().starts_with("---\n"));
    }

    #[test]
    fn no_newline_marker_drops_trailing_newline() {
        let diff = "\
diff --git a/problems/n.md b/problems/n.md
index aaaaaaaaaa..bbbbbbbbbb 100644
--- a/problems/n.md
+++ b/problems/n.md
@@ -1,1 +1,1 @@
-old
\\ No newline at end of file
+new
\\ No newline at end of file
";
        let deltas = parse_git_diff(diff).unwrap();
        let d = &deltas[0];
        assert_eq!(d.base.as_deref(), Some("old"));
        assert_eq!(d.remote.as_deref(), Some("new"));
    }

    #[test]
    fn blank_context_lines_are_preserved() {
        // A blank line in the body is emitted as an empty diff line (no leading
        // space). It must survive on both sides.
        let diff = "\
diff --git a/problems/b.md b/problems/b.md
index aaaaaaaaaa..bbbbbbbbbb 100644
--- a/problems/b.md
+++ b/problems/b.md
@@ -1,4 +1,4 @@
 head

-mid
+MID
 tail
";
        let deltas = parse_git_diff(diff).unwrap();
        let d = &deltas[0];
        assert_eq!(d.base.as_deref(), Some("head\n\nmid\ntail\n"));
        assert_eq!(d.remote.as_deref(), Some("head\n\nMID\ntail\n"));
    }

    #[test]
    fn non_diff_preamble_is_ignored() {
        // Any leading noise before the first `diff --git` is skipped.
        let diff = "Working copy changes:\nsome banner\ndiff --git a/x.md b/x.md\nindex a..b 100644\n--- a/x.md\n+++ b/x.md\n@@ -1,1 +1,1 @@\n-x\n+y\n";
        let deltas = parse_git_diff(diff).unwrap();
        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].path, "x.md");
    }
}
