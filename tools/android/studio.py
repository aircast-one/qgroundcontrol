#!/usr/bin/env python3
from __future__ import annotations

import os
import re
import shlex
import subprocess
import sys
from pathlib import Path
from typing import NamedTuple

_tools_dir = str(Path(__file__).resolve().parent.parent)
if _tools_dir not in sys.path:
    sys.path.insert(0, _tools_dir)

from _bootstrap import ensure_tools_dir

ensure_tools_dir(__file__)

from common.build_config import get_build_config_value
from common.file_traversal import find_repo_root
from common.generator import die, env, missing_keys, parse_cache, run

ROOT = find_repo_root(Path(__file__))
ANDROID_DIR = ROOT / "android"
BUILD_CONFIG = ROOT / ".github" / "build-config.json"
QT_PACKAGE_VERSION = re.compile(r'set\(PACKAGE_VERSION\s+"([^"]+)"\)')
JAVA_RELEASE_VERSION = re.compile(r'^JAVA_VERSION="(\d+)', re.MULTILINE)
JAVAC_VERSION = re.compile(r"javac (\d+)")
SDK_DIR_LINE = re.compile(r"^\s*sdk\.dir\s*[=:]\s*(.*)$")
PROPERTIES_ESCAPE = re.compile(r"\\(.)")
QT_ABI_DIR = "android_arm64_v8a"
ANDROID_ABI = "arm64-v8a"

REQUIRED_KEYS = (
    "CMAKE_BUILD_TYPE",
    "CMAKE_COMMAND",
    "CMAKE_PROJECT_NAME",
    "CMAKE_PROJECT_VERSION",
    "CMAKE_PROJECT_VERSION_MAJOR",
    "CMAKE_PROJECT_VERSION_MINOR",
    "CMAKE_PROJECT_VERSION_PATCH",
)

CONFIGURE_HINT = (
    "QGC_CONFIGURE_ARGS adds flags to the first configure of a build directory, e.g.\n"
    "  QGC_CONFIGURE_ARGS='-DQGC_ENABLE_GST_VIDEOSTREAMING=OFF' just android"
)


class Pins(NamedTuple):
    qt: str
    ndk: str
    platform: str
    min_sdk: str
    java: str
    abi: str


def pins() -> Pins:
    values = {
        name: get_build_config_value(key, config_file=BUILD_CONFIG)
        for name, key in (
            ("qt", "qt.version"),
            ("ndk", "android.ndk_full_version"),
            ("platform", "android.platform"),
            ("min_sdk", "android.min_sdk"),
            ("java", "android.java_version"),
        )
    }
    unset = sorted(name for name, value in values.items() if not value)
    if unset:
        die(f"{BUILD_CONFIG} has no value for: {', '.join(unset)}")
    return Pins(**values, abi=ANDROID_ABI)


def qt_version_at(root: Path) -> str:
    markers = (
        root / "lib/cmake/Qt6" / name
        for name in ("Qt6ConfigVersionImpl.cmake", "Qt6ConfigVersion.cmake")
    )
    found = (QT_PACKAGE_VERSION.search(m.read_text()) for m in markers if m.is_file())
    return next((f.group(1) for f in found if f), "")


def qt_roots(pinned: str) -> tuple[Path, Path]:
    override = env("QT_ANDROID_ROOT")
    base = Path(override) if override else Path.home() / "Qt" / pinned
    source = "QT_ANDROID_ROOT" if override else "the default location"
    android_root, host_root = base / QT_ABI_DIR, base / "macos"
    if not (android_root / "bin/qt-cmake").is_file():
        die(
            f"no Qt for Android at {android_root} ({source})",
            f".github/build-config.json pins Qt {pinned}: install it, or set QT_ANDROID_ROOT",
        )
    if not host_root.is_dir():
        die(f"Qt for Android needs a host Qt beside it, and {host_root} is not there")
    installed = qt_version_at(android_root)
    if installed != pinned:
        die(
            f"{android_root} is Qt {installed or 'unknown'}, but .github/build-config.json pins {pinned}",
            "build against the pinned Qt, or change the pin deliberately",
        )
    return android_root, host_root


