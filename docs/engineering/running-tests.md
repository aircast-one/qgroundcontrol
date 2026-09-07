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

**It knows what a complete run looks like.** The suite was for a long time
observed dying partway through, and a truncated run still ends in a
plausible-looking "222 passed, 0 failed" — reported as green more than once. The
runner reads the active `UT_REGISTER_TEST` entries out of `test/UnitTestList.cc`,
compares them against the suites that actually reported, and leads with

    INCOMPLETE RUN - 62 of 80 suites never ran.
      why: killed with SIGKILL from outside the process...
      stopped after: ParameterManagerTest
      did not run: ...
      do not read the totals below as a pass.

Exit code is `3` for an incomplete run, distinct from `1` for a real failure.

**It runs from a clone that other sessions cannot kill.** That truncation had one
cause, and it was never the tests. Parallel sessions on this machine run
`tools/macos/build-run.sh`, whose first act is
`pkill -9 -f 'build-test/Debug/AircastQGC.app'` — which matches the command line
of a unit-test run out of that same tree. Any parallel build killed the suite
mid-run. It looked like a flaky, load-dependent, SITL-dependent crash, and was
blamed on all three; none of them were it. The tell was the exit code: SIGKILL is
never raised by a crash, only by another process.

So the runner copies the app bundle to `$TMPDIR/qgc-testrun-<pid>/QGCSuite<pid>.app`
and runs that. `QGCSuite` alone is not enough — `tools/macos/run-tests.sh` opens
with `pkill -9 -x QGCSuite`, so the name carries the PID. Clones older than an
hour are swept on the next run, because `codesign --force` breaks APFS reflink
dedup and each one is a full ~100 MB copy.

**It keeps history.** Each full run appends a line to
`build-test/test-history.jsonl`, so `--history` can tell you a suite has flaked
four of the last six runs rather than leaving you to remember. The file lives in
the build directory and is not committed.

## Stop SITL before a full run

`LinkStateTest` and `VehicleLinkManagerTest` bind UDP 14550. So does the SITL
container (`-p 14550-14555:14550/udp`). With SITL up, those suites report
failures that have nothing to do with your change, and a long full run can die
partway through.

    docker stop aircast-sitl        # and kill any mav_bridge
    python3 tools/run-tests.py
    docker run -d --rm --platform linux/amd64 --name aircast-sitl \
      -p 5760-5765:5760-5765 -p 14550-14555:14550-14555/udp \
      ghcr.io/pavliha/aircast-sitl:latest

The runner checks UDP 14550 before starting and prints a warning naming the
holder, but it does not refuse to run — sometimes you only want a suite that
does not touch links.

With the port free the whole suite passes: 576 tests, 80 suites, no failures.
The link-suite failures previously blamed on machine load were this. The
*truncation* blamed on it was not — see the clone above.

## Testing the runner

```
python3 tools/test_run_tests.py
```

Covers output parsing, the three-way classification and the history arithmetic
with fixtures, so the thing that decides whether a failure gets believed is
itself checked.
