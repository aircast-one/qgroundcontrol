package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RangeSlider
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.mapspike.TRACK_VIEW
import one.aircast.mapspike.TrackPoint
import one.aircast.mapspike.VehicleMap
import one.aircast.mapspike.aircast
import one.aircast.mapspike.currentMapType
import one.aircast.mapspike.optText
import one.aircast.mapspike.qgcRasterStyle
import one.aircast.mapspike.trackReading
import org.json.JSONObject
import java.util.Locale

internal const val OFFLINE_MAPS_GROUP = "offlineMapsSettings"
internal const val OFFLINE_MAPS_VIEW = "view.offlineMaps"
internal const val OFFLINE_START = "offlineMaps.startDownload"
internal const val OFFLINE_RESUME = "offlineMaps.resume"
internal const val OFFLINE_CANCEL = "offlineMaps.cancel"
internal const val OFFLINE_DELETE = "offlineMaps.delete"
private const val OFFLINE_POLL_MS = 1000L
private const val MIN_ZOOM_PATH = "settings.offlineMapsSettings.minZoomLevelDownload"
private const val MAX_ZOOM_PATH = "settings.offlineMapsSettings.maxZoomLevelDownload"
private const val DEFAULT_MIN_ZOOM = 13
private const val DEFAULT_MAX_ZOOM = 19
private const val SLIDER_MIN_ZOOM = 1f
private const val SLIDER_MAX_ZOOM = 20f

internal data class OfflineSet(
    val id: Long,
    val name: String,
    val mapType: String,
    val defaultSet: Boolean,
    val zoomText: String,
    val totalText: String,
    val uniqueText: String,
    val downloadedText: String,
    val sizeText: String,
    val tileCountText: String,
    val errorCount: Int,
    val errorCountText: String,
    val downloadStatus: String,
    val downloading: Boolean,
    val complete: Boolean,
)

internal data class OfflineEstimate(val tileCountText: String, val tileSizeText: String, val tooMany: Boolean)

internal data class OfflineMaps(
    val available: Boolean,
    val reason: String,
    val sets: List<OfflineSet>,
    val mapList: List<String>,
    val uniqueName: String,
    val takenNames: List<String>,
    val estimate: OfflineEstimate?,
)

internal data class OfflineRegion(val west: Double, val north: Double, val east: Double, val south: Double)

private fun strings(view: JSONObject, key: String): List<String> =
    view.optJSONArray(key)?.let { array -> (0 until array.length()).map { array.optString(it) } } ?: emptyList()

internal fun offlineMaps(view: JSONObject?): OfflineMaps? =
    view?.takeIf { it.optString("class") == "OfflineMaps" }?.let {
        val sets = it.optJSONArray("sets")
        OfflineMaps(
            available = it.optBoolean("available"),
            reason = it.optText("reason"),
            sets = (0 until (sets?.length() ?: 0)).mapNotNull { index ->
                sets?.optJSONObject(index)?.let { set ->
                    OfflineSet(
                        id = set.optLong("id"),
                        name = set.optText("name"),
                        mapType = set.optText("mapTypeStr"),
                        defaultSet = set.optBoolean("defaultSet"),
                        zoomText = set.optText("zoomText"),
                        totalText = set.optText("totalText"),
                        uniqueText = set.optText("uniqueText"),
                        downloadedText = set.optText("downloadedText"),
                        sizeText = set.optText("sizeText"),
                        tileCountText = set.optText("tileCountText"),
                        errorCount = set.optInt("errorCount"),
                        errorCountText = set.optText("errorCountText"),
                        downloadStatus = set.optText("downloadStatus"),
                        downloading = set.optBoolean("downloading"),
                        complete = set.optBoolean("complete"),
                    )
                }
            },
            mapList = strings(it, "mapList"),
            uniqueName = it.optText("uniqueName"),
            takenNames = strings(it, "takenNames"),
            estimate = it.optJSONObject("estimate")?.let { estimate ->
                OfflineEstimate(estimate.optText("tileCountText"), estimate.optText("tileSizeText"), estimate.optBoolean("tooMany"))
            },
        )
    }

internal fun offlineRegion(corners: List<TrackPoint>): OfflineRegion? =
    corners.takeIf { it.isNotEmpty() }?.let { points ->
        OfflineRegion(
            west = points.minOf { it.longitude },
            north = points.maxOf { it.latitude },
            east = points.maxOf { it.longitude },
            south = points.minOf { it.latitude },
        )
    }

private fun coordinate(value: Double): String = String.format(Locale.US, "%.7f", value)

