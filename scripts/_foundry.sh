#!/usr/bin/env bash
set -euo pipefail
# Wrapper to run Foundry tools in a container
# Usage: _foundry.sh <cmd...>
docker run --rm --network host ghcr.io/foundry-rs/foundry:latest "$@"
