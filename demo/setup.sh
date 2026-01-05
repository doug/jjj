#!/usr/bin/env bash
#
# Demo Setup Script for jjj
# This script creates a demo jj repository to test jjj functionality
#
# Prerequisites:
# - jj (Jujutsu) must be installed
# - jjj must be built (run `cargo build --release` in the parent directory)

set -euo pipefail

DEMO_DIR="demo-repo"
JJJ_BIN="../target/release/jjj"

echo "🔧 Setting up jjj demo environment..."
echo

# Check if jj is installed
if ! command -v jj &> /dev/null; then
    echo "❌ Error: jj (Jujutsu) is not installed or not in PATH"
    echo "   Install from: https://github.com/martinvonz/jj"
    exit 1
fi

# Check if jjj binary exists
if [ ! -f "$JJJ_BIN" ]; then
    echo "❌ Error: jjj binary not found at $JJJ_BIN"
    echo "   Please run: cargo build --release"
    exit 1
fi

# Remove existing demo repo if it exists
if [ -d "$DEMO_DIR" ]; then
    echo "🗑️  Removing existing demo repository..."
    rm -rf "$DEMO_DIR"
fi

# Create demo directory
mkdir -p "$DEMO_DIR"
cd "$DEMO_DIR"

echo "📦 Initializing jj repository..."
jj git init --colocate

# Configure git/jj user for the demo
jj config set --repo user.name "Demo User"
jj config set --repo user.email "demo@example.com"

echo "✅ Repository initialized"
echo

# Create some initial files
echo "📝 Creating initial project files..."

cat > README.md <<'EOF'
# Demo Project

This is a demo project for testing jjj (Jujutsu Project Manager).

## Features

- User authentication
- API endpoints
- Database layer
EOF

cat > src/main.rs <<'EOF'
fn main() {
    println!("Hello, demo!");
}
EOF

cat > src/auth.rs <<'EOF'
pub fn authenticate(user: &str, password: &str) -> bool {
    // TODO: Implement actual authentication
    !user.is_empty() && !password.is_empty()
}
EOF

cat > src/api.rs <<'EOF'
pub fn handle_request(path: &str) -> String {
    match path {
        "/" => "Welcome!".to_string(),
        "/api/status" => "OK".to_string(),
        _ => "Not found".to_string(),
    }
}
EOF

mkdir -p src
jj describe -m "Initial commit: Basic project structure"

echo "✅ Initial files created"
echo

# Create first feature change
echo "🔨 Creating feature changes..."

cat >> src/auth.rs <<'EOF'

pub fn hash_password(password: &str) -> String {
    // Simple hash for demo purposes
    format!("hashed_{}", password)
}
EOF

jj describe -m "Add password hashing function"
CHANGE1_ID=$(jj log -r @ -T change_id --no-graph)

# Create another change
jj new
cat >> src/api.rs <<'EOF'

pub fn get_user(id: u64) -> Option<String> {
    // TODO: Fetch from database
    if id == 1 {
        Some("Alice".to_string())
    } else {
        None
    }
}
EOF

jj describe -m "Add user lookup API endpoint"
CHANGE2_ID=$(jj log -r @ -T change_id --no-graph)

# Create one more change
jj new
cat > src/db.rs <<'EOF'
pub struct Database {
    connection: String,
}

impl Database {
    pub fn new(url: &str) -> Self {
        Self {
            connection: url.to_string(),
        }
    }

    pub fn query(&self, sql: &str) -> Vec<String> {
        println!("Executing: {}", sql);
        vec![]
    }
}
EOF

jj describe -m "Add database layer"
CHANGE3_ID=$(jj log -r @ -T change_id --no-graph)

echo "✅ Feature changes created"
echo

# Initialize jjj
echo "🎯 Initializing jjj..."
"$JJJ_BIN" init

echo "✅ jjj initialized"
echo

echo "✨ Demo environment setup complete!"
echo
echo "📍 Location: $(pwd)"
echo
echo "🎮 Next steps:"
echo "   1. cd $DEMO_DIR"
echo "   2. Run the demo commands in demo-commands.sh"
echo
echo "💡 Or try these commands:"
echo "   $JJJ_BIN board                  # View the Kanban board"
echo "   $JJJ_BIN task new \"Fix auth bug\" --tag backend"
echo "   $JJJ_BIN task list"
echo "   $JJJ_BIN review request @alice"
echo "   $JJJ_BIN dashboard"
echo
