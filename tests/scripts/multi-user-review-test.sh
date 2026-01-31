#!/bin/bash
#
# Multi-User Code Review Integration Test
#
# This script simulates a complete code review workflow between two users
# (Alice and Bob) using separate working directories to simulate different
# machines/checkouts.
#
# Usage: ./multi-user-review-test.sh [path-to-jjj-binary]
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Get jjj binary path
JJJ_BIN="${1:-$(which jjj 2>/dev/null || echo "./target/debug/jjj")}"

if [[ ! -x "$JJJ_BIN" ]]; then
    echo -e "${RED}Error: jjj binary not found at $JJJ_BIN${NC}"
    echo "Usage: $0 [path-to-jjj-binary]"
    exit 1
fi

# Check for jj
if ! command -v jj &> /dev/null; then
    echo -e "${RED}Error: jj (Jujutsu) is not installed${NC}"
    exit 1
fi

echo -e "${BLUE}Using jjj binary: $JJJ_BIN${NC}"

# Create test directory
TEST_DIR=$(mktemp -d)
echo -e "${BLUE}Test directory: $TEST_DIR${NC}"

cleanup() {
    echo -e "\n${YELLOW}Cleaning up...${NC}"
    rm -rf "$TEST_DIR"
}
trap cleanup EXIT

# Setup directories
ORIGIN_DIR="$TEST_DIR/origin.git"
ALICE_DIR="$TEST_DIR/alice"
BOB_DIR="$TEST_DIR/bob"

# Helper functions
alice() {
    echo -e "\n${GREEN}[ALICE]${NC} $*"
    (cd "$ALICE_DIR" && "$@")
}

bob() {
    echo -e "\n${YELLOW}[BOB]${NC} $*"
    (cd "$BOB_DIR" && "$@")
}

jjj_alice() {
    alice "$JJJ_BIN" "$@"
}

jjj_bob() {
    bob "$JJJ_BIN" "$@"
}

# Sync metadata between users via git (works around jj push limitations)
# jjj uses a separate workspace for metadata, so we need to ensure that workspace is updated
sync_alice_to_bob() {
    echo -e "  ${BLUE}Syncing metadata: Alice -> Origin -> Bob${NC}"
    # Push from Alice
    (cd "$ALICE_DIR" && git push origin jjj/meta --force 2>/dev/null || true)
    # Fetch to Bob and update local branch
    (cd "$BOB_DIR" && git fetch origin jjj/meta:jjj/meta --force 2>/dev/null || true)
    # Import into jj
    (cd "$BOB_DIR" && jj git import 2>/dev/null || true)
    # Create a new commit based on the updated bookmark (workaround for immutability)
    # Only if the workspace exists (it gets created on first jjj command)
    if [[ -d "$BOB_DIR/.jj/jjj-meta" ]]; then
        (cd "$BOB_DIR/.jj/jjj-meta" && jj new "jjj/meta" 2>/dev/null || true)
    fi
}

sync_bob_to_alice() {
    echo -e "  ${BLUE}Syncing metadata: Bob -> Origin -> Alice${NC}"
    # Push from Bob
    (cd "$BOB_DIR" && git push origin jjj/meta --force 2>/dev/null || true)
    # Fetch to Alice and update local branch
    (cd "$ALICE_DIR" && git fetch origin jjj/meta:jjj/meta --force 2>/dev/null || true)
    # Import into jj
    (cd "$ALICE_DIR" && jj git import 2>/dev/null || true)
    # Create a new commit based on the updated bookmark (workaround for immutability)
    # Only if the workspace exists (it gets created on first jjj command)
    if [[ -d "$ALICE_DIR/.jj/jjj-meta" ]]; then
        (cd "$ALICE_DIR/.jj/jjj-meta" && jj new "jjj/meta" 2>/dev/null || true)
    fi
}

