package one.aircast.android.ui

import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcPath
import one.aircast.map.optText
import org.json.JSONObject

internal const val DEEP_LINK_SETUP_PATH = "view.deepLinkSetup"
private const val ACCEPT = "deepLinkSetup.accept"
private const val DECLINE = "deepLinkSetup.decline"

internal data class DeepLinkSetup(val title: String, val text: String, val accept: String, val decline: String)

internal fun deepLinkSetup(view: JSONObject?): DeepLinkSetup? =
    view?.takeIf { it.optBoolean("show") }?.let { DeepLinkSetup(it.optText("title"), it.optText("text"), it.optText("accept"), it.optText("decline")) }

@Composable
fun DeepLinkSetupPrompt() {
    val view by qgcPath(DEEP_LINK_SETUP_PATH)
    val setup = remember(view) { deepLinkSetup(view) } ?: return
    val answer = { path: String -> offMainDetached { Qgc.invoke(path) } }
    AlertDialog(
        onDismissRequest = { answer(DECLINE) },
        title = { Text(setup.title) },
        text = { Text(setup.text) },
        confirmButton = { TextButton(onClick = { answer(ACCEPT) }) { Text(setup.accept) } },
        dismissButton = { TextButton(onClick = { answer(DECLINE) }) { Text(setup.decline) } },
    )
}
