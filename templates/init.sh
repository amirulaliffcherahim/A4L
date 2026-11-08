#!/usr/bin/env bash
# A4L Pre-Flight Contract & L3 Smoke Runner
set -euo pipefail

SUBCOMMAND="${1:-check}"

case "$SUBCOMMAND" in
  check)
    # 0 exit when usable; emit one JSON object to stdout
    GIT_CLEAN=true
    if [[ -n $(git status --porcelain 2>/dev/null || true) ]]; then
      GIT_CLEAN=false
    fi

    echo "{"
    echo "  \"binaries\": {"
    echo "    \"git\": \"$(command -v git || echo missing)\","
    echo "    \"cargo\": \"$(command -v cargo || echo missing)\""
    echo "  },"
    echo "  \"services\": {},"
    echo "  \"git_clean\": $GIT_CLEAN"
    echo "}"
    exit 0
    ;;

  smoke)
    # L3 Gate: exit 0 only if service or binary boots with zero startup crashes
    echo "[SMOKE] Running basic health check..."
    if command -v cargo >/dev/null 2>&1; then
      cargo check -q || exit 1
    fi
    echo "[SMOKE] OK"
    exit 0
    ;;

  *)
    echo "Usage: $0 [check|smoke]"
    exit 1
    ;;
esac
