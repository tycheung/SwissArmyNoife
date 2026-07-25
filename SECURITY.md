# Security Policy

## Supported versions

| Version | Supported |
|---------|-----------|
| `0.1.x` (main) | Yes — pre-release; report issues promptly |

## Reporting a vulnerability

**Do not** open a public GitHub issue for security-sensitive reports.

Please email the maintainers (or use GitHub **Private vulnerability reporting** if enabled
on the repository) with:

1. Description of the issue and impact
2. Steps to reproduce / proof of concept
3. Affected commit or version if known
4. Whether you plan a fix / patch

We aim to acknowledge reports within **7 days** and to provide a remediation plan or
fix within a reasonable window for the pre-1.0 codebase.

## Scope highlights

In scope examples:

- Secret leakage via MCP tool results, logs, `Debug`/`Display`, or audit payloads
- Sandbox / filesystem jail escapes (once shipped)
- Vault key misuse or ciphertext malleability in `vault`
- Dependency supply-chain issues flagged by `cargo deny` / advisories

Out of scope examples:

- Denial of service against a local single-user stdio MCP with ambient trust
- Issues solely in Nimbusware or the commercial marketplace repos
- Social engineering / physical access

## Security-related configuration

| Variable | Notes |
|----------|-------|
| `VAULT_KEY` | 64 hex chars (32-byte key). Prefer setting this in production; ephemeral keys are for dev only |
| `DB_PATH` / `CONFIG_DIR` | Keep DB files on access-controlled storage |

Never paste vault keys or provider credentials into issues, chat logs, or commits.

## Coordinated disclosure

Please allow time for a fix before public disclosure. We appreciate responsible disclosure
and will credit reporters who wish to be named (unless you prefer anonymity).
