package one.aircast.mapspike

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

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
    val mission = remember { MissionModel() }
    var missionRevision by remember { mutableStateOf(0) }
    var selectedWaypoint by remember { mutableStateOf<Int?>(null) }

    val available by mapBool("vehicles.activeVehicleAvailable")
    val latitude by mapDouble("vehicle.latitude")
    val longitude by mapDouble("vehicle.longitude")
    val mode by mapString("vehicle.flightMode")

    Box(Modifier.fillMaxSize()) {
        VehicleMap(
            modifier = Modifier.fillMaxSize(),
            mapStyle = mapStyle,
            follow = follow,
            mission = mission,
            missionRevision = missionRevision,
            onMissionChanged = { missionRevision++ },
            onWaypointSelected = { selectedWaypoint = it },
        )

        Surface(
            Modifier.align(Alignment.TopCenter).fillMaxWidth().padding(8.dp),
            color = MaterialTheme.colorScheme.surface.copy(alpha = 0.85f),
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
                Row(follow) { follow = it }
                Text(
                    "Waypoints: ${mission.size.also { missionRevision }} " +
                        "· long-press to add, drag to move, tap to select",
                    style = MaterialTheme.typography.bodySmall,
                )
                selectedWaypoint?.let { id ->
                    androidx.compose.material3.TextButton(onClick = {
                        mission.remove(id)
                        selectedWaypoint = null
                        missionRevision++
                    }) { Text("Delete selected waypoint") }
                }
            }
        }
    }
}

@Composable
private fun Row(follow: Boolean, onChange: (Boolean) -> Unit) {
    androidx.compose.foundation.layout.Row(verticalAlignment = Alignment.CenterVertically) {
        Switch(checked = follow, onCheckedChange = onChange)
        Text("Follow vehicle", Modifier.padding(start = 8.dp), style = MaterialTheme.typography.bodySmall)
    }
}
