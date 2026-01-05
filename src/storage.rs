use crate::error::{JjjError, Result};
use crate::jj::JjClient;
use crate::models::{Comment, ProjectConfig, ReviewManifest, Task};
use std::fs;
use std::path::{Path, PathBuf};

const META_BOOKMARK: &str = "jjj/meta";
const CONFIG_FILE: &str = "config.toml";
const TASKS_DIR: &str = "tasks";
const REVIEWS_DIR: &str = "reviews";

/// Storage layer for jjj metadata
pub struct MetadataStore {
    /// Path to the metadata directory (checked out from jjj/meta)
    meta_path: PathBuf,

    /// JJ client for interacting with the repository
    jj_client: JjClient,
}

impl MetadataStore {
    /// Create a new metadata store
    pub fn new(jj_client: JjClient) -> Result<Self> {
        let repo_root = jj_client.repo_root().to_path_buf();
        let meta_path = repo_root.join(".jj").join("jjj-meta");

        Ok(Self {
            meta_path,
            jj_client,
        })
    }

    /// Initialize the metadata store (create jjj/meta bookmark)
    pub fn init(&self) -> Result<()> {
        // Check if already initialized
        if self.jj_client.bookmark_exists(META_BOOKMARK)? {
            return Err("jjj is already initialized".into());
        }

        // Create an empty orphan root
        let change_id = self.jj_client.new_empty_change("Initialize jjj metadata")?;

        // Create the bookmark
        self.jj_client.create_bookmark(META_BOOKMARK, &change_id)?;

        // Checkout the meta bookmark to create the directory structure
        self.ensure_meta_checkout()?;

        // Create initial structure
        fs::create_dir_all(self.meta_path.join(TASKS_DIR))?;
        fs::create_dir_all(self.meta_path.join(REVIEWS_DIR))?;

        // Create default config
        let default_config = ProjectConfig::default();
        self.save_config(&default_config)?;

        // Commit the initial structure
        self.commit_changes("Initialize jjj structure")?;

        Ok(())
    }

    /// Ensure the metadata directory is checked out
    fn ensure_meta_checkout(&self) -> Result<()> {
        if !self.meta_path.exists() {
            fs::create_dir_all(&self.meta_path)?;
            // Checkout jjj/meta into the metadata path
            self.jj_client.checkout(META_BOOKMARK)?;
        }
        Ok(())
    }

    /// Get the path to the metadata directory
    pub fn meta_path(&self) -> &Path {
        &self.meta_path
    }

    /// Load project configuration
    pub fn load_config(&self) -> Result<ProjectConfig> {
        self.ensure_meta_checkout()?;

        let config_path = self.meta_path.join(CONFIG_FILE);
        if !config_path.exists() {
            return Ok(ProjectConfig::default());
        }

        let content = fs::read_to_string(config_path)?;
        let config: ProjectConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save project configuration
    pub fn save_config(&self, config: &ProjectConfig) -> Result<()> {
        self.ensure_meta_checkout()?;

        let config_path = self.meta_path.join(CONFIG_FILE);
        let content = toml::to_string_pretty(config)?;
        fs::write(config_path, content)?;

        Ok(())
    }

    /// Load a task by ID
    pub fn load_task(&self, task_id: &str) -> Result<Task> {
        self.ensure_meta_checkout()?;

        let task_path = self.meta_path.join(TASKS_DIR).join(format!("{}.json", task_id));
        if !task_path.exists() {
            return Err(JjjError::TaskNotFound(task_id.to_string()));
        }

        let content = fs::read_to_string(task_path)?;
        let task: Task = serde_json::from_str(&content)?;
        Ok(task)
    }

    /// Save a task
    pub fn save_task(&self, task: &Task) -> Result<()> {
        self.ensure_meta_checkout()?;

        let tasks_dir = self.meta_path.join(TASKS_DIR);
        fs::create_dir_all(&tasks_dir)?;

        let task_path = tasks_dir.join(format!("{}.json", task.id));
        let content = serde_json::to_string_pretty(task)?;
        fs::write(task_path, content)?;

        Ok(())
    }

    /// Delete a task
    pub fn delete_task(&self, task_id: &str) -> Result<()> {
        self.ensure_meta_checkout()?;

        let task_path = self.meta_path.join(TASKS_DIR).join(format!("{}.json", task_id));
        if !task_path.exists() {
            return Err(JjjError::TaskNotFound(task_id.to_string()));
        }

        fs::remove_file(task_path)?;
        Ok(())
    }

    /// List all tasks
    pub fn list_tasks(&self) -> Result<Vec<Task>> {
        self.ensure_meta_checkout()?;

        let tasks_dir = self.meta_path.join(TASKS_DIR);
        if !tasks_dir.exists() {
            return Ok(Vec::new());
        }

        let mut tasks = Vec::new();
        for entry in fs::read_dir(tasks_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(&path)?;
                if let Ok(task) = serde_json::from_str::<Task>(&content) {
                    tasks.push(task);
                }
            }
        }

        Ok(tasks)
    }

