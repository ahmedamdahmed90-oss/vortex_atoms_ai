# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Security Vulnerability

If you discover a security vulnerability in Vortex Atoms AI, please report it responsibly.

### How to Report

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, please report them to:

- **Email**: security@vortexatoms.ai (or maintainer email)
- **GitHub Security Advisory**: [Create a private advisory](https://github.com/mansour2024/vortex_atoms_ai/security/advisories/new)

### What to Include

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

### Response Time

- **Initial Response**: Within 48 hours
- **Update Frequency**: Every 3-5 days until resolved
- **Target Resolution**: Within 30 days

## Security Best Practices

### For Users

1. **Keep Updated**: Always use the latest version
2. **Local Only**: This is designed for local deployment - do not expose to the internet without proper security
3. **Model Safety**: Be cautious with model outputs - they may produce unexpected or harmful content
4. **Data Privacy**: All processing is local - no data is sent to external servers

### For Developers

1. **Input Validation**: Always validate and sanitize user inputs
2. **Dependency Updates**: Keep dependencies updated
3. **Code Review**: All changes must be reviewed before merging
4. **Testing**: Write tests for security-critical code

## Security Model

- **Authentication**: Bearer tokens with `user` (`vxt_…`) / `admin` (`vxa_…`)
  roles, auto-generated on first run into the user profile
  (`%LOCALAPPDATA%\vortex_atoms_ai\auth.json`). All `/v1` endpoints (except
  `/v1/health`) and `/ws` require a token.
- **CORS**: Strict same-origin by default; extra origins only via
  `security.allowed_origins`. Unknown websites cannot drive the local server.
- **Privileged endpoints** (`models/swap`, `knowledge/import`, `/v1/admin/*`)
  require the admin token and are audit-logged (`audit.log`).
- **Fail-closed remote binds**: non-loopback `--host` values are refused
  unless token auth is enabled **and** `--allow-remote` is passed.
- **Input hardening**: temperature range, prompt/batch/import size caps,
  knowledge-import directory sandbox, model filename + repo allowlists,
  SHA-256 cache-integrity manifest, per-IP rate limiting, `x-request-id`s.
- **Output safety**: model-rendered markdown links allowlisted
  (http/https/mailto/relative only — no `javascript:` XSS); strict CSP +
  `frame-ancestors 'none'` + nosniff/referrer/permissions headers on the UI.
- **Secrets at rest**: tokens and chat sessions DPAPI-encrypted (Windows,
  user-scoped) with user-only file ACLs; 30-day session retention + purge.
- **Process control**: single-instance mutex, graceful Ctrl+C/tray shutdown
  (30 s drain), server-side WS idle/ping/size bounds, `--read-only` kiosk
  mode, loopback-only admin, config hot-reload, metrics, disk-space guard.
- **Supply chain**: reproducible knowledge artifacts (CI-verified), release
  checksums + SLSA provenance + embedded SBOM (`cargo auditable`),
  `cargo audit` + `npm audit` in CI, log-hygiene lint.
- **Local Network**: Remote/LAN use additionally supports `--tls-cert` /
  `--tls-key` (HTTPS). Never port-forward without token auth + TLS.
- **Model Safety**: LLM outputs are not filtered - use responsibly

## Release Signing (Windows Authenticode)

Release binaries are hashed (`sha256sums`) with SLSA build provenance, but
are not yet Authenticode-signed (requires a purchased code-signing
certificate). To sign before distributing `VortexAtomsAI-Setup.exe`:

```powershell
signtool sign /fd SHA256 /a /tr http://timestamp.digicert.com /td SHA256 dist\VortexAtomsAI-Setup.exe
signtool verify /pa dist\VortexAtomsAI-Setup.exe
```

## Vulnerability Disclosure

We follow responsible disclosure practices:

1. Vulnerability is reported privately
2. Fix is developed and tested
3. Patch is released
4. Public disclosure after 7 days of patch release
