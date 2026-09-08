package one.aircast.mapspike

// sendToVehicle returns void, and nothing reachable through the bridge says the
// aircraft accepted the plan. `dirty` looked like that signal and is not:
// measured against a vehicle that never acknowledges, it went false anyway and
// the map said "Uploaded to vehicle". The bridge polls properties and has no
// signal for compliance, so an upload can be reported as asked and never as done.
enum class UploadOutcome { Sent, NotStarted }

fun uploadOutcome(invoked: Boolean): UploadOutcome =
    if (invoked) UploadOutcome.Sent else UploadOutcome.NotStarted

fun uploadMessage(outcome: UploadOutcome): String = when (outcome) {
    UploadOutcome.Sent -> "Upload sent to vehicle"
    UploadOutcome.NotStarted -> "Upload did not start"
}

// loadFromVehicle is void for the same reason and carries the same trap in the
// other direction: a download that never arrived leaves the plan alone, which
// is indistinguishable from one that returned exactly what was already there.
// Believing you hold the aircraft's mission when you hold your own is how an
// edit gets made against the wrong baseline.
fun downloadMessage(outcome: UploadOutcome): String = when (outcome) {
    UploadOutcome.Sent -> "Download requested from vehicle"
    UploadOutcome.NotStarted -> "Download did not start"
}
