package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.mapspike.optText
import org.json.JSONObject

private const val POWER_CALC_VIEW = "view.powerCalc"
private const val POWER_CALCULATE = "vehicleConfig.calculate"
private const val READING_POLL_MS = 1000L

internal data class PowerCalculator(
    val title: String,
    val help: String,
    val measure: String,
    val measuredLabel: String,
    val readingLabel: String,
    val paramLabel: String,
    val button: String,
    val noReading: String,
    val batteryIndex: Int,
    val param: String,
)

internal fun powerCalculator(json: JSONObject?): PowerCalculator? = json?.takeIf { it.optText("param").isNotBlank() }?.let {
    PowerCalculator(
        title = it.optText("title"),
        help = it.optText("help"),
        measure = it.optText("measure"),
        measuredLabel = it.optText("measuredLabel"),
        readingLabel = it.optText("readingLabel"),
        paramLabel = it.optText("paramLabel"),
        button = it.optText("button"),
        noReading = it.optText("noReading"),
        batteryIndex = it.optInt("batteryIndex"),
        param = it.optText("param"),
    )
}

internal fun powerCalcPath(calculator: PowerCalculator): String =
    "$POWER_CALC_VIEW(${calculator.measure},${calculator.batteryIndex},${calculator.param})"

@Composable
internal fun PowerCalcDialog(calculator: PowerCalculator, onDone: () -> Unit) {
    val scope = rememberCoroutineScope()
    var measured by remember { mutableStateOf("") }
    var live by remember { mutableStateOf<JSONObject?>(null) }
    var refusal by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(calculator) {
        while (isActive) {
            live = withContext(Dispatchers.Default) { Qgc.get(powerCalcPath(calculator)) }
            delay(READING_POLL_MS)
        }
    }

    val available = live?.optBoolean("readingAvailable") == true
    AlertDialog(
        onDismissRequest = onDone,
        title = { Text(calculator.title) },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(calculator.help, style = MaterialTheme.typography.bodySmall)
                if (!available && calculator.noReading.isNotBlank()) {
                    Text(calculator.noReading, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall)
                }
                OutlinedTextField(
                    value = measured,
                    onValueChange = { measured = it },
                    label = { Text(calculator.measuredLabel) },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                )
                if (available) Text("${calculator.readingLabel} ${live?.optText("readingText").orEmpty()}")
                Text("${calculator.paramLabel} ${live?.optText("paramText").orEmpty()}")
                refusal?.let { Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall) }
            }
        },
        confirmButton = {
            TextButton(enabled = available, onClick = {
                scope.launch {
                    refusal = withContext(Dispatchers.Default) {
                        Qgc.refusalOf(POWER_CALCULATE, calculator.param, calculator.measure, calculator.batteryIndex, measured)
                    }
                }
            }) { Text(calculator.button) }
        },
        dismissButton = { TextButton(onClick = onDone) { Text("Close") } },
    )
}
