package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Checkbox
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import one.aircast.map.FlightMapPosition
import one.aircast.map.TrackPoint
import one.aircast.map.optText
import org.json.JSONObject
import java.util.Locale

internal data class FastCompass(
    val invocation: String,
    val help: String,
    val vehicleHasPosition: Boolean,
    val gcsLatitude: Double?,
    val gcsLongitude: Double?,
)

internal data class FastCompassChoice(
    val enabled: Boolean,
    val useGcs: Boolean,
    val latitude: String,
    val longitude: String,
    val useMap: Boolean = false,
    val mapPosition: TrackPoint? = null,
)

internal fun fastCompass(json: JSONObject?): FastCompass? = json?.let {
    val gcs = it.optJSONObject("gcsPosition")
    val valid = gcs?.optBoolean("valid") == true
    FastCompass(
        invocation = it.optText("invocation"),
        help = it.optText("help"),
        vehicleHasPosition = it.optBoolean("vehicleHasPosition"),
        gcsLatitude = gcs?.takeIf { valid }?.optDouble("latitude"),
        gcsLongitude = gcs?.takeIf { valid }?.optDouble("longitude"),
    )
}

internal fun initialFastCompassChoice(fast: FastCompass, mapPosition: TrackPoint? = FlightMapPosition.latest): FastCompassChoice =
    FastCompassChoice(enabled = false, useGcs = fast.gcsLatitude != null, latitude = "0.00", longitude = "0.00", mapPosition = mapPosition)

internal fun fastCompassArguments(fast: FastCompass, choice: FastCompassChoice): List<Any> =
    if (choice.useGcs && fast.gcsLatitude != null && fast.gcsLongitude != null) {
        listOf(fast.gcsLatitude, fast.gcsLongitude)
    } else if (choice.useMap && choice.mapPosition != null) {
        listOf(choice.mapPosition.latitude, choice.mapPosition.longitude)
    } else {
        listOf(choice.latitude.toDoubleOrNull() ?: choice.latitude, choice.longitude.toDoubleOrNull() ?: choice.longitude)
    }

@Composable
internal fun FastCompassBlock(fast: FastCompass, choice: FastCompassChoice, onChange: (FastCompassChoice) -> Unit) {
    val asksForPosition = choice.enabled && !fast.vehicleHasPosition
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(fast.help, style = MaterialTheme.typography.bodySmall)
        LabeledCheckbox("Fast Calibration", choice.enabled) { onChange(choice.copy(enabled = it)) }
        if (asksForPosition) Text("Vehicle has no valid position, please provide it", style = MaterialTheme.typography.bodySmall)
        if (asksForPosition && fast.gcsLatitude != null) LabeledCheckbox("Use GCS position instead", choice.useGcs) { onChange(choice.copy(useGcs = it)) }
        if (asksForPosition && fast.gcsLatitude == null && choice.mapPosition != null) LabeledCheckbox("Use current map position instead", choice.useMap) { onChange(choice.copy(useMap = it)) }
        val shownMap = choice.mapPosition?.takeIf { choice.useMap }
        if (shownMap != null) Text(String.format(Locale.US, "Lat: %.4f Lon: %.4f", shownMap.latitude, shownMap.longitude), style = MaterialTheme.typography.bodySmall)
        if (choice.enabled && !choice.useGcs && shownMap == null) {
            CoordinateField("Latitude", choice.latitude) { onChange(choice.copy(latitude = it)) }
            CoordinateField("Longitude", choice.longitude) { onChange(choice.copy(longitude = it)) }
        }
    }
}

@Composable
private fun LabeledCheckbox(label: String, checked: Boolean, onCheck: (Boolean) -> Unit) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        Checkbox(checked = checked, onCheckedChange = onCheck)
        Text(label)
    }
}

@Composable
private fun CoordinateField(label: String, value: String, onValue: (String) -> Unit) {
    OutlinedTextField(
        value = value,
        onValueChange = onValue,
        label = { Text(label) },
        singleLine = true,
        isError = value.toDoubleOrNull() == null,
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
        modifier = Modifier.fillMaxWidth(),
    )
}
