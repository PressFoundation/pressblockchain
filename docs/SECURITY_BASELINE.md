# Security baseline (Press Blockchain)

## Principles
- No secrets in git. No secrets in logs.
- Run docker as `pressblockchain` user when possible; root only for system-level actions.
- Every service has least privilege and only necessary ports exposed.
- Module toggles must never break core chain functionality.

## Mandatory controls
- Firewall: allow only needed ports; prefer reverse proxy to expose 80/443.
- Docker socket access: only deployer container uses it (audited). Treat as privileged.
- Rate limits on public RPC and API endpoints.
- Strict CORS and origin allowlists for dashboards.

## Operational
- Rotate OpenAI/Discord/Telegram tokens if ever exposed.
- Backups: config/ + chain state + indexer db.
- Audit: keep REPO_MANIFEST.json and release checksums.
