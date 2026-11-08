package one.aircast.android.ui

import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import one.aircast.android.bridge.AppCommands
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcPath
import one.aircast.map.optText
import org.json.JSONObject

internal const val BETA_DIAGNOSTICS_PATH = "view.betaDiagnostics"

internal data class BetaDiagnostics(
    val title: String,
    val text: String,
    val accept: String,
    val dismiss: String,
    val endpoint: String,
)

internal fun betaDiagnostics(view: JSONObject?): BetaDiagnostics? =
    view?.takeIf { it.optBoolean("show") }?.let {
        BetaDiagnostics(
            title = it.optText("title"),
            text = it.optText("text"),
            accept = it.optText("accept"),
            dismiss = it.optText("dismiss"),
            endpoint = it.optText("endpoint"),
        )
    }

@Composable
fun BetaDiagnosticsDialog() {
    val view by qgcPath(BETA_DIAGNOSTICS_PATH)
    val prompt = remember(view) { betaDiagnostics(view) } ?: return
    val accept = { offMainDetached { AppCommands.acceptBetaDiagnostics() } }
    val dismiss = { offMainDetached { AppCommands.dismissBetaDiagnostics() } }

    AlertDialog(
        onDismissRequest = dismiss,
        title = { Text(prompt.title) },
        text = { Text(prompt.text) },
        confirmButton = { Button(onClick = accept) { Text(prompt.accept) } },
        dismissButton = { TextButton(onClick = dismiss) { Text(prompt.dismiss) } },
    )
}
