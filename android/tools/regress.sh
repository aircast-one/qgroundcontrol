#!/bin/bash
export PATH="$PATH:$HOME/Library/Android/sdk/platform-tools"
S="$(cd "$(dirname "$0")" && pwd)"
TAG="${1:-run}"
set -u

# Four sessions share this checkout, so "$S/apmvehicle.py" is the SAME string for all of
# them and no path anchor separates one session's fake from another's. Killing by pattern
# here would stop a peer's vehicle mid-run, and a fake that stops sending reads as an
# unbuilt feature rather than as interference. Refuse instead, and kill only our own PID.
if pgrep -f "$S/apmvehicle.py" >/dev/null; then
    echo "REFUSED: an apmvehicle.py is already running and this checkout is shared - it may be"
    echo "  another session's. Stop it yourself if it is yours: pkill -f '\$S/apmvehicle.py'"
    exit 1
fi
HANDSET="$("$S/handset-ip.sh")"
[ -n "$HANDSET" ] || { echo "FAIL: no handset address - is it attached and on Wi-Fi?"; exit 1; }
echo "handset at $HANDSET"
RC_RSSI=203 BATT_PCT=70 BATT_STATE=0 nohup python3 "$S/apmvehicle.py" "$HANDSET" > "$S/regress_$TAG.simlog" 2>&1 &
SIM=$!
python3 -c "import time; time.sleep(5)"
kill -0 "$SIM" 2>/dev/null || { echo "FAIL: sim did not start"; tail -3 "$S/regress_$TAG.simlog"; exit 1; }

adb shell am force-stop one.aircast.android
"$S/ui.sh" front || { echo "FAIL: app did not come to the front"; exit 1; }
python3 -c "import time; time.sleep(30)"

shot() {
    "$S/ui.sh" shot "$S/${TAG}_$1.png" || echo "FAIL: $1"
}

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

RTL=20; TERMINATE=185; PARACHUTE=208; MISSION_START=300
FLIGHT_COMMAND="^(ARM |TAKEOFF |LAND |CMD ($RTL|$TERMINATE|$PARACHUTE|$MISSION_START) )"
COMMANDED=$(grep -cE "$FLIGHT_COMMAND" "$S/regress_$TAG.simlog")
if [ "$COMMANDED" != "0" ]; then
    echo "FAIL: the run commanded the vehicle $COMMANDED time(s) - a tap landed on a flight control"
    grep -E "$FLIGHT_COMMAND" "$S/regress_$TAG.simlog" | sed 's/^/  /'
    exit 1
fi
echo "commanded the vehicle: 0"

echo "--- sim saw ---"
grep -c "STATUSTEXT" "$S/regress_$TAG.simlog" | sed 's/^/statustext sent: /'
grep -c "LOG_REQUEST_LIST" "$S/regress_$TAG.simlog" | sed 's/^/log list requests: /'
grep -oE "CAMERA_INFORMATION sent for [0-9]+" "$S/regress_$TAG.simlog" | sort -u | sed 's/^/  /'

adb shell am force-stop one.aircast.android
kill "$SIM" 2>/dev/null
