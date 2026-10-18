package one.aircast.android.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.relocation.BringIntoViewRequester
import androidx.compose.foundation.relocation.bringIntoViewRequester
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import one.aircast.android.R
import one.aircast.map.aircast
import kotlin.math.abs

private val CHART_HEIGHT = 112.dp

@Composable
internal fun DeviceFlightsScreen(props: DeviceScreenProps) {
    val context = LocalContext.current
    var lanes by remember { mutableStateOf(storedLanes(context)) }
    val view = deviceView("flights", ::parseFlights) ?: return FootNote("Waiting for the device to answer…")
    val pickLanes = { keys: List<String> ->
        storeLanes(context, keys)
        lanes = keys
    }
    DeviceText("One entry per run of this device. None of it reaches you in the air — the ground station only gets the few lines worth interrupting a flight for.")
    if (view.flights.isEmpty()) DeviceNotice(DeviceBadge(DeviceLevel.Neutral, "No flights recorded yet. This device starts one every time it powers up."))
    view.flights.map { flight -> key(flight.id) { FlightRow(flight, props.query["flight"] == flight.id, lanes, pickLanes) } }
}

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun FlightRow(flight: FlightEntry, linked: Boolean, lanes: List<String>?, onLanes: (List<String>) -> Unit) {
    var active by remember { mutableStateOf<Int?>(null) }
    val anchor = remember { BringIntoViewRequester() }
    LaunchedEffect(linked) { if (linked) anchor.bringIntoView() }
    Column(Modifier.fillMaxWidth().bringIntoViewRequester(anchor)) {
        DeviceDisclosure(flight.title, flight.summary, flight.badge, initiallyOpen = flight.open || linked) {
            FlightBody(flight, active, { active = it }, lanes, onLanes)
        }
        HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
    }
}

