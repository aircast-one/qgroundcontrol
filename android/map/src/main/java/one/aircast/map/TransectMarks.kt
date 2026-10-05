package one.aircast.map

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Path
import org.maplibre.android.maps.Style
import org.maplibre.android.style.expressions.Expression
import org.maplibre.android.style.layers.LineLayer
import org.maplibre.android.style.layers.Property
import org.maplibre.android.style.layers.PropertyFactory
import org.maplibre.android.style.layers.SymbolLayer
import org.maplibre.android.style.sources.GeoJsonSource
import org.maplibre.geojson.Feature
import org.maplibre.geojson.FeatureCollection
import org.maplibre.geojson.LineString
import org.maplibre.geojson.Point

private const val TRANSECT_ARROW_SOURCE = "aircast-transect-arrows"
private const val TRANSECT_ARROW_LAYER = "aircast-transect-arrow-layer"
private const val TRANSECT_STUB_SOURCE = "aircast-transect-stubs"
private const val TRANSECT_STUB_LAYER = "aircast-transect-stub-layer"
private const val TRANSECT_ARROW_IMAGE = "aircast-transect-arrow"
private const val ARROW_BEARING = "bearing"
private const val ARROW_SIZE_PX = 30
private const val ENTRY_QUARTER = 1
private const val EXIT_QUARTER = 3

data class TransectArrow(val at: TrackPoint, val bearing: Double)

data class TransectMarks(val arrows: List<TransectArrow>, val stubs: List<List<TrackPoint>>)

internal fun arrowOn(from: TrackPoint, to: TrackPoint, quarter: Int): TransectArrow {
    val at = pointAt(from, metresBetween(from, to) / 4 * quarter, azimuthBetween(from, to))
    return TransectArrow(at, azimuthBetween(at, to))
}

fun transectMarks(points: List<TrackPoint>, turnaround: Boolean, current: Boolean): TransectMarks {
    val step = if (turnaround) 4 else 2
    val first = if (turnaround) 1 else 0
    val last = points.size - (if (turnaround) 2 else 1)
    val count = points.size / step
    val segment = { from: Int, quarter: Int -> points.getOrNull(from)?.let { a -> points.getOrNull(from + 1)?.let { b -> arrowOn(a, b, quarter) } } }
    val arrows = when {
        !current || points.size < 2 -> emptyList()
        else -> listOfNotNull(
            segment(first, ENTRY_QUARTER),
            segment(first + step, ENTRY_QUARTER).takeIf { count > 3 },
            segment(last - 1, EXIT_QUARTER),
            segment(last - step - 1, EXIT_QUARTER).takeIf { count > 3 },
        )
    }
    val stubs = when {
        current || !turnaround || points.size < 2 -> emptyList()
        else -> listOf(points.take(2), points.takeLast(2))
    }
    return TransectMarks(arrows, stubs)
}

private fun chevron(): Bitmap {
    val bitmap = Bitmap.createBitmap(ARROW_SIZE_PX, ARROW_SIZE_PX, Bitmap.Config.ARGB_8888)
    val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = android.graphics.Color.WHITE
        style = Paint.Style.STROKE
        strokeWidth = 3f
    }
    val half = ARROW_SIZE_PX / 2f
    Canvas(bitmap).drawPath(Path().apply { moveTo(0f, half); lineTo(half, 0f); lineTo(ARROW_SIZE_PX.toFloat(), half) }, paint)
    return bitmap
}

private const val GIMBAL_WEDGE_SOURCE = "aircast-gimbal-wedges"
private const val GIMBAL_WEDGE_LAYER = "aircast-gimbal-wedge-layer"
private const val GIMBAL_WEDGE_IMAGE = "aircast-gimbal-wedge"
private const val GIMBAL_WEDGE_PX = 72
private const val GIMBAL_WEDGE_SWEEP = 90f

fun gimbalWedges(items: List<MissionItem>): List<TransectArrow> =
    items.filter { it.heading.isFinite() && it.gimbalYaw.isFinite() }
        .map { TransectArrow(TrackPoint(it.latitude, it.longitude), it.heading + it.gimbalYaw) }

