#!/usr/bin/env bash
set -euo pipefail

# System-wide bootstrap (RUN AS ROOT)
# - creates 'pressblockchain' user
# - installs baseline packages
# - ensures docker is installed and user is in docker group
# - prepares /opt/pressblockchain with correct perms

PRESS_USER="${PRESS_USER:-pressblockchain}"
PRESS_HOME="/home/${PRESS_USER}"
INSTALL_DIR="${INSTALL_DIR:-/opt/pressblockchain}"
REPO_DIR="${REPO_DIR:-${INSTALL_DIR}/press-blockchain}"

echo "[bootstrap] Running as: $(id -u):$(id -un)"

if [ "$(id -u)" -ne 0 ]; then
  echo "[bootstrap] ERROR: must run as root"
  exit 1
fi

# Create user if missing
if ! id "${PRESS_USER}" >/dev/null 2>&1; then
  useradd -m -s /bin/bash "${PRESS_USER}"
  echo "[bootstrap] Created user ${PRESS_USER}"
fi

# Install basics (Alma/RHEL + Debian fallbacks)
if command -v dnf >/dev/null 2>&1; then
  dnf -y install git curl ca-certificates jq || true
  dnf -y install docker docker-compose-plugin || true
  systemctl enable --now docker || true
elif command -v apt-get >/dev/null 2>&1; then
  apt-get update -y
  apt-get install -y git curl ca-certificates jq docker.io docker-compose-plugin
  systemctl enable --now docker || true
fi

# Ensure docker group and membership
getent group docker >/dev/null 2>&1 || groupadd docker
usermod -aG docker "${PRESS_USER}" || true

# Prepare install dir
mkdir -p "${INSTALL_DIR}"
chown -R "${PRESS_USER}:${PRESS_USER}" "${INSTALL_DIR}"

# State directory hardening
mkdir -p "${REPO_DIR}/state"
chown -R "${PRESS_USER}:${PRESS_USER}" "${REPO_DIR}/state"
chmod 750 "${REPO_DIR}/state" || true

echo "[bootstrap] Done. Next: run deploy as ${PRESS_USER}"
