import datetime
import os
import re
import subprocess
import sys

CORE = os.environ.get("FRESH_CORE", "/Users/pavliha/Code/aircast/qgroundcontrol/core-rs/src")
BRIDGE = os.environ.get("FRESH_BRIDGE", "/Users/pavliha/Code/aircast/qgroundcontrol/src/Bridge")
LIB = os.environ.get("FRESH_LIB", "/Users/pavliha/Code/aircast/qgroundcontrol/build-android/Release/libAircastQGC_arm64-v8a.so")
APP = "one.aircast.android"


def newest(directory, suffixes):
    best = None
    for base, _, names in os.walk(directory):
        for name in names:
            if not name.endswith(suffixes):
                continue
            path = os.path.join(base, name)
            stamp = os.path.getmtime(path)
            if best is None or stamp > best[0]:
                best = (stamp, path)
    return best


def installed_at():
    serial = os.environ.get("ANDROID_SERIAL")
    target = f"adb -s {serial}" if serial else "adb"
    out = subprocess.run(
        f"{target} shell dumpsys package {APP}", shell=True, capture_output=True, text=True
    ).stdout
    found = re.search(r"lastUpdateTime=(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2})", out)
    if not found:
        return None
    return datetime.datetime.strptime(found.group(1), "%Y-%m-%d %H:%M:%S").timestamp()


def show(stamp):
    return datetime.datetime.fromtimestamp(stamp).strftime("%H:%M:%S")


def against(path):
    if not os.path.exists(path):
        raise SystemExit(f"no such source: {path}")
    lib = os.path.getmtime(LIB)
    source = os.path.getmtime(path)
    print(f"{os.path.basename(path)} changed {show(source)}")
    print(f"library built   {show(lib)}")
    if source > lib:
        print(f"  STALE: the library predates {os.path.basename(path)} - it cannot hold what that file added")
        return 1
    print(f"  the library postdates {os.path.basename(path)}, so it can hold what that file defines")
    print("  other files may have changed since; this answers one question, not the general one")
    return 0


def main():
    if "--for" in sys.argv:
        return against(sys.argv[sys.argv.index("--for") + 1])
    core = newest(CORE, (".rs",))
    bridge = newest(BRIDGE, (".cc", ".h"))
    source = max([s for s in (core, bridge) if s], key=lambda s: s[0])
    if not os.path.exists(LIB):
        raise SystemExit(f"no library at {LIB}")
    lib = os.path.getmtime(LIB)
    device = installed_at()

    print(f"newest source   {show(source[0])}  {os.path.basename(source[1])}")
    print(f"library built   {show(lib)}")
    print(f"app installed   {show(device) if device else 'unknown - is the device attached?'}")

    behind = []
    if source[0] > lib:
        behind.append(
            f"the library is {int((source[0] - lib) // 60)} min older than {os.path.basename(source[1])}"
            " - rebuild the aar before believing any reading"
        )
    if device and lib > device:
        behind.append(
            f"the app on the handset is {int((lib - device) // 60)} min older than the library"
            " - installDebug before believing any reading"
        )
    for line in behind:
        print(f"  STALE: {line}")
    if not behind:
        print("  every link is at least as new as the one before it")
    return 1 if behind else 0


sys.exit(main())
