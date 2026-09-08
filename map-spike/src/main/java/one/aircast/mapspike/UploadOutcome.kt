package one.aircast.mapspike

enum class UploadOutcome { Sent, NotStarted }

fun uploadOutcome(invoked: Boolean): UploadOutcome =
    if (invoked) UploadOutcome.Sent else UploadOutcome.NotStarted

fun uploadMessage(outcome: UploadOutcome): String = when (outcome) {
    UploadOutcome.Sent -> "Upload sent to vehicle"
    UploadOutcome.NotStarted -> "Upload did not start"
}

fun downloadMessage(outcome: UploadOutcome): String = when (outcome) {
    UploadOutcome.Sent -> "Download requested from vehicle"
    UploadOutcome.NotStarted -> "Download did not start"
}
