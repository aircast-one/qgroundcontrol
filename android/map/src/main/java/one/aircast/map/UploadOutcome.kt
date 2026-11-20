package one.aircast.map

enum class UploadOutcome { Sent, NotStarted }

fun uploadOutcome(invoked: Boolean): UploadOutcome =
    if (invoked) UploadOutcome.Sent else UploadOutcome.NotStarted

fun uploadMessage(outcome: UploadOutcome): String = when (outcome) {
    UploadOutcome.Sent -> "Upload sent to vehicle"
    UploadOutcome.NotStarted -> "Upload did not start"
}

fun pauseRefusedText(reason: String): String = "The aircraft did not pause, so the plan was not uploaded. $reason"

fun downloadMessage(outcome: UploadOutcome): String = when (outcome) {
    UploadOutcome.Sent -> "Download requested from vehicle"
    UploadOutcome.NotStarted -> "Download did not start"
}
