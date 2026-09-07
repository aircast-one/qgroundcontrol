package one.aircast.mapspike

// A typed altitude has to survive a half-finished number without throwing the
// edit away: "4" on the way to "47" is not a reason to reject a keystroke. Only
// a value that parses and is not below the ground is worth sending.
fun parsedAltitude(text: String): Double? =
    text.trim().toDoubleOrNull()?.takeIf { it >= 0.0 && it.isFinite() }

fun altitudeFieldText(altitude: Double): String =
    if (altitude.isNaN()) "" else altitude.toInt().toString()
