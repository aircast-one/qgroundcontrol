package one.aircast.android.ui

import androidx.compose.foundation.Canvas
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shadow
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.text.TextMeasurer
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.qgcDerived
import org.json.JSONObject

internal const val VIDEO_AR_VIEW = "view.videoAr"
private val ROUTE_COLOUR = Color(0xFFFFE9A6)
private val HOME_COLOUR = Color(0xFF4CAF50)
private val EDGE_COLOUR = Color.Black.copy(alpha = 0.6f)
private val ROUTE_WIDTH = 3.dp
private val EDGE_WIDTH = 2.dp
private val MARK_RADIUS = 7.dp
private val LABEL_GAP = 4.dp

internal data class ArSpot(val x: Float, val up: Float)

internal data class ArMark(val at: ArSpot, val label: String, val text: String)

internal data class ArReading(val home: ArMark?, val waypoints: List<ArMark>, val route: List<Pair<ArSpot, ArSpot>>)

private fun spot(json: JSONObject?): ArSpot? =
    json?.takeIf { it.has("x") && it.has("up") }?.let { ArSpot(it.optDouble("x").toFloat(), it.optDouble("up").toFloat()) }

private fun mark(json: JSONObject?): ArMark? =
    spot(json)?.let { ArMark(it, json?.optString("label").orEmpty(), json?.optString("text").orEmpty()) }

internal fun arReading(view: JSONObject?): ArReading? = view?.takeIf { it.optBoolean("available") }?.let { shown ->
    val waypoints = shown.optJSONArray("waypoints")
    val route = shown.optJSONArray("route")
    ArReading(
        home = mark(shown.optJSONObject("home")),
        waypoints = (0 until (waypoints?.length() ?: 0)).mapNotNull { mark(waypoints?.optJSONObject(it)) },
        route = (0 until (route?.length() ?: 0)).mapNotNull { index ->
            route?.optJSONObject(index)?.let { leg -> spot(leg.optJSONObject("from"))?.let { start -> spot(leg.optJSONObject("to"))?.let { end -> start to end } } }
        },
    )
}

internal fun arOffset(spot: ArSpot, width: Float, height: Float): Offset = Offset(spot.x * width, height / 2 - spot.up * width)

private fun DrawScope.marked(measurer: TextMeasurer, style: TextStyle, mark: ArMark, fill: Color) {
    val at = arOffset(mark.at, size.width, size.height)
    drawCircle(EDGE_COLOUR, MARK_RADIUS.toPx() + EDGE_WIDTH.toPx(), at)
    drawCircle(fill, MARK_RADIUS.toPx(), at)
    val caption = listOf(mark.label, mark.text).filter { it.isNotBlank() }.joinToString(" · ")
    val laid = measurer.measure(caption, style)
    drawText(laid, topLeft = Offset(at.x - laid.size.width / 2f, at.y - MARK_RADIUS.toPx() - LABEL_GAP.toPx() - laid.size.height))
}

@Composable
internal fun VideoArOverlay(modifier: Modifier = Modifier) {
    val reading = qgcDerived(VIDEO_AR_VIEW, ::arReading).value ?: return
    val measurer = rememberTextMeasurer()
    val style = MaterialTheme.typography.labelMedium.copy(color = Color.White, shadow = Shadow(Color.Black, blurRadius = 4f))
    Canvas(modifier.clipToBounds()) {
        reading.route.map { (from, to) ->
            val (start, end) = arOffset(from, size.width, size.height) to arOffset(to, size.width, size.height)
            drawLine(EDGE_COLOUR, start, end, ROUTE_WIDTH.toPx() + EDGE_WIDTH.toPx() * 2, StrokeCap.Round)
            drawLine(ROUTE_COLOUR, start, end, ROUTE_WIDTH.toPx(), StrokeCap.Round)
        }
        reading.waypoints.map { marked(measurer, style, it, Color.White) }
        reading.home?.let { marked(measurer, style, it, HOME_COLOUR) }
    }
}
