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
