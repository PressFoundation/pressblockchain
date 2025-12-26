# AlmaLinux firewall rules (example)

Use `firewalld`. Prefer exposing only 80/443 via a reverse proxy, and keep internal service ports private.
If you must expose by port during bring-up, open minimally and close later.

## View active ports
```bash
firewall-cmd --state
firewall-cmd --get-active-zones
firewall-cmd --zone=public --list-all
```

## Temporary open (bring-up only)
```bash
# Deployer UI (temporary)
firewall-cmd --zone=public --add-port=3005/tcp --permanent
# Status UI (temporary)
firewall-cmd --zone=public --add-port=3007/tcp --permanent
# RPC (only if you need public RPC; otherwise keep private)
firewall-cmd --zone=public --add-port=8545/tcp --permanent
# Outlet API (temporary)
firewall-cmd --zone=public --add-port=8814/tcp --permanent

firewall-cmd --reload
```

## Recommended production posture
- Keep **8545** behind reverse proxy + rate limiting or private only
- Expose public endpoints via **Nginx/Cloudflare** on 80/443
- Use allowlists for admin dashboards (deployer/status)

## Close ports (after proxy is in place)
```bash
firewall-cmd --zone=public --remove-port=3005/tcp --permanent
firewall-cmd --zone=public --remove-port=3007/tcp --permanent
firewall-cmd --zone=public --remove-port=8814/tcp --permanent
firewall-cmd --reload
```
