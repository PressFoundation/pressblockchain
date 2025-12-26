# Reverse proxy guide (non-invasive)

Goal: keep `pressblockchain.io` root on **38.146.25.78** untouched, and host infra on **38.146.25.37**.

## Recommended DNS
Point to **38.146.25.37**:
- deploy.pressblockchain.io
- status.pressblockchain.io
- rpc.pressblockchain.io
- explorer.pressblockchain.io
- wallet.pressblockchain.io
- bots.pressblockchain.io

Keep on **38.146.25.78**:
- pressblockchain.io
- www.pressblockchain.io

## Non-invasive bind strategy
- Never bind to `0.0.0.0:80` or `:443` on the root host if it can impact existing site.
- Bind explicitly to infra IP and safe ports, then route 80/443 via your existing proxy layer.

Templates are in `deploy/nginx-templates/` and intentionally use **8085/8086** to avoid collisions.

## Port conflict checks
```bash
ss -ltnp | grep -E ':80|:443|:8085|:8086|:3005|:3007|:8545'
```

## If ports 80/443 are already in use
Use Cloudflare/cPanel reverse proxy or an alternate listener port and map externally.
