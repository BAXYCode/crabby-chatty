#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   scripts/wait-for-postgres.sh "postgres://user:pass@host:5432/db" [timeout_seconds]
#
# Requires:
#   - pg_isready (from postgresql-client) OR (fallback) nc

DATABASE_URL="${1:-}"
TIMEOUT="${2:-30}"

if [[ -z "${DATABASE_URL}" ]]; then
  echo "Usage: $0 <DATABASE_URL> [timeout_seconds]" >&2
  exit 2
fi

deadline=$(( $(date +%s) + TIMEOUT ))

echo "Waiting for Postgres to accept connections (timeout: ${TIMEOUT}s)…"

# Prefer pg_isready if available
if command -v pg_isready >/dev/null 2>&1; then
  while true; do
    if pg_isready -d "${DATABASE_URL}" >/dev/null 2>&1; then
      echo "Postgres is ready."
      exit 0
    fi
    if (( $(date +%s) >= deadline )); then
      echo "Timed out waiting for Postgres." >&2
      exit 1
    fi
    sleep 0.5
  done
fi

# Fallback: TCP probe using nc (netcat)
if command -v nc >/dev/null 2>&1; then
  host="$(python3 - <<'PY'
import os, sys
from urllib.parse import urlparse
u = urlparse(os.environ["DATABASE_URL"])
print(u.hostname or "")
PY
DATABASE_URL="${DATABASE_URL}")"
  port="$(python3 - <<'PY'
import os, sys
from urllib.parse import urlparse
u = urlparse(os.environ["DATABASE_URL"])
print(u.port or 5432)
PY
DATABASE_URL="${DATABASE_URL}")"

  if [[ -z "${host}" ]]; then
    echo "Could not parse host from DATABASE_URL; install pg_isready (postgresql-client) instead." >&2
    exit 2
  fi

  while true; do
    if nc -z "${host}" "${port}" >/dev/null 2>&1; then
      echo "Postgres TCP port is reachable."
      exit 0
    fi
    if (( $(date +%s) >= deadline )); then
      echo "Timed out waiting for Postgres TCP port." >&2
      exit 1
    fi
    sleep 0.5
  done
fi

echo "Neither pg_isready nor nc is installed. Install postgresql-client (recommended)." >&2
exit 2

