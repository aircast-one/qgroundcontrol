package one.aircast.android.ui

import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import org.json.JSONObject
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainInOrder
import one.aircast.android.bridge.qgcPath

internal const val ADVANCED_UI_PATH = "view.advancedUi"
internal const val SET_ADVANCED_UI = "advancedUi.set"

internal data class AdvancedUi(val shown: Boolean, val title: String, val confirmation: String)

internal fun advancedUi(view: JSONObject?): AdvancedUi = AdvancedUi(
    shown = view?.optBoolean("shown", true) ?: true,
    title = view?.optString("title").orEmpty(),
    confirmation = view?.optString("confirmation").orEmpty(),
)

@Composable
internal fun advancedUiShown(): Boolean {
    val view by qgcPath(ADVANCED_UI_PATH)
    return advancedUi(view).shown
}

@Composable
internal fun AdvancedModeConfirmation(mode: AdvancedUi, onDismiss: () -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(mode.title) },
        text = { Text(mode.confirmation) },
        confirmButton = {
            TextButton(onClick = {
                onDismiss()
                offMainInOrder { Qgc.invoke(SET_ADVANCED_UI, !mode.shown) }
            }) { Text("Yes") }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("No") } },
    )
}
