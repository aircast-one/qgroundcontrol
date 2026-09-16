import json
import os
import subprocess
import time

TOOLS = os.path.dirname(os.path.abspath(__file__))
APP = "one.aircast.app"


def sh(command):
    return subprocess.run(command, shell=True, capture_output=True, text=True).stdout.strip()


def view(path):
    raw = sh(f"{TOOLS}/probe.sh get '{path}'")
    try:
        return json.loads(raw)
    except json.JSONDecodeError:
        raise SystemExit(f"probe returned no JSON for {path}: {raw[:200]}")


def find(selector):
    return sh(f"{TOOLS}/ui.sh find {selector}")


def wake():
    if "Awake" not in sh("adb shell dumpsys power | grep mWakefulness="):
        sh("adb shell input keyevent KEYCODE_WAKEUP")
        time.sleep(2)


def freshness():
    out = subprocess.run(
        f"python3 {TOOLS}/fresh.py", shell=True, capture_output=True, text=True
    )
    for line in out.stdout.splitlines():
        if "STALE" in line:
            print(f"  rig: {line.strip()}")


def vehicle_present():
    raw = sh(f"{TOOLS}/probe.sh get vehicle.vehicleLinkManager.communicationLost")
    if '"value": true' in raw or '"value":true' in raw:
        print("  rig: COMMUNICATION LOST - no vehicle is talking. vehicle.latitude keeps its")
        print("       last value for at least a minute after the fake dies, so the map looks")
        print("       fine and anything placed relative to it lands nowhere.")
        return False
    return True


def ensure_app():
    wake()
    freshness()
    vehicle_present()
    if "REFUSED" in sh(f"{TOOLS}/ui.sh text 2>&1"):
        sh(f"adb shell am start -n {APP}/one.aircast.android.MainActivity")
        time.sleep(8)
        sh(f"{TOOLS}/probe.sh on")


def on_plan():
    ensure_app()
    if find("text='Save as…'"):
        sh("adb shell input keyevent BACK")
        time.sleep(2)
    for _ in range(3):
        if find("text=Fence"):
            return
        plan = find("text=Plan")
        if not plan:
            raise SystemExit("PRECONDITION: no Plan tab on screen to switch to")
        sh(f"{TOOLS}/ui.sh tap {plan}")
        time.sleep(4)
    raise SystemExit("PRECONDITION: tapped Plan three times and the Plan tab never came up")


def tap_label(label):
    where = find(f"text='{label}'")
    if not where:
        on_plan()
        where = find(f"text='{label}'")
    if not where:
        raise SystemExit(f"PRECONDITION: no control labelled {label} on screen")
    sh(f"{TOOLS}/ui.sh tap {where}")
    time.sleep(3)


def new_plan():
    on_plan()
    tap_label("File")
    tap_label("New plan")
    discard = find("text='Discard and start new'")
    if discard:
        sh(f"{TOOLS}/ui.sh tap {discard}")
        time.sleep(3)


def long_press(x, y):
    sh(f"adb shell input swipe {x} {y} {x} {y} 900")
    time.sleep(3)


def items():
    return view("view.missionItems").get("items", [])


def coordinates():
    return [
        (
            item.get("sequence"),
            round((item.get("coordinate") or {}).get("latitude", 0.0), 6),
            round((item.get("coordinate") or {}).get("longitude", 0.0), 6),
        )
        for item in items()
    ]


def open_list():
    handle = find("content-desc='Show the plan as a list'")
    if not handle:
        raise SystemExit("PRECONDITION: the plan list handle is not on screen")
    sh(f"{TOOLS}/ui.sh tap {handle}")
    time.sleep(3)


def close_list():
    shut = find("content-desc='Close sheet'")
    if shut:
        sh(f"{TOOLS}/ui.sh tap {shut}")
        time.sleep(2)


def adding_after():
    for line in sh(f"{TOOLS}/ui.sh text").split("|"):
        if "Adding after #" in line:
            return int(line.strip().split("#")[1])
    return None
