#!/usr/bin/env python3
import base64
import json
import os
import re
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CLIPS = ROOT / "android/app/src/main/res/raw"
MODES = ROOT / "groundstation/src/modes.rs"
MODEL = os.environ.get("VOICE_MODEL", "gemini-3.1-flash-tts-preview")
VOICE = os.environ.get("VOICE", "Kore")
STYLE = "Say in a calm, clear, confident voice, like a modern aircraft cockpit voice alert, at an even pace:"

FIXED = [
    "warning",
    "armed",
    "disarmed",
    "signal lost",
    "signal regained",
    "battery level low",
    "battery level is critical",
    "battery level emergency",
    "battery failed",
    "battery unhealthy",
    "minimum altitude fence breached",
    "maximum altitude fence breached",
    "boundary fence breached",
    "communication lost on primary link",
    "communication lost on secondary link",
    "communication regained on primary link",
    "communication regained on secondary link",
    "switching communication back to the direct link",
    "switching communication to secondary link",
    "flight mode change refused for now",
    "flight mode change denied",
    "flight mode change not supported",
    "flight mode change failed",
    "Battery needed to return home. Return now.",
]

SPOKEN = {
    "rtl": "return to launch",
    "autortl": "auto return to launch",
    "adsb": "A D S B",
    "fbw": "fly by wire",
    "quadplane": "quad plane",
    "qland": "Q land",
    "autotune": "auto tune",
    "autorotate": "auto rotate",
    "systemid": "system I D",
    "zigzag": "zig zag",
    "surftrak": "surf track",
    "gps": "G P S",
}


def clip(text):
    return re.sub(r"[^a-z0-9]+", "_", text.lower()).strip("_")


def spoken(text):
    return " ".join(SPOKEN.get(word.lower(), word) for word in text.split())


def phrases():
    modes = sorted(set(re.findall(r'name: "([^"]+)"', MODES.read_text())))
    return FIXED + [f"{mode} flight mode" for mode in modes]


def synthesize(text, voice, attempts=5):
    body = {
        "contents": [{"parts": [{"text": f"{STYLE} {spoken(text)}"}]}],
        "generationConfig": {"responseModalities": ["AUDIO"], "speechConfig": {"voiceConfig": {"prebuiltVoiceConfig": {"voiceName": voice}}}},
    }
    request = urllib.request.Request(
        f"https://generativelanguage.googleapis.com/v1beta/models/{MODEL}:generateContent",
        data=json.dumps(body).encode(),
        headers={"x-goog-api-key": os.environ["GEMINI_API_KEY"], "Content-Type": "application/json"},
    )
    try:
        with urllib.request.urlopen(request, timeout=120) as response:
            return base64.b64decode(json.load(response)["candidates"][0]["content"]["parts"][0]["inlineData"]["data"])
    except urllib.error.HTTPError as error:
        if attempts <= 1 or error.code not in (400, 429, 500, 503):
            raise
        time.sleep(30 if error.code == 429 else 5)
        return synthesize(text, voice, attempts - 1)


def record(text, out, voice=VOICE):
    trim = "silenceremove=start_periods=1:start_threshold=-45dB,areverse,silenceremove=start_periods=1:start_threshold=-45dB,areverse"
    subprocess.run(
        ["ffmpeg", "-loglevel", "error", "-y", "-f", "s16le", "-ar", "24000", "-ac", "1", "-i", "-",
         "-af", f"{trim},loudnorm=I=-14:TP=-1.5:LRA=7", "-ar", "24000", "-c:a", "aac", "-b:a", "48k", str(out)],
        input=synthesize(text, voice),
        check=True,
    )


def main(force):
    CLIPS.mkdir(parents=True, exist_ok=True)
    wanted = {clip(text): text for text in phrases()}
    stale = [path for path in CLIPS.glob("*.m4a") if path.stem not in wanted]
    for path in stale:
        path.unlink()
    missing = [(name, text) for name, text in wanted.items() if force or not (CLIPS / f"{name}.m4a").exists()]
    for name, text in missing:
        print(f"{name}: {text}", flush=True)
        record(text, CLIPS / f"{name}.m4a")
    print(f"{len(wanted)} clips, {len(missing)} recorded, {len(stale)} removed")


if __name__ == "__main__":
    main("--force" in sys.argv[1:])