def declared_sdk_dir(local: Path) -> str | None:
    matches = (SDK_DIR_LINE.match(line) for line in local.read_text().splitlines())
    declared = [PROPERTIES_ESCAPE.sub(r"\1", match.group(1).strip()) for match in matches if match]
    if not declared:
        die(
            f"{local} exists but declares no sdk.dir",
            "add sdk.dir=<path to the Android SDK>, or set ANDROID_SDK_ROOT",
        )
    return declared[0]


def sdk_root() -> Path:
    override = env("ANDROID_SDK_ROOT") or env("ANDROID_HOME")
    local = ANDROID_DIR / "local.properties"
    if not override and local.is_file():
        override = declared_sdk_dir(local)
    root = Path(override) if override else Path.home() / "Library/Android/sdk"
    if not (root / "platform-tools").is_dir():
        die(
            f"no Android SDK at {root}",
            "set ANDROID_SDK_ROOT, or put sdk.dir in android/local.properties",
        )
    return root


def ndk_root(sdk: Path, pinned: str) -> Path:
    root = Path(env("ANDROID_NDK_ROOT") or sdk / "ndk" / pinned)
    if not root.is_dir():
        installed = (
            sorted(p.name for p in (sdk / "ndk").glob("*")) if (sdk / "ndk").is_dir() else []
        )
        die(
            f"NDK {pinned} (pinned in .github/build-config.json) is not at {root}",
            f"installed: {', '.join(installed) or 'none'}",
        )
    return root


def java_version_at(root: Path) -> str:
    release = root / "release"
    if release.is_file():
        found = JAVA_RELEASE_VERSION.search(release.read_text())
        if found:
            return found.group(1)
    result = subprocess.run([str(root / "bin/javac"), "-version"], capture_output=True, text=True)
    found = JAVAC_VERSION.search(result.stdout + result.stderr)
    return found.group(1) if found else ""


def java_home(pinned: str) -> Path:
    override = env("JAVA_HOME")
    root = Path(override) if override else Path(f"/opt/homebrew/opt/openjdk@{pinned}")
    source = "JAVA_HOME" if override else "the default location"
    if not (root / "bin/javac").is_file():
        die(
            f"no JDK at {root} ({source})",
            f".github/build-config.json pins JDK {pinned}:"
            f" brew install openjdk@{pinned}, or set JAVA_HOME to it",
        )
    installed = java_version_at(root)
    if installed != pinned:
        die(
            f"{root} is JDK {installed or 'unknown'}, but .github/build-config.json pins {pinned}",
            "Gradle compiles at the pinned release, so a different JDK will not link",
        )
    return root


def toolchain_env(sdk: Path, ndk: Path, java: Path) -> dict[str, str]:
    return {
        **os.environ,
        "JAVA_HOME": str(java),
        "ANDROID_SDK_ROOT": str(sdk),
        "ANDROID_NDK_ROOT": str(ndk),
        "PATH": f"{java}/bin:{os.environ['PATH']}",
    }


def configure(build: Path, pinned: Pins, sdk: Path, ndk: Path, java: Path) -> None:
    android_root, host_root = qt_roots(pinned.qt)
    extra = shlex.split(env("QGC_CONFIGURE_ARGS") or "")
    run(
        [
            str(android_root / "bin/qt-cmake"),
            "-S",
            str(ROOT),
            "-B",
            str(build),
            "-G",
            "Ninja",
            "-DCMAKE_BUILD_TYPE=Release",
            f"-DQT_HOST_PATH={host_root}",
            f"-DANDROID_SDK_ROOT={sdk}",
            f"-DANDROID_NDK_ROOT={ndk}",
            f"-DQGC_QT_ANDROID_MIN_SDK_VERSION={pinned.min_sdk}",
            f"-DQGC_QT_ANDROID_TARGET_SDK_VERSION={pinned.platform}",
            *extra,
        ],
        ROOT,
        f"configuring {build} failed\n{CONFIGURE_HINT}",
        environment=toolchain_env(sdk, ndk, java),
    )


