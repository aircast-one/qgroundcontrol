package one.aircast.android.ui

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.BatteryManager
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.map.aircast
import one.aircast.map.optText
import org.json.JSONObject

internal data class PhoneBattery(val percent: Int, val charging: Boolean)

internal fun phoneBattery(level: Int, scale: Int, status: Int): PhoneBattery? =
    if (level < 0 || scale <= 0) {
        null
    } else {
        PhoneBattery(
            percent = level * 100 / scale,
            charging = status == BatteryManager.BATTERY_STATUS_CHARGING || status == BatteryManager.BATTERY_STATUS_FULL,
        )
    }

internal fun gcsBatteryPath(battery: PhoneBattery): String = "view.gcsBattery(${battery.percent},${battery.charging})"

internal data class GcsBatteryReading(val state: String, val levelText: String, val stateText: String, val heading: String, val title: String)

internal fun gcsBatteryReading(view: JSONObject?): GcsBatteryReading? =
    view?.takeIf { it.has("state") }?.let {
        GcsBatteryReading(it.optText("state"), it.optText("levelText"), it.optText("stateText"), it.optText("heading"), it.optText("title"))
    }

@Composable
private fun batteryColour(state: String): Color = when (state) {
    "charging" -> MaterialTheme.aircast.success
    "critical" -> MaterialTheme.colorScheme.error
    "low" -> MaterialTheme.aircast.warning
    else -> MaterialTheme.colorScheme.onSurfaceVariant
}

@Composable
internal fun rememberGcsBattery(): GcsBatteryReading? {
    val context = LocalContext.current
    var battery by remember { mutableStateOf<PhoneBattery?>(null) }
    var reading by remember { mutableStateOf<GcsBatteryReading?>(null) }

    DisposableEffect(context) {
        val receiver = object : BroadcastReceiver() {
            override fun onReceive(context: Context, intent: Intent) {
                battery = phoneBattery(
                    intent.getIntExtra(BatteryManager.EXTRA_LEVEL, -1),
                    intent.getIntExtra(BatteryManager.EXTRA_SCALE, -1),
                    intent.getIntExtra(BatteryManager.EXTRA_STATUS, -1),
                )
            }
        }
        context.registerReceiver(receiver, IntentFilter(Intent.ACTION_BATTERY_CHANGED))
        onDispose { context.unregisterReceiver(receiver) }
    }

    LaunchedEffect(battery) {
        val current = battery ?: return@LaunchedEffect
        reading = withContext(Dispatchers.Default) { gcsBatteryReading(Qgc.get(gcsBatteryPath(current))) }
    }
    return reading
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun GcsBatteryCell(reading: GcsBatteryReading?) {
    var open by remember { mutableStateOf(false) }
    val shown = reading ?: return
    Text(
        shown.levelText,
        style = MaterialTheme.typography.labelMedium,
        color = batteryColour(shown.state),
        maxLines = 1,
        modifier = Modifier
            .clickable(onClickLabel = shown.heading) { open = true }
            .semantics { contentDescription = "${shown.title} ${shown.levelText}" },
    )

    if (open) {
        ModalBottomSheet(onDismissRequest = { open = false }) {
            Column(Modifier.fillMaxWidth().padding(bottom = 24.dp)) {
                Text(shown.heading, style = MaterialTheme.typography.titleMedium, modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp))
                ListItem(headlineContent = { Text("Charge") }, trailingContent = { Text(shown.levelText) })
                ListItem(headlineContent = { Text("State") }, trailingContent = { Text(shown.stateText) })
            }
        }
    }
}
