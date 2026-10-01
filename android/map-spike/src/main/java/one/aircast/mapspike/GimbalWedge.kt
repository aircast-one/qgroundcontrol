package one.aircast.mapspike

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.LinearGradient
import android.graphics.Paint
import android.graphics.Path
import android.graphics.Shader
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

const val GIMBAL_AZIMUTH_VIEW = "view.gimbalAzimuth"
private const val GIMBAL_SOURCE = "aircast-gimbal-azimuth"
private const val GIMBAL_LAYER = "aircast-gimbal-azimuth-layer"
private const val GIMBAL_IMAGE = "aircast-gimbal-wedge"
private const val YAW_PROPERTY = "yaw"
private const val OPACITY_PROPERTY = "opacity"
private const val ACTIVE_OPACITY = 1.0f
private const val OTHER_OPACITY = 0.4f
private const val WEDGE_SIZE_PX = 48

data class GimbalAzimuth(val yaw: Double, val active: Boolean)

fun gimbalAzimuths(view: JSONObject?): List<GimbalAzimuth> {
    val listed = view?.optJSONArray("gimbals") ?: return emptyList()
    return (0 until listed.length()).mapNotNull { listed.optJSONObject(it) }.map { GimbalAzimuth(it.optDouble("yaw"), it.optBoolean("active")) }
}

object GimbalBridge {
    fun read(): List<GimbalAzimuth> = gimbalAzimuths(runCatching { JSONObject(QGCBridge.get(GIMBAL_AZIMUTH_VIEW)) }.getOrNull())
}

fun gimbalFeatures(latitude: Double, longitude: Double, gimbals: List<GimbalAzimuth>): FeatureCollection =
    FeatureCollection.fromFeatures(
        gimbals.filter { isPlottable(latitude, longitude) && !it.yaw.isNaN() }.map { gimbal ->
            Feature.fromGeometry(
                Point.fromLngLat(longitude, latitude),
                JsonObject().apply {
                    addProperty(YAW_PROPERTY, gimbal.yaw)
                    addProperty(OPACITY_PROPERTY, if (gimbal.active) ACTIVE_OPACITY else OTHER_OPACITY)
                },
            )
        },
    )

private fun wedge(): Bitmap {
    val size = WEDGE_SIZE_PX.toFloat()
    val bitmap = Bitmap.createBitmap(WEDGE_SIZE_PX, WEDGE_SIZE_PX, Bitmap.Config.ARGB_8888)
    val path = Path().apply {
        moveTo(size / 2f, size)
        lineTo(size * 0.0f, 0f)
        lineTo(size / 2f, size * 0.25f)
        lineTo(size, 0f)
        close()
    }
    val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        shader = LinearGradient(0f, 0f, 0f, size, android.graphics.Color.argb(0, 255, 255, 255), android.graphics.Color.argb(128, 255, 255, 255), Shader.TileMode.CLAMP)
    }
    Canvas(bitmap).drawPath(path, paint)
    return bitmap
}

fun installGimbalLayer(style: Style) {
    if (style.getSource(GIMBAL_SOURCE) != null) return
    style.addSource(GeoJsonSource(GIMBAL_SOURCE))
    style.addImage(GIMBAL_IMAGE, wedge())
    style.addLayer(
        SymbolLayer(GIMBAL_LAYER, GIMBAL_SOURCE).withProperties(
            PropertyFactory.iconImage(GIMBAL_IMAGE),
            PropertyFactory.iconAnchor(Property.ICON_ANCHOR_BOTTOM),
            PropertyFactory.iconRotate(Expression.get(YAW_PROPERTY)),
            PropertyFactory.iconRotationAlignment(Property.ICON_ROTATION_ALIGNMENT_MAP),
            PropertyFactory.iconOpacity(Expression.get(OPACITY_PROPERTY)),
            PropertyFactory.iconAllowOverlap(true),
            PropertyFactory.iconIgnorePlacement(true),
        ),
    )
}

fun renderGimbals(style: Style, latitude: Double, longitude: Double, gimbals: List<GimbalAzimuth>) {
    (style.getSource(GIMBAL_SOURCE) as? GeoJsonSource)?.setGeoJson(gimbalFeatures(latitude, longitude, gimbals))
}
