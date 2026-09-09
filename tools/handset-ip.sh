#!/bin/bash
# The handset's address over Wi-Fi, which DHCP changes without warning. Every script that
# sends MAVLink at it should ask rather than remember: a stale address looks exactly like a
# broken app, "No vehicle" and nothing on the wire.
export PATH="$PATH:$HOME/Library/Android/sdk/platform-tools"
adb shell ip -f inet addr show wlan0 2>/dev/null |
    sed -n 's/.*inet \([0-9.]*\)\/.*/\1/p' |
    head -1
