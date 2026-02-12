#!/usr/bin/env bash
#
# Interactive Demo Script for jjj
# This script demonstrates the main features of jjj
#
# Run this after executing setup.sh

set -euo pipefail

JJJ="../target/release/jjj"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

function print_section() {
    echo
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo
}

function run_command() {
    echo -e "${GREEN}$ $@${NC}"
    echo
    "$@"
    echo
}

function pause() {
    echo -e "${YELLOW}Press Enter to continue...${NC}"
    read
}

echo
echo "🎯 jjj Interactive Demo"
echo "======================"
echo
echo "This demo showcases jjj's core functionality:"
echo "  - Task management with Kanban board"
echo "  - Code review workflow"
echo "  - Change tracking"
echo
pause

# ============================================================================
print_section "1. TASK MANAGEMENT: Creating Tasks"

echo "Let's create some tasks to track our work:"
echo

run_command $JJJ task new "Implement user authentication" --tag backend --tag security

run_command $JJJ task new "Add password reset API" --tag backend --tag api --column TODO

run_command $JJJ task new "Update login UI" --tag frontend --column TODO

run_command $JJJ task new "Write API documentation" --tag docs --column TODO

echo "✅ Created 4 tasks"
pause

# ============================================================================
print_section "2. VIEWING THE KANBAN BOARD"

echo "View all tasks on the Kanban board:"
echo

run_command $JJJ board

pause

# ============================================================================
print_section "3. LISTING AND FILTERING TASKS"

echo "List all tasks:"
echo

run_command $JJJ task list

echo "Filter tasks by tag:"
echo

run_command $JJJ task list --tag backend

pause

# ============================================================================
print_section "4. WORKING ON A TASK"

echo "Let's start working on a task:"
echo

echo "First, let's see what changes we have:"
echo

run_command jj log -r 'all()' -T 'change_id.short() ++ " " ++ description'

echo
echo "Now let's attach our current change to task T-1:"
echo

run_command $JJJ task attach T-1

echo "Move the task to 'In Progress':"
echo

run_command $JJJ task move T-1 "In Progress"

echo "View the updated board:"
echo

run_command $JJJ board

pause

# ============================================================================
print_section "5. TASK DETAILS"

echo "View detailed information about a task:"
echo

run_command $JJJ task show T-1

pause

# ============================================================================
print_section "6. CODE REVIEW: Requesting a Review"

echo "Request a code review for the current change:"
echo

run_command $JJJ review request alice bob

echo "✅ Review requested!"
pause

# ============================================================================
print_section "7. LISTING REVIEWS"

echo "See all pending reviews:"
echo

run_command $JJJ review list

pause

# ============================================================================
print_section "8. ADDING REVIEW COMMENTS"

echo "Let's add some comments to the review:"
echo

# Get current change ID
CHANGE_ID=$(jj log -r @ -T change_id --no-graph | tr -d '\n')

echo "Adding a general comment:"
echo

run_command $JJJ review comment "$CHANGE_ID" --body "Overall structure looks good!"

echo "Adding an inline comment (simulated):"
echo
echo -e "${GREEN}$ $JJJ review comment $CHANGE_ID --file src/auth.rs --line 42 --body 'Consider using a stronger hashing algorithm'${NC}"
echo
echo "(This would work if src/auth.rs existed at line 42)"
echo

pause

# ============================================================================
print_section "9. REVIEW STATUS"

echo "Check the status of our review:"
echo

run_command $JJJ review status "$CHANGE_ID"

pause

# ============================================================================
print_section "10. APPROVING A REVIEW"

echo "As a reviewer, approve the change:"
echo

run_command $JJJ review approve "$CHANGE_ID"

echo "Check status again:"
echo

run_command $JJJ review status "$CHANGE_ID"

pause

# ============================================================================
print_section "11. DASHBOARD VIEW"

echo "View your personal dashboard:"
echo

run_command $JJJ dashboard

pause

# ============================================================================
print_section "12. ADVANCED: Moving Tasks Through Workflow"

echo "Let's simulate a complete workflow:"
echo

echo "Create a new task for the database layer:"
echo
run_command $JJJ task new "Add database connection pooling" --tag backend --tag database

echo "Move it to 'In Progress':"
echo
run_command $JJJ task move T-5 "In Progress"

echo "Then to 'Review':"
echo
run_command $JJJ task move T-5 "Review"

echo "Finally to 'Done':"
echo
run_command $JJJ task move T-5 "Done"

echo "View the updated board:"
echo
run_command $JJJ board

pause

# ============================================================================
print_section "13. EDITING TASKS"

echo "Update a task's title and tags:"
echo

run_command $JJJ task edit T-2 --title "Enhanced password reset API with email" --add-tag notifications

echo "View the changes:"
echo

run_command $JJJ task show T-2

pause

# ============================================================================
print_section "14. FILTERING BY COLUMN"

echo "See all completed tasks:"
echo

run_command $JJJ task list --column Done

echo "See tasks in progress:"
echo

run_command $JJJ task list --column "In Progress"

pause

# ============================================================================
print_section "15. EXPLORING THE METADATA"

echo "jjj stores all data in the jjj bookmark."
echo "Let's explore what's there:"
echo

run_command jj bookmark list

echo
echo "The jjj bookmark contains all tasks and reviews."
echo "This data is synced with 'jj push/pull' just like code!"
echo

pause

# ============================================================================
print_section "DEMO COMPLETE! 🎉"

echo "You've explored the main features of jjj:"
echo
echo "  ✅ Created and managed tasks"
echo "  ✅ Organized work with a Kanban board"
echo "  ✅ Requested and managed code reviews"
echo "  ✅ Added review comments"
echo "  ✅ Tracked task progress"
echo "  ✅ Used the personal dashboard"
echo
echo "Key concepts:"
echo "  • All metadata is stored in the jjj bookmark"
echo "  • Tasks and reviews are version-controlled files"
echo "  • Everything is distributed and offline-capable"
echo "  • Changes are tracked by stable Change IDs"
echo
echo "Try exploring more commands:"
echo "  $JJJ task --help"
echo "  $JJJ review --help"
echo "  $JJJ board"
echo
echo "To reset the demo:"
echo "  cd .."
echo "  rm -rf demo-repo"
echo "  ./setup.sh"
echo
