from __future__ import annotations

import os
import re
import subprocess
import sys
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from pathlib import Path

CACHE_LINE = re.compile(r"^([A-Za-z0-9_]+):[A-Z]+=(.*)$")


def env(name: str) -> str | None:
    value = os.environ.get(name)
    return value if value and value.strip() else None


def die(*lines: str) -> None:
    print(*lines, sep="\n", file=sys.stderr)
    raise SystemExit(1)


def run(
    command: list[str],
    cwd: Path,
    failure: str,
    environment: dict[str, str] | None = None,
) -> None:
    if subprocess.run(command, cwd=cwd, env=environment).returncode != 0:
        die(failure)


def parse_cache(cache: Path) -> dict[str, str]:
    entries = (CACHE_LINE.match(line) for line in cache.read_text().splitlines())
    return {match.group(1): match.group(2) for match in entries if match}


def missing_keys(values: dict[str, str], required: tuple[str, ...]) -> list[str]:
    return [key for key in required if not values.get(key)]