@Composable
private fun FlightBody(flight: FlightEntry, active: Int?, onActive: (Int?) -> Unit, lanes: List<String>?, onLanes: (List<String>) -> Unit) {
    var alertsOnly by remember { mutableStateOf(false) }
    Column(Modifier.fillMaxWidth().padding(bottom = 16.dp)) {
        if (hasLines(flight)) {
            ButtonRow {
                FilterChip(selected = !alertsOnly, onClick = { alertsOnly = false }, label = { Text("Everything") })
                FilterChip(selected = alertsOnly, onClick = { alertsOnly = true }, label = { Text("Alerts only") })
            }
            flight.sections.mapIndexed { index, part -> key(index, part.title) { FlightPartRows(part, alertsOnly, active, onActive) } }
        } else {
            DeviceText("Nothing worth recording — the device had no faults to report.")
        }
        flight.trace?.let { FlightLanes(it, active, lanes, onLanes) }
        if (flight.dropped.isNotEmpty()) DeviceNotice(DeviceBadge(DeviceLevel.Warn, flight.dropped))
        FlightFiles(flight)
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun FlightPartRows(part: FlightPart, alertsOnly: Boolean, active: Int?, onActive: (Int?) -> Unit) {
    val muted = MaterialTheme.colorScheme.onSurfaceVariant
    Column(Modifier.fillMaxWidth().padding(horizontal = 8.dp)) {
        if (part.title.isNotEmpty()) {
            FlowRow(Modifier.padding(start = 8.dp, end = 8.dp, top = 16.dp, bottom = 4.dp), horizontalArrangement = Arrangement.spacedBy(12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(part.title.uppercase(), style = MaterialTheme.typography.labelMedium, color = muted)
                if (part.span.isNotEmpty()) Text(part.span, style = MaterialTheme.typography.labelSmall, fontFamily = FontFamily.Monospace, color = muted)
            }
        }
        part.videos.map { video -> key(video.camera) { SortieVideoLinks(video) } }
        part.lines.filter { !alertsOnly || it.alert }.map { line -> key(line.index) { LineRow(line, active == line.index, onActive) } }
    }
}

@Composable
private fun lineColor(line: FlightLine): Color = when {
    line.heldBack -> MaterialTheme.colorScheme.onSurfaceVariant
    line.level == DeviceLevel.Neutral -> MaterialTheme.colorScheme.onSurface
    else -> deviceLevelColor(line.level)
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun LineRow(line: FlightLine, active: Boolean, onActive: (Int?) -> Unit) {
    val muted = MaterialTheme.colorScheme.onSurfaceVariant
    FlowRow(
        Modifier.fillMaxWidth()
            .background(if (active) MaterialTheme.colorScheme.surfaceContainerHighest else Color.Transparent, RoundedCornerShape(4.dp))
            .clickable { onActive(if (active) null else line.index) }
            .padding(horizontal = 8.dp, vertical = 4.dp),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
        verticalArrangement = Arrangement.spacedBy(2.dp),
    ) {
        Text(line.clock, style = MaterialTheme.typography.bodySmall, fontFamily = FontFamily.Monospace, color = muted, modifier = Modifier.width(64.dp))
        Text(line.text, style = MaterialTheme.typography.bodyMedium, color = lineColor(line), modifier = Modifier.weight(1f))
        if (line.repeats.isNotEmpty()) Text(line.repeats, style = MaterialTheme.typography.labelSmall, fontFamily = FontFamily.Monospace, color = muted)
        if (line.heldBack) {
            Text(
                "pilot not told",
                style = MaterialTheme.typography.labelSmall,
                color = muted,
                modifier = Modifier.border(1.dp, MaterialTheme.colorScheme.outlineVariant, CircleShape).padding(horizontal = 8.dp, vertical = 2.dp),
            )
        }
    }
}

@Composable
private fun SortieVideoLinks(video: SortieVideo) {
    Column(Modifier.padding(horizontal = 8.dp, vertical = 4.dp)) {
        LinkButton(deviceUrl(video.url), video.label, look = LinkLook.Outlined, icon = R.drawable.ic_download)
        video.share?.let { SortieShareRow(it) }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun SortieShareRow(share: SortieShare) {
    val scope = rememberCoroutineScope()
    val copy = rememberCopy()
    val muted = MaterialTheme.colorScheme.onSurfaceVariant
    var job by remember { mutableStateOf(IDLE_RENDER) }
    var polls by remember { mutableIntStateOf(0) }
    var asking by remember { mutableStateOf(false) }
    var showingQr by remember { mutableStateOf(false) }
    LaunchedEffect(job.id, polls) {
        if (renderRunning(job) && job.id.isNotEmpty()) {
            delay(RENDER_POLL_MS)
            job = readRender(job.id)
            polls++
        }
    }
    Column {
        if (job.state.isEmpty() || job.state == "failed") OutlinedButton(onClick = { asking = true }) { Text(share.label) }
        when (job.state) {
            "uploading" -> Text("Sending to Aircast cloud… ${renderPercent(job)}%", style = MaterialTheme.typography.bodySmall, color = muted)
            "rendering" -> Text("Burning the telemetry in… this takes a few minutes; you can leave this page.", style = MaterialTheme.typography.bodySmall, color = muted)
            "failed" -> Text(job.error, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
            "done" -> {
                FlowRow {
                    IconButton(onClick = { showingQr = true }) { Icon(painterResource(R.drawable.ic_qr_code_2), "Show the share link as a QR code") }
                    LinkButton(job.url, "Open the share version", look = LinkLook.Text, external = true)
                    TextButton(onClick = { copy(job.url) }) { Text("Copy link") }
                }
                Text(renderExpiry(job), style = MaterialTheme.typography.labelSmall, color = muted)
            }
        }
    }
    if (showingQr) QrDialog("Share version", job.url) { showingQr = false }
    if (asking) ConfirmDialog(ActionConfirm(share.label, share.confirm, "Send"), { scope.launch { job = startRender(share) } }) { asking = false }
}

private class LanePlot {
    var zoom by mutableStateOf<Zoom?>(null)
    var cursor by mutableStateOf<Double?>(null)
    var brush by mutableStateOf<Zoom?>(null)

    fun down(at: Double) {
        brush = Zoom(at, at)
        cursor = at
    }

    fun move(at: Double) {
        brush = brush?.copy(to = at)
    }

    fun up() {
        brush?.takeIf { abs(it.to - it.from) >= MIN_BRUSH }?.let { zoom = composeZoom(zoom, it.from, it.to) }
        brush = null
    }
}

@Composable
private fun FlightLanes(trace: FlightTrace, active: Int?, lanes: List<String>?, onLanes: (List<String>) -> Unit) {
    val plot = remember { LanePlot() }
    val keys = laneKeys(lanes, trace.lanes.filter { it.default }.map { it.key })
    val shown = trace.lanes.filter { it.key in keys }
    Column(Modifier.fillMaxWidth().padding(top = 8.dp)) {
        HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
        ButtonRow {
            trace.lanes.map { lane ->
                key(lane.key) {
                    FilterChip(
                        selected = lane.key in keys,
                        onClick = { onLanes(toggled(keys, lane.key)) },
                        label = {
                            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                                Text(lane.chip)
                                if (lane.peak.isNotEmpty()) Text(lane.peak, style = MaterialTheme.typography.labelSmall, fontFamily = FontFamily.Monospace, color = MaterialTheme.aircast.warning)
                            }
                        },
                    )
                }
            }
        }
        if (trace.spans.isNotEmpty() || plot.zoom != null) {
            ButtonRow {
                trace.spans.map { span ->
                    key(span.label) { FilterChip(selected = plot.zoom == Zoom(span.from, span.to), onClick = { plot.zoom = Zoom(span.from, span.to) }, label = { Text(span.label) }) }
                }
                if (plot.zoom != null) TextButton(onClick = { plot.zoom = null }) { Text("Whole flight") }
            }
        }
        shown.mapIndexed { index, lane -> key(lane.key) { LaneChart(lane, trace, plot, active, index == shown.lastIndex) } }
        Text(
            "Pick the lanes you want above. One cursor across them: point at any lane to read every sample at that moment. Drag across a lane to zoom them all, or jump to one sortie with the chips above. " +
                "Dashed lines are the moments listed above, in their colour; point at a line to pick out its mark.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(start = 16.dp, end = 16.dp, top = 12.dp),
        )
    }
}

private fun fractionIn(x: Float, width: Int): Double = if (width > 0) (x / width).coerceIn(0f, 1f).toDouble() else 0.0

@Composable
private fun markerColor(level: Int): Color = if (level == DeviceLevel.Neutral) MaterialTheme.colorScheme.onSurfaceVariant else deviceLevelColor(level)

@Composable
private fun LaneChart(lane: TraceLane, trace: FlightTrace, plot: LanePlot, active: Int?, last: Boolean) {
    val slice = zoomSlice(lane.values.size, plot.zoom)
    val shown = lane.values.slice(slice)
    val start = trace.startUnix + slice.first.toLong() * trace.step
    val grid = gridLines(ceilingOf(shown, lane.max), lane.unit)
    val markers = markerLines(trace.markers, trace.step, shown.size, slice.first * trace.step)
    val readout = plot.cursor?.let { readoutAt(shown, lane.notes.drop(slice.first).take(shown.size), it, lane.unit) }
    val lines = remember(shown, lane.max) { segments(shown, lane.max) }
    val muted = MaterialTheme.colorScheme.onSurfaceVariant
    val ink = MaterialTheme.colorScheme.primary
    val rule = MaterialTheme.colorScheme.onSurface
    val markerInks = markers.map { markerColor(it.level) }
    Column(Modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, top = 16.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(lane.label.uppercase(), style = MaterialTheme.typography.labelSmall, color = muted, modifier = Modifier.weight(1f))
            readout?.let { Text("${clockAt(start + it.index.toLong() * trace.step, trace.tzOffset, true)} · ${it.text}", style = MaterialTheme.typography.labelSmall, fontFamily = FontFamily.Monospace) }
        }
        Box(
            Modifier.fillMaxWidth()
                .padding(top = 4.dp)
                .height(CHART_HEIGHT)
                .pointerInput(plot) { detectTapGestures { at -> plot.cursor = fractionIn(at.x, size.width) } }
                .pointerInput(plot) {
                    detectHorizontalDragGestures(
                        onDragStart = { at -> plot.down(fractionIn(at.x, size.width)) },
                        onDragEnd = plot::up,
                        onDragCancel = { plot.brush = null },
                    ) { change, _ -> plot.move(fractionIn(change.position.x, size.width)) }
                },
        ) {
            Canvas(Modifier.matchParentSize().semantics { contentDescription = "${lane.label} over the flight" }) {
                val across = size.width / 100f
                val down = size.height / PLOT_BOX.toFloat()
                grid.map { line -> drawLine(rule.copy(alpha = 0.12f), Offset(0f, line.y.toFloat() * down), Offset(size.width, line.y.toFloat() * down), 1.dp.toPx()) }
                markers.zip(markerInks).map { (marker, color) ->
                    val picked = marker.index == active
                    val x = marker.x.toFloat() * across
                    drawLine(color.copy(alpha = if (picked) 1f else 0.7f), Offset(x, 0f), Offset(x, size.height), 1.dp.toPx(), pathEffect = if (picked) null else PathEffect.dashPathEffect(floatArrayOf(2.dp.toPx(), 2.dp.toPx())))
                }
                lines.map { run ->
                    val path = Path().apply {
                        moveTo(run.first().x * across, run.first().y * down)
                        run.drop(1).map { lineTo(it.x * across, it.y * down) }
                    }
                    drawPath(path, ink, style = Stroke(width = 1.5.dp.toPx()))
                }
                plot.brush?.let { brush -> drawRect(ink.copy(alpha = 0.15f), Offset(minOf(brush.from, brush.to).toFloat() * size.width, 0f), Size(abs(brush.to - brush.from).toFloat() * size.width, size.height)) }
                readout?.let { drawLine(rule.copy(alpha = 0.4f), Offset(it.x.toFloat() * across, 0f), Offset(it.x.toFloat() * across, size.height), 1.dp.toPx()) }
            }
            grid.map { line ->
                key(line.y) {
                    Text(
                        line.label,
                        style = MaterialTheme.typography.labelSmall,
                        fontFamily = FontFamily.Monospace,
                        color = muted,
                        modifier = Modifier.align(Alignment.TopEnd)
                            .offset(y = CHART_HEIGHT * (line.y / PLOT_BOX).toFloat() - 8.dp)
                            .background(MaterialTheme.colorScheme.surface.copy(alpha = 0.8f), RoundedCornerShape(4.dp))
                            .padding(horizontal = 4.dp),
                    )
                }
            }
        }
        if (last) {
            Row(Modifier.fillMaxWidth().padding(top = 2.dp), horizontalArrangement = Arrangement.SpaceBetween) {
                ticks(start, trace.step, shown.size) { clockAt(it, trace.tzOffset, false) }.map { tick ->
                    key(tick.x) { Text(tick.label, style = MaterialTheme.typography.labelSmall, fontFamily = FontFamily.Monospace, color = muted) }
                }
            }
            if (shown.size > 1) Text("${spanLabel(trace.step * (shown.size - 1))} of flight", style = MaterialTheme.typography.labelSmall, color = muted)
        }
    }
}

@Composable
private fun FlightFiles(flight: FlightEntry) {
    val clipboard = LocalClipboardManager.current
    val notice = LocalChangeNotice.current
    val link = flightLink(deviceUrl("/fly/"), flight.id)
    Column(Modifier.fillMaxWidth().padding(top = 12.dp)) {
        HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
        ButtonRow {
            LinkButton(deviceUrl(flight.csv), "Samples as CSV", look = LinkLook.Outlined)
            LinkButton(deviceUrl(flight.json), "Everything as JSON", look = LinkLook.Text)
            TextButton(onClick = {
                clipboard.setText(AnnotatedString(link))
                notice.show("Link copied — it opens straight on this flight.", null)
            }) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Icon(painterResource(R.drawable.ic_link), null, Modifier.size(18.dp))
                    Text("Copy link to this flight")
                }
            }
        }
        Text(
            "The CSV is one row per sample — every lane above and more besides: signal and radio, the aircraft's height, distance, battery and satellites, video per camera, viewers, what was dropped, and the box's own load and memory. " +
                "The JSON adds every line above.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(horizontal = 16.dp),
        )
    }
}
