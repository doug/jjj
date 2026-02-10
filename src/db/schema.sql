-- jjj SQLite schema v1
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
    parent_id TEXT,
    milestone_id TEXT,
    assignee TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    description TEXT DEFAULT '',
    context TEXT DEFAULT '',
    dissolved_reason TEXT,
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
    force_accepted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    approach TEXT DEFAULT '',
    tradeoffs TEXT DEFAULT '',
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
    file_path TEXT,
    line_number INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    body TEXT DEFAULT '',
    replies TEXT DEFAULT '[]',  -- JSON array
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
    content='',
    contentless_delete=1
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);
CREATE INDEX IF NOT EXISTS idx_events_event_type ON events(event_type);
CREATE INDEX IF NOT EXISTS idx_events_entity_id ON events(entity_id);
CREATE INDEX IF NOT EXISTS idx_solutions_problem_id ON solutions(problem_id);
CREATE INDEX IF NOT EXISTS idx_critiques_solution_id ON critiques(solution_id);
CREATE INDEX IF NOT EXISTS idx_problems_milestone_id ON problems(milestone_id);
CREATE INDEX IF NOT EXISTS idx_problems_parent_id ON problems(parent_id);