assert_contains() {
    local output="$1"
    local expected="$2"
    local message="$3"
    if [[ "$output" == *"$expected"* ]]; then
        echo -e "  ${GREEN}✓${NC} $message"
    else
        echo -e "  ${RED}✗${NC} $message"
        echo -e "  ${RED}Expected to find: $expected${NC}"
        echo -e "  ${RED}In output: $output${NC}"
        exit 1
    fi
}

assert_not_contains() {
    local output="$1"
    local unexpected="$2"
    local message="$3"
    if [[ "$output" != *"$unexpected"* ]]; then
        echo -e "  ${GREEN}✓${NC} $message"
    else
        echo -e "  ${RED}✗${NC} $message"
        echo -e "  ${RED}Did not expect to find: $unexpected${NC}"
        echo -e "  ${RED}In output: $output${NC}"
        exit 1
    fi
}

section() {
    echo -e "\n${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
}

# ============================================================================
section "SETUP: Creating shared repository"
# ============================================================================

# Create bare origin repo
echo "Creating bare origin repository..."
git init --bare "$ORIGIN_DIR"

# Create Alice's working directory
echo "Creating Alice's working directory..."
mkdir -p "$ALICE_DIR"
cd "$ALICE_DIR"
git init
git config user.name "alice"
git config user.email "alice@example.com"
echo "# Project" > README.md
git add README.md
git commit -m "Initial commit"
git remote add origin "$ORIGIN_DIR"
git push -u origin main

# Initialize jj for Alice (colocated)
jj git init --colocate
jj bookmark track main@origin 2>/dev/null || jj bookmark track main --remote origin 2>/dev/null || true

# Initialize jjj for Alice
jjj_alice init

# Push metadata using git directly
echo "Pushing jjj/meta via git..."
(cd "$ALICE_DIR" && git push origin jjj/meta 2>/dev/null || true)

# Create Bob's working directory (clone)
echo "Creating Bob's working directory..."
git clone "$ORIGIN_DIR" "$BOB_DIR"
cd "$BOB_DIR"
git config user.name "bob"
git config user.email "bob@example.com"

# Initialize jj for Bob (colocated)
jj git init --colocate

# Fetch jjj/meta and set up local bookmark
(cd "$BOB_DIR" && git fetch origin jjj/meta && git checkout -b jjj/meta FETCH_HEAD 2>/dev/null && git checkout main || true)

# Import the bookmark into jj
bob jj git import

# Initialize jjj for Bob (will detect existing metadata)
jjj_bob init 2>/dev/null || true

echo -e "\n${GREEN}Setup complete!${NC}"

# ============================================================================
section "STEP 1: Alice creates a problem"
# ============================================================================

OUTPUT=$(jjj_alice problem new "Add user authentication")
assert_contains "$OUTPUT" "p1" "Problem p1 created"

# Sync metadata
sync_alice_to_bob

# ============================================================================
section "STEP 2: Alice creates a solution and requests Bob's review"
# ============================================================================

OUTPUT=$(jjj_alice solution new "JWT-based authentication" --problem p1 --reviewer bob)
assert_contains "$OUTPUT" "s1" "Solution s1 created"
assert_contains "$OUTPUT" "Awaiting review" "Review request mentioned"
assert_contains "$OUTPUT" "@bob" "Bob mentioned as reviewer"

# Check that a review critique was created
OUTPUT=$(jjj_alice critique list --solution s1)
assert_contains "$OUTPUT" "c1" "Critique c1 created"
assert_contains "$OUTPUT" "Awaiting review from @bob" "Review critique has correct title"

# Sync metadata
sync_alice_to_bob

# ============================================================================
section "STEP 3: Bob fetches and sees his review queue"
# ============================================================================

# Bob checks his status - sees the review request (may be REVIEW or BLOCKED depending on view)
OUTPUT=$(jjj_bob status)
assert_contains "$OUTPUT" "Awaiting review from @bob" "Bob sees the review request"
# The critique info is shown regardless of category
assert_contains "$OUTPUT" "c1" "Bob sees critique c1"