internal fun offlineMapsPath(mapType: String?, region: OfflineRegion?, minZoom: Int, maxZoom: Int): String =
    if (mapType == null || region == null) {
        OFFLINE_MAPS_VIEW
    } else {
        "$OFFLINE_MAPS_VIEW($mapType,${coordinate(region.west)},${coordinate(region.north)},${coordinate(region.east)},${coordinate(region.south)},$minZoom,$maxZoom)"
    }

private fun zoomSetting(path: String, fallback: Int): Int =
    Qgc.get(path).optDouble("value").takeIf { !it.isNaN() }?.toInt() ?: fallback

private suspend fun act(path: String, vararg args: Any?): String? = withContext(Dispatchers.IO) { Qgc.refusalOf(path, *args) }

@Composable
fun OfflineMapsSection() {
    val scope = rememberCoroutineScope()
    var maps by remember { mutableStateOf<OfflineMaps?>(null) }
    var polls by remember { mutableIntStateOf(0) }
    var shown by remember { mutableStateOf<OfflineSet?>(null) }
    var adding by remember { mutableStateOf(false) }
    var refusal by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(polls) {
        while (isActive) {
            maps = withContext(Dispatchers.IO) { offlineMaps(Qgc.get(OFFLINE_MAPS_VIEW)) }
            delay(OFFLINE_POLL_MS)
        }
    }

    val read = maps ?: return
    if (!read.available) {
        FootNote(read.reason.ifBlank { "There is no map tile cache on this device." })
        return
    }
    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        read.sets.forEach { set ->
            Row(
                Modifier.fillMaxWidth().clickable { shown = set }.padding(vertical = 8.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Text(set.name, modifier = Modifier.weight(1f))
                if (set.downloading) CircularProgressIndicator(Modifier.size(16.dp), strokeWidth = 2.dp)
                if (set.errorCount > 0) Text(set.errorCountText, color = MaterialTheme.aircast.alert)
                Text(set.downloadStatus, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            HorizontalDivider()
        }
        Button(onClick = { adding = true }) { Text("Add New Set") }
        refusal?.let { Text(it, color = MaterialTheme.aircast.alert) }
    }

    shown?.let { picked ->
        val current = read.sets.find { it.id == picked.id } ?: picked
        OfflineSetDialog(
            set = current,
            onDismiss = { shown = null },
            onAction = { path ->
                scope.launch {
                    refusal = act(path, current.id)
                    if (path == OFFLINE_DELETE) shown = null
                    polls++
                }
            },
        )
    }

    if (adding) {
        OfflineSetEditor(
            onDismiss = { adding = false },
            onDownload = { name, mapType, region, minZoom, maxZoom ->
                scope.launch {
                    refusal = act(OFFLINE_START, name, mapType, region.west, region.north, region.east, region.south, minZoom, maxZoom)
                    if (refusal == null) adding = false
                    polls++
                }
            },
        )
    }
}

@Composable
private fun InfoLine(label: String, value: String) {
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(label, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.weight(0.45f))
        Text(value, modifier = Modifier.weight(0.55f))
    }
}

@Composable
private fun OfflineSetDialog(set: OfflineSet, onDismiss: () -> Unit, onAction: (String) -> Unit) {
    var confirming by remember { mutableStateOf(false) }
    if (confirming) {
        AlertDialog(
            onDismissRequest = { confirming = false },
            title = { Text("Confirm Delete") },
            text = {
                Text(
                    if (set.defaultSet) {
                        "This will delete all tiles INCLUDING the tile sets you have created yourself.\n\nIs this really what you want?"
                    } else {
                        "Delete ${set.name} and all its tiles.\n\nIs this really what you want?"
                    },
                )
            },
            confirmButton = {
                TextButton(onClick = {
                    confirming = false
                    onAction(OFFLINE_DELETE)
                }) { Text("Delete") }
            },
            dismissButton = { TextButton(onClick = { confirming = false }) { Text("Cancel") } },
        )
        return
    }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(set.name) },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                if (set.defaultSet) {
                    InfoLine("Size:", set.sizeText)
                    InfoLine("Tile Count:", set.tileCountText)
                } else {
                    Text(set.mapType, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    InfoLine("Zoom Levels:", set.zoomText)
                    InfoLine("Total:", set.totalText)
                    InfoLine("Unique:", set.uniqueText)
                    InfoLine("Downloaded:", set.downloadedText)
                    InfoLine("Error Count:", set.errorCountText)
                }
            }
        },
        confirmButton = {
            Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                if (!set.defaultSet && !set.complete && !set.downloading) {
                    TextButton(onClick = { onAction(OFFLINE_RESUME) }) { Text("Resume Download") }
                }
                if (!set.defaultSet && set.downloading) {
                    TextButton(onClick = { onAction(OFFLINE_CANCEL) }) { Text("Cancel Download") }
                }
                TextButton(onClick = { confirming = true }) { Text("Delete") }
                TextButton(onClick = onDismiss) { Text(if (set.defaultSet) "Close" else "Ok") }
            }
        },
    )
}

