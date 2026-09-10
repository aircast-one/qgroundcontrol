package one.aircast.mapspike

import kotlin.math.cos
import kotlin.math.pow
import org.json.JSONObject

private const val EQUATOR_METRES = 40_075_016.686
private const val TILE_PIXELS = 512.0

data class MapScale(val text: String, val pixels: Double)

fun metresPerPixel(latitude: Double, zoom: Double): Double =
    EQUATOR_METRES * cos(Math.toRadians(latitude)) / (TILE_PIXELS * 2.0.pow(zoom))

fun metresAcross(latitude: Double, zoom: Double, maxPixels: Double): Double? {
    val perPixel = metresPerPixel(latitude, zoom)
    if (!perPixel.isFinite() || perPixel <= 0.0 || maxPixels <= 0.0) {
        return null
    }
    return perPixel * maxPixels
}

internal fun mapScaleBar(view: JSONObject?, maxPixels: Double): MapScale? {
    if (view?.optBoolean("available") != true) {
        return null
    }
    val fraction = view.optDouble("fraction", Double.NaN)
    if (!fraction.isFinite() || fraction <= 0.0) {
        return null
    }
    val text = view.optString("text")
    if (text.isBlank()) {
        return null
    }
    return MapScale(text, fraction * maxPixels)
}
