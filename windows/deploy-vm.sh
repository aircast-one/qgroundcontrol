#!/usr/bin/env bash
set -uo pipefail
shopt -s extglob dotglob

here="$(cd "$(dirname "$0")" && pwd)"
proj="$here/src/AircastQGC"
stage='C:/aircast-head/AircastQGC'
opts=(-o ConnectTimeout=20 -o ServerAliveInterval=15 -o ServerAliveCountMax=4)
quiet() { grep -viE 'pq\.html|post-quantum|store now|upgraded|Warning: Permanently' || true; }

level="$(sysctl -n kern.memorystatus_vm_pressure_level)"
if [ "$level" -ge 2 ]; then
  echo "the Mac is under memory pressure (level $level): not building on the VM" >&2
  exit 75
fi

ssh "${opts[@]}" winvm 'cmd /c "if not exist C:\aircast-head\AircastQGC mkdir C:\aircast-head\AircastQGC"' 2>&1 | quiet
[ "${PIPESTATUS[0]}" -eq 0 ] || { echo "winvm is not answering" >&2; exit 69; }
(cd "$proj" && scp "${opts[@]}" -rq !(bin|obj|runtime) "winvm:$stage/") 2>&1 | quiet
scp "${opts[@]}" -q "$here/tools/buildrun.ps1" 'winvm:C:/Temp/buildrun.ps1' 2>&1 | quiet
# shellcheck disable=SC2029
ssh "${opts[@]}" winvm "powershell -ExecutionPolicy Bypass -NoProfile -File C:\Temp\buildrun.ps1 $*" 2>&1 | quiet
exit "${PIPESTATUS[0]}"
