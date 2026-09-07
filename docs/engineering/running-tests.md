# Running the unit tests

```
python3 tools/run-tests.py                 # every suite
python3 tools/run-tests.py LinkStateTest   # one suite
python3 tools/run-tests.py --history       # which suites have been flaking
```

Exit code is `0` when nothing genuinely failed, `1` when something did, `2` when
the binary is stale.

## What it does that running the binary directly does not

**It refuses a stale binary.** The runner compares `build-test/Debug/…/AircastQGC`
against the newest file under `src/` and `test/` and stops if the binary is
older. This is not hypothetical: a suite once reported a test that had already
been deleted, because the run used a binary from before the change. Rebuild, or
pass `--allow-stale` if you know why you want it.

**It only prints what you need.** A full run emits several thousand `PASS` lines,
which is enough to truncate in most viewers. The runner prints one headline plus
the failures.

**It tells you whether a failure is real.** Every failing suite is re-run on its
own, three times by default (`--repeats`), and lands in one of three buckets:

| Bucket | Meaning | What to do |
|---|---|---|
| `REAL` | fails in the full run and in every isolated run | a genuine failure; fix it |
| `UNSTABLE` | fails some isolated runs | the test itself is flaky, independent of load |
| `LOAD FLAKE` | fails in the full run, passes every isolated run | starved of CPU by the rest of the suite |

That distinction matters here because several suites in this repo wait on
timeouts and fail when the machine is loaded. Reading a full-run failure as a
regression, or waving one away as "just a flake", have both wasted real time. A
`LOAD FLAKE` verdict is reported but does not fail the run; `REAL` does.

**It keeps history.** Each full run appends a line to
`build-test/test-history.jsonl`, so `--history` can tell you a suite has flaked
four of the last six runs rather than leaving you to remember. The file lives in
the build directory and is not committed.

## Known unstable suites

`VehicleLinkManagerTest` and `LinkStateTest` both wait on connection timeouts.
`VehicleLinkManagerTest::_highLatencyLinkTest` has been measured failing roughly
one isolated run in three, so it is `UNSTABLE`, not merely load-sensitive — a
full-run failure there is not automatically noise.

## Testing the runner

```
python3 tools/test_run_tests.py
```

Covers output parsing, the three-way classification and the history arithmetic
with fixtures, so the thing that decides whether a failure gets believed is
itself checked.
