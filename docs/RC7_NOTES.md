# RC7 Notes

## What’s new
- Deployer API:
  - `GET /v1/steps` returns runtime step-state (best-effort, container-based)
  - `POST /v1/deploy-start` starts `scripts/deploy_rc6.sh` in background and logs to `state/deployer_run.log`
- Deployer UI:
  - Start Deploy button
  - Refresh Steps button
  - Cards rendering the returned JSON

## Operational workflow
1) Configure `.env` + DNS
2) Click **Start Deploy**
3) Click **Refresh Steps**
4) View logs via existing runtime/log view if needed
