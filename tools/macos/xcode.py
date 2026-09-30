#!/usr/bin/env python3
from __future__ import annotations

import os
import plistlib
import re
import shlex
import shutil
import subprocess
import sys
from functools import reduce
from pathlib import Path

_tools_dir = str(Path(__file__).resolve().parent.parent)
if _tools_dir not in sys.path:
    sys.path.insert(0, _tools_dir)

from _bootstrap import ensure_tools_dir

ensure_tools_dir(__file__)

from common.file_traversal import find_repo_root
from common.generator import die, env, missing_keys, parse_cache, run

ROOT = find_repo_root(Path(__file__))
PROJECT_DIR = ROOT / "macos"
PLIST_VARIABLE = re.compile(r"\$\{(\w+)\}")

REQUIRED_KEYS = (
    "CMAKE_BUILD_TYPE",
    "CMAKE_COMMAND",
    "CMAKE_OSX_DEPLOYMENT_TARGET",
    "CMAKE_PROJECT_DESCRIPTION",
    "CMAKE_PROJECT_NAME",
    "CMAKE_PROJECT_VERSION",
    "CMAKE_PROJECT_VERSION_MAJOR",
    "CMAKE_PROJECT_VERSION_MINOR",
    "CMAKE_PROJECT_VERSION_PATCH",
    "QGC_APP_COPYRIGHT",
    "QGC_APP_DISPLAY_NAME",
    "QGC_MACOS_BUNDLE_ID",
    "QGC_MACOS_ICON_PATH",
    "QGC_MACOS_PLIST_PATH",
)

CONFIGURE_HINT = (
    "QGC_CONFIGURE_ARGS adds flags to the first configure of a build directory, e.g.\n"
    "  QGC_CONFIGURE_ARGS='-DGStreamer_ROOT_DIR=$HOME/gstreamer-sdk/macos-1.28.4/root"
    " -DGStreamer_USE_FRAMEWORK=OFF' just xcode"
)


def pinned_qt_version() -> str:
    result = subprocess.run(
        [sys.executable, str(ROOT / "tools/setup/read_config.py"), "--get", "qt.version"],
        capture_output=True,
        text=True,
        cwd=ROOT,
    )
    if result.returncode != 0 or not result.stdout.strip():
        die("cannot read qt.version from .github/build-config.json", result.stderr.strip())
    return result.stdout.strip()


def installed_qt_version(root: Path) -> str:
    result = subprocess.run(
        [str(root / "bin/qmake"), "-query", "QT_VERSION"], capture_output=True, text=True
    )
    return result.stdout.strip() if result.returncode == 0 else ""


def qt_root() -> Path:
    pinned = pinned_qt_version()
    override = env("QT_ROOT_DIR")
    root = Path(override) if override else Path.home() / "Qt" / pinned / "macos"
    source = "QT_ROOT_DIR" if override else "the default location"
    if not (root / "bin/qt-cmake").is_file():
        die(
            f"no Qt for macOS at {root} ({source})",
            f".github/build-config.json pins Qt {pinned}: install it, or set QT_ROOT_DIR to it",
        )
    installed = installed_qt_version(root)
    if installed != pinned:
        die(
            f"{root} is Qt {installed or 'unknown'}, but .github/build-config.json pins {pinned}",
            "build the standalone app against the pinned Qt, or change the pin deliberately",
        )
    return root


def configure(build: Path) -> None:
    root = qt_root()
    extra = shlex.split(env("QGC_CONFIGURE_ARGS") or "")
    run(
        [
            sys.executable,
            str(ROOT / "tools/configure.py"),
            "-B",
            str(build),
            "--release",
            "--qt-root",
            str(root),
            "--",
            "-DQGC_HEADLESS_CORE=ON",
            "-DQGC_BUILD_TESTING=OFF",
            f"-DQt6_DIR={root}/lib/cmake/Qt6",
            *extra,
        ],
        ROOT,
        f"configuring {build} failed\n{CONFIGURE_HINT}",
    )


def read_cache(build: Path) -> dict[str, str]:
    cache = build / "CMakeCache.txt"
    if not cache.is_file():
        configure(build)
    if not cache.is_file():
        die(f"{cache} still does not exist after configuring", CONFIGURE_HINT)
    values = parse_cache(cache)
    headless = values.get("QGC_HEADLESS_CORE", "OFF")
    if headless != "ON":
        die(
            f"{build} has QGC_HEADLESS_CORE={headless}",
            "the standalone app links a core with no QML: reconfigure with -DQGC_HEADLESS_CORE=ON",
        )
    missing = missing_keys(values, REQUIRED_KEYS)
    if missing:
        die(
            f"{cache} has no value for: {', '.join(missing)}",
            f"reconfigure {build}, or set QGC_CORE_BUILD_DIR to a QGC build directory",
        )
    return values


