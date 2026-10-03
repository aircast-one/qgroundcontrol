package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Fact
import one.aircast.mapspike.optText
import org.json.JSONObject

private const val DISTANCE_SUFFIX = "cameraCalc.distanceToSurface"
private const val DENSITY_SUFFIX = "cameraCalc.imageDensity"
private val OVERLAP_SUFFIXES = setOf("cameraCalc.frontalOverlap", "cameraCalc.sideOverlap")
private val SPACING_SUFFIXES = setOf("cameraCalc.adjustedFootprintFrontal", "cameraCalc.adjustedFootprintSide")

internal data class CameraCalcBlock(
    val brand: String,
    val model: String,
    val brands: List<String>,
    val models: List<String>,
    val manual: Boolean,
    val custom: Boolean,
    val valueSetIsDistance: Boolean,
    val valueSetIsDistancePath: String,
    val brandPath: String,
    val modelPath: String,
    val facts: List<Pair<String, Fact>>,
    val distanceMode: Int? = null,
    val distanceModes: List<Pair<Int, String>> = emptyList(),
    val distanceModePath: String = "",
)

internal fun distanceModeTitle(block: CameraCalcBlock): String? =
    block.distanceModes.firstOrNull { it.first == block.distanceMode }?.second

private fun strings(block: JSONObject, key: String): List<String> =
    block.optJSONArray(key)?.let { list -> (0 until list.length()).map { list.optString(it) } } ?: emptyList()

internal fun cameraCalc(view: JSONObject?): CameraCalcBlock? =
    view?.optJSONObject("camera")?.let { block ->
        val facts = block.optJSONArray("facts")
        val brand = block.optText("brand")
        CameraCalcBlock(
            brand = brand,
            model = block.optText("model"),
            brands = strings(block, "brands"),
            models = strings(block, "models"),
            manual = brand == block.optText("manualName"),
            custom = block.optBoolean("custom"),
            valueSetIsDistance = block.optBoolean("valueSetIsDistance", true),
            valueSetIsDistancePath = block.optText("valueSetIsDistancePath"),
            brandPath = block.optText("brandPath"),
            modelPath = block.optText("modelPath"),
            distanceMode = if (block.isNull("distanceMode")) null else block.optInt("distanceMode"),
            distanceModes = block.optJSONArray("distanceModes")?.let { list ->
                (0 until list.length()).mapNotNull { list.optJSONObject(it) }.map { it.optInt("raw") to it.optText("title") }
            }.orEmpty(),
            distanceModePath = block.optText("distanceModePath"),
            facts = (0 until (facts?.length() ?: 0)).mapNotNull { index ->
                facts?.optJSONObject(index)?.let { control -> factFromControl(control)?.let { control.optText("pathSuffix") to it } }
            },
        )
    }

internal fun shownCameraFacts(block: CameraCalcBlock): List<Fact> =
    block.facts
        .filter { (suffix, _) ->
            when {
                block.manual -> suffix == DISTANCE_SUFFIX || suffix in SPACING_SUFFIXES
                suffix == DISTANCE_SUFFIX -> block.valueSetIsDistance
                suffix == DENSITY_SUFFIX -> !block.valueSetIsDistance
                suffix in OVERLAP_SUFFIXES -> true
                else -> true
            }
        }
        .map { it.second }

@Composable
internal fun CameraCalcHeader(block: CameraCalcBlock, onWrite: (String, Any) -> Unit) {
    Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text("Camera", style = MaterialTheme.typography.titleSmall)
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
            Choice(block.brand, block.brands) { onWrite(block.brandPath, it) }
            if (!block.manual && !block.custom) {
                Choice(block.model.ifEmpty { "Model" }, block.models) { onWrite(block.modelPath, it) }
            }
        }
        distanceModeTitle(block)?.let { current ->
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                Text("Altitude", style = MaterialTheme.typography.bodyMedium)
                Choice(current, block.distanceModes.map { it.second }) { title ->
                    block.distanceModes.firstOrNull { it.second == title }?.let { onWrite(block.distanceModePath, it.first) }
                }
            }
        }
        if (!block.manual) {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                Text("Set by", style = MaterialTheme.typography.bodyMedium)
                FilterChip(selected = block.valueSetIsDistance, onClick = { onWrite(block.valueSetIsDistancePath, true) }, label = { Text("Altitude") })
                FilterChip(selected = !block.valueSetIsDistance, onClick = { onWrite(block.valueSetIsDistancePath, false) }, label = { Text("Ground res") })
            }
        }
    }
}

@Composable
private fun Choice(label: String, options: List<String>, onPick: (String) -> Unit) {
    var open by remember { mutableStateOf(false) }
    Box {
        OutlinedButton(onClick = { open = true }, enabled = options.isNotEmpty()) { Text(label) }
        DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
            options.forEach { option ->
                DropdownMenuItem(text = { Text(option) }, onClick = {
                    open = false
                    onPick(option)
                })
            }
        }
    }
}
