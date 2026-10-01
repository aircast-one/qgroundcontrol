package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Fact
import one.aircast.android.bridge.Qgc

internal const val SET_RC_TO_PARAM = "parameters.setRcToParam"
private const val DEFAULT_RC_SCALE = "1.0"
private val TUNING_IDS = listOf(1, 2, 3)

internal data class RcToParam(val scale: Double, val center: Double, val tuningIndex: Int, val min: Double, val max: Double)

internal fun rcToParam(scale: String, center: String, tuningIndex: Int, min: String, max: String): RcToParam? {
    val numbers = listOf(scale, center, min, max).map { it.trim().toDoubleOrNull() }
    if (numbers.any { it == null }) return null
    val (s, c, lo, hi) = numbers.map { it!! }
    return RcToParam(s, c, tuningIndex, lo, hi)
}

@Composable
internal fun RcToParamDialog(fact: Fact, onDismiss: () -> Unit) {
    var scale by remember { mutableStateOf(DEFAULT_RC_SCALE) }
    var center by remember { mutableStateOf((fact.value as? Number)?.toString() ?: fact.valueString) }
    var min by remember { mutableStateOf(fact.minString) }
    var max by remember { mutableStateOf(fact.maxString) }
    var tuningIndex by remember { mutableIntStateOf(0) }
    var choosing by remember { mutableStateOf(false) }
    var refusal by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    val entered = rcToParam(scale, center, tuningIndex, min, max)
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("RC To Param") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("Bind an RC Channel to a parameter value. Tuning IDs can be mapped to an RC Channel from Radio Setup page.", style = MaterialTheme.typography.bodySmall)
                Text("Parameter  ${fact.name}")
                Box {
                    OutlinedButton(onClick = { choosing = true }) { Text("Tuning ID ${TUNING_IDS[tuningIndex]}") }
                    DropdownMenu(expanded = choosing, onDismissRequest = { choosing = false }) {
                        TUNING_IDS.forEachIndexed { index, id ->
                            DropdownMenuItem(text = { Text("$id") }, onClick = {
                                tuningIndex = index
                                choosing = false
                            })
                        }
                    }
                }
                listOf(Triple("Scale", scale) { v: String -> scale = v }, Triple("Center Value", center) { v: String -> center = v }, Triple("Min Value", min) { v: String -> min = v }, Triple("Max Value", max) { v: String -> max = v }).forEach { (label, value, onChange) ->
                    OutlinedTextField(value = value, onValueChange = onChange, label = { Text(label) }, singleLine = true, keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal))
                }
                Text("Double check that all values are correct prior to confirming dialog.", style = MaterialTheme.typography.bodySmall)
                refusal?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            }
        },
        confirmButton = {
            TextButton(enabled = entered != null, onClick = {
                val chosen = entered ?: return@TextButton
                scope.launch {
                    refusal = withContext(Dispatchers.Default) { Qgc.refusalOf(SET_RC_TO_PARAM, fact.name, chosen.scale, chosen.center, chosen.tuningIndex, chosen.min, chosen.max) }
                    if (refusal == null) onDismiss()
                }
            }) { Text("OK") }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}
