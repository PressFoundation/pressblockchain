#!/bin/sh
set -e

# If /state/deploy.json exists, export key addresses for role-claim reads
if [ -f /state/deploy.json ]; then
  export COUNCIL_REGISTRY_ADDR=$(cat /state/deploy.json | sed -n 's/.*"councilRegistry": *"\([^"]*\)".*/\1/p' | head -n1)
  export BOND_VAULT_ADDR=$(cat /state/deploy.json | sed -n 's/.*"bondVault": *"\([^"]*\)".*/\1/p' | head -n1)
fi

exec /usr/local/bin/press_auth_api
