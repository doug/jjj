# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in jjj, please report it responsibly.

**Do not open a public GitHub issue for security vulnerabilities.**

Instead, please use [GitHub's private vulnerability reporting](https://github.com/doug/jjj/security/advisories/new) to submit your report. This ensures the issue can be addressed before public disclosure.

### What to include

- Description of the vulnerability
- Steps to reproduce
- Affected versions
- Potential impact

### Response timeline

- **Acknowledgment:** Within 48 hours
- **Assessment:** Within 1 week
- **Fix or mitigation:** As soon as practical, depending on severity

## Supported Versions

| Version | Supported |
|---------|-----------|
| Latest release | Yes |
| Older versions | No |

## Scope

jjj stores project metadata (problem/solution/critique records) in an orphaned git bookmark. It does not handle authentication, secrets, or network services directly. Security concerns most likely involve:

- Command injection via shell automation rules
- Path traversal in metadata file operations
- Unsafe deserialization of YAML frontmatter or TOML config
