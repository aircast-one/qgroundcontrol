package one.aircast.mapspike

import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge

data class EditableShape(
    val path: String,
    val midpoints: List<TrackPoint>,
    val splitInvokable: String,
    val canRemoveVertex: Boolean,
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
    )
}
