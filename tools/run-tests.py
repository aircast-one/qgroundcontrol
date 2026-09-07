#!/usr/bin/env python3
import argparse
import json
import re
import subprocess
import sys
import time
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
TEST_BUILD = REPO / "build-test"
TEST_APP = TEST_BUILD / "Debug/AircastQGC.app/Contents/MacOS/AircastQGC"
HISTORY = REPO / "build-test/test-history.jsonl"

RESULT_RE = re.compile(r"^(PASS|FAIL!|SKIP|QFATAL|XFAIL)\s*:\s*(\w+)::(\w+)\(\)")
TOTALS_RE = re.compile(r"^Totals:\s*(\d+) passed, (\d+) failed, (\d+) skipped, (\d+) blacklisted, (\d+)ms")


def newest_source_mtime():
    roots = (REPO / "src", REPO / "test")
    files = (p for root in roots for p in root.rglob("*")
             if p.suffix in {".cc", ".h", ".qml", ".txt"} and p.is_file())
    return max((p.stat().st_mtime for p in files), default=0.0)


def port_contention():
    held = subprocess.run(["lsof", "-nP", "-iUDP:14550"], capture_output=True, text=True)
    holders = [line.split()[0] for line in held.stdout.splitlines()[1:] if line.split()]
    if not holders:
        return None
    return (f"UDP 14550 is held by {', '.join(sorted(set(holders)))} - the link suites cannot "
            f"bind and will report false failures. Stop SITL (docker stop aircast-sitl) "
            f"and any mav_bridge first.")


def staleness():
    if not TEST_APP.exists():
        return f"{TEST_APP} does not exist - build it first"
    newest = newest_source_mtime()
    if TEST_APP.stat().st_mtime < newest:
        age = newest - TEST_APP.stat().st_mtime
        return f"binary is {age / 60:.1f} min older than the newest source - rebuild before trusting this"
    return None


def run_suite(name):
    args = [str(TEST_APP), "--allow-multiple", f"--unittest:{name}" if name else "--unittest"]
    started = time.monotonic()
    proc = subprocess.run(args, capture_output=True, text=True, timeout=3600)
    return proc.stdout + proc.stderr, time.monotonic() - started


def parse(output):
    results = []
    durations = {}
    suite_order = []
    current = None
    for raw in output.splitlines():
        line = raw.strip()
        outcome = RESULT_RE.match(line)
        if outcome:
            status, suite, case = outcome.groups()
            results.append((status, suite, case))
            current = suite
            if suite not in suite_order:
                suite_order.append(suite)
            continue
        totals = TOTALS_RE.match(line)
        if totals and current:
            durations[current] = int(totals.group(5))
            current = None
    failures = [(suite, case) for status, suite, case in results if status in {"FAIL!", "QFATAL"}]
    counts = Counter(status for status, _, _ in results)
    return {
        "suites": suite_order,
        "durations": durations,
        "failures": sorted(set(failures)),
        "passed": counts["PASS"],
        "failed": len(set(failures)),
        "skipped": counts["SKIP"],
    }


def classify(failures, repeats, verbose):
    suites = sorted({suite for suite, _ in failures})
    verdicts = {}
    for suite in suites:
        if verbose:
            print(f"  re-running {suite} alone x{repeats}...", file=sys.stderr)
        runs = [parse(run_suite(suite)[0]) for _ in range(repeats)]
        failed_runs = [run for run in runs if run["failures"]]
        cases = sorted({case for run in failed_runs for _, case in run["failures"]})
        verdicts[suite] = {
            "verdict": ("FLAKE" if not failed_runs
                        else "REAL" if len(failed_runs) == repeats
                        else "UNSTABLE"),
            "alone_failed": len(failed_runs),
            "alone_runs": repeats,
            "alone_failures": cases,
        }
    return verdicts


def load_history():
    if not HISTORY.exists():
        return []
    return [json.loads(line) for line in HISTORY.read_text().splitlines() if line.strip()]


def flake_rate(history, suite):
    seen = [run for run in history if suite in run.get("suites", [])]
    flaked = [run for run in seen if suite in run.get("flaky", [])]
    return len(flaked), len(seen)


