#!/bin/bash
# Drives every surface exercised on 2026-09-08 and captures each one.
# Usage: regress.sh <tag>
export PATH="$PATH:$HOME/Library/Android/sdk/platform-tools"
S="$(cd "$(dirname "$0")" && pwd)"
TAG="${1:-run}"
set -u

pkill -f apmvehicle.py 2>/dev/null
HANDSET="$("$S/handset-ip.sh")"
[ -n "$HANDSET" ] || { echo "FAIL: no handset address - is it attached and on Wi-Fi?"; exit 1; }
echo "handset at $HANDSET"
RC_RSSI=203 BATT_PCT=70 BATT_STATE=0 nohup python3 "$S/apmvehicle.py" "$HANDSET" > "$S/regress_$TAG.simlog" 2>&1 &
python3 -c "import time; time.sleep(5)"
pgrep -f apmvehicle.py >/dev/null || { echo "FAIL: sim did not start"; tail -3 "$S/regress_$TAG.simlog"; exit 1; }

adb shell am force-stop one.aircast.android
"$S/ui.sh" front || { echo "FAIL: app did not come to the front"; exit 1; }
python3 -c "import time; time.sleep(30)"

shot() {
    "$S/ui.sh" shot "$S/${TAG}_$1.png" || echo "FAIL: $1"
}

# Analyze rows, with a vehicle connected: Log Download 429, Vibration 614,
# Console 799, Inspector 984. Preflight moved to the Fly tab's Actions sheet.
shot fly
"$S/ui.sh" tap 573 1744; python3 -c "import time; time.sleep(3)"; shot actions
"$S/ui.sh" key 4;   python3 -c "import time; time.sleep(2)"
"$S/ui.sh" tap 814 2108; python3 -c "import time; time.sleep(3)"
"$S/ui.sh" tap 300 614;  python3 -c "import time; time.sleep(4)"; shot vibration
"$S/ui.sh" key 4;   python3 -c "import time; time.sleep(2)"
"$S/ui.sh" tap 300 429;  python3 -c "import time; time.sleep(8)"; shot logs
"$S/ui.sh" key 4;   python3 -c "import time; time.sleep(2)"
"$S/ui.sh" tap 995 2109; python3 -c "import time; time.sleep(3)"; shot settings
"$S/ui.sh" tap 82 2109;  python3 -c "import time; time.sleep(4)"; shot flyback

echo "--- sim saw ---"
grep -c "STATUSTEXT" "$S/regress_$TAG.simlog" | sed 's/^/statustext sent: /'
grep -c "LOG_REQUEST_LIST" "$S/regress_$TAG.simlog" | sed 's/^/log list requests: /'
grep -oE "CAMERA_INFORMATION sent for [0-9]+" "$S/regress_$TAG.simlog" | sort -u | sed 's/^/  /'

adb shell am force-stop one.aircast.android
pkill -f apmvehicle.py 2>/dev/null
