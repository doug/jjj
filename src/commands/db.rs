use crate::cli::DbAction;
use crate::context::CommandContext;
use crate::db::{self, Database};
use crate::embeddings::EmbeddingClient;
use crate::error::Result;
use crate::local_config::LocalConfig;

pub fn execute(ctx: &CommandContext, action: DbAction) -> Result<()> {
    match action {
        DbAction::Status => status(ctx),
        DbAction::Rebuild => rebuild(ctx),
    }
}

fn status(ctx: &CommandContext) -> Result<()> {
    let jj_client = ctx.jj();
    let repo_root = jj_client.repo_root();
    let db_path = repo_root.join(".jj").join("jjj.db");

    if !db_path.exists() {
        println!("Database: not initialized");
        println!("Run any jjj command to initialize, or 'jjj db rebuild' to create.");
        return Ok(());
    }

    let db = Database::open(&db_path)?;
    let conn = db.conn();

    // Get schema version
    let version: String = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "unknown".to_string());

    // Count entities
    let problems: i64 = conn.query_row("SELECT COUNT(*) FROM problems", [], |row| row.get(0))?;
    let solutions: i64 = conn.query_row("SELECT COUNT(*) FROM solutions", [], |row| row.get(0))?;
    let critiques: i64 = conn.query_row("SELECT COUNT(*) FROM critiques", [], |row| row.get(0))?;
    let milestones: i64 =
        conn.query_row("SELECT COUNT(*) FROM milestones", [], |row| row.get(0))?;

    // Count FTS documents
    let fts_count: i64 = conn.query_row("SELECT COUNT(*) FROM fts", [], |row| row.get(0))?;

    // Get embedding info
    let (embedding_count, embedding_model) = {
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM embeddings", [], |row| row.get(0))?;
        let model: Option<String> = conn
            .query_row("SELECT model FROM embeddings LIMIT 1", [], |row| row.get(0))
            .ok();
        (count, model)
    };

    // Check dirty flag
    let dirty: bool = conn
        .query_row("SELECT value FROM meta WHERE key = 'dirty'", [], |row| {
            let v: String = row.get(0)?;
            Ok(v == "true" || v == "1")
        })
        .unwrap_or(false);

    // Print status
    println!("Database: {}", db_path.display());
    println!("Schema version: v{}", version);
    println!(
        "Entities: {} problems, {} solutions, {} critiques, {} milestones",
        problems, solutions, critiques, milestones
    );
    println!("FTS index: {} documents", fts_count);

    let total_entities = problems + solutions + critiques + milestones;
    if let Some(model) = embedding_model {
        println!(
            "Embeddings: {}/{} (model: {})",
            embedding_count, total_entities, model
        );
    } else {
        println!("Embeddings: none");
    }

    println!(
        "Sync status: {}",
        if dirty {
            "dirty (uncommitted changes)"
        } else {
            "clean"
        }
    );

    Ok(())
}

fn rebuild(ctx: &CommandContext) -> Result<()> {
    let jj_client = ctx.jj();
    let repo_root = jj_client.repo_root();
    let db_path = repo_root.join(".jj").join("jjj.db");

    // Delete existing database to force full rebuild
    if db_path.exists() {
        std::fs::remove_file(&db_path)?;
    }

    let db = Database::open(&db_path)?;

    println!("Loading from markdown...");
    db::load_from_markdown(&db, &ctx.store)?;

    println!("Rebuilding FTS index...");
    db::rebuild_fts(&db)?;

    // Try to rebuild embeddings
    let local_config = LocalConfig::load(repo_root);
    let embedding_client =
        EmbeddingClient::from_config(&local_config, local_config.embeddings_explicitly_enabled());

    if let Some(client) = embedding_client {
        println!("Rebuilding embeddings (model: {})...", client.model());
        db::rebuild_embeddings(&db, &client)?;
        let (count, _) = db::count_embeddings(db.conn(), Some(client.model()))?;
        println!("  {} embeddings computed", count);
    } else {
        println!("Embeddings: skipped (no embedding service available)");
    }

    println!("Done!");
    Ok(())
}
