import json
import re
import subprocess
import sys
import time
import urllib.request

PORT = "8790"
STICKS = "/tmp/aircast-sticks"
TOOLS = __file__.rsplit("/", 1)[0]
CENTRE = [1500] * 8
CHANNEL = {"throttle": 2, "yaw": 3, "roll": 0, "pitch": 1}
HIGH = {"up": True, "right": True, "down": False, "left": False}


def view():
    request = urllib.request.Request(
        "http://127.0.0.1:%s/bridge/get?path=view.radio" % PORT,
        headers={"X-QGC-Debug-Api": "1"})
    return json.load(urllib.request.urlopen(request, timeout=6))


def hold(values):
    open(STICKS, "w").write(" ".join(str(v) for v in values) + "\n")


def tap(label):
    spot = subprocess.run(["bash", TOOLS + "/ui.sh", "find", "text=" + label],
                          capture_output=True, text=True).stdout.split()
    if len(spot) != 2:
        return False
    subprocess.run(["bash", TOOLS + "/ui.sh", "tap"] + spot, capture_output=True)
    return True


def sticks_for(text):
    lowered = text.lower()
    stick = next((s for s in CHANNEL if s in lowered), None)
    way = next((w for w in HIGH if re.search(r"\b%s\b" % w, lowered)), None)
    if stick is None or way is None:
        return None
    return [1900 if HIGH[way] and i == CHANNEL[stick]
            else 1100 if i == CHANNEL[stick]
            else 1500
            for i in range(8)]


def main():
    last = ""
    for _ in range(60):
        state = view()
        text = state["statusText"].strip()
        if text and text != last:
            print("STEP: %s" % text.splitlines()[0][:90], flush=True)
        started = bool(last)
        last = text or last
        if not state["calibrating"]:
            if started:
                print("DONE", flush=True)
                return 0
            tap("Calibrate")
            time.sleep(3)
            continue
        values = sticks_for(text)
        lowered = text.lower()
        if values:
            hold(values)
        elif "center" in lowered:
            hold(CENTRE)
        elif "switches" in lowered:
            for extreme in (1100, 1900):
                hold(CENTRE[:4] + [extreme] * 4)
                time.sleep(1.5)
        if state["nextEnabled"] and (values is None and "center" not in lowered
                                     or "click next" in lowered):
            time.sleep(1.0)
            tap("Next")
        time.sleep(2.5)
    print("GAVE UP still calibrating", flush=True)
    return 1


if __name__ == "__main__":
    sys.exit(main())