def write_xcconfig(values: dict[str, str], build: Path) -> None:
    settings = {
        "QGC_ROOT": str(ROOT),
        "QGC_CMAKE": values["CMAKE_COMMAND"],
        "QGC_APP_NAME": values["CMAKE_PROJECT_NAME"],
        "QGC_CORE_BUILD_DIR": str(build),
        "QGC_CORE_LIB_DIR": str(build / values["CMAKE_BUILD_TYPE"]),
        "QGC_BRIDGE_DIR": str(ROOT / "src/Bridge"),
        "PRODUCT_BUNDLE_IDENTIFIER": values["QGC_MACOS_BUNDLE_ID"],
        "MACOSX_DEPLOYMENT_TARGET": values["CMAKE_OSX_DEPLOYMENT_TARGET"],
        "MARKETING_VERSION": ".".join(
            values[f"CMAKE_PROJECT_VERSION_{part}"] for part in ("MAJOR", "MINOR")
        ),
        "CURRENT_PROJECT_VERSION": values["CMAKE_PROJECT_VERSION"],
    }
    (PROJECT_DIR / "Local.xcconfig").write_text(
        "".join(f"{key} = {value}\n" for key, value in settings.items())
    )


def write_info_plist(values: dict[str, str]) -> None:
    template_path = Path(values["QGC_MACOS_PLIST_PATH"])
    major, minor, patch = (
        values[f"CMAKE_PROJECT_VERSION_{part}"] for part in ("MAJOR", "MINOR", "PATCH")
    )
    substitutions = {
        "MACOSX_BUNDLE_BUNDLE_NAME": values["QGC_APP_DISPLAY_NAME"],
        "MACOSX_BUNDLE_GUI_IDENTIFIER": "$(PRODUCT_BUNDLE_IDENTIFIER)",
        "MACOSX_BUNDLE_EXECUTABLE_NAME": "$(EXECUTABLE_NAME)",
        "MACOSX_BUNDLE_BUNDLE_VERSION": values["CMAKE_PROJECT_VERSION"],
        "MACOSX_BUNDLE_SHORT_VERSION_STRING": f"{major}.{minor}",
        "MACOSX_BUNDLE_LONG_VERSION_STRING": f"{major}.{minor}.{patch}",
        "MACOSX_BUNDLE_INFO_STRING": values["CMAKE_PROJECT_DESCRIPTION"],
        "MACOSX_BUNDLE_COPYRIGHT": values["QGC_APP_COPYRIGHT"],
        "MACOSX_BUNDLE_ICON_FILE": Path(values["QGC_MACOS_ICON_PATH"]).name,
        "CMAKE_OSX_DEPLOYMENT_TARGET": values["CMAKE_OSX_DEPLOYMENT_TARGET"],
    }
    rendered = reduce(
        lambda text, item: text.replace(f"${{{item[0]}}}", item[1]),
        substitutions.items(),
        template_path.read_text(),
    )
    unresolved = sorted(set(PLIST_VARIABLE.findall(rendered)))
    if unresolved:
        die(
            f"{template_path} uses variables this generator cannot fill: {', '.join(unresolved)}",
            "add them to substitutions in tools/macos/xcode.py",
        )
    (PROJECT_DIR / "Info.plist").write_bytes(
        plistlib.dumps({**plistlib.loads(rendered.encode()), "QGCNativeUI": True})
    )


def build_core(values: dict[str, str], build: Path) -> None:
    name = values["CMAKE_PROJECT_NAME"]
    run(
        [values["CMAKE_COMMAND"], "--build", str(build), "--target", name],
        ROOT,
        f"building the {name} core in {build} failed",
    )
    dylib = build / values["CMAKE_BUILD_TYPE"] / f"lib{name}.dylib"
    if not dylib.is_file():
        die(
            f"the core built but {dylib} is not there",
            f"the Xcode targets link -l{name} out of that directory",
        )


def generate() -> None:
    if shutil.which("xcodegen") is None:
        die("xcodegen not found: brew install xcodegen")
    run(
        ["xcodegen", "generate", "--quiet"],
        PROJECT_DIR,
        "xcodegen failed",
        environment={**os.environ, "QGC_ROOT": str(ROOT)},
    )


def main() -> int:
    if sys.platform != "darwin":
        die("the standalone app is macOS only")
    requested = Path(env("QGC_CORE_BUILD_DIR") or "build-hl")
    build = requested if requested.is_absolute() else ROOT / requested
    values = read_cache(build)
    write_xcconfig(values, build)
    write_info_plist(values)
    build_core(values, build)
    generate()
    print(
        f"macos/{values['CMAKE_PROJECT_NAME']}.xcodeproj generated against"
        f" {build} ({values['CMAKE_BUILD_TYPE']})"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