def record(summary, verdicts):
    HISTORY.parent.mkdir(parents=True, exist_ok=True)
    entry = {
        "at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "suites": summary["suites"],
        "passed": summary["passed"],
        "failed": summary["failed"],
        "skipped": summary["skipped"],
        "flaky": [s for s, v in verdicts.items() if v["verdict"] == "FLAKE"],
        "unstable": [s for s, v in verdicts.items() if v["verdict"] == "UNSTABLE"],
        "real": [s for s, v in verdicts.items() if v["verdict"] == "REAL"],
        "durations": summary["durations"],
    }
    with HISTORY.open("a") as handle:
        handle.write(json.dumps(entry) + "\n")
    return entry


def report(summary, verdicts, history, stale, contention=None):
    lines = []
    if contention:
        lines.append(f"WARNING: {contention}")
        lines.append("")
    if stale:
        lines.append(f"STALE BINARY: {stale}")
        lines.append("")

    real = {s: v for s, v in verdicts.items() if v["verdict"] == "REAL"}
    unstable = {s: v for s, v in verdicts.items() if v["verdict"] == "UNSTABLE"}
    flaky = {s: v for s, v in verdicts.items() if v["verdict"] == "FLAKE"}

    headline = "PASS" if not real else f"FAIL - {len(real)} suite(s) genuinely failing"
    lines.append(f"{headline}   {summary['passed']} passed, {summary['failed']} failed, "
                 f"{summary['skipped']} skipped, across {len(summary['suites'])} suites")

    if real:
        lines.append("")
        lines.append("REAL FAILURES (fail in the suite AND every isolated run):")
        for suite, verdict in sorted(real.items()):
            cases = ", ".join(verdict["alone_failures"]) or "see output"
            lines.append(f"  {suite}: {cases}")

    if unstable:
        lines.append("")
        lines.append("UNSTABLE (fails some isolated runs - flaky in itself, not just under load):")
        for suite, verdict in sorted(unstable.items()):
            cases = ", ".join(verdict["alone_failures"]) or "see output"
            lines.append(f"  {suite}: failed {verdict['alone_failed']}/{verdict['alone_runs']} "
                         f"isolated runs - {cases}")

    if flaky:
        lines.append("")
        lines.append("LOAD FLAKES (fail in the full run, pass alone):")
        for suite, verdict in sorted(flaky.items()):
            hit, seen = flake_rate(history, suite)
            lines.append(f"  {suite}: passed {verdict['alone_runs']}/{verdict['alone_runs']} "
                         f"isolated runs; flaked {hit + 1} of last {seen + 1} full runs")

    if not real and not flaky and not unstable:
        lines.append("No failures.")
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description="Run QGC tests and classify failures")
    parser.add_argument("suite", nargs="?", default="", help="single suite, or all when omitted")
    parser.add_argument("--no-retry", action="store_true", help="skip isolation re-runs")
    parser.add_argument("--repeats", type=int, default=3, help="isolation re-runs per failing suite")
    parser.add_argument("--allow-stale", action="store_true", help="run even if the binary predates sources")
    parser.add_argument("--history", action="store_true", help="print flake history and exit")
    args = parser.parse_args()

    history = load_history()

    if args.history:
        suites = sorted({s for run in history for s in run.get("suites", [])})
        rows = [(s, *flake_rate(history, s)) for s in suites]
        flaky_rows = sorted([r for r in rows if r[1]], key=lambda r: -r[1] / max(r[2], 1))
        print(f"{len(history)} recorded runs")
        print("\n".join(f"  {s}: flaked {hit} of {seen}" for s, hit, seen in flaky_rows)
              or "  no flakes recorded")
        return 0

    contention = port_contention()
    stale = staleness()
    if stale and not args.allow_stale:
        print(f"STALE BINARY: {stale}", file=sys.stderr)
        print("rebuild, or pass --allow-stale to run anyway", file=sys.stderr)
        return 2

    output, _ = run_suite(args.suite)
    summary = parse(output)

    verdicts = ({} if args.no_retry or not summary["failures"]
                else classify(summary["failures"], args.repeats, verbose=True))

    entry = record(summary, verdicts) if not args.suite else None
    print(report(summary, verdicts, history, stale, contention))
    if entry:
        print(f"\nrecorded to {HISTORY.relative_to(REPO)}")

    return 1 if any(v["verdict"] == "REAL" for v in verdicts.values()) else 0


if __name__ == "__main__":
    sys.exit(main())
