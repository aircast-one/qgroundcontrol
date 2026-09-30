#!/usr/bin/env python3
"""Tests for tools/macos/xcode.py."""

from __future__ import annotations

import plistlib
from typing import TYPE_CHECKING

import pytest
from common.generator import env
from macos.xcode import read_cache, write_info_plist

if TYPE_CHECKING:
    from pathlib import Path

HEADLESS_CACHE = """//a comment line
CMAKE_BUILD_TYPE:STRING=Release
CMAKE_COMMAND:INTERNAL=/opt/homebrew/bin/cmake
CMAKE_OSX_DEPLOYMENT_TARGET:STRING=13.0
CMAKE_PROJECT_DESCRIPTION:STATIC=Aircast ground station
CMAKE_PROJECT_NAME:STATIC=AircastQGC
CMAKE_PROJECT_VERSION:STATIC=5.4.16
CMAKE_PROJECT_VERSION_MAJOR:STATIC=5
CMAKE_PROJECT_VERSION_MINOR:STATIC=4
CMAKE_PROJECT_VERSION_PATCH:STATIC=16
QGC_APP_COPYRIGHT:STRING=Copyright (c) 2026 Aircast
QGC_APP_DISPLAY_NAME:STRING=Aircast QGC
QGC_HEADLESS_CORE:BOOL=ON
QGC_MACOS_BUNDLE_ID:STRING=org.aircast.qgc
QGC_MACOS_ICON_PATH:FILEPATH=/repo/deploy/macos/qgroundcontrol.icns
QGC_MACOS_PLIST_PATH:FILEPATH=/repo/deploy/macos/MacOSXBundleInfo.plist.in
"""


def cache_at(tmp_path: Path, text: str) -> Path:
    build = tmp_path / "build-hl"
    build.mkdir()
    (build / "CMakeCache.txt").write_text(text)
    return build


def test_env_treats_blank_as_absent(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("QGC_TEST_VALUE", "  ")
    assert env("QGC_TEST_VALUE") is None
    monkeypatch.setenv("QGC_TEST_VALUE", "build-hl")
    assert env("QGC_TEST_VALUE") == "build-hl"


def test_read_cache_returns_every_required_key(tmp_path: Path) -> None:
    values = read_cache(cache_at(tmp_path, HEADLESS_CACHE))
    assert values["CMAKE_PROJECT_VERSION"] == "5.4.16"
    assert values["QGC_MACOS_BUNDLE_ID"] == "org.aircast.qgc"


def test_read_cache_rejects_a_core_that_still_builds_qml(tmp_path: Path) -> None:
    build = cache_at(
        tmp_path, HEADLESS_CACHE.replace("QGC_HEADLESS_CORE:BOOL=ON", "QGC_HEADLESS_CORE:BOOL=OFF")
    )
    with pytest.raises(SystemExit):
        read_cache(build)


def test_read_cache_rejects_a_cache_missing_a_key(tmp_path: Path) -> None:
    build = cache_at(
        tmp_path, HEADLESS_CACHE.replace("QGC_APP_DISPLAY_NAME:STRING=Aircast QGC\n", "")
    )
    with pytest.raises(SystemExit):
        read_cache(build)


def test_write_info_plist_substitutes_from_the_cmake_template(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    template = tmp_path / "Info.plist.in"
    template.write_text(
        "<?xml version='1.0' encoding='UTF-8'?>\n"
        '<plist version="1.0"><dict>\n'
        "<key>CFBundleName</key><string>${MACOSX_BUNDLE_BUNDLE_NAME}</string>\n"
        "<key>CFBundleExecutable</key><string>${MACOSX_BUNDLE_EXECUTABLE_NAME}</string>\n"
        "<key>CFBundleIdentifier</key><string>${MACOSX_BUNDLE_GUI_IDENTIFIER}</string>\n"
        "<key>CFBundleShortVersionString</key><string>${MACOSX_BUNDLE_SHORT_VERSION_STRING}</string>\n"
        "<key>LSMinimumSystemVersion</key><string>${CMAKE_OSX_DEPLOYMENT_TARGET}</string>\n"
        "</dict></plist>\n"
    )
    monkeypatch.setattr("macos.xcode.PROJECT_DIR", tmp_path)
    values = read_cache(cache_at(tmp_path, HEADLESS_CACHE))
    values["QGC_MACOS_PLIST_PATH"] = str(template)

    write_info_plist(values)

    written = plistlib.loads((tmp_path / "Info.plist").read_bytes())
    assert written["CFBundleName"] == "Aircast QGC"
    assert written["CFBundleShortVersionString"] == "5.4"
    assert written["LSMinimumSystemVersion"] == "13.0"
    assert written["CFBundleExecutable"] == "$(EXECUTABLE_NAME)"
    assert written["CFBundleIdentifier"] == "$(PRODUCT_BUNDLE_IDENTIFIER)"
    assert written["QGCNativeUI"] is True


def test_write_info_plist_refuses_an_unknown_template_variable(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    template = tmp_path / "Info.plist.in"
    template.write_text("<plist><dict><key>X</key><string>${SOMETHING_NEW}</string></dict></plist>")
    monkeypatch.setattr("macos.xcode.PROJECT_DIR", tmp_path)
    values = read_cache(cache_at(tmp_path, HEADLESS_CACHE))
    values["QGC_MACOS_PLIST_PATH"] = str(template)

    with pytest.raises(SystemExit):
        write_info_plist(values)
