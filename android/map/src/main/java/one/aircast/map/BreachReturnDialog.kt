package one.aircast.map

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import java.util.Locale

fun breachAltitudeText(altitude: Double?): String = altitude?.let { String.format(Locale.US, "%.1f", it) }.orEmpty()

@Composable
fun BreachReturnDialog(breach: BreachReturn, onDismiss: () -> Unit, onAltitude: (Double) -> Unit, onRemove: () -> Unit) {
    var typed by remember(breach) { mutableStateOf(breachAltitudeText(breach.altitude)) }
    val value = typedNumber(typed)
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Breach return point") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(
                    value = typed,
                    onValueChange = { typed = it },
                    label = { Text("Altitude") },
                    suffix = { Text(breach.units) },
                    singleLine = true,
                    isError = value == null,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                )
                TextButton(onClick = onRemove) { Text("Remove breach return point", color = MaterialTheme.colorScheme.error) }
            }
        },
        confirmButton = { TextButton(onClick = { value?.let(onAltitude) }, enabled = value != null && breach.altitudePath.isNotEmpty()) { Text("Set") } },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Close") } },
    )
}
