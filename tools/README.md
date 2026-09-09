# Rig

The scripts that drive a handset against a simulated vehicle. They were written in a session
scratchpad, which does not survive the session; they live here so the next one does not rebuild
them.

Everything assumes `adb` on the PATH. Ask `handset-ip.sh` for the address rather than remembering one: DHCP moved it mid-session once, and a stale address looks exactly like a broken app — "No vehicle" and nothing on the wire. Paths inside the scripts
still point at the scratchpad they were written in — fix those before use, or run them from a
copy there.

| script | what it does |
|---|---|
| `handset-ip.sh` | The handset's Wi-Fi address, from `adb`. |
| `ui.sh` | `front`, `tap`, `swipe`, `text`, `key`, `shot`. Every input checks `topResumedActivity` first and refuses rather than guess: `am start` returns before the window is up, and a tap sent into that gap lands in whatever the user had open. |
| `apmvehicle.py` | An ArduCopter-shaped MAVLink vehicle: heartbeats, GPS, battery, RC, vibration, two camera components, log download, and a print for every command it receives. `RC_RSSI`, `BATT_PCT`, `BATT_STATE`, `NOFIX` and `VIBRATION` are environment knobs. |
| `device-lock.sh` | `take`/`drop` around `/tmp/aircast-device.lock`, and stops the handset dozing while held. |
| `regress.sh` | Drives Fly, the Actions sheet, Vibration, Log Download and Settings, captures each, and fails a capture under 100 000 bytes because a sleeping screen photographs as a small black rectangle. |
| `detrig.sh` | `up`/`down` for the detection overlay: an SSE feed on 8099 and a TCP video stream on 8100, both through `adb reverse`, with the device ini pointed at `127.0.0.1`. |
| `detfeed.py` | The SSE detection feed `detrig.sh` serves — two boxes, one of them moving. |
| `watchprobe.py` | `on`/`off` around timing instrumentation in `Watcher::_poll`. It asserts the poll body is in the shape it expects, so it fails loudly when the bridge changes rather than patching the wrong thing. |

The rule these were built to serve: an empty screen is not evidence that a screen works. Every
one of them reports what the vehicle received or what the screen actually showed, not that a
command was sent.
