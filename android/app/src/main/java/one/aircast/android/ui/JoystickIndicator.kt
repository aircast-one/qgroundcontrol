package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.qgcPath
import one.aircast.mapspike.aircast
import one.aircast.mapspike.optText
import org.json.JSONObject

internal data class JoystickDetail(val label: String, val value: String, val warn: Boolean)

internal data class JoystickBadge(val heading: String, val enabledText: String, val warn: Boolean, val typeText: String, val inputsText: String, val details: List<JoystickDetail> = emptyList())

internal fun joystickBadge(view: JSONObject?): JoystickBadge? =
    view?.optJSONObject("indicator")?.let {
        val rows = it.optJSONArray("details")
        JoystickBadge(
            it.optText("heading"), it.optText("enabledText"), it.optBoolean("warn"), it.optText("typeText"), it.optText("inputsText"),
            (0 until (rows?.length() ?: 0)).mapNotNull { i -> rows?.optJSONObject(i) }.map { row -> JoystickDetail(row.optText("label"), row.optText("value"), row.optBoolean("warn")) },
        )
    }

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun JoystickIndicatorCell() {
    val view by qgcPath(JOYSTICK_VIEW)
    val badge = remember(view) { joystickBadge(view) } ?: return
    var open by remember { mutableStateOf(false) }
    val tint = if (badge.warn) MaterialTheme.aircast.warning else Color.Unspecified

    Text("Joystick", style = MaterialTheme.typography.labelMedium, color = tint, maxLines = 1, modifier = Modifier.clickable { open = true })

    if (open) {
        ModalBottomSheet(onDismissRequest = { open = false }) {
            Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp).padding(bottom = 24.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(badge.heading, style = MaterialTheme.typography.titleMedium)
                Text("Enabled:  ${badge.enabledText}", style = MaterialTheme.typography.bodyMedium, color = tint)
                Text("Type:  ${badge.typeText}", style = MaterialTheme.typography.bodyMedium)
                Text("Inputs:  ${badge.inputsText}", style = MaterialTheme.typography.bodyMedium)
                badge.details.forEach { detail ->
                    Text("${detail.label}  ${detail.value}", style = MaterialTheme.typography.bodyMedium, color = if (detail.warn) MaterialTheme.colorScheme.error else Color.Unspecified)
                }
            }
        }
    }
}