# ============================================================================
section "STEP 4: Bob reviews and finds an issue"
# ============================================================================

# Bob examines the solution
OUTPUT=$(jjj_bob solution show s1)
assert_contains "$OUTPUT" "JWT-based authentication" "Bob can see solution details"

# Bob creates a critique
OUTPUT=$(jjj_bob critique new s1 "Token expiration is too long - should be 15 minutes not 24 hours" --severity high)
assert_contains "$OUTPUT" "c2" "Critique c2 created"
assert_contains "$OUTPUT" "high" "Severity is high"

# Sync metadata
sync_bob_to_alice

# ============================================================================
section "STEP 5: Alice sees the critique"
# ============================================================================

# Alice checks her status
OUTPUT=$(jjj_alice status)
assert_contains "$OUTPUT" "BLOCKED" "Alice sees solution is blocked"
assert_contains "$OUTPUT" "Token expiration" "Alice sees Bob's critique"

# Alice can see both critiques
OUTPUT=$(jjj_alice critique list --solution s1)
assert_contains "$OUTPUT" "c1" "Review critique visible"
assert_contains "$OUTPUT" "c2" "Issue critique visible"

# ============================================================================
section "STEP 6: Alice addresses the critique"
# ============================================================================

# Alice fixes the issue (simulated by just marking it addressed)
OUTPUT=$(jjj_alice critique address c2)
assert_contains "$OUTPUT" "addressed" "Critique marked as addressed"

# Sync metadata
sync_alice_to_bob

# ============================================================================
section "STEP 7: Bob verifies the fix and completes his review"
# ============================================================================

# Bob checks the critique status
OUTPUT=$(jjj_bob critique list --solution s1)
assert_contains "$OUTPUT" "addressed" "c2 shows as addressed"
assert_contains "$OUTPUT" "open" "c1 still open (review not complete)"

# Bob is satisfied with the fix and completes his review by dismissing c1
OUTPUT=$(jjj_bob critique dismiss c1)
assert_contains "$OUTPUT" "dismissed" "Review critique dismissed (LGTM)"

# Sync metadata
sync_bob_to_alice

# ============================================================================
section "STEP 8: Alice accepts the solution"
# ============================================================================

# Alice checks status - should show ready
OUTPUT=$(jjj_alice status)
assert_contains "$OUTPUT" "READY" "Solution shows as ready"

# Alice can now accept
OUTPUT=$(jjj_alice solution accept s1 <<< "n")
assert_contains "$OUTPUT" "accepted" "Solution accepted"

# Verify the solution is accepted
OUTPUT=$(jjj_alice solution show s1)
assert_contains "$OUTPUT" "accepted" "Solution status is accepted"

# ============================================================================
section "VERIFICATION: Check final state"
# ============================================================================

echo "Checking final critique states..."

OUTPUT=$(jjj_alice critique list --solution s1)
assert_contains "$OUTPUT" "dismissed" "c1 is dismissed"
assert_contains "$OUTPUT" "addressed" "c2 is addressed"
assert_not_contains "$OUTPUT" "?open" "No open critiques"

echo -e "\n${GREEN}════════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}ALL TESTS PASSED!${NC}"
echo -e "${GREEN}════════════════════════════════════════════════════════════════${NC}"

echo -e "\nThe multi-user code review workflow completed successfully:"
echo "  1. Alice created problem p1"
echo "  2. Alice created solution s1 with review request for Bob"
echo "  3. Bob saw the review request in his queue"
echo "  4. Bob raised critique c2 about token expiration"
echo "  5. Alice saw and addressed the critique"
echo "  6. Bob verified and completed his review (dismissed c1)"
echo "  7. Alice accepted the solution"
echo ""
echo "This demonstrates the unified critique model where:"
echo "  - Review requests are critiques with a reviewer field"
echo "  - Multiple users can coordinate via shared jjj/meta bookmark"
echo "  - Sign-off is expressed by dismissing the review critique"
