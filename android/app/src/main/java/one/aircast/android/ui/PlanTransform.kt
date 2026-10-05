package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Checkbox
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcPath
import one.aircast.map.TrackPoint
import org.json.JSONObject

internal const val PLAN_TRANSFORM_VIEW = "view.planTransform"
internal const val OFFSET_MISSION = "plan.missionController.offsetMission"
internal const val REPOSITION_MISSION = "plan.missionController.repositionMission"
internal const val ROTATE_MISSION = "plan.missionController.rotateMission"

internal fun transformHome(view: JSONObject?): TrackPoint? =
    view?.optJSONObject("home")?.let { TrackPoint(it.optDouble("latitude"), it.optDouble("longitude")) }

internal data class DistanceUnit(val name: String, val metresPerUnit: Double)

private val METRES = DistanceUnit("m", 1.0)

internal fun transformUnit(view: JSONObject?, axis: String): DistanceUnit =
    view?.let { DistanceUnit(it.optString("${axis}Unit").ifBlank { "m" }, it.optDouble("${axis}MetresPerUnit")) }
        ?.takeIf { it.metresPerUnit.isFinite() && it.metresPerUnit > 0 } ?: METRES

internal fun offsetArgs(east: String, north: String, up: String, horizontal: DistanceUnit, vertical: DistanceUnit, takeoff: Boolean, landing: Boolean): List<Any>? {
    val values = listOf(east to horizontal, north to horizontal, up to vertical).map { (typed, unit) -> one.aircast.map.typedNumber(typed.ifBlank { "0" })?.times(unit.metresPerUnit) }
    return values.takeIf { list -> list.all { it != null && it.isFinite() } }?.let { list -> list.map { it!! } + listOf(takeoff, landing) }
}

@Composable
private fun NumberField(label: String, value: String, onChange: (String) -> Unit) {
    OutlinedTextField(
        value = value,
        onValueChange = onChange,
        label = { Text(label) },
        singleLine = true,
        isError = value.isNotBlank() && one.aircast.map.typedNumber(value) == null,
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Text),
        modifier = Modifier.fillMaxWidth(),
    )
}

@Composable
private fun CheckRow(label: String, checked: Boolean, onChange: (Boolean) -> Unit) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        Checkbox(checked = checked, onCheckedChange = onChange)
        Text(label)
    }
}

@Composable
fun PlanTransformDialog(onDismiss: () -> Unit) {
    val view by qgcPath(PLAN_TRANSFORM_VIEW)
    val home = transformHome(view)
    val horizontal = transformUnit(view, "horizontal")
    val vertical = transformUnit(view, "vertical")
    val scope = rememberCoroutineScope()
    var east by remember { mutableStateOf("0") }
    var north by remember { mutableStateOf("0") }
    var up by remember { mutableStateOf("0") }
    var offsetTakeoff by remember { mutableStateOf(false) }
    var offsetLanding by remember { mutableStateOf(false) }
    var degrees by remember { mutableStateOf("0") }
    var rotateTakeoff by remember { mutableStateOf(false) }
    var rotateLanding by remember { mutableStateOf(false) }
    var repositioning by remember { mutableStateOf(false) }
    var refusal by remember { mutableStateOf<String?>(null) }
    val note = MaterialTheme.typography.bodySmall

    fun apply(path: String, args: List<Any>) {
        scope.launch {
            refusal = withContext(Dispatchers.IO) { Qgc.refusalOf(path, *args.toTypedArray()) }
        }
    }

    if (repositioning && home != null) {
        EditPositionDialog(
            at = home,
            onDismiss = { repositioning = false },
            title = "Reposition mission",
            confirm = "Move to Position",
            vehicleConfirm = "Move to Vehicle Position",
        ) { latitude, longitude ->
            repositioning = false
            apply(REPOSITION_MISSION, listOf(JSONObject().put("latitude", latitude).put("longitude", longitude)))
        }
        return
    }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Transform") },
        text = {
            Column(Modifier.verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                Text("Offset mission", style = MaterialTheme.typography.titleSmall)
                NumberField("East (${horizontal.name})", east) { east = it }
                NumberField("North (${horizontal.name})", north) { north = it }
                NumberField("Up (${vertical.name})", up) { up = it }
                CheckRow("Also move takeoff items", offsetTakeoff) { offsetTakeoff = it }
                CheckRow("Also move landing items", offsetLanding) { offsetLanding = it }
                Text("Note: Home altitude is not modified.", style = note)
                val offset = offsetArgs(east, north, up, horizontal, vertical, offsetTakeoff, offsetLanding)
                OutlinedButton(onClick = { offset?.let { apply(OFFSET_MISSION, it) } }, enabled = offset != null) { Text("Apply offset") }

                HorizontalDivider()
                Text("Reposition mission", style = MaterialTheme.typography.titleSmall)
                if (home == null) Text("Home position must be set to reposition the mission.", style = note)
                OutlinedButton(onClick = { repositioning = true }, enabled = home != null) { Text("Move to Position") }

                HorizontalDivider()
                Text("Rotate mission", style = MaterialTheme.typography.titleSmall)
                if (home == null) Text("Home position must be set to rotate the mission.", style = note)
                NumberField("Clockwise (deg)", degrees) { degrees = it }
                CheckRow("Also move takeoff items", rotateTakeoff) { rotateTakeoff = it }
                CheckRow("Also move landing items", rotateLanding) { rotateLanding = it }
                Text("Note: Complex items are rotated by moving their reference coordinate: their geometry and orientation are not changed.", style = note)
                val rotation = one.aircast.map.typedNumber(degrees)
                OutlinedButton(
                    onClick = { rotation?.let { apply(ROTATE_MISSION, listOf(it, rotateTakeoff, rotateLanding)) } },
                    enabled = home != null && rotation != null,
                ) { Text("Apply rotation") }

                refusal?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            }
        },
        confirmButton = { TextButton(onClick = onDismiss) { Text("Close") } },
    )
}
