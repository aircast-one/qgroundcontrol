package one.aircast.mapspike

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

private const val PLAN_POLL_MS = 700L

class MapSpikeActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        MapBridge.start()
        org.maplibre.android.MapLibre.getInstance(this)
        val style = installQgcTileSource(this)
        setContent {
            MaterialTheme(colorScheme = darkColorScheme()) {
                Surface(Modifier.fillMaxSize()) { MapSpikeScreen(style) }
            }
        }
    }
}

@Composable
private fun MapSpikeScreen(mapStyle: String) {
    var follow by remember { mutableStateOf(true) }
    var items by remember { mutableStateOf<List<MissionItem>>(emptyList()) }
    var fences by remember { mutableStateOf<List<FencePolygon>>(emptyList()) }
    var rally by remember { mutableStateOf<List<RallyPoint>>(emptyList()) }
    var selected by remember { mutableStateOf<Int?>(null) }
    var busy by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    fun onBridge(label: String? = null, work: () -> Unit) {
        busy = label
        scope.launch {
            withContext(Dispatchers.Default) { work() }
            busy = null
        }
    }

    val available by mapBool("vehicles.activeVehicleAvailable")
    val latitude by mapDouble("vehicle.latitude")
    val longitude by mapDouble("vehicle.longitude")
    val mode by mapString("vehicle.flightMode")

    suspend fun refresh() {
        withContext(Dispatchers.Default) {
            val nextItems = PlanBridge.items()
            val nextFences = FenceBridge.polygons()
            val nextRally = FenceBridge.rally()
            withContext(Dispatchers.Main) {
                items = nextItems
                fences = nextFences
                rally = nextRally
            }
        }
    }

    LaunchedEffect(Unit) {
        while (true) {
            refresh()
            delay(PLAN_POLL_MS)
        }
    }

    Box(Modifier.fillMaxSize()) {
        VehicleMap(
            modifier = Modifier.fillMaxSize(),
            mapStyle = mapStyle,
            follow = follow,
            missionItems = items,
            fencePolygons = fences,
            rallyPoints = rally,
            editable = true,
            onAdd = { lat, lon -> onBridge { PlanBridge.appendWaypoint(lat, lon) } },
            onMove = { index, lat, lon -> onBridge { PlanBridge.moveItem(index, lat, lon) } },
            onWaypointSelected = { selected = it },
        )

        Surface(
            Modifier.align(Alignment.TopCenter).fillMaxWidth().padding(8.dp),
            color = MaterialTheme.colorScheme.surface.copy(alpha = 0.88f),
        ) {
            Column(Modifier.padding(12.dp)) {
                val ready by MapBridge.bridgeReady.collectAsState()
                Text(
                    when {
                        !ready -> "Bridge not running (start the main app first)"
                        available -> "Vehicle: ${mode.ifBlank { "connected" }}"
                        else -> "No vehicle"
                    },
                    style = MaterialTheme.typography.titleSmall,
                )
                Text(
                    if (latitude.isNaN() || longitude.isNaN()) {
                        "No position"
                    } else {
                        "%.6f, %.6f".format(latitude, longitude)
                    },
                    style = MaterialTheme.typography.bodySmall,
                )

                Row(verticalAlignment = Alignment.CenterVertically) {
                    Switch(checked = follow, onCheckedChange = { follow = it })
                    Text(
                        "Follow vehicle",
                        Modifier.padding(start = 8.dp),
                        style = MaterialTheme.typography.bodySmall,
                    )
                }

                Text(
                    busy ?: "Items ${items.size} · fences ${fences.size} · rally ${rally.size}",
                    style = MaterialTheme.typography.bodySmall,
                )

                Row(
                    Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(4.dp),
                ) {
                    TextButton(onClick = {
                        onBridge("Loading from vehicle") { PlanBridge.loadFromVehicle() }
                    }) { Text("Load") }

                    TextButton(onClick = {
                        onBridge("Sending to vehicle") { PlanBridge.sendToVehicle() }
                    }) { Text("Send") }

                    TextButton(onClick = {
                        onBridge("Adding fence") {
                            val centre = TrackPoint(latitude, longitude)
                            if (isPlottable(centre.latitude, centre.longitude)) {
                                FenceBridge.addInclusionPolygon(
                                    TrackPoint(centre.latitude + 0.002, centre.longitude - 0.002),
                                    TrackPoint(centre.latitude - 0.002, centre.longitude + 0.002),
                                )
                            }
                        }
                    }) { Text("Fence") }

                    TextButton(onClick = {
                        onBridge("Adding rally") { FenceBridge.addRallyPoint(latitude, longitude) }
                    }) { Text("Rally") }

                    selected?.let { index ->
                        TextButton(onClick = {
                            onBridge { PlanBridge.removeItem(index) }
                            selected = null
                        }) { Text("Delete #$index") }
                    }
                }
            }
        }
    }
}
