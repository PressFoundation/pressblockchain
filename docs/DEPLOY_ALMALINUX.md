# Deploy Press Blockchain on AlmaLinux (RC2)

This guide assumes:
- Infra on **38.146.25.37**
- Root site stays on **38.146.25.78**
- You point subdomains (rpc/explorer/wallet/deploy/status/bots/etc.) to **38.146.25.37**

## 1) Install prerequisites (run as root)
```bash
cd /opt
git clone <YOUR_GITHUB_REPO_URL> press-blockchain
cd press-blockchain
bash scripts/install_prereqs_almalinux.sh
```

## 2) Configure environment (run as pressblockchain user)
```bash
su - pressblockchain
cd /opt/press-blockchain
cp config/.env.example config/.env
nano config/.env
```

## 3) Clean start (optional but recommended)
```bash
bash scripts/clean_start.sh /opt/press-blockchain
```

## 4) Deploy
```bash
bash scripts/deploy_rc4.sh /opt/press-blockchain
```

## 5) Verify
- Deployer UI: `http://38.146.25.37:3005`
- Status UI: `http://38.146.25.37:3007`
- RPC: `http://38.146.25.37:8545`

## Notes
- Secrets are set in `config/.env` and should never be committed.
- If a module is disabled, core chain/rpc remains operational (graceful degradation).


- Outlet API: `http://38.146.25.37:8814/v1/health`


## 4.5) Verify DNS (recommended before mapping subdomains)
```bash
bash scripts/verify_dns.sh /opt/press-blockchain
```
