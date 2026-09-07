#!/usr/bin/env python3
import importlib.util
import sys
from pathlib import Path

spec = importlib.util.spec_from_file_location("run_tests", Path(__file__).with_name("run-tests.py"))
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)

SUITE_OUTPUT = """
PASS   : ADSBTest::initTestCase()
PASS   : ADSBTest::_adsbVehicleTest()
Totals: 2 passed, 0 failed, 0 skipped, 0 blacklisted, 95ms
PASS   : LinkStateTest::initTestCase()
FAIL!  : LinkStateTest::_stallFires() Compared values are not the same
PASS   : LinkStateTest::cleanupTestCase()
Totals: 2 passed, 1 failed, 0 skipped, 0 blacklisted, 32488ms
SKIP   : QGCBridgeCoreTest::_resolvesListIndex() nothing to index
Totals: 0 passed, 0 failed, 1 skipped, 0 blacklisted, 4ms
"""


def test_parse_counts_and_failures():
    parsed = runner.parse(SUITE_OUTPUT)
    assert parsed["passed"] == 4, parsed
    assert parsed["failed"] == 1, parsed
    assert parsed["skipped"] == 1, parsed
    assert parsed["failures"] == [("LinkStateTest", "_stallFires")], parsed
    assert parsed["suites"] == ["ADSBTest", "LinkStateTest", "QGCBridgeCoreTest"], parsed


def test_parse_records_per_suite_duration():
    parsed = runner.parse(SUITE_OUTPUT)
    assert parsed["durations"]["LinkStateTest"] == 32488, parsed["durations"]


def test_classify_buckets_by_isolated_failure_rate(monkeypatch=None):
    outcomes = {
        "AlwaysFails": ["FAIL!  : AlwaysFails::_a() boom"] * 3,
        "SometimesFails": ["FAIL!  : SometimesFails::_a() boom",
                           "PASS   : SometimesFails::_a()",
                           "PASS   : SometimesFails::_a()"],
        "PassesAlone": ["PASS   : PassesAlone::_a()"] * 3,
    }
    calls = {name: 0 for name in outcomes}

    def fake_run(name):
        line = outcomes[name][calls[name]]
        calls[name] += 1
        return line, 0.1

    original = runner.run_suite
    runner.run_suite = fake_run
    try:
        verdicts = runner.classify(
            [(name, "_a") for name in outcomes], repeats=3, verbose=False)
    finally:
        runner.run_suite = original

    assert verdicts["AlwaysFails"]["verdict"] == "REAL", verdicts
    assert verdicts["SometimesFails"]["verdict"] == "UNSTABLE", verdicts
    assert verdicts["SometimesFails"]["alone_failed"] == 1, verdicts
    assert verdicts["PassesAlone"]["verdict"] == "FLAKE", verdicts


def test_flake_rate_counts_only_runs_that_included_the_suite():
    history = [
        {"suites": ["A", "B"], "flaky": ["A"]},
        {"suites": ["A", "B"], "flaky": []},
        {"suites": ["B"], "flaky": []},
    ]
    assert runner.flake_rate(history, "A") == (1, 2)
    assert runner.flake_rate(history, "B") == (0, 3)


def test_report_leads_with_real_failures():
    summary = {"passed": 1, "failed": 1, "skipped": 0, "suites": ["X"], "durations": {}}
    verdicts = {"X": {"verdict": "REAL", "alone_failed": 3, "alone_runs": 3,
                      "alone_failures": ["_boom"]}}
    text = runner.report(summary, verdicts, [], None)
    assert text.startswith("FAIL"), text
    assert "_boom" in text, text


def test_report_says_pass_when_only_load_flakes():
    summary = {"passed": 5, "failed": 1, "skipped": 0, "suites": ["X"], "durations": {"X": 12}}
    verdicts = {"X": {"verdict": "FLAKE", "alone_failed": 0, "alone_runs": 3,
                      "alone_failures": []}}
    text = runner.report(summary, verdicts, [], None)
    assert text.startswith("PASS"), text
    assert "LOAD FLAKES" in text, text


def test_missing_suites_are_named_from_the_registered_list():
    summary = {"suites": ["ADSBTest", "LinkStateTest"]}
    original = runner.expected_suites
    runner.expected_suites = lambda: ["ADSBTest", "LinkStateTest", "VideoManagerTest", "GeoTest"]
    try:
        assert runner.missing_suites(summary) == ["VideoManagerTest", "GeoTest"]
    finally:
        runner.expected_suites = original


def test_report_refuses_to_call_a_truncated_run_a_pass():
    summary = {"passed": 222, "failed": 0, "skipped": 0,
               "suites": ["ADSBTest", "ParameterManagerTest"], "durations": {}}
    text = runner.report(summary, {}, [], None, None,
                         missing=["VideoManagerTest", "GeoTest"], exit_code=1)
    assert text.startswith("INCOMPLETE RUN"), text
    assert "ParameterManagerTest" in text, text
    assert "do not read the totals below as a pass" in text, text


def test_a_sigkill_is_explained_as_an_outside_kill_not_a_failure():
    summary = {"passed": 409, "failed": 0, "skipped": 0, "suites": ["A", "PipViewTest"], "durations": {}}
    text = runner.report(summary, {}, [], None, None, missing=["VideoTileTest"], exit_code=-9)
    assert "SIGKILL" in text, text
    assert "Nothing failed" in text, text
    assert "build-run.sh" in text, text


def test_an_unknown_exit_code_still_reports_the_number():
    summary = {"passed": 1, "failed": 0, "skipped": 0, "suites": ["A"], "durations": {}}
    text = runner.report(summary, {}, [], None, None, missing=["B"], exit_code=1)
    assert "binary exited 1" in text, text


def test_expected_suites_reads_the_real_list():
    names = runner.expected_suites()
    assert len(names) > 50, len(names)
    assert "ADSBTest" in names, names[:5]


def main():
    tests = [value for name, value in sorted(globals().items())
             if name.startswith("test_") and callable(value)]
    failures = []
    for test in tests:
        try:
            test()
        except AssertionError as error:
            failures.append(f"{test.__name__}: {error}")
    print(f"{len(tests) - len(failures)}/{len(tests)} passed")
    print("\n".join(failures))
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
