package one.aircast.mapspike

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Path
import com.google.gson.JsonObject
import org.json.JSONObject
import org.maplibre.android.maps.Style
import org.maplibre.android.style.expressions.Expression
import org.maplibre.android.style.layers.Property
import org.maplibre.android.style.layers.PropertyFactory
import org.maplibre.android.style.layers.SymbolLayer
import org.maplibre.android.style.sources.GeoJsonSource
import org.maplibre.geojson.Feature
import org.maplibre.geojson.FeatureCollection
import org.maplibre.geojson.Point
import org.mavlink.qgroundcontrol.QGCBridge
import java.util.Locale

const val TRAFFIC_SOURCE = "traffic"
internal const val TRAFFIC_LAYER = "traffic-aircraft"
private const val TRAFFIC_VIEW = "view.adsbTraffic"
private const val ALERT_IMAGE = "traffic-alert"
private const val AWARENESS_IMAGE = "traffic-awareness"
private const val HEADING = "heading"
private const val ALERT = "alert"
private const val LABEL = "label"
private const val AIRCRAFT_PX = 44

data class TrafficMark(val latitude: Double, val longitude: Double, val heading: Double?, val alert: Boolean, val label: String)

private fun JSONObject.number(key: String): Double? = if (isNull(key)) null else optDouble(key).takeIf { !it.isNaN() }

fun trafficLabel(altitude: Double?, unit: String, callsign: String): String =
    altitude?.let { String.format(Locale.US, "%.0f %s\n%s", it, unit, callsign).trimEnd() } ?: ""

fun trafficMarks(view: JSONObject?): List<TrafficMark> {
    val contacts = view?.optJSONArray("contacts") ?: return emptyList()
    val unit = view.optJSONObject("units")?.optText("altitude").orEmpty()
    return (0 until contacts.length()).mapNotNull { index ->
        contacts.optJSONObject(index)?.let { contact ->
            val latitude = contact.number("latitude")
            val longitude = contact.number("longitude")
            if (latitude == null || longitude == null || !isPlottable(latitude, longitude)) {
                null
            } else {
                TrafficMark(
                    latitude = latitude,
                    longitude = longitude,
                    heading = contact.number("headingDegrees"),
                    alert = contact.optBoolean("alert"),
                    label = trafficLabel(contact.number("altitude"), unit, contact.optText("callsign")),
                )
            }
        }
    }
}

object TrafficBridge {
    fun read(): List<TrafficMark> = trafficMarks(runCatching { JSONObject(QGCBridge.get(TRAFFIC_VIEW)) }.getOrNull())
}

fun trafficFeatures(marks: List<TrafficMark>): FeatureCollection =
    FeatureCollection.fromFeatures(
        marks.map { mark ->
            Feature.fromGeometry(
                Point.fromLngLat(mark.longitude, mark.latitude),
                JsonObject().apply {
                    addProperty(HEADING, mark.heading ?: 0.0)
                    addProperty(ALERT, mark.alert)
                    addProperty(LABEL, mark.label)
                },
            )
        },
    )

private fun aircraft(fill: Int): Bitmap {
    val bitmap = Bitmap.createBitmap(AIRCRAFT_PX, AIRCRAFT_PX, Bitmap.Config.ARGB_8888)
    val s = AIRCRAFT_PX.toFloat()
    val path = Path().apply {
        moveTo(s * 0.5f, s * 0.06f)
        lineTo(s * 0.58f, s * 0.42f)
        lineTo(s * 0.94f, s * 0.58f)
        lineTo(s * 0.58f, s * 0.6f)
        lineTo(s * 0.56f, s * 0.82f)
        lineTo(s * 0.7f, s * 0.92f)
        lineTo(s * 0.3f, s * 0.92f)
        lineTo(s * 0.44f, s * 0.82f)
        lineTo(s * 0.42f, s * 0.6f)
        lineTo(s * 0.06f, s * 0.58f)
        lineTo(s * 0.42f, s * 0.42f)
        close()
    }
    val canvas = Canvas(bitmap)
    canvas.drawPath(path, Paint(Paint.ANTI_ALIAS_FLAG).apply { color = android.graphics.Color.BLACK; style = Paint.Style.STROKE; strokeWidth = 4f; strokeJoin = Paint.Join.ROUND })
    canvas.drawPath(path, Paint(Paint.ANTI_ALIAS_FLAG).apply { color = fill })
    return bitmap
}

fun installTrafficLayer(style: Style) {
    if (style.getSource(TRAFFIC_SOURCE) != null) return
    style.addImage(ALERT_IMAGE, aircraft(android.graphics.Color.parseColor("#FF5252")))
    style.addImage(AWARENESS_IMAGE, aircraft(android.graphics.Color.WHITE))
    style.addSource(GeoJsonSource(TRAFFIC_SOURCE))
    style.addLayer(
        SymbolLayer(TRAFFIC_LAYER, TRAFFIC_SOURCE).withProperties(
            PropertyFactory.iconImage(Expression.switchCase(Expression.get(ALERT), Expression.literal(ALERT_IMAGE), Expression.literal(AWARENESS_IMAGE))),
            PropertyFactory.iconRotate(Expression.get(HEADING)),
            PropertyFactory.iconRotationAlignment(Property.ICON_ROTATION_ALIGNMENT_MAP),
            PropertyFactory.iconAllowOverlap(true),
            PropertyFactory.iconIgnorePlacement(true),
            PropertyFactory.textField(Expression.get(LABEL)),
            PropertyFactory.textSize(11f),
            PropertyFactory.textColor("#FFFFFF"),
            PropertyFactory.textHaloColor("#000000"),
            PropertyFactory.textHaloWidth(1.5f),
            PropertyFactory.textAnchor(Property.TEXT_ANCHOR_TOP),
            PropertyFactory.textOffset(arrayOf(0f, 1.6f)),
            PropertyFactory.textAllowOverlap(true),
        ),
    )
}
