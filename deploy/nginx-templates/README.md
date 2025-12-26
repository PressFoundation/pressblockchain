# Nginx templates (non-invasive)

These templates are designed to **avoid touching** pressblockchain.io root on 38.146.25.78.

They bind explicitly to the infra IP (38.146.25.37) and forward to docker services.
Use them only if you want Nginx on 38.146.25.37. Otherwise you can proxy via cPanel/Cloudflare/etc.

Files:
- deploy.conf: deploy.pressblockchain.io -> deployer-ui + outlet wizard
- status.conf: status.pressblockchain.io -> status-ui
- bots.conf: bots.pressblockchain.io -> bots dashboard (when enabled)
