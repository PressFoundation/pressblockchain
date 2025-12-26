#!/usr/bin/env bash
set -euo pipefail

# Press Blockchain - AlmaLinux prerequisites
# Run as root.

dnf -y update
dnf -y install git curl ca-certificates tar unzip jq

# Docker CE repo
dnf -y install dnf-utils
dnf config-manager --add-repo https://download.docker.com/linux/centos/docker-ce.repo

dnf -y install docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
systemctl enable --now docker

# Create locked-down runtime user
id -u pressblockchain >/dev/null 2>&1 || useradd -m -s /bin/bash pressblockchain
usermod -aG docker pressblockchain

echo "OK: Docker installed, pressblockchain user created and added to docker group."
echo "Log out / log in for group changes to apply for the pressblockchain user."
