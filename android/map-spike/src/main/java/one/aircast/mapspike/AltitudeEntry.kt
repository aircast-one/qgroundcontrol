package one.aircast.mapspike

fun parsedAltitude(text: String): Double? =
    text.trim().toDoubleOrNull()?.takeIf { it >= 0.0 && it.isFinite() }

fun altitudeFieldText(altitude: Double): String =
    if (altitude.isNaN()) "" else altitude.toInt().toString()

const val LATITUDE_LIMIT = 90.0
const val LONGITUDE_LIMIT = 180.0

fun parsedCoordinate(text: String, limit: Double): Double? =
    text.trim().toDoubleOrNull()?.takeIf { it.isFinite() && it in -limit..limit }
