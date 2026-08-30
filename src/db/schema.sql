-- jjj SQLite schema v13
-- Runtime cache for fast queries and full-text search

-- Meta table for schema versioning and sync state
CREATE TABLE IF NOT EXISTS meta (
    key TEXT PRIMARY KEY,
    value TEXT
);

-- Problems table
CREATE TABLE IF NOT EXISTS problems (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open',
    priority TEXT NOT NULL DEFAULT 'medium',
    confidence TEXT NOT NULL DEFAULT 'unknown',
    parent_id TEXT,
    milestone_id TEXT,
    assignee TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    description TEXT DEFAULT '',
    dissolved_reason TEXT,
    github_issue INTEGER,
    tags TEXT DEFAULT '[]',
    -- When the claim was taken. A claim is a lease: without this the cached
    -- read cannot tell a live claim from one held by an agent that died an hour
    -- ago, and `problem list` was silently returning None for it while the
    -- markdown on disk had the timestamp all along.
    claimed_at TEXT,
    FOREIGN KEY (parent_id) REFERENCES problems(id),
    FOREIGN KEY (milestone_id) REFERENCES milestones(id)
);

-- Solutions table
CREATE TABLE IF NOT EXISTS solutions (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'proposed',
    problem_id TEXT NOT NULL,
    change_ids TEXT DEFAULT '[]',  -- JSON array
    supersedes TEXT,
    assignee TEXT,
    force_approved INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    approach TEXT DEFAULT '',
    github_pr INTEGER,
    github_branch TEXT,
    tags TEXT DEFAULT '[]',
    -- See the note on problems.claimed_at: a claim is a lease, and the cached
    -- read has to carry its age or nobody can tell a live one from a stale one.
    claimed_at TEXT,
    -- Findings this conjecture rests on (JSON array of finding ids). Without a
    -- machine-readable citation, "the evidence for this" lives only in prose and
    -- nothing can tell whether a measurement was ever used.
    cites TEXT DEFAULT '[]',
    FOREIGN KEY (problem_id) REFERENCES problems(id),
    FOREIGN KEY (supersedes) REFERENCES solutions(id)
);

-- Critiques table
CREATE TABLE IF NOT EXISTS critiques (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open',
    solution_id TEXT NOT NULL,
    severity TEXT NOT NULL DEFAULT 'medium',
    reviewer TEXT,
    author TEXT,
    file_path TEXT,
    line_number INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    argument TEXT DEFAULT '',
    replies TEXT DEFAULT '[]',  -- JSON array
    github_review_id INTEGER,
    -- Full-fidelity code-location fields so the critique row is lossless and
    -- the cache is authoritative for reads (Pillar 4). `line_number` above holds
    -- line_start for backward compatibility; line_end is stored separately.
    line_end INTEGER,
    code_context TEXT DEFAULT '[]',    -- JSON array
    context_before TEXT DEFAULT '[]',  -- JSON array
    context_after TEXT DEFAULT '[]',   -- JSON array
    -- Findings this refutation rests on. See solutions.cites.
    cites TEXT DEFAULT '[]',
    FOREIGN KEY (solution_id) REFERENCES solutions(id)
);

-- Milestones table
CREATE TABLE IF NOT EXISTS milestones (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'planning',
    target_date TEXT,
    assignee TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    description TEXT DEFAULT '',
    problem_ids TEXT DEFAULT '[]'  -- JSON array
);

-- Findings table: evidence about a problem.
--
-- jjj modelled conjectures and refutations but not the observations that
-- motivate them, so investigations were filed as solutions and then withdrawn
-- as "documented, not fixed". A finding has no approval state — a measurement
-- is cited or contradicted, never accepted.
CREATE TABLE IF NOT EXISTS findings (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'current',
    problem_id TEXT NOT NULL,
    author TEXT,
    superseded_by TEXT,
    refs TEXT DEFAULT '[]',   -- JSON array of related entity ids
    method TEXT,
    tags TEXT DEFAULT '[]',   -- JSON array
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    evidence TEXT DEFAULT '',
    FOREIGN KEY (problem_id) REFERENCES problems(id)
);

-- Events table for decision logging
CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    event_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    actor TEXT,
    rationale TEXT,
    refs TEXT DEFAULT '[]',    -- JSON array of related entity IDs
    extra TEXT DEFAULT '{}'    -- JSON object for type-specific data
);

-- Full-text search virtual table
CREATE VIRTUAL TABLE IF NOT EXISTS fts USING fts5(
    entity_type,
    entity_id,
    title,
    body,
    tokenize = 'porter ascii'
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);
CREATE INDEX IF NOT EXISTS idx_events_event_type ON events(event_type);
CREATE INDEX IF NOT EXISTS idx_events_entity_id ON events(entity_id);
CREATE INDEX IF NOT EXISTS idx_solutions_problem_id ON solutions(problem_id);
CREATE INDEX IF NOT EXISTS idx_critiques_solution_id ON critiques(solution_id);
CREATE INDEX IF NOT EXISTS idx_problems_milestone_id ON problems(milestone_id);
CREATE INDEX IF NOT EXISTS idx_problems_parent_id ON problems(parent_id);
CREATE INDEX IF NOT EXISTS idx_problems_github_issue ON problems(github_issue);
CREATE INDEX IF NOT EXISTS idx_solutions_github_pr ON solutions(github_pr);
CREATE INDEX IF NOT EXISTS idx_critiques_github_review_id ON critiques(github_review_id);
CREATE INDEX IF NOT EXISTS idx_findings_problem_id ON findings(problem_id);

-- Per-file content fingerprints, used by the incremental push-validation
-- reload to skip re-parsing markdown that has not changed since the cache
-- last saw it (see db::sync::load_from_markdown_incremental).
CREATE TABLE IF NOT EXISTS content_cache (
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    PRIMARY KEY (entity_type, entity_id)
);

-- Embeddings table for semantic search
CREATE TABLE IF NOT EXISTS embeddings (
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    model TEXT NOT NULL,
    dimensions INTEGER NOT NULL,
    embedding BLOB NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (entity_type, entity_id)
);

CREATE INDEX IF NOT EXISTS idx_embeddings_model ON embeddings(model);