    /// Generate next task ID
    pub fn next_task_id(&self) -> Result<String> {
        let tasks = self.list_tasks()?;

        let max_id = tasks
            .iter()
            .filter_map(|task| {
                task.id.strip_prefix("T-").and_then(|s| s.parse::<u32>().ok())
            })
            .max()
            .unwrap_or(0);

        Ok(format!("T-{}", max_id + 1))
    }

    /// Load a review manifest
    pub fn load_review(&self, change_id: &str) -> Result<ReviewManifest> {
        self.ensure_meta_checkout()?;

        let review_path = self.meta_path
            .join(REVIEWS_DIR)
            .join(change_id)
            .join("manifest.toml");

        if !review_path.exists() {
            return Err(JjjError::ReviewNotFound(change_id.to_string()));
        }

        let content = fs::read_to_string(review_path)?;
        let manifest: ReviewManifest = toml::from_str(&content)?;
        Ok(manifest)
    }

    /// Save a review manifest
    pub fn save_review(&self, manifest: &ReviewManifest) -> Result<()> {
        self.ensure_meta_checkout()?;

        let review_dir = self.meta_path.join(REVIEWS_DIR).join(&manifest.change_id);
        fs::create_dir_all(&review_dir)?;

        let manifest_path = review_dir.join("manifest.toml");
        let content = toml::to_string_pretty(manifest)?;
        fs::write(manifest_path, content)?;

        Ok(())
    }

    /// List all review manifests
    pub fn list_reviews(&self) -> Result<Vec<ReviewManifest>> {
        self.ensure_meta_checkout()?;

        let reviews_dir = self.meta_path.join(REVIEWS_DIR);
        if !reviews_dir.exists() {
            return Ok(Vec::new());
        }

        let mut reviews = Vec::new();
        for entry in fs::read_dir(reviews_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let manifest_path = path.join("manifest.toml");
                if manifest_path.exists() {
                    let content = fs::read_to_string(&manifest_path)?;
                    if let Ok(manifest) = toml::from_str::<ReviewManifest>(&content) {
                        reviews.push(manifest);
                    }
                }
            }
        }

        Ok(reviews)
    }

    /// Load a comment
    pub fn load_comment(&self, change_id: &str, comment_id: &str) -> Result<Comment> {
        self.ensure_meta_checkout()?;

        let comment_path = self.meta_path
            .join(REVIEWS_DIR)
            .join(change_id)
            .join("comments")
            .join(format!("{}.json", comment_id));

        if !comment_path.exists() {
            return Err(format!("Comment {} not found", comment_id).into());
        }

        let content = fs::read_to_string(comment_path)?;
        let comment: Comment = serde_json::from_str(&content)?;
        Ok(comment)
    }

    /// Save a comment
    pub fn save_comment(&self, comment: &Comment) -> Result<()> {
        self.ensure_meta_checkout()?;

        let comments_dir = self.meta_path
            .join(REVIEWS_DIR)
            .join(&comment.target_change_id)
            .join("comments");

        fs::create_dir_all(&comments_dir)?;

        let comment_path = comments_dir.join(format!("{}.json", comment.id));
        let content = serde_json::to_string_pretty(comment)?;
        fs::write(comment_path, content)?;

        Ok(())
    }

    /// List all comments for a change
    pub fn list_comments(&self, change_id: &str) -> Result<Vec<Comment>> {
        self.ensure_meta_checkout()?;

        let comments_dir = self.meta_path
            .join(REVIEWS_DIR)
            .join(change_id)
            .join("comments");

        if !comments_dir.exists() {
            return Ok(Vec::new());
        }

        let mut comments = Vec::new();
        for entry in fs::read_dir(comments_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(&path)?;
                if let Ok(comment) = serde_json::from_str::<Comment>(&content) {
                    comments.push(comment);
                }
            }
        }

        // Sort by timestamp
        comments.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        Ok(comments)
    }

    /// Generate next comment ID
    pub fn next_comment_id(&self, change_id: &str) -> Result<String> {
        let comments = self.list_comments(change_id)?;

        let max_id = comments
            .iter()
            .filter_map(|comment| {
                comment.id.strip_prefix("c-").and_then(|s| s.parse::<u32>().ok())
            })
            .max()
            .unwrap_or(0);

        Ok(format!("c-{}", max_id + 1))
    }

    /// Commit changes to the metadata
    fn commit_changes(&self, message: &str) -> Result<()> {
        // Save current working change
        let current_change = self.jj_client.current_change_id()?;

        // Switch to jjj/meta bookmark
        self.jj_client.checkout(META_BOOKMARK)?;

        // Create a new change on top of jjj/meta
        let meta_change = self.jj_client.new_empty_change(message)?;

        // The files are already written to disk in the save_* methods
        // jj will automatically track them in the new change

        // Update the bookmark to point to the new change
        self.jj_client.execute(&["bookmark", "set", META_BOOKMARK, "-r", &meta_change])?;

        // Switch back to the original working change
        self.jj_client.checkout(&current_change)?;

        Ok(())
    }

    /// Perform an operation on the metadata and commit
    pub fn with_metadata<F, R>(&self, message: &str, operation: F) -> Result<R>
    where
        F: FnOnce() -> Result<R>,
    {
        let result = operation()?;
        self.commit_changes(message)?;
        Ok(result)
    }
}
