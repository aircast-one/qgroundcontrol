#!/usr/bin/env bash
set -euo pipefail

DEVICE="${1:-}"
[ -n "$DEVICE" ] || { echo "Which device? Run \`just device <host>\`, or export AIRCAST_DEVICE=<host> once." >&2; exit 2; }
WEB="$(cd "$(dirname "$0")/.." && pwd)"
QGC="$(dirname "$WEB")"
AIRCASTD="${AIRCASTD_DIR:-$(dirname "$QGC")/aircastd}"
[ -x "$AIRCASTD/scripts/qgc-web-sync.sh" ] || { echo "No aircastd checkout at $AIRCASTD; point AIRCASTD_DIR at one." >&2; exit 2; }
source "$AIRCASTD/scripts/pi-host.sh"
TARGET="$(pi_ssh_target "$DEVICE")"

WORK="$(mktemp -d -t qgc-device)"
LOGS="$(mktemp -d -t qgc-device-logs)"
trap 'rm -rf "$WORK"' EXIT
VERSION="$(git -C "$QGC" describe --tags --always --dirty)"
START=$SECONDS

step() {
    local name="$1" began=$SECONDS
    shift
    printf '  %-8s ' "$name"
    if "$@" >"$LOGS/$name.log" 2>&1; then
        printf '✓ %ss\n' "$((SECONDS - began))"
    else
        printf '✗ %ss\n\n' "$((SECONDS - began))"
        tail -n 25 "$LOGS/$name.log" | sed 's/^/    /'
        printf '\n  Full log: %s\n' "$LOGS/$name.log"
        exit 1
    fi
}

web() (
    cd "$WEB"
    [ node_modules/.package-lock.json -nt package-lock.json ] || npm ci
    npx vite build --outDir "$WORK/dist" --emptyOutDir
)

echo "QGroundControl $VERSION → $DEVICE"
step device "$AIRCASTD/scripts/qgc-web-sync.sh" "$DEVICE" --check
step core "$WEB/build-core.sh"
step web web
step pack env AIRCAST_QGC_WEB_DIR="$WORK/packed" "$AIRCASTD/scripts/qgc-web.sh" "$WORK/dist"
step install "$AIRCASTD/scripts/qgc-web-sync.sh" "$DEVICE" "$WORK/packed" "local QGroundControl $VERSION"
rm -rf "$LOGS"
echo "Ready in $((SECONDS - START))s → http://${TARGET#*@}/fly/"
