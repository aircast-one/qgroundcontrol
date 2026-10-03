package one.aircast.mapspike

const val WAYPOINT_ALTITUDE_DECIMALS = 1
const val RALLY_ALTITUDE_DECIMALS = 2
const val SURFACE_DISTANCE_DECIMALS = 2
const val SURFACE_DISTANCE_MINIMUM = 0.1

fun typedNumber(text: String): Double? =
    text.trim().replace(',', '.').toDoubleOrNull()?.takeIf { it.isFinite() }

fun parsedAltitude(text: String): Double? = typedNumber(text)

fun parsedSurfaceDistance(text: String): Double? = typedNumber(text)?.takeIf { it >= SURFACE_DISTANCE_MINIMUM }

fun altitudeFieldText(altitude: Double, decimals: Int): String =
    if (altitude.isNaN()) "" else String.format(java.util.Locale.US, "%.${decimals}f", altitude).let { shown -> if ('.' in shown) shown.trimEnd('0').trimEnd('.') else shown }

const val LATITUDE_LIMIT = 90.0
const val LONGITUDE_LIMIT = 180.0

fun parsedCoordinate(text: String, limit: Double): Double? =
    typedNumber(text)?.takeIf { it in -limit..limit }