private fun wedge(): Bitmap {
    val bitmap = Bitmap.createBitmap(GIMBAL_WEDGE_PX, GIMBAL_WEDGE_PX, Bitmap.Config.ARGB_8888)
    val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = android.graphics.Color.argb(140, 255, 255, 255); style = Paint.Style.FILL }
    Canvas(bitmap).drawArc(android.graphics.RectF(0f, 0f, GIMBAL_WEDGE_PX.toFloat(), GIMBAL_WEDGE_PX.toFloat()), -90f - GIMBAL_WEDGE_SWEEP / 2, GIMBAL_WEDGE_SWEEP, true, paint)
    return bitmap
}

fun renderGimbalWedges(style: Style, items: List<MissionItem>) {
    (style.getSource(GIMBAL_WEDGE_SOURCE) as? GeoJsonSource)?.setGeoJson(
        FeatureCollection.fromFeatures(gimbalWedges(items).map { Feature.fromGeometry(Point.fromLngLat(it.at.longitude, it.at.latitude)).apply { addNumberProperty(ARROW_BEARING, it.bearing) } }),
    )
}

fun installTransectMarks(style: Style) {
    if (style.getSource(GIMBAL_WEDGE_SOURCE) == null) {
        style.addSource(GeoJsonSource(GIMBAL_WEDGE_SOURCE))
        style.addImage(GIMBAL_WEDGE_IMAGE, wedge())
        style.addLayer(
            SymbolLayer(GIMBAL_WEDGE_LAYER, GIMBAL_WEDGE_SOURCE).withProperties(
                PropertyFactory.iconImage(GIMBAL_WEDGE_IMAGE),
                PropertyFactory.iconRotate(Expression.get(ARROW_BEARING)),
                PropertyFactory.iconRotationAlignment(Property.ICON_ROTATION_ALIGNMENT_MAP),
                PropertyFactory.iconAllowOverlap(true),
                PropertyFactory.iconIgnorePlacement(true),
            ),
        )
    }
    if (style.getSource(TRANSECT_ARROW_SOURCE) != null) return
    style.addSource(GeoJsonSource(TRANSECT_STUB_SOURCE))
    style.addLayer(LineLayer(TRANSECT_STUB_LAYER, TRANSECT_STUB_SOURCE).withProperties(PropertyFactory.lineColor("#FFFFFF"), PropertyFactory.lineWidth(2f)))
    style.addSource(GeoJsonSource(TRANSECT_ARROW_SOURCE))
    style.addImage(TRANSECT_ARROW_IMAGE, chevron())
    style.addLayer(
        SymbolLayer(TRANSECT_ARROW_LAYER, TRANSECT_ARROW_SOURCE).withProperties(
            PropertyFactory.iconImage(TRANSECT_ARROW_IMAGE),
            PropertyFactory.iconRotate(Expression.get(ARROW_BEARING)),
            PropertyFactory.iconRotationAlignment(Property.ICON_ROTATION_ALIGNMENT_MAP),
            PropertyFactory.iconAllowOverlap(true),
            PropertyFactory.iconIgnorePlacement(true),
        ),
    )
}

fun renderTransectMarks(style: Style, surveys: List<Survey>, selected: Int?, legArrows: List<TransectArrow> = emptyList()) {
    val marks = surveys.map { transectMarks(it.transects, it.turnaround, it.index == selected) }
    (style.getSource(TRANSECT_ARROW_SOURCE) as? GeoJsonSource)?.setGeoJson(
        FeatureCollection.fromFeatures(
            (marks.flatMap { it.arrows } + legArrows).map { arrow ->
                Feature.fromGeometry(Point.fromLngLat(arrow.at.longitude, arrow.at.latitude)).apply { addNumberProperty(ARROW_BEARING, arrow.bearing) }
            },
        ),
    )
    (style.getSource(TRANSECT_STUB_SOURCE) as? GeoJsonSource)?.setGeoJson(
        FeatureCollection.fromFeatures(
            marks.flatMap { it.stubs }.map { stub -> Feature.fromGeometry(LineString.fromLngLats(stub.map { Point.fromLngLat(it.longitude, it.latitude) })) },
        ),
    )
}
