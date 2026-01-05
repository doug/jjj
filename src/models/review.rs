use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Review manifest for a specific change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewManifest {
    /// Change ID this review is for
    pub change_id: String,

    /// Author of the change
    pub author: String,

    /// Requested reviewers
    pub reviewers: Vec<String>,

    /// Current review status
    pub status: ReviewStatus,

    /// When the review was requested
    pub requested_at: DateTime<Utc>,

    /// When the review status last changed
    pub updated_at: DateTime<Utc>,

    /// Number of comments
    #[serde(default)]
    pub comment_count: usize,

    /// Whether this is a stack review
    #[serde(default)]
    pub is_stack: bool,
}

/// Status of a code review
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReviewStatus {
    /// Review has been requested
    Pending,

    /// Changes have been requested
    ChangesRequested,

    /// Review has been approved
    Approved,

    /// Review has been dismissed/cancelled
    Dismissed,
}

/// A comment on a change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    /// Unique comment ID (e.g., "c-998")
    pub id: String,

    /// Comment author
    pub author: String,

    /// When the comment was created
    pub timestamp: DateTime<Utc>,

    /// The change this comment is on
    pub target_change_id: String,

    /// File path (optional, for inline comments)
    pub file_path: Option<String>,

    /// Location in the file (optional)
    pub location: Option<CommentLocation>,

    /// Comment body
    pub body: String,

    /// Whether this comment is resolved
    #[serde(default)]
    pub resolved: bool,
}

/// Location of a comment within a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentLocation {
    /// Starting line number
    pub start_line: usize,

    /// Ending line number
    pub end_line: usize,

    /// Hash of surrounding context (for fuzzy matching after rebases)
    pub context_hash: String,

    /// The actual context lines (for fuzzy matching)
    #[serde(default)]
    pub context_lines: Vec<String>,
}

impl CommentLocation {
    /// Create a new comment location with context
    pub fn new(
        start_line: usize,
        end_line: usize,
        context_lines: Vec<String>,
    ) -> Self {
        use sha2::{Digest, Sha256};

        let context_str = context_lines.join("\n");
        let mut hasher = Sha256::new();
        hasher.update(context_str.as_bytes());
        let hash = format!("{:x}", hasher.finalize());

        Self {
            start_line,
            end_line,
            context_hash: hash[..16].to_string(), // First 16 chars
            context_lines,
        }
    }

    /// Try to relocate this comment in new file contents
    pub fn try_relocate(&self, new_lines: &[String]) -> Option<(usize, usize)> {
        // First try exact line match
        if self.start_line <= new_lines.len() {
            let current_context: Vec<String> = new_lines
                .iter()
                .skip(self.start_line.saturating_sub(2))
                .take(5)
                .cloned()
                .collect();

            if current_context == self.context_lines {
                return Some((self.start_line, self.end_line));
            }
        }

        // Try fuzzy matching
        self.fuzzy_match(new_lines)
    }

    fn fuzzy_match(&self, new_lines: &[String]) -> Option<(usize, usize)> {
        use similar::ChangeTag;

        let context_str = self.context_lines.join("\n");
        let mut best_match = None;
        let mut best_score = 0.0;

        // Scan through the file looking for the best match
        for (i, window) in new_lines.windows(self.context_lines.len()).enumerate() {
            let window_str = window.join("\n");
            let diff = similar::TextDiff::from_lines(&context_str, &window_str);

            // Calculate similarity score
            let mut matches = 0;
            let mut total = 0;

            for change in diff.iter_all_changes() {
                total += 1;
                if change.tag() == ChangeTag::Equal {
                    matches += 1;
                }
            }

            let score = if total > 0 {
                matches as f64 / total as f64
            } else {
                0.0
            };

            if score > best_score && score > 0.7 {
                best_score = score;
                let offset = self.start_line - (self.context_lines.len() / 2).max(1);
                best_match = Some((i + offset, i + offset + (self.end_line - self.start_line)));
            }
        }

        best_match
    }

}

impl Comment {
    /// Create a new comment
    pub fn new(
        id: String,
        author: String,
        target_change_id: String,
        body: String,
    ) -> Self {
        Self {
            id,
            author,
            timestamp: Utc::now(),
            target_change_id,
            file_path: None,
            location: None,
            body,
            resolved: false,
        }
    }

    /// Create a new inline comment with location
    pub fn new_inline(
        id: String,
        author: String,
        target_change_id: String,
        file_path: String,
        location: CommentLocation,
        body: String,
    ) -> Self {
        Self {
            id,
            author,
            timestamp: Utc::now(),
            target_change_id,
            file_path: Some(file_path),
            location: Some(location),
            body,
            resolved: false,
        }
    }
}
