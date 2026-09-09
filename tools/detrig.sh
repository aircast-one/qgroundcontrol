#!/bin/bash
# Drives the detection overlay end to end: SSE boxes on 8099 and a TCP video
# stream on 8100, both reached through adb reverse, with the device ini pointed
# at 127.0.0.1 so nothing depends on the phone's subnet.
#   detrig.sh up    - start feed, stream, ini, app
#   detrig.sh down  - stop everything and restore the ini
export PATH="$PATH:$HOME/Library/Android/sdk/platform-tools"
S="$(cd "$(dirname "$0")" && pwd)"
set -u

case "${1:-up}" in
up)
    pkill -f detfeed.py 2>/dev/null
    nohup python3 "$S/detfeed.py" > "$S/detfeed.log" 2>&1 &
    pkill -f "gst-launch-1.0" 2>/dev/null
    nohup gst-launch-1.0 -q videotestsrc pattern=ball is-live=true \
        ! video/x-raw,width=640,height=480,framerate=15/1 ! timeoverlay \
        ! x264enc tune=zerolatency bitrate=800 key-int-max=15 \
        ! mpegtsmux ! tcpserversink host=0.0.0.0 port=8100 > "$S/gst.log" 2>&1 &
    python3 -c "import time; time.sleep(3)"
    adb reverse tcp:8099 tcp:8099
    adb reverse tcp:8100 tcp:8100
    adb shell am force-stop one.aircast.android
    adb push "$S/det3.ini" /data/local/tmp/det3.ini >/dev/null
    adb shell "run-as one.aircast.android cp /data/local/tmp/det3.ini 'files/settings/Aircast/Aircast QGC Daily.ini'"
    adb shell am start -n one.aircast.android/.MainActivity >/dev/null
    echo "rig up - give the app 50 s, then screencap"
    ;;
down)
    adb shell am force-stop one.aircast.android
    adb push "$S/base.ini" /data/local/tmp/base.ini >/dev/null
    adb shell "run-as one.aircast.android cp /data/local/tmp/base.ini 'files/settings/Aircast/Aircast QGC Daily.ini'"
    adb shell rm -f /data/local/tmp/det3.ini /data/local/tmp/base.ini
    adb reverse --remove tcp:8099 2>/dev/null
    adb reverse --remove tcp:8100 2>/dev/null
    pkill -f detfeed.py 2>/dev/null
    pkill -f "gst-launch-1.0" 2>/dev/null
    echo "rig down - ini restored"
    ;;
esac
