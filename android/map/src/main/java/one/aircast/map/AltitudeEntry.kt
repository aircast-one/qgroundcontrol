package one.aircast.map

const val RALLY_ALTITUDE_DECIMALS = 2
const val SURFACE_DISTANCE_DECIMALS = 2
const val SURFACE_DISTANCE_MINIMUM = 0.1

fun typedNumber(text: String): Double? =
    text.trim().takeIf { typed -> typed.count { it == ',' } <= 1 && !(',' in typed && '.' in typed) }
        ?.replace(',', '.')?.toDoubleOrNull()?.takeIf { it.isFinite() }

fun parsedAltitude(text: String): Double? = typedNumber(text)

fun parsedSurfaceDistance(text: String, metresPerShown: Double): Double? =
    typedNumber(text)?.takeIf { it * metresPerShown >= SURFACE_DISTANCE_MINIMUM - 1e-9 }

fun metresPerUnit(units: String): Double = if (units.trim() == "ft") 0.3048 else 1.0

fun altitudeFieldText(altitude: Double, decimals: Int): String =
    if (altitude.isNaN()) "" else String.format(java.util.Locale.US, "%.${decimals}f", altitude).let { shown -> if ('.' in shown) shown.trimEnd('0').trimEnd('.') else shown }.let { if (it == "-0") "0" else it }

const val LATITUDE_LIMIT = 90.0
const val LONGITUDE_LIMIT = 180.0

fun parsedCoordinate(text: String, limit: Double): Double? =
    typedNumber(text)?.takeIf { it in -limit..limit }
