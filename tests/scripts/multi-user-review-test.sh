#!/bin/bash
#
# Multi-User Code Review Integration Test
#
# This script simulates a complete code review workflow between two users
# (Alice and Bob) using separate working directories to simulate different
# machines/checkouts. It includes actual code changes and line-specific critiques.
#
# Usage: ./multi-user-review-test.sh [path-to-jjj-binary]
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
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
    echo -e "  ${CYAN}Syncing: Alice -> Origin -> Bob${NC}"
    (cd "$ALICE_DIR" && "$JJJ_BIN" push --no-prompt 2>/dev/null) || true
    (cd "$BOB_DIR" && "$JJJ_BIN" fetch 2>/dev/null) || true
}

sync_bob_to_alice() {
    echo -e "  ${CYAN}Syncing: Bob -> Origin -> Alice${NC}"
    (cd "$BOB_DIR" && "$JJJ_BIN" push --no-prompt 2>/dev/null) || true
    (cd "$ALICE_DIR" && "$JJJ_BIN" fetch 2>/dev/null) || true
}

# Sync code changes between users
# With jj colocated repos, we push a bookmark pointing to Alice's code commit
# Bob can then view the code using `jj file show -r <bookmark>`
sync_code_alice_to_bob() {
    echo -e "  ${CYAN}Syncing code: Alice -> Origin -> Bob${NC}"
    # Create a bookmark for Alice's code commit and push it
    (cd "$ALICE_DIR" && jj bookmark set code-review -r @ 2>/dev/null || true)
    (cd "$ALICE_DIR" && jj git push -b code-review --allow-new 2>/dev/null || true)
    # Bob fetches and checks out the code
    (cd "$BOB_DIR" && jj git fetch 2>/dev/null || true)
    (cd "$BOB_DIR" && jj new code-review@origin 2>/dev/null || true)
}

