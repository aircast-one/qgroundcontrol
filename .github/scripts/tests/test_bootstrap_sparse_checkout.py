#!/usr/bin/env python3

from __future__ import annotations

import ast
from typing import TYPE_CHECKING

import pytest
import yaml
from _helpers import REPO_ROOT

if TYPE_CHECKING:
    from collections.abc import Iterator
    from pathlib import Path

WORKFLOWS_DIR = REPO_ROOT / ".github" / "workflows"
ACTIONS_DIR = REPO_ROOT / ".github" / "actions"
TOOLS_DIR = REPO_ROOT / "tools"
SCRIPTS_DIR = REPO_ROOT / ".github" / "scripts"

CI_BOOTSTRAP = ".github/scripts/ci_bootstrap.py"
TOOLS_BOOTSTRAP = "tools/_bootstrap.py"
BOOTSTRAP_TRIPWIRES: frozenset[str] = frozenset({CI_BOOTSTRAP, TOOLS_BOOTSTRAP})

BASE_BOOTSTRAP_PATHS: frozenset[str] = frozenset(
    {
        "tools/_bootstrap.py",
        "tools/common/__init__.py",
    }
)


def _common_module_to_path(module: str) -> str | None:
    rel = "tools/" + module.replace(".", "/") + ".py"
    return rel if (REPO_ROOT / rel).is_file() else None


def _type_checking_body_nodes(tree: ast.AST) -> set[ast.AST]:
    skip: set[ast.AST] = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.If):
            test = node.test
            is_tc = (isinstance(test, ast.Name) and test.id == "TYPE_CHECKING") or (
                isinstance(test, ast.Attribute) and test.attr == "TYPE_CHECKING"
            )
            if is_tc:
                for stmt in node.body:
                    skip.update(ast.walk(stmt))
    return skip


def _iter_common_imports(source: Path) -> Iterator[str]:
    try:
        tree = ast.parse(source.read_text(encoding="utf-8"))
    except (OSError, SyntaxError):
        return
    inside_common = TOOLS_DIR / "common" in source.parents
    skip = _type_checking_body_nodes(tree)
    for node in ast.walk(tree):
        if node in skip:
            continue
        if isinstance(node, ast.ImportFrom):
            if node.module is None:
                continue
            if node.level == 0 and node.module.split(".")[0] == "common":
                yield node.module
            elif node.level >= 1 and inside_common:
                yield f"common.{node.module}"
        elif isinstance(node, ast.Import):
            for alias in node.names:
                if alias.name.split(".")[0] == "common":
                    yield alias.name


def _required_common_paths(script: Path) -> frozenset[str]:
    required: set[str] = set()
    queue: list[Path] = [script]
    while queue:
        cur = queue.pop()
        for module in _iter_common_imports(cur):
            rel = _common_module_to_path(module)
            if rel and rel not in required:
                required.add(rel)
                queue.append(REPO_ROOT / rel)
    return frozenset(required)


def _scripts_in_block(entries: frozenset[str]) -> list[Path]:
    return [
        REPO_ROOT / e
        for e in entries
        if e.endswith(".py") and e not in BOOTSTRAP_TRIPWIRES and (REPO_ROOT / e).is_file()
    ]


def _iter_checkout_steps(doc: dict, source: str) -> Iterator[tuple[str, frozenset[str]]]:
    if "jobs" in doc:
        groups = [
            (name, job.get("steps"))
            for name, job in (doc.get("jobs") or {}).items()
            if isinstance(job, dict)
        ]
    else:
        groups = [("runs", (doc.get("runs") or {}).get("steps"))]
    for group_name, steps in groups:
        for index, step in enumerate(steps or []):
            if not isinstance(step, dict):
                continue
            if not str(step.get("uses", "")).startswith("actions/checkout"):
                continue
            paths = (step.get("with") or {}).get("sparse-checkout")
            if not isinstance(paths, str):
                continue
            entries = frozenset(p.strip() for p in paths.splitlines() if p.strip())
            yield f"{source}::{group_name}[step {index}]", entries


def _iter_sparse_checkout_blocks() -> Iterator[tuple[str, frozenset[str]]]:
    sources = sorted(WORKFLOWS_DIR.glob("*.yml")) + sorted(ACTIONS_DIR.glob("*/action.yml"))
    for path in sources:
        doc = yaml.safe_load(path.read_text(encoding="utf-8"))
        if not isinstance(doc, dict):
            continue
        yield from _iter_checkout_steps(doc, str(path.relative_to(REPO_ROOT)))


def test_bootstrap_sparse_checkout_matches_canonical() -> None:
    if not WORKFLOWS_DIR.is_dir():
        pytest.skip("workflows dir not in checkout")

    drift: list[str] = []
    for context, entries in _iter_sparse_checkout_blocks():
        stale = sorted(
            e
            for e in entries
            if e.endswith(".py")
            and not any(ch in e for ch in "*?[")
            and not (REPO_ROOT / e).is_file()
        )
        if stale:
            drift.append(f"{context} lists nonexistent files: {stale}")
        if not entries & BOOTSTRAP_TRIPWIRES:
            continue
        required: set[str] = set(BASE_BOOTSTRAP_PATHS)
        required |= _required_common_paths(TOOLS_DIR / "common" / "__init__.py")
        scripts = _scripts_in_block(entries)
        if any(str(s.relative_to(REPO_ROOT)).startswith(".github/scripts/") for s in scripts):
            required.add(CI_BOOTSTRAP)
        for script in scripts:
            required |= _required_common_paths(script)
        missing = sorted(required - entries)
        if missing:
            drift.append(f"{context} missing: {missing}")

    if drift:
        pytest.fail(
            "Workflows/actions referencing the bootstrap shim are missing files required by their scripts:\n  "
            + "\n  ".join(drift)
        )
