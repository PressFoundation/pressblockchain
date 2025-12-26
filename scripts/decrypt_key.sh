#!/usr/bin/env bash
set -euo pipefail
if [[ -z "${PRESS_KEYS_PASSPHRASE:-}" ]]; then
  echo "PRESS_KEYS_PASSPHRASE missing" >&2
  exit 2
fi
openssl enc -d -aes-256-gcm -pbkdf2 -pass env:PRESS_KEYS_PASSPHRASE