@Composable
private fun OfflineSetEditor(
    onDismiss: () -> Unit,
    onDownload: (String, String, OfflineRegion, Int, Int) -> Unit,
) {
    var mapType by remember { mutableStateOf(currentMapType()) }
    var zooms by remember { mutableStateOf(DEFAULT_MIN_ZOOM.toFloat()..DEFAULT_MAX_ZOOM.toFloat()) }
    var region by remember { mutableStateOf<OfflineRegion?>(null) }
    var name by remember { mutableStateOf<String?>(null) }
    var read by remember { mutableStateOf<OfflineMaps?>(null) }
    var centre by remember { mutableStateOf<TrackPoint?>(null) }
    var typeMenu by remember { mutableStateOf(false) }
    val minZoom = zooms.start.toInt()
    val maxZoom = zooms.endInclusive.toInt()

    LaunchedEffect(Unit) {
        val (min, max, at) = withContext(Dispatchers.IO) {
            Triple(zoomSetting(MIN_ZOOM_PATH, DEFAULT_MIN_ZOOM), zoomSetting(MAX_ZOOM_PATH, DEFAULT_MAX_ZOOM), trackReading(Qgc.get(TRACK_VIEW)).points.lastOrNull())
        }
        zooms = min.toFloat()..max.toFloat()
        centre = at
    }
    LaunchedEffect(mapType, region, minZoom, maxZoom) {
        read = withContext(Dispatchers.IO) { offlineMaps(Qgc.get(offlineMapsPath(mapType, region, minZoom, maxZoom))) }
        if (name == null) name = read?.uniqueName
    }

    val chosenName = name.orEmpty()
    val nameTaken = read?.takenNames?.contains(chosenName.trim()) == true
    val estimate = read?.estimate

    Dialog(onDismissRequest = onDismiss, properties = DialogProperties(usePlatformDefaultWidth = false)) {
        Surface(Modifier.fillMaxSize()) {
            Column(Modifier.fillMaxSize()) {
                Box(Modifier.weight(1f).fillMaxWidth()) {
                    VehicleMap(
                        modifier = Modifier.fillMaxSize(),
                        mapStyle = qgcRasterStyle(mapType),
                        follow = false,
                        centreRequest = if (centre == null) 0 else 1,
                        centreOn = centre,
                        onViewChanged = { corners -> region = offlineRegion(corners) },
                    )
                }
                Column(
                    Modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(16.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    Text("Add New Set", style = MaterialTheme.typography.titleMedium)
                    OutlinedTextField(
                        value = chosenName,
                        onValueChange = { name = it },
                        label = { Text("Name:") },
                        singleLine = true,
                        isError = nameTaken,
                        modifier = Modifier.fillMaxWidth(),
                    )
                    Box {
                        OutlinedButton(onClick = { typeMenu = true }) { Text("Map type: $mapType") }
                        DropdownMenu(expanded = typeMenu, onDismissRequest = { typeMenu = false }) {
                            read?.mapList.orEmpty().forEach { option ->
                                DropdownMenuItem(text = { Text(option) }, onClick = {
                                    typeMenu = false
                                    mapType = option
                                })
                            }
                        }
                    }
                    Text("Min/Max Zoom Levels")
                    RangeSlider(
                        value = zooms,
                        onValueChange = { zooms = it },
                        valueRange = SLIDER_MIN_ZOOM..SLIDER_MAX_ZOOM,
                        steps = (SLIDER_MAX_ZOOM - SLIDER_MIN_ZOOM).toInt() - 1,
                    )
                    Row(horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                        Text("Min Zoom: $minZoom")
                        Text("Max Zoom: $maxZoom")
                    }
                    estimate?.let {
                        InfoLine("Tile Count:", it.tileCountText)
                        InfoLine("Est Size:", it.tileSizeText)
                        if (it.tooMany) Text("Too many tiles", color = MaterialTheme.aircast.alert)
                    }
                    if (nameTaken) Text("Tile set with this name already exists", color = MaterialTheme.aircast.alert)
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        Button(
                            onClick = { region?.let { onDownload(chosenName.trim(), mapType, it, minZoom, maxZoom) } },
                            enabled = region != null && estimate != null && !estimate.tooMany && !nameTaken && chosenName.isNotBlank(),
                        ) { Text("Download") }
                        OutlinedButton(onClick = onDismiss) { Text("Cancel") }
                    }
                }
            }
        }
    }
}
