package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcPath
import one.aircast.mapspike.optText
import org.json.JSONObject

internal const val GIMBAL_INDICATOR_PATH = "view.gimbalIndicator"

internal data class GimbalChoice(val name: String, val managerCompid: Int, val deviceId: Int, val active: Boolean)

internal data class GimbalIndicatorState(
    val statusText: String,
    val pitchText: String,
    val yawText: String,
    val yawLockLabel: String,
    val yawLocked: Boolean,
    val retractOffered: Boolean,
    val controlOffered: Boolean,
    val controlLabel: String,
    val haveControl: Boolean,
    val gimbals: List<GimbalChoice>,
)

internal fun gimbalIndicator(view: JSONObject?): GimbalIndicatorState? =
    view?.takeIf { it.optBoolean("shown") }?.let {
        val listed = it.optJSONArray("gimbals")
        GimbalIndicatorState(
            statusText = it.optText("statusText"),
            pitchText = it.optText("pitchText"),
            yawText = it.optText("yawText"),
            yawLockLabel = it.optText("yawLockLabel"),
            yawLocked = it.optBoolean("yawLocked"),
            retractOffered = it.optBoolean("retractOffered"),
            controlOffered = it.optBoolean("controlOffered"),
            controlLabel = it.optText("controlLabel"),
            haveControl = it.optBoolean("haveControl"),
            gimbals = (0 until (listed?.length() ?: 0)).mapNotNull { index ->
                listed!!.optJSONObject(index)?.let { g ->
                    GimbalChoice(g.optText("name"), g.optInt("managerCompid"), g.optInt("deviceId"), g.optBoolean("active"))
                }
            },
        )
    }

internal fun gimbalCellText(state: GimbalIndicatorState): String =
    listOf(state.statusText, state.pitchText, state.yawText).filter { it.isNotBlank() }.joinToString(" · ")

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun GimbalIndicatorCell() {
    val view by qgcPath(GIMBAL_INDICATOR_PATH)
    val state = remember(view) { gimbalIndicator(view) } ?: return
    var open by remember { mutableStateOf(false) }
    var refusal by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    fun act(path: String, vararg args: Any?) {
        scope.launch {
            refusal = withContext(Dispatchers.Default) { Qgc.refusalOf(path, *args) }
            if (refusal == null) open = false
        }
    }

    Text(
        gimbalCellText(state),
        style = MaterialTheme.typography.labelMedium,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        maxLines = 1,
        modifier = Modifier.clickable { open = true },
    )

    if (open) {
        ModalBottomSheet(onDismissRequest = { open = false }) {
            Column(
                Modifier.fillMaxWidth().padding(horizontal = 20.dp).padding(bottom = 24.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Text("Gimbal", style = MaterialTheme.typography.titleMedium)
                if (state.gimbals.size > 1) {
                    Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                        state.gimbals.forEach { gimbal ->
                            FilterChip(
                                selected = gimbal.active,
                                onClick = { act("gimbal.select", gimbal.managerCompid, gimbal.deviceId) },
                                label = { Text(gimbal.name) },
                            )
                        }
                    }
                }
                Text(gimbalCellText(state), style = MaterialTheme.typography.bodyMedium)
                OutlinedButton(onClick = { act("gimbal.yawLock", !state.yawLocked) }, modifier = Modifier.fillMaxWidth()) { Text(state.yawLockLabel) }
                OutlinedButton(onClick = { act("gimbal.center") }, modifier = Modifier.fillMaxWidth()) { Text("Center") }
                OutlinedButton(onClick = { act("gimbal.tilt90") }, modifier = Modifier.fillMaxWidth()) { Text("Tilt 90") }
                OutlinedButton(onClick = { act("gimbal.pointHome") }, modifier = Modifier.fillMaxWidth()) { Text("Point Home") }
                if (state.retractOffered) {
                    OutlinedButton(onClick = { act("gimbal.retract") }, modifier = Modifier.fillMaxWidth()) { Text("Retract") }
                }
                if (state.controlOffered) {
                    Button(onClick = { act("gimbal.control", !state.haveControl) }, modifier = Modifier.fillMaxWidth()) { Text(state.controlLabel) }
                }
                refusal?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            }
        }
    }
}
