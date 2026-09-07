package one.aircast.mapspike

import kotlin.math.abs
import kotlin.math.cos
import kotlin.math.floor
import kotlin.math.log10
import kotlin.math.pow

private const val EQUATOR_METRES = 40_075_016.686
private const val TILE_PIXELS = 512.0

data class MapScale(val metres: Double, val pixels: Double, val label: String)

// MapLibre counts zoom against a 512 pixel tile, and a degree of longitude
// shrinks towards the poles, so the ground a pixel covers depends on both.
fun metresPerPixel(latitude: Double, zoom: Double): Double =
    EQUATOR_METRES * cos(Math.toRadians(latitude)) / (TILE_PIXELS * 2.0.pow(zoom))

// A bar reads as a measurement only if it ends on a number worth reading, so
// the width follows the distance rather than the other way round.
fun niceDistance(metres: Double): Double {
    if (metres <= 0 || metres.isNaN() || metres.isInfinite()) {
        return 0.0
    }
    val magnitude = 10.0.pow(floor(log10(metres)))
    return listOf(5.0, 2.0, 1.0)
        .map { it * magnitude }
        .firstOrNull { it <= metres }
        ?: magnitude
}

private fun scaleLabel(metres: Double): String =
    if (metres >= 1000) {
        val km = metres / 1000
        if (abs(km - km.toInt()) < 1e-9) "${km.toInt()} km" else "%.1f km".format(km)
    } else {
        "${metres.toInt()} m"
    }

fun mapScale(latitude: Double, zoom: Double, maxPixels: Double): MapScale? {
    val perPixel = metresPerPixel(latitude, zoom)
    if (perPixel <= 0 || perPixel.isNaN() || perPixel.isInfinite() || maxPixels <= 0) {
        return null
    }
    val metres = niceDistance(perPixel * maxPixels)
    if (metres <= 0) {
        return null
    }
    return MapScale(metres, metres / perPixel, scaleLabel(metres))
}
