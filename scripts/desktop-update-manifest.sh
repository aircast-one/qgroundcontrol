#!/usr/bin/env bash
set -euo pipefail

ASSETS="${1:?Usage: $0 <assets dir> <short version> <download url> <notes file>}"
SHORT="${2:?}"
DOWNLOADS="${3:?}"
NOTES="${4:?}"

platform() {
  local sig="$ASSETS/$2.sig"
  if [ ! -f "$sig" ]; then
    echo "No signed $2 in $ASSETS" >&2
    return 1
  fi
  jq -n --arg key "$1" --arg signature "$(cat "$sig")" --arg url "$DOWNLOADS/$2" '{($key): {signature: $signature, url: $url}}'
}

PLATFORMS=$(
  {
    platform darwin-aarch64 "AircastQGCNative-$SHORT.app.tar.gz"
    platform windows-x86_64 "AircastQGCNative-$SHORT-windows-x64-setup.exe"
    platform windows-aarch64 "AircastQGCNative-$SHORT-windows-arm64-setup.exe"
  } | jq -s add
)

jq -n \
  --arg version "${SHORT#v}" \
  --rawfile notes "$NOTES" \
  --arg date "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --argjson platforms "$PLATFORMS" \
  '{version: $version, notes: $notes, pub_date: $date, platforms: $platforms}'
