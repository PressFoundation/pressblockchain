#!/usr/bin/env bash
set -euo pipefail
# Runs a script inside foundry docker image with repo mounted.
IMAGE="${FOUNDRY_IMAGE:-ghcr.io/foundry-rs/foundry:stable}"
REPO_DIR="${REPO_DIR:-/repo}"
docker run --rm -v "$(pwd)":${REPO_DIR} -w ${REPO_DIR}   -e RPC_URL -e PRIVATE_KEY -e TOKEN_NAME -e TOKEN_SYMBOL -e TOTAL_SUPPLY_WEI -e OWNER_ADDR -e LIMITS_ENABLED -e MAX_TX -e MAX_WALLET   ${IMAGE} bash -lc "$*"
