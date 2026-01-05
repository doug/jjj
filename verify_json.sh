#!/bin/bash
set -e

# Create a temp directory
TEST_DIR=$(mktemp -d)
echo "Running tests in $TEST_DIR"
cd "$TEST_DIR"

# Initialize jj repo
jj git init .

# Path to jjj binary
JJJ_BIN="/Users/doug/src/jjj/target/debug/jjj"

# Initialize jjj
"$JJJ_BIN" init

# Create a task
"$JJJ_BIN" task new "Test Task" --feature "F-1" --tag "test"

# Verify task list JSON
echo "Verifying task list JSON..."
"$JJJ_BIN" task list --json > tasks.json
cat tasks.json
if ! grep -q "Test Task" tasks.json; then
    echo "Error: Task not found in JSON output"
    exit 1
fi

# Verify board JSON
echo "Verifying board JSON..."
"$JJJ_BIN" board --json > board.json
cat board.json
if ! grep -q "TODO" board.json; then
    echo "Error: Column not found in JSON output"
    exit 1
fi

# Verify dashboard JSON
echo "Verifying dashboard JSON..."
"$JJJ_BIN" dashboard --json > dashboard.json
cat dashboard.json
if ! grep -q "my_tasks" dashboard.json; then
    echo "Error: my_tasks field not found in JSON output"
    exit 1
fi

echo "All JSON tests passed!"
rm -rf "$TEST_DIR"
