#!/usr/bin/env bash
set -euo pipefail
# Encrypt stdin to stdout using OpenSSL AES-256-GCM with passphrase
# Requires PRESS_KEYS_PASSPHRASE env var
if [[ -z "${PRESS_KEYS_PASSPHRASE:-}" ]]; then
  echo "PRESS_KEYS_PASSPHRASE missing" >&2
  exit 2
fi
openssl enc -aes-256-gcm -pbkdf2 -salt -pass env:PRESS_KEYS_PASSPHRASE
