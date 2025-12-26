# Modules and profiles

Press Blockchain uses **Docker Compose profiles** as the deployment mechanism for feature modules.

## Guarantees
- `press-rpc` and `press-validator` are **core** and must remain functional even if every other module is disabled.
- Disabling a module must not break:
  - chain/RPC
  - token deploy/redeploy ability (via deployer)
  - basic wallet connectivity (when enabled)

## How to disable a module
Edit `config/modules.json` and set the module to `false`, then redeploy:
```bash
bash scripts/clean_start.sh /opt/press-blockchain
bash scripts/deploy_rc4.sh /opt/press-blockchain
```

## Mapping
- `deployer` => Deployer UI/API
- `status_page` => status.pressblockchain.io UI/API
- `outlet_wizard` => outlet UI
- `outlet_api` => outlet backend API
- `oracle` => Press Oracle service
- `bots` => Discord/Telegram bots + dashboard
- `dev_dapps` => developer dapps suite

RC5 adds a Deployer UI view to inspect enabled modules and meta.
