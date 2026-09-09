#!/bin/bash
# Drives every surface exercised on 2026-09-08 and captures each one.
# Usage: regress.sh <tag>
export PATH="$PATH:$HOME/Library/Android/sdk/platform-tools"
S="$(cd "$(dirname "$0")" && pwd)"
TAG="${1:-run}"
set -u

pkill -f apmvehicle.py 2>/dev/null
RC_RSSI=203 BATT_PCT=70 BATT_STATE=0 nohup python3 "$S/apmvehicle.py" 192.168.1.58 > "$S/regress_$TAG.simlog" 2>&1 &
python3 -c "import time; time.sleep(5)"
pgrep -f apmvehicle.py >/dev/null || { echo "FAIL: sim did not start"; tail -3 "$S/regress_$TAG.simlog"; exit 1; }

adb shell am force-stop one.aircast.android
adb shell am start -n one.aircast.android/.MainActivity >/dev/null
python3 -c "import time; time.sleep(30)"

shot() {
    adb exec-out screencap -p > "$S/${TAG}_$1.png"
    local bytes
    bytes=$(stat -f%z "$S/${TAG}_$1.png")
    if [ "$bytes" -lt 100000 ]; then echo "FAIL: $1 capture is ${bytes}B - device asleep?"; else echo "ok  $1 (${bytes}B)"; fi
}

# Analyze rows, with a vehicle connected: Log Download 429, Vibration 614,
# Console 799, Inspector 984. Preflight moved to the Fly tab's Actions sheet.
shot fly
adb shell input tap 573 1744; python3 -c "import time; time.sleep(3)"; shot actions
adb shell input keyevent 4;   python3 -c "import time; time.sleep(2)"
adb shell input tap 814 2108; python3 -c "import time; time.sleep(3)"
adb shell input tap 300 614;  python3 -c "import time; time.sleep(4)"; shot vibration
adb shell input keyevent 4;   python3 -c "import time; time.sleep(2)"
adb shell input tap 300 429;  python3 -c "import time; time.sleep(8)"; shot logs
adb shell input keyevent 4;   python3 -c "import time; time.sleep(2)"
adb shell input tap 995 2109; python3 -c "import time; time.sleep(3)"; shot settings
adb shell input tap 82 2109;  python3 -c "import time; time.sleep(4)"; shot flyback

echo "--- sim saw ---"
grep -c "STATUSTEXT" "$S/regress_$TAG.simlog" | sed 's/^/statustext sent: /'
grep -c "LOG_REQUEST_LIST" "$S/regress_$TAG.simlog" | sed 's/^/log list requests: /'
grep -oE "CAMERA_INFORMATION sent for [0-9]+" "$S/regress_$TAG.simlog" | sort -u | sed 's/^/  /'

adb shell am force-stop one.aircast.android
pkill -f apmvehicle.py 2>/dev/null
