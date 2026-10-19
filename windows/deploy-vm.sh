#!/usr/bin/env bash
set -uo pipefail
shopt -s extglob dotglob

here="$(cd "$(dirname "$0")" && pwd)"
proj="$here/src/AircastQGC"
stage='C:/aircast-head/AircastQGC'
quiet() { grep -viE 'pq\.html|post-quantum|store now|upgraded|Warning: Permanently' || true; }

ssh winvm 'cmd /c "if not exist C:\aircast-head\AircastQGC mkdir C:\aircast-head\AircastQGC"' 2>&1 | quiet
(cd "$proj" && scp -rq !(bin|obj|runtime) "winvm:$stage/") 2>&1 | quiet
scp -q "$here/tools/buildrun.ps1" 'winvm:C:/Temp/buildrun.ps1' 2>&1 | quiet
ssh winvm 'powershell -ExecutionPolicy Bypass -NoProfile -File C:\Temp\buildrun.ps1' 2>&1 | quiet
exit "${PIPESTATUS[0]}"