sync_code_bob_to_alice() {
    echo -e "  ${CYAN}Syncing code: Bob -> Origin -> Alice${NC}"
    # Same for Bob
    (cd "$BOB_DIR" && jj bookmark set code-review -r @ 2>/dev/null || true)
    (cd "$BOB_DIR" && jj git push -b code-review --allow-new 2>/dev/null || true)
    # Alice fetches and checks out the code
    (cd "$ALICE_DIR" && jj git fetch 2>/dev/null || true)
    (cd "$ALICE_DIR" && jj new code-review@origin 2>/dev/null || true)
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

assert_file_contains() {
    local file="$1"
    local expected="$2"
    local message="$3"
    if grep -q "$expected" "$file" 2>/dev/null; then
        echo -e "  ${GREEN}✓${NC} $message"
    else
        echo -e "  ${RED}✗${NC} $message"
        echo -e "  ${RED}Expected to find '$expected' in $file${NC}"
        exit 1
    fi
}

section() {
    echo -e "\n${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
}

show_file() {
    local file="$1"
    local label="$2"
    echo -e "  ${CYAN}─── $label ───${NC}"
    cat -n "$file" | sed 's/^/  /'
    echo -e "  ${CYAN}───────────────────────${NC}"
}

# ============================================================================
section "SETUP: Creating shared repository with initial codebase"
# ============================================================================

# Create bare origin repo
echo "Creating bare origin repository..."
git init --bare "$ORIGIN_DIR"

# Create Alice's working directory with initial project structure
echo "Creating Alice's working directory..."
mkdir -p "$ALICE_DIR"
cd "$ALICE_DIR"
git init
git config user.name "alice"
git config user.email "alice@example.com"

# Create initial project structure
cat > README.md << 'EOF'
# Auth Service

A simple authentication service.

## Usage

```python
from auth import AuthService
auth = AuthService()
token = auth.login(username, password)
```
EOF

mkdir -p src tests
cat > src/__init__.py << 'EOF'
"""Auth service package."""
EOF

cat > src/config.py << 'EOF'
"""Configuration constants."""

# Database settings
DATABASE_URL = "sqlite:///auth.db"

# Server settings
HOST = "0.0.0.0"
PORT = 8080
EOF

cat > tests/__init__.py << 'EOF'
"""Test package."""
EOF

git add .
git commit -m "Initial project structure"
git remote add origin "$ORIGIN_DIR"
git push -u origin main

# Initialize jj for Alice (colocated)
jj git init --colocate
jj bookmark track main@origin 2>/dev/null || jj bookmark track main --remote origin 2>/dev/null || true

# Initialize jjj for Alice
jjj_alice init

# Push metadata using git directly
echo "Pushing jjj via git..."
(cd "$ALICE_DIR" && git push origin jjj 2>/dev/null || true)

# Create Bob's working directory (clone)
echo "Creating Bob's working directory..."
git clone "$ORIGIN_DIR" "$BOB_DIR"
cd "$BOB_DIR"
git config user.name "bob"
git config user.email "bob@example.com"

# Initialize jj for Bob (colocated)
jj git init --colocate

# Fetch jjj and set up local bookmark
(cd "$BOB_DIR" && git fetch origin jjj && git checkout -b jjj FETCH_HEAD 2>/dev/null && git checkout main || true)

# Import the bookmark into jj
bob jj git import

# Initialize jjj for Bob (will detect existing metadata)
jjj_bob init 2>/dev/null || true

echo -e "\n${GREEN}Setup complete!${NC}"

# ============================================================================
section "STEP 1: Alice creates a problem for user authentication"
# ============================================================================

OUTPUT=$(jjj_alice problem new "Implement user authentication" << 'EOF'
We need a secure authentication system that:
- Validates user credentials against a database
- Issues JWT tokens for authenticated sessions
- Handles token expiration and refresh
EOF
)
assert_contains "$OUTPUT" "p1" "Problem p1 created"

# Sync metadata
sync_alice_to_bob

# ============================================================================
section "STEP 2: Alice writes authentication code and creates a solution"
# ============================================================================

echo "Alice writes the initial authentication implementation..."

# Alice creates the auth module with intentional issues for Bob to critique
cat > "$ALICE_DIR/src/auth.py" << 'EOF'
"""User authentication module."""

import hashlib
import time
import json
import base64

# Token expiration: 24 hours (in seconds)
TOKEN_EXPIRATION = 86400

# Simple in-memory user store (for demo)
USERS = {
    "admin": "5f4dcc3b5aa765d61d8327deb882cf99",  # password: "password"
    "user1": "e10adc3949ba59abbe56e057f20f883e",  # password: "123456"
}


def hash_password(password):
    """Hash a password using MD5."""
    return hashlib.md5(password.encode()).hexdigest()


def verify_password(username, password):
    """Verify username and password combination."""
    if username not in USERS:
        return False
    return USERS[username] == hash_password(password)


def generate_token(username):
    """Generate a JWT-like token for the user."""
    header = {"alg": "none", "typ": "JWT"}
    payload = {
        "sub": username,
        "iat": int(time.time()),
        "exp": int(time.time()) + TOKEN_EXPIRATION,
    }

    header_b64 = base64.b64encode(json.dumps(header).encode()).decode()
    payload_b64 = base64.b64encode(json.dumps(payload).encode()).decode()

    # No signature for simplicity
    return f"{header_b64}.{payload_b64}."


def validate_token(token):
    """Validate a token and return the username if valid."""
    try:
        parts = token.split(".")
        if len(parts) != 3:
            return None

        payload_b64 = parts[1]
        payload = json.loads(base64.b64decode(payload_b64))

        if payload["exp"] < time.time():
            return None

        return payload["sub"]
    except Exception:
        return None


def login(username, password):
    """Authenticate user and return a token."""
    if verify_password(username, password):
        return generate_token(username)
    return None
EOF

# Alice also creates a test file
cat > "$ALICE_DIR/tests/test_auth.py" << 'EOF'
"""Tests for authentication module."""

import sys
sys.path.insert(0, 'src')

from auth import hash_password, verify_password, generate_token, validate_token, login


def test_hash_password():
    """Test password hashing."""
    result = hash_password("password")
    assert result == "5f4dcc3b5aa765d61d8327deb882cf99"


def test_verify_password_valid():
    """Test valid password verification."""
    assert verify_password("admin", "password") == True


def test_verify_password_invalid():
    """Test invalid password verification."""
    assert verify_password("admin", "wrong") == False


def test_generate_token():
    """Test token generation."""
    token = generate_token("admin")
    assert token is not None
    assert "." in token


def test_validate_token():
    """Test token validation."""
    token = generate_token("admin")
    username = validate_token(token)
    assert username == "admin"


def test_login_success():
    """Test successful login."""
    token = login("admin", "password")
    assert token is not None


def test_login_failure():
    """Test failed login."""
    token = login("admin", "wrongpassword")
    assert token is None


if __name__ == "__main__":
    test_hash_password()
    test_verify_password_valid()
    test_verify_password_invalid()
    test_generate_token()
    test_validate_token()
    test_login_success()
    test_login_failure()
    print("All tests passed!")
EOF

show_file "$ALICE_DIR/src/auth.py" "src/auth.py (Alice's implementation)"

# Alice commits her changes with jj
alice jj describe -m "Add JWT-based user authentication"

# Create the solution with Bob as reviewer
OUTPUT=$(jjj_alice solution new "JWT-based authentication with MD5 hashing" --problem p1 --reviewer bob << 'EOF'
Implementation approach:
- Use MD5 for password hashing (fast and simple)
- Generate JWT-like tokens with base64 encoding
- 24-hour token expiration for convenience
- In-memory user store for demo purposes
EOF
)
assert_contains "$OUTPUT" "s1" "Solution s1 created"
assert_contains "$OUTPUT" "@bob" "Bob mentioned as reviewer"

# Show what critiques exist
OUTPUT=$(jjj_alice critique list --solution s1)
assert_contains "$OUTPUT" "c1" "Review critique c1 created"
assert_contains "$OUTPUT" "Awaiting review from @bob" "Review critique has correct title"

# Sync both metadata and code
sync_alice_to_bob
sync_code_alice_to_bob

# ============================================================================
section "STEP 3: Bob fetches and reviews Alice's code"
# ============================================================================

# Bob checks his status
OUTPUT=$(jjj_bob status)
assert_contains "$OUTPUT" "Awaiting review from @bob" "Bob sees the review request"

# Bob fetches Alice's code changes
bob jj git fetch
bob jj log --limit 3

# Bob examines the solution
OUTPUT=$(jjj_bob solution show s1)
assert_contains "$OUTPUT" "JWT-based authentication" "Bob can see solution details"

# Bob looks at Alice's code
echo -e "\n${YELLOW}[BOB]${NC} Reviewing Alice's code..."
show_file "$BOB_DIR/src/auth.py" "src/auth.py"

# ============================================================================
section "STEP 4: Bob raises security critiques on specific lines"
# ============================================================================

# Bob finds multiple issues and creates line-specific critiques

# Critique 1: MD5 is insecure (line 19)
OUTPUT=$(jjj_bob critique new s1 "MD5 is cryptographically broken - use bcrypt or argon2" \
    --severity critical \
    --file src/auth.py \
    --line 19)
assert_contains "$OUTPUT" "c2" "Critique c2 created (MD5 issue)"

# Critique 2: Token has no signature (line 31-32)
OUTPUT=$(jjj_bob critique new s1 "JWT uses 'alg: none' - tokens can be forged without signature verification" \
    --severity critical \
    --file src/auth.py \
    --line 31)
assert_contains "$OUTPUT" "c3" "Critique c3 created (unsigned JWT)"

# Critique 3: Token expiration too long (line 10)
OUTPUT=$(jjj_bob critique new s1 "24-hour token expiration is too long for security - recommend 15-30 minutes" \
    --severity high \
    --file src/auth.py \
    --line 10)
assert_contains "$OUTPUT" "c4" "Critique c4 created (token expiration)"

# Critique 4: Hardcoded credentials (line 13-15)
OUTPUT=$(jjj_bob critique new s1 "Hardcoded user credentials in source code - should use environment variables or secure vault" \
    --severity high \
    --file src/auth.py \
    --line 13)
assert_contains "$OUTPUT" "c5" "Critique c5 created (hardcoded credentials)"

# List all critiques
echo -e "\n${YELLOW}[BOB]${NC} All critiques on s1:"
jjj_bob critique list --solution s1

# Sync metadata back to Alice
sync_bob_to_alice

# ============================================================================
section "STEP 5: Alice sees all critiques and her solution is blocked"
# ============================================================================

# Alice checks her status
OUTPUT=$(jjj_alice status)
assert_contains "$OUTPUT" "BLOCKED" "Alice sees solution is blocked"
assert_contains "$OUTPUT" "MD5" "Alice sees the MD5 critique"

# Alice lists all critiques
echo -e "\n${GREEN}[ALICE]${NC} Reviewing critiques..."
OUTPUT=$(jjj_alice critique list --solution s1)
assert_contains "$OUTPUT" "c2" "MD5 critique visible"
assert_contains "$OUTPUT" "c3" "JWT signature critique visible"
assert_contains "$OUTPUT" "c4" "Token expiration critique visible"
assert_contains "$OUTPUT" "c5" "Hardcoded credentials critique visible"

# Alice views the critical MD5 critique
echo -e "\n${GREEN}[ALICE]${NC} Viewing critique c2 details:"
jjj_alice critique show c2

# ============================================================================
section "STEP 6: Alice fixes the security issues"
# ============================================================================

echo "Alice rewrites the authentication module with security fixes..."

# Alice creates a secure version
cat > "$ALICE_DIR/src/auth.py" << 'EOF'
"""User authentication module - Secure implementation."""

import hashlib
import hmac
import time
import json
import base64
import os
import secrets

# Token expiration: 15 minutes (in seconds)
TOKEN_EXPIRATION = 900

# Secret key for HMAC signing (should be from environment in production)
SECRET_KEY = os.environ.get("AUTH_SECRET_KEY", secrets.token_hex(32))


def hash_password(password, salt=None):
    """Hash a password using PBKDF2-SHA256 with salt."""
    if salt is None:
        salt = secrets.token_hex(16)

    # Use PBKDF2 with 100,000 iterations
    key = hashlib.pbkdf2_hmac(
        'sha256',
        password.encode(),
        salt.encode(),
        100000
    )
    return f"{salt}${key.hex()}"


def verify_password(stored_hash, password):
    """Verify a password against a stored hash."""
    try:
        salt, _ = stored_hash.split('$')
        return hmac.compare_digest(
            stored_hash,
            hash_password(password, salt)
        )
    except ValueError:
        return False


def generate_token(username):
    """Generate a signed JWT token for the user."""
    header = {"alg": "HS256", "typ": "JWT"}
    payload = {
        "sub": username,
        "iat": int(time.time()),
        "exp": int(time.time()) + TOKEN_EXPIRATION,
        "jti": secrets.token_hex(8),  # Unique token ID
    }

    header_b64 = base64.urlsafe_b64encode(
        json.dumps(header).encode()
    ).decode().rstrip('=')

    payload_b64 = base64.urlsafe_b64encode(
        json.dumps(payload).encode()
    ).decode().rstrip('=')

    # Create HMAC-SHA256 signature
    message = f"{header_b64}.{payload_b64}"
    signature = hmac.new(
        SECRET_KEY.encode(),
        message.encode(),
        hashlib.sha256
    ).digest()
    signature_b64 = base64.urlsafe_b64encode(signature).decode().rstrip('=')

    return f"{header_b64}.{payload_b64}.{signature_b64}"


def validate_token(token):
    """Validate a token signature and expiration, return username if valid."""
    try:
        parts = token.split(".")
        if len(parts) != 3:
            return None

        header_b64, payload_b64, signature_b64 = parts

        # Verify signature
        message = f"{header_b64}.{payload_b64}"
        expected_sig = hmac.new(
            SECRET_KEY.encode(),
            message.encode(),
            hashlib.sha256
        ).digest()

        # Pad base64 if needed
        signature_b64_padded = signature_b64 + '=' * (4 - len(signature_b64) % 4)
        actual_sig = base64.urlsafe_b64decode(signature_b64_padded)

        if not hmac.compare_digest(expected_sig, actual_sig):
            return None

        # Decode and check expiration
        payload_b64_padded = payload_b64 + '=' * (4 - len(payload_b64) % 4)
        payload = json.loads(base64.urlsafe_b64decode(payload_b64_padded))

        if payload["exp"] < time.time():
            return None

        return payload["sub"]
    except Exception:
        return None


class UserStore:
    """Secure user credential storage."""

    def __init__(self):
        self._users = {}

    def add_user(self, username, password):
        """Add a user with a securely hashed password."""
        self._users[username] = hash_password(password)

    def verify(self, username, password):
        """Verify user credentials."""
        if username not in self._users:
            # Constant-time comparison to prevent timing attacks
            verify_password(hash_password("dummy"), password)
            return False
        return verify_password(self._users[username], password)


# Default user store (populate from secure source in production)
_user_store = UserStore()


def login(username, password):
    """Authenticate user and return a signed token."""
    if _user_store.verify(username, password):
        return generate_token(username)
    return None


def register_user(username, password):
    """Register a new user."""
    _user_store.add_user(username, password)
EOF

show_file "$ALICE_DIR/src/auth.py" "src/auth.py (Alice's secure implementation)"

# Alice commits her security fixes
alice jj describe -m "Fix security issues: PBKDF2, HMAC-SHA256 JWT, 15min expiry"

# Alice addresses each critique with explanation
echo -e "\n${GREEN}[ALICE]${NC} Addressing critiques with fixes..."

OUTPUT=$(jjj_alice critique address c2 << 'EOF'
Fixed: Replaced MD5 with PBKDF2-SHA256 using 100,000 iterations and random salt.
See hash_password() on lines 18-27.
EOF
)
assert_contains "$OUTPUT" "addressed" "c2 addressed (MD5 fix)"

OUTPUT=$(jjj_alice critique address c3 << 'EOF'
Fixed: JWT now uses HMAC-SHA256 signing with a secret key.
Added signature verification in validate_token().
See lines 47-67 for signing, lines 70-97 for verification.
EOF
)
assert_contains "$OUTPUT" "addressed" "c3 addressed (JWT signing)"

OUTPUT=$(jjj_alice critique address c4 << 'EOF'
Fixed: Token expiration reduced from 24 hours to 15 minutes (900 seconds).
See line 12.
EOF
)
assert_contains "$OUTPUT" "addressed" "c4 addressed (token expiration)"

OUTPUT=$(jjj_alice critique address c5 << 'EOF'
Fixed: Removed hardcoded credentials. Added UserStore class that hashes
passwords securely. In production, users would be loaded from a secure database.
Secret key now comes from AUTH_SECRET_KEY environment variable.
See lines 15, 100-118.
EOF
)
assert_contains "$OUTPUT" "addressed" "c5 addressed (no hardcoded creds)"

# Sync both metadata and code
sync_alice_to_bob
sync_code_alice_to_bob

# ============================================================================
section "STEP 7: Bob verifies Alice's fixes"
# ============================================================================

# Bob fetches the updated code
bob jj git fetch

# Bob reviews the updated code
echo -e "\n${YELLOW}[BOB]${NC} Reviewing Alice's security fixes..."
show_file "$BOB_DIR/src/auth.py" "src/auth.py (updated)"

# Bob checks critique statuses
echo -e "\n${YELLOW}[BOB]${NC} Checking critique statuses:"
OUTPUT=$(jjj_bob critique list --solution s1)
assert_contains "$OUTPUT" "addressed" "Critiques show as addressed"

# Bob verifies specific fixes
assert_file_contains "$BOB_DIR/src/auth.py" "pbkdf2_hmac" "PBKDF2 is now used"
assert_file_contains "$BOB_DIR/src/auth.py" "HS256" "HMAC-SHA256 signing added"
assert_file_contains "$BOB_DIR/src/auth.py" "TOKEN_EXPIRATION = 900" "15-minute expiration"
assert_file_contains "$BOB_DIR/src/auth.py" "UserStore" "UserStore class exists"

echo -e "\n${YELLOW}[BOB]${NC} All security fixes verified! Signing off..."

# Bob dismisses his review critique (LGTM)
OUTPUT=$(jjj_bob critique dismiss c1 << 'EOF'
LGTM! All security concerns have been addressed:
- PBKDF2 with proper iterations and salt
- HMAC-SHA256 signed JWTs
- Reasonable token expiration
- No hardcoded credentials
Great work on the security improvements!
EOF
)
assert_contains "$OUTPUT" "dismissed" "Review critique dismissed (LGTM)"

# Sync metadata
sync_bob_to_alice

# ============================================================================
section "STEP 8: Alice accepts the solution"
# ============================================================================

# Alice checks status - should show ready
OUTPUT=$(jjj_alice status)
assert_contains "$OUTPUT" "READY" "Solution shows as ready"

# Alice accepts the solution
OUTPUT=$(jjj_alice solution approve s1 <<< "n")
assert_contains "$OUTPUT" "approved" "Solution approved"

# Verify the solution is accepted
OUTPUT=$(jjj_alice solution show s1)
assert_contains "$OUTPUT" "approved" "Solution status is approved"

# ============================================================================
section "VERIFICATION: Check final state"
# ============================================================================

echo "Checking final critique states..."

OUTPUT=$(jjj_alice critique list --solution s1)
assert_contains "$OUTPUT" "dismissed" "c1 is dismissed (review complete)"
assert_contains "$OUTPUT" "addressed" "Security critiques are addressed"
assert_not_contains "$OUTPUT" "?open" "No open critiques"

# Verify the code is secure
echo -e "\nVerifying final code security properties..."
assert_file_contains "$ALICE_DIR/src/auth.py" "pbkdf2_hmac" "Uses PBKDF2"
assert_file_contains "$ALICE_DIR/src/auth.py" "hmac.compare_digest" "Constant-time comparison"
assert_file_contains "$ALICE_DIR/src/auth.py" "secrets.token" "Cryptographic random"
assert_file_contains "$ALICE_DIR/src/auth.py" "100000" "100k PBKDF2 iterations"

echo -e "\n${GREEN}════════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}ALL TESTS PASSED!${NC}"
echo -e "${GREEN}════════════════════════════════════════════════════════════════${NC}"

echo -e "\nThe multi-user code review workflow completed successfully:"
echo "  1. Alice created problem p1 (Implement user authentication)"
echo "  2. Alice wrote src/auth.py with intentional security issues"
echo "  3. Alice created solution s1 with Bob as reviewer"
echo "  4. Bob reviewed the code and raised 4 line-specific critiques:"
echo "     - c2: MD5 is insecure (line 19) [critical]"
echo "     - c3: JWT has no signature (line 31) [critical]"
echo "     - c4: 24-hour token expiration (line 10) [high]"
echo "     - c5: Hardcoded credentials (line 13) [high]"
echo "  5. Alice rewrote auth.py with security fixes"
echo "  6. Alice addressed each critique with explanations"
echo "  7. Bob verified the fixes and signed off (dismissed c1)"
echo "  8. Alice accepted the solution"
echo ""
echo "This demonstrates:"
echo "  - Actual code changes tracked alongside review metadata"
echo "  - Line-specific critiques with file and line references"
echo "  - Multiple critiques at different severity levels"
echo "  - Addressing critiques with detailed explanations"
echo "  - Code sync between users via jj/git"
echo "  - The unified critique model for reviews and issues"
