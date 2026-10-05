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
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.qgcPath
import one.aircast.android.bridge.qgcValue
import one.aircast.android.bridge.settingControl
import one.aircast.map.aircast

private const val SUPPORT_LINKS_VIEW = "view.links"
private val SUPPORT_HOST_SETTING = settingControl("settings.mavlinkSettings.forwardMavlinkAPMSupportHostName")

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun SupportForwardingCell() {
    val links by qgcPath(SUPPORT_LINKS_VIEW)
    if (links?.optBoolean("supportForwarding") != true) return
    val host by qgcValue(SUPPORT_HOST_SETTING)
    var open by remember { mutableStateOf(false) }
    Text("Support", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.aircast.success, modifier = Modifier.clickable { open = true })
    if (open) {
        ModalBottomSheet(onDismissRequest = { open = false }) {
            Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp).padding(bottom = 24.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("Mavlink traffic is being forwarded to a support server", style = MaterialTheme.typography.bodyMedium)
                Text("Server name:  ${host?.toString().orEmpty()}", style = MaterialTheme.typography.bodyMedium)
            }
        }
    }
}
