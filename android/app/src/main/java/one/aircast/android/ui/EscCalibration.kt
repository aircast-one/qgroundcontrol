package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.map.optText
import org.json.JSONObject

internal const val DIALOG_CONTROL = "dialog"
private const val ESC_CAL_VIEW = "view.escCalibration"
private const val ESC_CAL_POLL_MS = 500L

internal data class EscCalibrationState(val highlight: String, val text: String, val running: Boolean = false)

internal fun escCalibration(view: JSONObject?): EscCalibrationState? = view?.takeIf { it.optBoolean("open") }?.let {
    EscCalibrationState(it.optText("highlight"), it.optText("text"), it.optBoolean("running"))
}

@Composable
internal fun EscCalibrationDialog(onClose: () -> Unit) {
    var state by remember { mutableStateOf<EscCalibrationState?>(null) }
    LaunchedEffect(Unit) {
        withContext(Dispatchers.Default) { Qgc.invoke("escCalibration.start") }
        while (isActive) {
            state = withContext(Dispatchers.Default) { escCalibration(Qgc.get(ESC_CAL_VIEW)) }
            delay(ESC_CAL_POLL_MS)
        }
    }
    val error = MaterialTheme.colorScheme.error
    AlertDialog(
        onDismissRequest = {},
        title = { Text("ESC Calibration") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                val shown = state
                Text(
                    buildAnnotatedString {
                        withStyle(SpanStyle(fontWeight = FontWeight.Bold, color = error)) { append(shown?.highlight.orEmpty()) }
                        append(shown?.text ?: "Starting ESC calibration...")
                    },
                )
            }
        },
        confirmButton = {
            TextButton(enabled = state?.running == false, onClick = {
                offMainDetached { Qgc.invoke("escCalibration.close") }
                onClose()
            }) { Text("OK") }
        },
    )
}
