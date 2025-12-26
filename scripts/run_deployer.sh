#!/usr/bin/env bash
set -euo pipefail

# Run deployer as non-root (recommended)
# - Executes installer backend commands and docker compose
# - Assumes 'pressblockchain' user is in docker group

PRESS_USER="${PRESS_USER:-pressblockchain}"
INSTALL_DIR="${INSTALL_DIR:-/opt/pressblockchain}"
REPO_DIR="${REPO_DIR:-${INSTALL_DIR}/press-blockchain}"

if [ "$(id -u)" -eq 0 ]; then
  echo "[run] Refusing to run deploy as root."
  echo "[run] Use: sudo -u ${PRESS_USER} -H bash $0"
  exit 1
fi

cd "${REPO_DIR}"

# Ensure env exists
if [ ! -f config/press.env ]; then
  cp config/press.env.example config/press.env
fi

# Run installer API command-mode deploy (no web needed)
python3 apps/installer-api/main.py up || python3 apps/installer-api/main.py retry
