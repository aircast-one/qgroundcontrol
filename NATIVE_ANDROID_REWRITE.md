
### The proximity warning, closed end to end, 2026-09-12

`8f9c0c9e7` verified on the handset inside the sensor's window:

    metric   3.2 m right
    feet     10.5 ft right

3.2 m is 10.498 ft, so the conversion and the restored tenth are both right. The
telemetry strip read `0.0 ft` and `1421.7 ft` in the same run, which is the
units fix confirmed live a second time.

That closes the defect recorded on 2026-09-11 as "the Fly view's obstacle
distance is metres-only because no core view serves it", and with it the last of
the unit class. The path it took is worth the summary: the head built the string,
so it could not follow the operator; the core took the measurement and the head
kept the filter; the filter turned out to hide a reading of zero, which is a
touching obstacle; and the first shared spelling was right for route distances
and too coarse for a proximity warning. Four rounds, three of them found by
looking at the handset rather than at the code.
