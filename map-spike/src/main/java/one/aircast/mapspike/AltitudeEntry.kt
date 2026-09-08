package one.aircast.mapspike

fun parsedAltitude(text: String): Double? =
    text.trim().toDoubleOrNull()?.takeIf { it >= 0.0 && it.isFinite() }

fun altitudeFieldText(altitude: Double): String =
    if (altitude.isNaN()) "" else altitude.toInt().toString()
