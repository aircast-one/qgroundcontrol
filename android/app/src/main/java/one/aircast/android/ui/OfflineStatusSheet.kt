package one.aircast.android.ui

import androidx.compose.foundation.clickable
import one.aircast.android.bridge.LinkCommands
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import one.aircast.map.AircastSheet
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import org.json.JSONObject

internal const val OFFLINE_STATUS_VIEW = "view.offlineStatus"
private const val OFFLINE_LINKS_PATH = "links.linkConfigurations"
private const val CONNECTIONS_SETTINGS_PAGE = "Connections"
private const val OFFLINE_POLL_MS = 1000L

internal data class OfflineLink(val index: Int, val text: String, val description: String, val retry: Boolean, val failed: Boolean, val connected: Boolean)

internal data class OfflineStatus(
    val title: String,
    val footnote: String,
    val busy: Boolean,
    val noLinks: Boolean,
    val editAddress: Boolean,
    val links: List<OfflineLink>,
)

internal fun offlineStatus(view: JSONObject?): OfflineStatus? =
    view?.takeIf { it.has("title") }?.let { json ->
        val links = json.optJSONArray("links")
        OfflineStatus(
            title = json.optString("title"),
            footnote = json.optString("footnote"),
            busy = json.optBoolean("busy"),
            noLinks = json.optBoolean("noLinks"),
            editAddress = json.optBoolean("editAddress"),
            links = (0 until (links?.length() ?: 0)).mapNotNull { links?.optJSONObject(it) }.map {
                OfflineLink(it.optInt("index"), it.optString("text"), it.optString("description"), it.optBoolean("retry"), it.optBoolean("failed"), it.optBoolean("connected"))
            },
        )
    }

private fun openConnectionSettings(navigation: AppNavigationState, onDismiss: () -> Unit) {
    navigation.settingsPage = CONNECTIONS_SETTINGS_PAGE
    onDismiss()
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun OfflineStatusSheet(onDismiss: () -> Unit) {
    val navigation = LocalAppNavigation.current
    var status by remember { mutableStateOf<OfflineStatus?>(null) }
    LaunchedEffect(Unit) {
        while (isActive) {
            status = withContext(Dispatchers.Default) { offlineStatus(Qgc.get(OFFLINE_STATUS_VIEW)) }
            delay(OFFLINE_POLL_MS)
        }
    }
    AircastSheet(onDismissRequest = onDismiss) {
        val shown = status ?: return@AircastSheet
        Column(Modifier.fillMaxWidth().padding(bottom = 24.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Text(shown.title, style = MaterialTheme.typography.titleMedium, modifier = Modifier.padding(horizontal = 20.dp))
            Row(
                Modifier.padding(horizontal = 20.dp).let { if (shown.editAddress) it.clickable { openConnectionSettings(navigation, onDismiss) } else it },
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                if (shown.busy) CircularProgressIndicator(Modifier.size(16.dp), strokeWidth = 2.dp)
                Text(shown.footnote, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            if (shown.noLinks) {
                Button(onClick = { openConnectionSettings(navigation, onDismiss) }, modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 8.dp)) { Text("Add link\u2026") }
            }
            shown.links.forEach { link ->
                ListItem(
                    headlineContent = { Text(link.text) },
                    supportingContent = { Text(link.description, color = if (link.failed) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurfaceVariant) },
                    trailingContent = {
                        when {
                            link.connected -> CircularProgressIndicator(Modifier.size(16.dp), strokeWidth = 2.dp)
                            link.retry -> Text("Retry", style = MaterialTheme.typography.labelMedium)
                        }
                    },
                    modifier = Modifier.clickable {
                        offMainDetached {
                            if (link.connected) Qgc.invoke("$OFFLINE_LINKS_PATH.${link.index}.link.disconnect")
                            else LinkCommands.connect("@$OFFLINE_LINKS_PATH.${link.index}")
                        }
                    },
                )
            }
            HorizontalDivider()
            ListItem(
                headlineContent = { Text("Connection settings") },
                modifier = Modifier.clickable { openConnectionSettings(navigation, onDismiss) },
            )
        }
    }
}
