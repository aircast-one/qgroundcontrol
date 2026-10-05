package one.aircast.map

import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge

data class EditableShape(
    val path: String,
    val midpoints: List<TrackPoint>,
    val splitInvokable: String,
    val canRemoveVertex: Boolean,
    val edgeLengths: List<String> = emptyList(),
    val distanceUnit: String = "m",
    val metresPerUnit: Double = 1.0,
    val caption: String = "",
    val circleCaption: String = "",
)

fun editableShape(path: String, shape: String = ""): EditableShape? {
    val call = if (shape.isBlank()) "view.polygon($path)" else "view.polygon($path,$shape)"
    val view = runCatching { JSONObject(QGCBridge.get(call)) }.getOrNull() ?: return null
    if (view.optText("class") != "EditablePolygon") return null
    val between = view.optJSONArray("midpoints")
    return EditableShape(
        path = path,
        midpoints = (0 until (between?.length() ?: 0)).mapNotNull { coordinate(between?.optJSONObject(it)) },
        splitInvokable = view.optText("splitInvokable"),
        canRemoveVertex = view.optBoolean("canRemoveVertex"),
        edgeLengths = view.optJSONArray("edgeLengths")?.let { list -> (0 until list.length()).map { list.optString(it) } }.orEmpty(),
        distanceUnit = view.optText("horizontalUnit").ifBlank { "m" },
        metresPerUnit = view.optDouble("horizontalMetresPerUnit", 1.0).takeIf { it.isFinite() && it > 0 } ?: 1.0,
        caption = view.optText("caption"),
        circleCaption = view.optText("circleCaption"),
    )
}

fun edgeLabels(hit: MapHit?, fences: List<FencePolygon>, surveys: List<Survey>): List<LandingLabel> {
    val shape = when (hit) {
        is MapHit.FenceVertex -> fences.firstOrNull { it.index == hit.polygon }?.editable
        is MapHit.SurveyVertex -> surveys.firstOrNull { it.index == hit.item }?.editable
        else -> null
    } ?: return emptyList()
    return shape.midpoints.zip(shape.edgeLengths).map { (at, text) -> LandingLabel(at, text) }
}
