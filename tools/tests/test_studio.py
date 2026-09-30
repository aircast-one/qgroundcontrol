#!/usr/bin/env python3
"""Tests for tools/android/studio.py."""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest
from android.studio import (
    Pins,
    declared_sdk_dir,
    java_version_at,
    validate_cache,
    version_code,
)
from common.generator import env, parse_cache

if TYPE_CHECKING:
    from pathlib import Path

PINS = Pins(
    qt="6.11.1",
    ndk="27.2.12479018",
    platform="36",
    min_sdk="28",
    java="21",
    abi="arm64-v8a",
)
ANDROID_CACHE = {
    "ANDROID_ABI": "arm64-v8a",
    "QT_HOST_PATH": "/Users/x/Qt/6.11.1/macos",
    "ANDROID_NDK_ROOT": "/Users/x/Library/Android/sdk/ndk/27.2.12479018",
    "CMAKE_BUILD_TYPE": "Release",
    "CMAKE_COMMAND": "/opt/homebrew/bin/cmake",
    "CMAKE_PROJECT_NAME": "AircastQGC",
    "CMAKE_PROJECT_VERSION": "5.4.16",
    "CMAKE_PROJECT_VERSION_MAJOR": "5",
    "CMAKE_PROJECT_VERSION_MINOR": "4",
    "CMAKE_PROJECT_VERSION_PATCH": "16",
}


def test_env_treats_blank_as_absent(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("QGC_TEST_VALUE", "")
    assert env("QGC_TEST_VALUE") is None
    monkeypatch.setenv("QGC_TEST_VALUE", "   ")
    assert env("QGC_TEST_VALUE") is None
    monkeypatch.setenv("QGC_TEST_VALUE", "5.4.16")
    assert env("QGC_TEST_VALUE") == "5.4.16"


def test_parse_cache_reads_typed_entries(tmp_path: Path) -> None:
    cache = tmp_path / "CMakeCache.txt"
    cache.write_text(
        "//a comment\n"
        "CMAKE_PROJECT_VERSION:STATIC=5.4.16\n"
        "CMAKE_PREFIX_PATH:UNINITIALIZED=\n"
        "not a cache line\n"
    )
    values = parse_cache(cache)
    assert values["CMAKE_PROJECT_VERSION"] == "5.4.16"
    assert values["CMAKE_PREFIX_PATH"] == ""
    assert "//a comment" not in values


def test_validate_cache_accepts_a_pinned_android_build(tmp_path: Path) -> None:
    validate_cache(dict(ANDROID_CACHE), tmp_path, tmp_path / "CMakeCache.txt", PINS)


def test_validate_cache_rejects_a_host_build(tmp_path: Path) -> None:
    values = {key: value for key, value in ANDROID_CACHE.items() if key != "ANDROID_ABI"}
    with pytest.raises(SystemExit):
        validate_cache(values, tmp_path, tmp_path / "CMakeCache.txt", PINS)


def test_validate_cache_rejects_an_unpinned_qt(tmp_path: Path) -> None:
    values = {**ANDROID_CACHE, "QT_HOST_PATH": "/Users/x/Qt/6.10.3/macos"}
    with pytest.raises(SystemExit):
        validate_cache(values, tmp_path, tmp_path / "CMakeCache.txt", PINS)


def test_validate_cache_rejects_an_unpinned_ndk(tmp_path: Path) -> None:
    values = {**ANDROID_CACHE, "ANDROID_NDK_ROOT": "/Users/x/sdk/ndk/26.1.10909125"}
    with pytest.raises(SystemExit):
        validate_cache(values, tmp_path, tmp_path / "CMakeCache.txt", PINS)


def test_validate_cache_names_every_missing_key(tmp_path: Path) -> None:
    values = {key: value for key, value in ANDROID_CACHE.items() if key != "CMAKE_PROJECT_NAME"}
    with pytest.raises(SystemExit):
        validate_cache(values, tmp_path, tmp_path / "CMakeCache.txt", PINS)


@pytest.mark.parametrize(
    ("line", "expected"),
    [
        ("sdk.dir=/opt/sdk", "/opt/sdk"),
        ("  sdk.dir = /opt/sdk  ", "/opt/sdk"),
        ("sdk.dir:/opt/sdk", "/opt/sdk"),
        ("sdk.dir=C\\:\\\\Android\\\\sdk", "C:\\Android\\sdk"),
    ],
)
def test_declared_sdk_dir_accepts_properties_spellings(
    tmp_path: Path, line: str, expected: str
) -> None:
    local = tmp_path / "local.properties"
    local.write_text(f"# a comment\n{line}\n")
    assert declared_sdk_dir(local) == expected


def test_declared_sdk_dir_refuses_a_file_without_the_key(tmp_path: Path) -> None:
    local = tmp_path / "local.properties"
    local.write_text("ndk.dir=/opt/ndk\n")
    with pytest.raises(SystemExit):
        declared_sdk_dir(local)


def test_java_version_at_reads_the_release_file(tmp_path: Path) -> None:
    (tmp_path / "release").write_text('JAVA_VERSION="21.0.5"\nOS_ARCH="aarch64"\n')
    assert java_version_at(tmp_path) == "21"


def test_version_code_is_monotonic_across_releases() -> None:
    codes = [version_code(*parts) for parts in ((5, 4, 16), (5, 4, 17), (5, 5, 0), (6, 0, 0))]
    assert codes == sorted(codes)
    assert len(set(codes)) == len(codes)


def test_version_code_matches_the_documented_layout() -> None:
    assert version_code(5, 4, 16) == 5040016


@pytest.mark.parametrize("parts", [(5, 100, 0), (5, 4, 10000)])
def test_version_code_refuses_a_version_that_would_collide(parts: tuple[int, int, int]) -> None:
    with pytest.raises(SystemExit):
        version_code(*parts)
