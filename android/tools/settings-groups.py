import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
CORE = ROOT / "core-rs/src/settingsgroups.rs"
HEAD = ROOT / "android/app/src/main/java/one/aircast/android/ui/SettingsScreen.kt"

EXEMPT = {
    "FirmwareUpgrade": "flashing firmware needs a USB host and a bootloader dance this head does not do",
    "Viewer3D": "the 3D viewer is out of scope until the Qt viewer's future is decided",
    "PacketRadio": "the page needs the bespoke adapter picker block. The scope question is settled - "
        "packet radio belongs on a handset - and the library builds for Android as of f37267c73, but "
        "libusb cannot enumerate without a file descriptor handed in from Java, so a page drawn now "
        "would list no adapter on any device",
    "MavlinkActions": "two paths to JSON files that must already be on the device; the generic "
        "renderer draws two text fields nobody can usefully fill without a file picker",
    "BrandImage": "same shape - two image paths with no way to choose a file on a phone",
    "FlightMode": "twelve comma-separated lists of hidden mode names per airframe. The mode picker "
        "reads what these produce; drawing the raw lists is worse than not drawing them",
}


def core_groups():
    return re.findall(r'Group \{ name: "([^"]+)"', CORE.read_text())


def head_paths():
    return set(re.findall(r'"settings\.(\w+)"', HEAD.read_text()))


def stem(name):
    run = re.match(r"[A-Z]+", name).group()
    keep = len(run) - 1 if len(run) > 1 and len(run) < len(name) else len(run)
    return name[:keep].lower() + name[keep:]


def drawn(name, paths):
    return stem(name) in paths or stem(name) + "Settings" in paths


def main():
    paths = head_paths()
    missing = [name for name in core_groups() if not drawn(name, paths)]
    unexplained = [name for name in missing if name not in EXEMPT]
    for name in missing:
        print(f"  {name:26} {EXEMPT.get(name, 'UNREACHABLE - no entry and no stated reason')}")
    print(f"{len(core_groups())} groups served, {len(missing)} not drawn, {len(unexplained)} without a reason")
    stale = [name for name in EXEMPT if drawn(name, paths)]
    for name in stale:
        print(f"  {name} is drawn now - drop its exemption")
    return 1 if unexplained or stale else 0


if __name__ == "__main__":
    sys.exit(main())
