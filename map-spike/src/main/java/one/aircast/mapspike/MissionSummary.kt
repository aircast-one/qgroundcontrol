package one.aircast.mapspike

import kotlin.math.roundToInt

private fun distanceText(metres: Double): String =
    if (metres < 1000) "%.0f m".format(metres) else "%.2f km".format(metres / 1000)

private fun durationText(seconds: Double): String {
    val whole = seconds.roundToInt()
    return if (whole < 3600) {
        "%d:%02d".format(whole / 60, whole % 60)
    } else {
        "%d:%02d:%02d".format(whole / 3600, whole % 3600 / 60, whole % 60)
    }
}

fun missionSummary(distanceMetres: Double, seconds: Double): String {
    val distance = distanceMetres.takeIf { !it.isNaN() && it > 0.0 }
    val duration = seconds.takeIf { !it.isNaN() && it > 0.0 }

    return listOfNotNull(
        distance?.let { distanceText(it) },
        duration?.let { durationText(it) },
    ).joinToString(" · ")
}
