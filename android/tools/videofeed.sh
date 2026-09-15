#!/bin/bash
# A decodable video stream for the emulator, so the video surface actually renders.
#
#   videofeed.sh            serve on 5600 and forward it
#   videofeed.sh stop       stop serving
#
# Nothing on the emulator can decode a stream the host is not sending, and the
# Fly view's detection overlay composes only while video.decoding is true - so
# without this, every detection check reads as a dead feature rather than an
# unconfigured one.
#
# TCP rather than UDP because `adb reverse` forwards TCP only. The head dials
# out to 127.0.0.1:5600, which the reverse lands on this machine.
#
# Set it up once in the app: Settings > Video > Video source > TCP-MPEG2 Video
# Stream, then Video TCP Url > 127.0.0.1:5600 > Set.
export PATH="$PATH:$HOME/Library/Android/sdk/platform-tools"
# Four devices are usually attached here, so an unqualified adb reverse fails with
# "more than one device" and the stream looks broken when only the forward is.
if [ -z "${ANDROID_SERIAL:-}" ]; then
    ANDROID_SERIAL="$(adb devices | awk '$2 == "device" && $1 !~ /_adb\._tcp\./ { print $1; exit }')"
    [ -n "$ANDROID_SERIAL" ] && export ANDROID_SERIAL
fi
PORT="${QGC_VIDEO_PORT:-5600}"

if [ "${1:-}" = "stop" ]; then
    pkill -f "tcpserversink host=0.0.0.0 port=$PORT" && echo "stopped" || echo "nothing serving on $PORT"
    exit 0
fi

if lsof -nP -iTCP:$PORT -sTCP:LISTEN > /dev/null 2>&1; then
    echo "already serving on $PORT"
else
    nohup gst-launch-1.0 -q videotestsrc is-live=true pattern=ball \
        ! video/x-raw,width=640,height=480,framerate=15/1 \
        ! x264enc tune=zerolatency bitrate=800 key-int-max=15 \
        ! mpegtsmux ! tcpserversink host=0.0.0.0 port=$PORT > /tmp/videofeed.log 2>&1 &
    sleep 3
    lsof -nP -iTCP:$PORT -sTCP:LISTEN > /dev/null 2>&1 || { echo "FAILED to serve on $PORT - see /tmp/videofeed.log" >&2; exit 1; }
    echo "serving h.264 over MPEG-TS on $PORT"
fi

adb reverse "tcp:$PORT" "tcp:$PORT" > /dev/null 2>&1 &&
    echo "reversed tcp:$PORT - set the app's TCP url to 127.0.0.1:$PORT" ||
    echo "WARNING: adb reverse failed; the head cannot reach the stream" >&2