def validate_cache(values: dict[str, str], build: Path, cache: Path, pinned: Pins) -> None:
    if not values.get("ANDROID_ABI"):
        die(
            f"{build} is a host build, not an Android one",
            "set QGC_CORE_BUILD_DIR to an Android build directory, or remove that one",
        )
    host = values.get("QT_HOST_PATH", "")
    if pinned.qt not in host:
        die(
            f"{build} was configured against {host or 'an unknown Qt'},"
            f" but .github/build-config.json pins {pinned.qt}",
            f"remove {build} so it is configured against the pinned Qt",
        )
    if values.get("ANDROID_NDK_ROOT", "").rsplit("/", 1)[-1] != pinned.ndk:
        die(
            f"{build} was configured against NDK {values.get('ANDROID_NDK_ROOT', 'unknown')},"
            f" but .github/build-config.json pins {pinned.ndk}",
            f"remove {build} so it is configured against the pinned NDK",
        )
    missing = missing_keys(values, REQUIRED_KEYS)
    if missing:
        die(
            f"{cache} has no value for: {', '.join(missing)}",
            f"reconfigure {build}, or set QGC_CORE_BUILD_DIR to a QGC build directory",
        )


def configured_cache(build: Path, pinned: Pins, sdk: Path, ndk: Path, java: Path) -> dict[str, str]:
    cache = build / "CMakeCache.txt"
    if not cache.is_file():
        configure(build, pinned, sdk, ndk, java)
    if not cache.is_file():
        die(f"{cache} still does not exist after configuring", CONFIGURE_HINT)
    values = parse_cache(cache)
    validate_cache(values, build, cache, pinned)
    return values


def build_aar(values: dict[str, str], build: Path, sdk: Path, ndk: Path, java: Path) -> Path:
    run(
        [values["CMAKE_COMMAND"], "--build", str(build), "--target", "aar"],
        ROOT,
        f"building the aar in {build} failed",
        environment=toolchain_env(sdk, ndk, java),
    )
    aar = build / "android-build" / f"{values['CMAKE_PROJECT_NAME']}.aar"
    if not aar.is_file():
        die(
            f"the aar target built but {aar} is not there", "the Gradle modules depend on that file"
        )
    return aar


def version_code(major: int, minor: int, patch: int) -> int:
    if not 0 <= minor < 100 or not 0 <= patch < 10000:
        die(
            f"version {major}.{minor}.{patch} does not fit the versionCode layout"
            " (minor < 100, patch < 10000)",
            "Play rejects a versionCode that does not increase, and this one would collide",
        )
    return major * 1000000 + minor * 10000 + patch


def write_properties(values: dict[str, str], pinned: Pins, aar: Path, java: Path) -> None:
    version = values["CMAKE_PROJECT_VERSION"]
    major, minor, patch = (
        int(values[f"CMAKE_PROJECT_VERSION_{part}"]) for part in ("MAJOR", "MINOR", "PATCH")
    )
    settings = {
        "aar": str(aar),
        "versionName": env("AIRCAST_VERSION_NAME") or version,
        "versionCode": env("AIRCAST_VERSION_CODE") or str(version_code(major, minor, patch)),
        "abi": pinned.abi,
        "javaHome": str(java),
    }
    (ANDROID_DIR / "qgc.properties").write_text(
        "".join(f"{key}={value}\n" for key, value in settings.items())
    )


def main() -> int:
    if sys.platform != "darwin":
        die("this generator resolves a macOS host Qt and a Homebrew JDK; use CI on other hosts")
    pinned = pins()
    sdk = sdk_root()
    ndk = ndk_root(sdk, pinned.ndk)
    java = java_home(pinned.java)
    requested = Path(env("QGC_CORE_BUILD_DIR") or "build-android")
    build = requested if requested.is_absolute() else ROOT / requested
    values = configured_cache(build, pinned, sdk, ndk, java)
    aar = build_aar(values, build, sdk, ndk, java)
    write_properties(values, pinned, aar, java)
    print(f"android/qgc.properties written for {aar} ({values['CMAKE_BUILD_TYPE']})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
