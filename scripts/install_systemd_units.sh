#!/usr/bin/env bash
set -euo pipefail

# Install systemd units (RUN AS ROOT)
if [ "$(id -u)" -ne 0 ]; then
  echo "Run as root"
  exit 1
fi

SRC_DIR="/opt/pressblockchain/press-blockchain/systemd"
cp -f "${SRC_DIR}/pressblockchain-stack.service" /etc/systemd/system/pressblockchain-stack.service
cp -f "${SRC_DIR}/pressblockchain-watchdog.service" /etc/systemd/system/pressblockchain-watchdog.service
cp -f "${SRC_DIR}/pressblockchain-watchdog.timer" /etc/systemd/system/pressblockchain-watchdog.timer

systemctl daemon-reload
systemctl enable --now pressblockchain-stack.service
systemctl enable --now pressblockchain-watchdog.timer

echo "Installed and started Press Blockchain systemd units."
