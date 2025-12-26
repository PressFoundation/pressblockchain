# Hardening checklist (RC3)

- [ ] DNS points subdomains to 38.146.25.37
- [ ] Reverse proxy binds ONLY to 38.146.25.37 (do not touch 38.146.25.78 root)
- [ ] config/.env set with correct ports and secrets
- [ ] Clean Start works and removes orphan containers/networks/volumes
- [ ] Deployer UI: Preflight OK, Fix & Retry OK
- [ ] Outlet wizard: domain check OK, outlet create OK, token deploy OK, test tx OK
- [ ] Status page reflects all enabled modules
- [ ] If any module disabled => chain/RPC stays healthy
